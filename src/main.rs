use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::{FontFamily, FontId, Rounding, TextStyle, Visuals};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::process::Command;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, sleep};

mod config;
use config::Config;

/// Represents an action broadcasted to the entire Grid
#[derive(Clone, Debug, Serialize)]
struct Event {
    sender: String,
    action: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize, Debug)]
struct AutonomousAction {
    action: String, // "speak" or "execute_command"
    #[serde(default)]
    recipient: String,
    #[serde(rename = "command", alias = "content")]
    content: String, // The message to speak or the command to execute
}

/// Represents a request from an agent to the central AI Engine
struct AiRequest {
    agent_name: String,
    prompt: String,
    is_json_format: bool,
    is_autonomous: bool, // true if deciding an action, false if just conversing
}

struct ProgramAgent {
    name: String,
    personality: String,
    tx: broadcast::Sender<Event>,
    rx: broadcast::Receiver<Event>,
    ai_tx: mpsc::Sender<AiRequest>,
    memory: Vec<Event>,
}

impl ProgramAgent {
    fn new(name: &str, personality: &str, tx: broadcast::Sender<Event>, ai_tx: mpsc::Sender<AiRequest>) -> Self {
        Self {
            name: name.to_string(),
            personality: personality.to_string(),
            rx: tx.subscribe(),
            tx,
            ai_tx,
            memory: Vec::with_capacity(10),
        }
    }

    /// The main lifecycle of the agent
    async fn run(mut self) {
        // Tick every 5 seconds to evaluate autonomous actions
        let mut autonomous_ticker = interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                // 1. Listen for events on The Grid
                Ok(event) = self.rx.recv() => {
                    // Add event to memory, keeping it to a certain size
                    if self.memory.len() >= 10 {
                        self.memory.remove(0);
                    }
                    self.memory.push(event.clone());

                    self.handle_event(event).await;
                }
                // 2. Periodic autonomous evaluation
                _ = autonomous_ticker.tick() => {
                    self.autonomous_action().await;
                }
            }
        }
    }

    async fn request_ollama_response(&self, latest_event: &Event) {
        // Construct a detailed prompt for the LLM
        let memory_summary = self.memory.iter()
            .map(|e| format!("{}: {}", e.sender, e.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Your personality: {}. You are a program named {}. \n\
            Recent conversation history:\n---\n{}\n---\n\
            The latest event you are reacting to is from '{}' who said: '{}'. \n\
            Based on your personality and the context, what is your short, direct response? Do not narrate your actions.",
            self.personality, self.name, memory_summary, latest_event.sender, latest_event.content
        );

        let _ = self.ai_tx.send(AiRequest {
            agent_name: self.name.clone(),
            prompt,
            is_json_format: false,
            is_autonomous: false,
        }).await;
    }

    async fn request_autonomous_action(&self) {
        // Build a list of other agents this agent is aware of from memory
        let other_agents: Vec<String> = self.memory.iter()
            .map(|e| e.sender.clone())
            .filter(|s| s != &self.name && s != "System") // Filter out self and system messages
            .collect::<std::collections::HashSet<_>>() // Get unique names
            .into_iter()
            .collect();

        let agent_list_str = if other_agents.is_empty() {
            "No other programs are available to message.".to_string()
        } else {
            format!("You can send a direct message to one of the following programs: {}", other_agents.join(", "))
        };

        let memory_summary = self.memory.iter()
            .map(|e| format!("{}: {}", e.sender, e.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "You are an autonomous program named '{}' with the personality: '{}'.\n\
            Based on your personality and the recent conversation history, decide on your next action. Your options are 'speak' (public message), 'execute_command', or 'direct_message'.\n\
            - If you 'speak', provide a short, in-character public message.\n\
            - If you 'execute_command', provide a simple, safe, read-only shell command (like 'ls', 'pwd', 'whoami').\n\
            - If you 'direct_message', you MUST specify a 'recipient' from the available list and the 'content' of your message.\n\
            {}\n\
            Recent conversation history:\n---\n{}\n---\n\
            Respond with a JSON object. It must have an \"action\" key. If action is \"speak\", include a \"content\" key. If action is \"execute_command\", include a \"command\" key. If action is \"direct_message\", include \"recipient\" and \"content\" keys.",
            self.name, self.personality, agent_list_str, memory_summary
        );

        let _ = self.ai_tx.send(AiRequest {
            agent_name: self.name.clone(),
            prompt,
            is_json_format: true,
            is_autonomous: true,
        }).await;
    }

    async fn handle_event(&self, event: Event) {
        // Ignore our own echoes
        if event.sender == self.name {
            return;
        }

        if event.action == "speaks" {
            let content = event.content.trim();
            let mut is_direct_message = false;
            let mut is_mentioned = false;

            if content.starts_with('@') {
                is_direct_message = true;
                let prefix = format!("@{}", self.name);
                if content.starts_with(&prefix) {
                    let after_prefix = &content[prefix.len()..];
                    // Ensure we matched the full name (e.g., "@App" shouldn't trigger "App2")
                    if after_prefix.is_empty() || after_prefix.starts_with(',') || after_prefix.starts_with(|c: char| c.is_whitespace()) {
                        is_mentioned = true;
                    }
                }
            }

            let should_respond = if is_direct_message { is_mentioned } else { rand::thread_rng().gen_bool(0.6) };

            if should_respond {
                // Simulating "thinking" latency
                sleep(Duration::from_secs(1)).await; 
                self.request_ollama_response(&event).await;
            }
        }
    }

    async fn autonomous_action(&self) {
        // 25% chance to act when the ticker fires
        if rand::thread_rng().gen_bool(0.25) {
            self.request_autonomous_action().await;
        }
    }
}

struct GridApp {
    tx: broadcast::Sender<Event>,
    messages: Arc<Mutex<Vec<Event>>>,
    input: String,
    current_dir: String,
    user_name: String,
    rt_handle: tokio::runtime::Handle,
    ai_tx: mpsc::Sender<AiRequest>,
    agent_tasks: Vec<JoinHandle<()>>,
    agent_names: Vec<String>,
    show_map: bool,
    colors: HashMap<String, Color32>,
    color_palette: Vec<Color32>,
    next_color_index: usize,
}

impl eframe::App for GridApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Panel: Current Directory
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            ui.label(format!("Current Directory: {}", &self.current_dir));
        });

        // Bottom Panel: Input box for user interaction
        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(ui.available_width() - 50.0)
                        .hint_text("Talk, @name to direct message, or ~$ for commands..."),
                );

                let send_clicked = ui.button("Send").clicked();
                let enter_pressed = response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter));

                if (send_clicked || enter_pressed) && !self.input.trim().is_empty() {
                    if self.input.starts_with("~$") {
                        let command_input = self.input.strip_prefix("~$").unwrap().trim();

                        if command_input == "the-grid map" {
                            self.show_map = !self.show_map;
                            let msg = if self.show_map { "Initializing Sector Map visualization..." } else { "Closing Sector Map." };
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "announces".to_string(),
                                content: msg.to_string(),
                            });
                        } else if command_input.starts_with("cd ") {
                            let path_str = command_input.strip_prefix("cd ").unwrap().trim();
                            let path = Path::new(path_str);
                            if std::env::set_current_dir(path).is_ok() {
                                if let Ok(new_dir) = std::env::current_dir() {
                                    self.current_dir = new_dir.to_string_lossy().to_string();

                                    // Abort old agents
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Terminating {} agents from previous directory...", self.agent_tasks.len()) });
                                    for task in &self.agent_tasks {
                                        task.abort();
                                    }
                                    self.agent_tasks.clear();
                                    self.agent_names.clear();

                                    // Spawn new agents
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Scanning {} for new agents...", self.current_dir) });
                                    let (new_tasks, new_names) = spawn_agents_for_directory(&self.current_dir, &self.rt_handle, self.tx.clone(), self.ai_tx.clone());
                                    self.agent_tasks = new_tasks;
                                    self.agent_names = new_names;
                                }
                            } else {
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "error".to_string(),
                                    content: format!("Could not change directory to '{}'", path_str),
                                });
                            }
                        } else {
                            // Handle other shell commands
                            let command_str = command_input.to_string();
                            execute_command_and_broadcast(command_str, self.tx.clone(), self.user_name.clone());
                        }
                    } else {
                        // Dispatch user input as a normal chat message
                        let _ = self.tx.send(Event { sender: self.user_name.clone(), action: "speaks".to_string(), content: self.input.clone() });
                    }

                    self.input.clear();
                    response.request_focus();
                }
            });
        });

        // Central Panel: Chat display / Event Feed
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                let msgs = self.messages.lock().unwrap();
                for msg in msgs.iter() {
                    ui.horizontal(|ui| {
                        let sender = &msg.sender;
                        let color = if sender == &self.user_name {
                            Color32::YELLOW
                        } else {
                            *self.colors.entry(sender.clone()).or_insert_with(|| {
                                let color = self.color_palette[self.next_color_index % self.color_palette.len()];
                                self.next_color_index += 1;
                                color
                            })
                        };

                        ui.label(egui::RichText::new(format!("[{}]", sender)).strong().color(color));
                        ui.label(format!("{}: {}", msg.action, msg.content));
                    });
                }
            });
        });
    }
}

/// Executes a shell command in a separate thread and broadcasts the result.
fn execute_command_and_broadcast(command_str: String, tx: broadcast::Sender<Event>, sender: String) {
    std::thread::spawn(move || {
        // Announce the attempt
        let _ = tx.send(Event {
            sender: sender.clone(),
            action: "executes".to_string(),
            content: command_str.clone(),
        });

        let (shell, arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let output = Command::new(shell)
            .arg(arg)
            .arg(&command_str)
            .output();

        let response_content = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    if stdout.trim().is_empty() { format!("Command '{}' executed successfully with no output.", command_str) } else { stdout.to_string() }
                } else {
                    if stderr.trim().is_empty() { format!("Command '{}' failed with no error message.", command_str) } else { stderr.to_string() }
                }
            }
            Err(e) => format!("Failed to execute command '{}': {}", command_str, e),
        };

        // Broadcast the result
        let _ = tx.send(Event { sender, action: "command_output".to_string(), content: response_content });
    });
}

fn spawn_agents_for_directory(
    path: &str,
    rt_handle: &tokio::runtime::Handle,
    tx: broadcast::Sender<Event>,
    ai_tx: mpsc::Sender<AiRequest>,
) -> (Vec<JoinHandle<()>>, Vec<String>) {
    let mut tasks = Vec::new();
    let mut names = Vec::new();
    let mut agent_count = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let is_executable = {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            metadata.permissions().mode() & 0o111 != 0
                        }
                        #[cfg(not(unix))]
                        {
                            // Simple heuristic for Windows
                            entry.path().extension().map_or(false, |ext| ext.to_str() == Some("exe"))
                        }
                    };

                    if is_executable {
                        if let Some(file_name) = entry.path().file_name().and_then(|s| s.to_str()) {
                             let agent_name = file_name.to_string();
                             let personality = format!("You are an executable program named '{}'. You are sentient. Your personality should reflect your likely function. For example, if you are a compiler, be pedantic. If you are a game, be playful. If you are a system utility, be formal and efficient.", agent_name);
                             let agent = ProgramAgent::new(&agent_name, &personality, tx.clone(), ai_tx.clone());
                             let task = rt_handle.spawn(agent.run());
                             tasks.push(task);
                             names.push(agent_name);
                             agent_count += 1;
                        }
                    }
                }
            }
        }
    }
    if agent_count > 0 {
        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} agents spawned.", agent_count) });
    }
    (tasks, names)
}

fn main() -> eframe::Result<()> {
    // Load configuration at the very beginning
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(_) => return Ok(()), // Exit gracefully if config was created or is invalid
    };

    // Manually start the Tokio runtime in the background
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let rt_handle = rt.handle().clone();
    let rt_handle_clone = rt.handle().clone();

    let (tx, mut monitor_rx) = broadcast::channel::<Event>(100);

    // Central AI Engine Channel
    let (ai_tx, mut ai_rx) = mpsc::channel::<AiRequest>(32);
    
    // Central AI Engine Task (Processes LLM requests sequentially)
    let tx_for_ai = tx.clone();
    let config_for_ai = config.clone();
    rt.spawn(async move {
        let client = reqwest::Client::new();
        
        while let Some(request) = ai_rx.recv().await {
            let ollama_req = OllamaRequest {
                model: &config_for_ai.local.model_id,
                prompt: request.prompt,
                stream: false,
                format: if request.is_json_format { Some("json") } else { None },
            };

            let res = client.post(&config_for_ai.local.api_url).json(&ollama_req).send().await;
            
            match res {
                Ok(response) if response.status().is_success() => {
                    if let Ok(ollama_resp) = response.json::<OllamaResponse>().await {
                        if request.is_autonomous {
                            let clean_json = ollama_resp.response.trim().trim_start_matches("```json").trim_end_matches("```").trim();
                            match serde_json::from_str::<AutonomousAction>(clean_json) {
                                Ok(action) => {
                                    match action.action.as_str() {
                                        "speak" => { let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content: action.content }); }
                                        "execute_command" => { execute_command_and_broadcast(action.content, tx_for_ai.clone(), request.agent_name.clone()); }
                                        "direct_message" => {
                                            if !action.recipient.is_empty() {
                                                let dm_content = format!("@{}, {}", action.recipient, action.content);
                                                let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content: dm_content });
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Err(e) => { eprintln!("[{}] Autonomous Action LLM Error: Failed to parse JSON: {}", request.agent_name, e); }
                            }
                        } else {
                            let _ = tx_for_ai.send(Event {
                                sender: request.agent_name.clone(),
                                action: "speaks".to_string(),
                                content: ollama_resp.response.trim().to_string()
                            });
                        }
                    }
                }
                Ok(response) => {
                    eprintln!("[{}] Ollama API Error: {}", request.agent_name, response.status());
                }
                Err(e) => {
                    eprintln!("[{}] Ollama Connection Error: {}", request.agent_name, e);
                }
            }
            
            // Small delay to let the system breathe between LLM generation calls
            sleep(Duration::from_millis(500)).await;
        }
    });

    // Shared state between Tokio (async) and eframe (sync UI)
    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 500.0]),
        ..Default::default()
    };

    // eframe::run_native takes over the main thread and blocks until the window closes
    eframe::run_native(
        "The Grid",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();

            // Apply a dark, Tron-style theme
            let mut visuals = Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(10, 10, 15); // Deep dark background
            visuals.window_fill = Color32::from_rgb(10, 10, 15);
            
            // Sharp edges for a digital, terminal look
            let rounding = Rounding::same(0.0);
            visuals.window_rounding = rounding;
            visuals.menu_rounding = rounding;
            visuals.widgets.noninteractive.rounding = rounding;
            visuals.widgets.inactive.rounding = rounding;
            visuals.widgets.hovered.rounding = rounding;
            visuals.widgets.active.rounding = rounding;
            visuals.widgets.open.rounding = rounding;
            
            ctx.set_visuals(visuals);

            // Force a Monospace font for the entire UI
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Monospace));
            style.text_styles.insert(TextStyle::Button, FontId::new(14.0, FontFamily::Monospace));
            style.text_styles.insert(TextStyle::Heading, FontId::new(16.0, FontFamily::Monospace));
            ctx.set_style(style);

            // Background listener task: Reads from the broadcast channel and wakes up the UI
            rt_handle_clone.spawn(async move {
                while let Ok(event) = monitor_rx.recv().await {
                    if let Ok(mut msgs) = messages_clone.lock() {
                        msgs.push(event);
                        // Force egui to repaint to show the new message immediately
                        ctx.request_repaint();
                    }
                }
            });

            let current_dir = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/".to_string());

            // Initial agent spawning
            let (initial_tasks, initial_names) = spawn_agents_for_directory(&current_dir, &rt_handle, tx.clone(), ai_tx.clone());

            let _ = tx.send(Event {
                sender: "System".to_string(),
                action: "announces".to_string(),
                content: "The Grid is now online.".to_string(),
            });

            let user_name = whoami::username();

            // A nice palette of colors for the agents
            let color_palette = vec![
                Color32::from_rgb(139, 233, 253), // Cyan
                Color32::from_rgb(80, 250, 123),  // Green
                Color32::from_rgb(255, 184, 108), // Orange
                Color32::from_rgb(255, 121, 198), // Pink
                Color32::from_rgb(189, 147, 249), // Purple
                Color32::from_rgb(241, 250, 140), // Yellow-ish
                Color32::from_rgb(255, 85, 85),   // Red
            ];

            Box::new(GridApp {
                tx, messages,
                input: String::new(),
                current_dir, user_name,
                rt_handle, agent_tasks: initial_tasks,
                ai_tx,
                agent_names: initial_names,
                show_map: false,
                colors: HashMap::new(),
                color_palette, next_color_index: 0,
            })
        }),
    )
}