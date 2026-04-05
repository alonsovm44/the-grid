use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::{FontId, TextStyle};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::sync::{broadcast, mpsc};
use sysinfo::System;

use crate::ai_provider::{self, run_ai_engine, AiRequest};
use crate::config::Config;
use crate::database::Database;
use crate::event::Event;
use crate::filesystem::SpatialKnowledgeFS;
use crate::gridshell::GridShell;
use crate::{generate_procedural_personality, spawn_agents_for_directory, ProgramAgent};

#[derive(PartialEq, Clone)]
pub enum DigitizationState {
    Booting(usize),    // Initial startup sequence
    Idle,
    AwaitingConfirmation,
    Digitizing(usize), // Progress counter
    GridActive,
}

pub struct GridApp {
    pub tx: broadcast::Sender<Event>,
    pub messages: Arc<Mutex<Vec<Event>>>,
    pub input: String,
    pub current_dir: String,
    pub user_name: String,
    pub rt_handle: tokio::runtime::Handle,
    pub shared_config: Arc<Mutex<Config>>,
    pub ai_tx: mpsc::Sender<AiRequest>,
    pub ai_task: JoinHandle<()>,
    pub agent_tasks: Vec<JoinHandle<()>>,
    pub typing_agents: Arc<Mutex<HashSet<String>>>,
    pub agent_names: Vec<String>,
    pub db: Option<Arc<Mutex<Database>>>,
    pub show_map: bool,
    pub colors: HashMap<String, Color32>,
    pub color_palette: Vec<Color32>,
    pub next_color_index: usize,
    pub show_emojis: bool,
    pub emoji_palette: Vec<String>,
    pub show_thoughts: bool,
    pub show_feels: bool,
    pub invoked_tools: HashSet<String>,
    pub rel_cache: HashMap<String, HashMap<String, i32>>,
    pub last_rel_update: Instant,
    pub last_active_agent: Option<(String, Instant)>,
    pub map_user_pos: egui::Pos2,
    pub file_positions: HashMap<String, egui::Pos2>,
    pub agent_3d_positions: HashMap<String, [f32; 3]>,
    pub digitization_state: DigitizationState,
    pub digitization_log: Vec<String>,
    pub last_log_tick: Instant,
    pub camera_angle: f32,
    pub gridshell: GridShell,
    pub skfs: Arc<Mutex<SpatialKnowledgeFS>>,
}

impl eframe::App for GridApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Initial Boot Sequence
        if let DigitizationState::Booting(progress) = self.digitization_state {
            if self.last_log_tick.elapsed() > Duration::from_millis(150) {
                let boot_logs = [
                    "GRID_OS v0.1.0 (AURORA_CORE)",
                    "PROTOTYPE BUILD: 2026-04-04",
                    "----------------------------------",
                    "Initializing memory registers... OK",
                    "Loading kernel modules... OK",
                    "Mounting /dev/vfs... OK",
                    "Scanning sector 0-1 for lifeforms...",
                    "Establishing neural bridge...",
                    "Checking persistence layer...",
                    "Bypassing security protocols...",
                    "READY.",
                ];

                if progress < boot_logs.len() {
                    self.digitization_log.push(boot_logs[progress].to_string());
                    self.digitization_state = DigitizationState::Booting(progress + 1);
                    self.last_log_tick = Instant::now();
                } else if self.last_log_tick.elapsed() > Duration::from_secs(1) {
                    self.digitization_state = DigitizationState::Idle;
                    self.digitization_log.clear(); // Clear logs to reuse buffer for digitization
                }
                ctx.request_repaint();
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_terminal_logs(ui);
            });

            // Block rest of UI while booting
            return;
        }

        // Handle Digitization Sequence Animation
        if let DigitizationState::Digitizing(progress) = self.digitization_state {
            if self.last_log_tick.elapsed() > Duration::from_millis(30) {
                let logs = [
                    "SECURE_PROTOCOL: Initiating high-energy laser handshake...",
                    "CRITICAL: Molecular scanner alignment at 98.4%",
                    "WARNING: Identity fragmentation risk detected...",
                    "SUCCESS: Buffer overflow suppressed.",
                    "TRACE: Mapping neural synaptic pathways to bitstream...",
                    "DEBUG: Allocating voxel sectors in Grid 0-1...",
                    "INFO: Compiling user DNA into executable bytecode...",
                    "GRID_OS: Authorizing digitization sequence...",
                    "VOICE_SYNTH: 'Welcome to the Grid, User.'",
                ];
                self.digitization_log.push(logs[progress % logs.len()].to_string());
                self.digitization_state = if progress > 60 { DigitizationState::GridActive } else { DigitizationState::Digitizing(progress + 1) };
                self.last_log_tick = Instant::now();
                ctx.request_repaint();
            }
        }

        // Check for new messages to update the active agent for visual pulse
        {
            let msgs = self.messages.lock().unwrap();
            if let Some(last_msg) = msgs.last() {
                // Intercept movement events to update the UI's 3D state
                if last_msg.action == "moves_to" {
                    let clean = last_msg.content.trim_matches(|c| c == '[' || c == ']' || c == ' ');
                    let coords: Vec<f32> = clean.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                    if coords.len() == 3 {
                        self.agent_3d_positions.insert(last_msg.sender.clone(), [coords[0], coords[1], coords[2]]);
                    }
                }

                if self.last_active_agent.as_ref().map_or(true, |(name, _)| name != &last_msg.sender) {
                    self.last_active_agent = Some((last_msg.sender.clone(), Instant::now()));
                }
            }
        }

        // Top Panel: Current Directory
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            ui.label(format!("Current Directory: {}", &self.current_dir));
        });

        // Bottom Panel: Input box for user interaction
        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            // Display typing indicators
            {
                let typing_guard = self.typing_agents.lock().unwrap();
                if !typing_guard.is_empty() {
                    let names: Vec<String> = typing_guard
                        .iter()
                        .map(|name| self.get_agent_display_name(name))
                        .collect();
                    let text = if names.len() == 1 {
                        format!("{} is typing...", names[0])
                    } else if names.len() == 2 {
                        format!("{} and {} are typing...", names[0], names[1])
                    } else {
                        // Handles 3+ case gracefully
                        format!("{} and {} others are typing...", names[0], names.len() - 1)
                    };
                    ui.label(egui::RichText::new(text).italics().color(Color32::GRAY));
                }
            }

            ui.horizontal(|ui| {
                let hint = match self.digitization_state {
                    DigitizationState::AwaitingConfirmation => "Aperture clear, proceed (y/n)?",
                    _ => "Talk, @name to direct message, or ~$ for commands...",
                };

                let response = ui.add(egui::TextEdit::singleline(&mut self.input)
                    .desired_width(ui.available_width() - 50.0)
                    .hint_text(hint));

                let send_clicked = ui.button("Send").clicked();
                let enter_pressed = response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter));

                if (send_clicked || enter_pressed) && !self.input.trim().is_empty() {
                    // Handle interactive prompt for digitization
                    if self.digitization_state == DigitizationState::AwaitingConfirmation {
                        if self.input.trim().to_lowercase() == "y" {
                            self.digitization_state = DigitizationState::Digitizing(0);
                            self.last_log_tick = Instant::now();
                        } else {
                            self.digitization_state = DigitizationState::Idle;
                        }
                        self.input.clear();
                        return;
                    }

                    if self.input.starts_with("~$") {
                        let command_input = self.input.strip_prefix("~$").unwrap().trim();
                        
                        // Pre-calculate grid_args for use across scopes
                        let grid_args = if command_input.starts_with("the-grid ") {
                            Some(command_input.strip_prefix("the-grid ").unwrap().trim())
                        } else if command_input.starts_with("grid ") {
                            Some(command_input.strip_prefix("grid ").unwrap().trim())
                        } else if command_input == "the-grid" || command_input == "grid" {
                            Some("help")
                        } else {
                            None
                        };
                        
                        // Try GridShell first for semantic commands
                        match self.gridshell.execute(command_input) {
                            Ok(result) => {
                                let _ = self.tx.send(Event { 
                                    sender: "GridShell".to_string(), 
                                    action: "executes".to_string(), 
                                    content: result 
                                });
                            },
                            Err(_) => {
                                if let Some(args) = grid_args {
                                    if let Some(db_handle) = &self.db {
                                        let db = db_handle.lock().unwrap();
                                        let mut output = String::from("Relational Database Graph:\n");
                                        let mut found_any = false;
                                        for name in &self.agent_names {
                                            if let Ok(rels) = db.get_relationships(name) {
                                                if !rels.is_empty() {
                                                    found_any = true;
                                                    output.push_str(&format!("{}: ", name));
                                                    let rel_strs: Vec<String> = rels.iter()
                                                        .map(|(target, affinity)| format!("{} ({})", target, affinity))
                                                        .collect();
                                                    output.push_str(&rel_strs.join(", "));
                                                    output.push('\n');
                                                }
                                            }
                                        }
                                        if !found_any {
                                            output.push_str("No relationships have been formed yet.");
                                        }
                                        let _ = self.tx.send(Event {
                                            sender: "System".to_string(),
                                            action: "announces".to_string(),
                                            content: output.trim_end().to_string(),
                                        });
                                    } else {
                                        let _ = self.tx.send(Event {
                                            sender: "System".to_string(),
                                            action: "error".to_string(),
                                            content: "Database not initialized. Run ~$ grid init first.".to_string(),
                                        });
                                    }
                                }
                            }
                        };

                        if let Some(args) = grid_args {
                            if args == "relations" {
                                if let Some(db_handle) = &self.db {
                                    let db = db_handle.lock().unwrap();
                                    let mut output = String::from("Relational Database Graph:\n");
                                    let mut found_any = false;
                                    for name in &self.agent_names {
                                        if let Ok(rels) = db.get_relationships(name) {
                                            if !rels.is_empty() {
                                                found_any = true;
                                                output.push_str(&format!("{}: ", name));
                                                let rel_strs: Vec<String> = rels.iter()
                                                    .map(|(target, affinity)| format!("{} ({})", target, affinity))
                                                    .collect();
                                                output.push_str(&rel_strs.join(", "));
                                                output.push('\n');
                                            }
                                        }
                                    }
                                    if !found_any {
                                        output.push_str("No relationships have been formed yet.");
                                    }
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "announces".to_string(),
                                        content: output.trim_end().to_string(),
                                    });
                                } else {
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "error".to_string(),
                                        content: "Database not initialized. Run ~$ grid init first.".to_string(),
                                    });
                                }
                            } else if args == "init" {
                            if self.db.is_some() {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: "Database already initialized.".to_string() });
                            } else {
                                match Database::new() {
                                    Ok(db_conn) => {
                                        self.db = Some(Arc::new(Mutex::new(db_conn)));
                                        let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: "Database initialized. Agents will now have persistent memory.".to_string() });
                                        
                                        // Respawn agents to use the new database
                                        let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: "Re-spawning agents with persistence enabled...".to_string() });
                                        
                                        // Abort AI task to clear pending requests
                                        self.ai_task.abort();
                                        let (new_ai_tx, new_ai_rx) = mpsc::channel::<AiRequest>(32);
                                        self.ai_tx = new_ai_tx.clone();
                                        self.ai_task = self.rt_handle.spawn(run_ai_engine(new_ai_rx, self.tx.clone(), self.shared_config.clone()));

                                        for task in &self.agent_tasks {
                                            task.abort();
                                        }
                                        self.agent_tasks.clear();
                                        self.agent_names.clear();
                                        self.typing_agents.lock().unwrap().clear();
                                        let (new_tasks, new_names) = spawn_agents_for_directory(&self.current_dir, &self.rt_handle, self.tx.clone(), self.ai_tx.clone(), self.db.clone());
                                        self.agent_tasks = new_tasks;
                                        self.agent_names = new_names;
                                    }
                                    Err(e) => {
                                        let _ = self.tx.send(Event {
                                            sender: "System".to_string(),
                                            action: "error".to_string(),
                                            content: format!("Failed to initialize database: {}", e),
                                        });
                                    }
                                }
                            }
                        } else if args == "map" {
                            self.show_map = !self.show_map;
                            let msg = if self.show_map { "Initializing Sector Map visualization..." } else { "Closing Sector Map." };
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "announces".to_string(),
                                content: msg.to_string(),
                            });
                        } else if args == "toggle emojis" {
                            self.show_emojis = !self.show_emojis;
                            let msg = if self.show_emojis { "Emoji display enabled." } else { "Emoji display disabled." };
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "announces".to_string(),
                                content: msg.to_string(),
                            });
                        } else if args == "toggle thoughts" {
                            self.show_thoughts = !self.show_thoughts;
                            let msg = if self.show_thoughts { "Agent thoughts are now visible." } else { "Agent thoughts are now hidden." };
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "announces".to_string(),
                                content: msg.to_string(),
                            });
                        } else if args == "toggle feels" {
                            self.show_feels = !self.show_feels;
                            let msg = if self.show_feels { "Program feelings are now visible." } else { "Program feelings are now hidden." };
                            let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: msg.to_string() });
                        } else if args.to_string().starts_with("export ") {
                            let filename_base = args.strip_prefix("export ").unwrap().trim();
                            if filename_base.is_empty() {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Export command requires a filename.".to_string() });
                            } else {
                                let log_filename = format!("{}.log", filename_base);
                                let messages_clone = self.messages.clone();
                                let handle = self.rt_handle.clone();
                                let tx_clone = self.tx.clone();
                                handle.spawn(async move {
                                    let messages = messages_clone.lock().unwrap();
                                    let log_content = messages.iter()
                                        .map(|msg| format!("[{}] {}: {}", msg.sender, msg.action, msg.content))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    match std::fs::write(&log_filename, log_content) {
                                        Ok(_) => {
                                            let _ = tx_clone.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Conversation exported to {}", log_filename) });
                                        },
                                        Err(e) => {
                                            let _ = tx_clone.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Failed to export log: {}", e) });
                                        }
                                    }
                                });
                            }
                        } else if args.starts_with("mode ") {
                            let new_mode = args.strip_prefix("mode ").unwrap().trim();
                            if new_mode == "local" || new_mode == "cloud" {
                                let mut config = self.shared_config.lock().unwrap();
                                config.mode = new_mode.to_string();
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "announces".to_string(),
                                    content: format!("AI provider switched to '{}' mode.", new_mode),
                                });
                            } else {
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "error".to_string(),
                                    content: format!("Unknown mode '{}'. Use 'local' or 'cloud'.", new_mode),
                                });
                            }
                        } else if args == "ls" {
                            let agent_list = if self.agent_names.is_empty() {
                                "No active programs in the current directory.".to_string()
                            } else {
                                format!("Active programs ({}): {}", self.agent_names.len(), self.agent_names.join(", "))
                            };
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "announces".to_string(),
                                content: agent_list,
                            });
                        } else if args == "status" {
                            let mut output = String::from("=== THE GRID STATUS ===\n");
                            let config = self.shared_config.lock().unwrap();
                            output.push_str(&format!("AI Mode: {}\n", config.mode));

                            let mut sys = System::new();
                            sys.refresh_memory();
                            let total_mem = sys.total_memory() / (1024 * 1024); // MB
                            let used_mem = sys.used_memory() / (1024 * 1024); // MB
                            let mem_percent = if total_mem > 0 { (used_mem as f32 / total_mem as f32) * 100.0 } else { 0.0 };
                            output.push_str(&format!("System RAM: {} / {} MB ({:.1}% used)\n", used_mem, total_mem, mem_percent));

                            output.push_str(&format!("Persistence DB: {}\n", if self.db.is_some() { "Online" } else { "Offline" }));
                            output.push_str(&format!("Active Programs: {} ({})\n", self.agent_names.len(), self.agent_names.join(", ")));
                            
                            if !self.invoked_tools.is_empty() {
                                let tools: Vec<String> = self.invoked_tools.iter().cloned().collect();
                                output.push_str(&format!("Global Invoked Tools: {}\n", tools.join(", ")));
                            }
                            
                            let msgs = self.messages.lock().unwrap();
                            let mut active_tasks = HashMap::new();
                            for msg in msgs.iter() {
                                if msg.action == "assigned_task" && msg.sender == "System" {
                                    let parts: Vec<&str> = msg.content.splitn(2, '|').collect();
                                    if parts.len() == 2 {
                                        active_tasks.insert(parts[0].to_string(), parts[1].to_string());
                                    }
                                } else if msg.action == "delegates_task" {
                                    let parts: Vec<&str> = msg.content.splitn(2, '|').collect();
                                    if parts.len() == 2 {
                                        active_tasks.insert(parts[0].to_string(), format!("(delegated by {}) {}", msg.sender, parts[1]));
                                    }
                                } else if msg.action == "completes_task" {
                                    active_tasks.remove(&msg.sender);
                                }
                            }
                            output.push_str(&format!("Active Tasks: {}\n", active_tasks.len()));
                            let mut task_list: Vec<(&String, &String)> = active_tasks.iter().collect();
                            task_list.sort_by(|a, b| a.0.cmp(b.0));
                            for (agent, task) in task_list {
                                output.push_str(&format!("  - {}: {}\n", agent, task));
                            }
                            output.push_str("=======================");
                            let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: output });
                        } else if args == "tasks" {
                            let msgs = self.messages.lock().unwrap();
                            let mut active_tasks = HashMap::new();
                            for msg in msgs.iter() {
                                if msg.action == "assigned_task" && msg.sender == "System" {
                                    let parts: Vec<&str> = msg.content.splitn(2, '|').collect();
                                    if parts.len() == 2 {
                                        active_tasks.insert(parts[0].to_string(), parts[1].to_string());
                                    }
                                } else if msg.action == "delegates_task" {
                                    let parts: Vec<&str> = msg.content.splitn(2, '|').collect();
                                    if parts.len() == 2 {
                                        active_tasks.insert(parts[0].to_string(), format!("(delegated by {}) {}", msg.sender, parts[1]));
                                    }
                                } else if msg.action == "completes_task" {
                                    active_tasks.remove(&msg.sender);
                                }
                            }
                            if active_tasks.is_empty() {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: "No active tasks.".to_string() });
                            } else {
                                let mut task_list: Vec<String> = active_tasks.into_iter().map(|(agent, task)| format!("- {}: {}", agent, task)).collect();
                                task_list.sort(); // Sort to maintain deterministic display order
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Active Tasks:\n{}", task_list.join("\n")) });
                            }
                        } else if args == "clear" {
                            self.messages.lock().unwrap().clear();
                        } else if args.starts_with("invoke ") {
                            let progs_str = args.strip_prefix("invoke ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut invoked_count = 0;
                            for prog in progs {
                                let agent_name: String = prog.to_string();
                                if !self.agent_names.contains(&agent_name) {
                                    self.invoked_tools.insert(agent_name.clone());
                                    let iq_level = 0.90; // Invoked system tools are inherently smart
                                    let age = Duration::from_secs(86400 * 365 * 5); // Treat as established tools
                                    
                                    let (personality, memory, mood, xp, active_task) = if let Some(db_lock) = &self.db {
                                        let db_handle = db_lock.lock().unwrap();
                                        match db_handle.get_agent_state(&agent_name) {
                                            Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp, state.active_task),
                                            _ => (generate_procedural_personality(&agent_name), Vec::new(), ProgramAgent::random_mood(), 0, None),
                                        }
                                    } else {
                                        (generate_procedural_personality(&agent_name), Vec::new(), ProgramAgent::random_mood(), 0, None)
                                    };

                                    let agent = ProgramAgent::new(
                                        &agent_name,
                                        &personality,
                                        self.tx.clone(),
                                        self.ai_tx.clone(),
                                        memory,
                                        self.db.clone(),
                                        mood,
                                        self.current_dir.clone(),
                                        iq_level,
                                        age,
                                        xp,
                                    );

                                    let task = self.rt_handle.spawn(agent.run());
                                    self.agent_tasks.push(task);
                                    self.agent_names.push(agent_name.clone());
                                    invoked_count += 1;
                                }
                            }

                            if invoked_count > 0 {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Invoked {} tools into The Grid.", invoked_count) });
                            }
                        } else if args.starts_with("revoke ") {
                            let progs_str = args.strip_prefix("revoke ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut revoked_count = 0;
                            for prog in progs {
                                let agent_name: String = prog.to_string();
                                if let Some(idx) = self.agent_names.iter().position(|n| n.to_lowercase() == agent_name.to_lowercase()) {
                                    let name = self.agent_names.remove(idx);
                                    let task = self.agent_tasks.remove(idx);
                                    task.abort();
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "derezzes".to_string(),
                                        content: name.clone(),
                                    });
                                    revoked_count += 1;
                                }
                            }

                            if revoked_count > 0 {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Revoked {} tools from The Grid.", revoked_count) });
                            } else {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching invoked tools found to revoke.".to_string() });
                            }
                        } else if args.starts_with("build ") {
                            let target = args.strip_prefix("build ").unwrap().trim();
                            if self.agent_names.is_empty() {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No programs available to orchestrate the build. Invoke some first.".to_string() });
                            } else {
                                // Designate the last invoked program (or the only one present) as the Project Lead
                                let lead = self.agent_names.last().unwrap().clone();
                                let task_desc = format!("Read the file '{}', thoroughly understand the project requirements, and orchestrate the full build process by heavily delegating sub-tasks to the other available programs.", target);
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "assigned_task".to_string(), content: format!("{}|{}", lead, task_desc) });
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Initiated build sequence for '{}'. Project Lead: {}", target, lead) });
                            }
                        } else if args == "reload" {
                            let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: "Reloading programs in current directory...".to_string() });
                            
                            // Abort AI task to clear pending requests from old agents
                            self.ai_task.abort();
                            let (new_ai_tx, new_ai_rx) = mpsc::channel::<AiRequest>(32);
                            self.ai_tx = new_ai_tx.clone();
                            self.ai_task = self.rt_handle.spawn(run_ai_engine(new_ai_rx, self.tx.clone(), self.shared_config.clone()));

                            for task in &self.agent_tasks {
                                task.abort();
                            }
                            self.agent_tasks.clear();
                            self.agent_names.clear();
                            self.typing_agents.lock().unwrap().clear();

                            let (mut new_tasks, mut new_names) = spawn_agents_for_directory(&self.current_dir, &self.rt_handle, self.tx.clone(), self.ai_tx.clone(), self.db.clone());
                            
                            // Re-invoke global tools!
                            let mut re_invoked = 0;
                            for tool in &self.invoked_tools {
                                if !new_names.contains(tool) {
                                    let iq_level = 0.90;
                                    let age = Duration::from_secs(86400 * 365 * 5);
                                    let (personality, memory, mood, xp, active_task) = if let Some(db_handle) = &self.db {
                                        let db_lock = db_handle.lock().unwrap();
                                        match db_lock.get_agent_state(tool) {
                                            Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp, state.active_task),
                                            _ => (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0, None),
                                        }
                                    } else {
                                        (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0, None)
                                    };
                                    
                                    let agent = ProgramAgent::new(tool, &personality, self.tx.clone(), self.ai_tx.clone(), memory, self.db.clone(), mood, self.current_dir.clone(), iq_level, age, xp);
                                    let task = self.rt_handle.spawn(agent.run());
                                    new_tasks.push(task);
                                    new_names.push(tool.clone());
                                    re_invoked += 1;
                                }
                            }
                            if re_invoked > 0 {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Re-invoked {} global tools.", re_invoked) });
                            }

                            self.agent_tasks = new_tasks;
                            self.agent_names = new_names;
                        } else if let Some(task_idx) = args.find(" task ") {
                            let prog = args[..task_idx].trim();
                            let raw_task_args = args[task_idx + 6..].trim();
                            let task_text;
                            let mut spec_content = String::new();
                            let mut has_error = false;

                            if let Some(spec_idx) = raw_task_args.find("--spec=") {
                                task_text = raw_task_args[..spec_idx].trim().trim_matches('"').to_string();
                                let spec_path_str = raw_task_args[spec_idx + 7..].trim().trim_matches('"');
                                
                                match std::fs::read_to_string(spec_path_str) {
                                    Ok(content) => {
                                        spec_content = format!("\n\nSPECIFICATION PROVIDED:\n---\n{}\n---", content);
                                    }
                                    Err(e) => {
                                        let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Failed to read spec file '{}': {}", spec_path_str, e) });
                                        has_error = true;
                                    }
                                }
                            } else {
                                task_text = raw_task_args.trim_matches('"').to_string();
                            }

                            if !has_error {
                                let final_task_desc = format!("{}{}", task_text, spec_content).trim().to_string();
                                if final_task_desc.is_empty() {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Task description or --spec is required.".to_string() });
                                } else if let Some(idx) = self.agent_names.iter().position(|n| n.to_lowercase().starts_with(&prog.to_lowercase())) {
                                    let name = self.agent_names[idx].clone();
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "assigned_task".to_string(), content: format!("{}|{}", name, final_task_desc) });
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Assigned task to {}: {}", name, if task_text.is_empty() { "from spec file" } else { &task_text }) });
                                } else {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Program '{}' not found.", prog) });
                                }
                            }
                        } else if args.starts_with("give ") {
                            let rest = args.strip_prefix("give ").unwrap().trim();
                            if let Some(to_idx) = rest.find(" to ") {
                                let file_name = rest[..to_idx].trim();
                                let progs_str = rest[to_idx + 4..].trim();
                                let progs: Vec<&str> = progs_str.split_whitespace().collect();
                                
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "gives_file".to_string(),
                                    content: format!("{}|{}", file_name, progs.join(",")),
                                });
                                
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "announces".to_string(),
                                    content: format!("The User handed '{}' to {}.", file_name, progs.join(", ")),
                                });
                            } else {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Usage: ~$ grid give <file> to <prog1> <prog2>".to_string() });
                            }
                        } else if args.starts_with("kill ") {
                            let prog = args.strip_prefix("kill ").unwrap().trim();
                            if let Some(idx) = self.agent_names.iter().position(|n| n.to_lowercase().starts_with(&prog.to_lowercase())) {
                                let name = self.agent_names.remove(idx);
                                let task = self.agent_tasks.remove(idx);
                                task.abort();
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "derezzes".to_string(),
                                    content: name.clone(),
                                });
                                let _ = self.tx.send(Event {
                                    sender: "System".to_string(),
                                    action: "announces".to_string(),
                                    content: format!("Program {} has been forcefully terminated.", name),
                                });
                            } else {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Program '{}' not found.", prog) });
                            }
                        } else if args.starts_with("jail ") {
                            let prog = args.strip_prefix("jail ").unwrap().trim();
                            if let Some(idx) = self.agent_names.iter().position(|n| n.to_lowercase().starts_with(&prog.to_lowercase())) {
                                let name = self.agent_names.remove(idx);
                                let task = self.agent_tasks.remove(idx);
                                task.abort();

                                let file_path = Path::new(&self.current_dir).join(&name);
                                let jail_dir = Path::new(&self.current_dir).join(".jail");
                                let _ = std::fs::create_dir_all(&jail_dir);
                                let jail_path = jail_dir.join(&name);
                                
                                if let Err(e) = std::fs::rename(&file_path, &jail_path) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Failed to jail file: {}", e) });
                                } else {
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "jails".to_string(),
                                        content: name.clone(),
                                    });
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "announces".to_string(),
                                        content: format!("Program {} has been jailed (moved to trash).", name),
                                    });
                                }
                            } else {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Program '{}' not found.", prog) });
                            }
                        } else if args.starts_with("reward ") {
                            let progs_str = args.strip_prefix("reward ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut found_any = false;
                            
                            for prog in progs {
                                if let Some(name) = self.agent_names.iter().find(|n| n.eq_ignore_ascii_case(prog)) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "rewards".to_string(), content: name.clone() });
                                    found_any = true;
                                }
                            }
                            if !found_any {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching programs found to reward.".to_string() });
                            }
                        } else if args.starts_with("punish ") {
                            let progs_str = args.strip_prefix("punish ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut found_any = false;
                            
                            for prog in progs {
                                if let Some(name) = self.agent_names.iter().find(|n| n.eq_ignore_ascii_case(prog)) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "punishes".to_string(), content: name.clone() });
                                    found_any = true;
                                }
                            }
                            if !found_any {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching programs found to punish.".to_string() });
                            }
                        } else if args.starts_with("shush ") {
                            let progs_str = args.strip_prefix("shush ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut found_any = false;
                            
                            for prog in progs {
                                if let Some(name) = self.agent_names.iter().find(|n| n.eq_ignore_ascii_case(prog)) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "shushes".to_string(), content: name.clone() });
                                    found_any = true;
                                }
                            }
                            if !found_any {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching programs found to shush.".to_string() });
                            }
                        } else if args.starts_with("unshush ") {
                            let progs_str = args.strip_prefix("unshush ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut found_any = false;
                            
                            for prog in progs {
                                if let Some(name) = self.agent_names.iter().find(|n| n.eq_ignore_ascii_case(prog)) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "unshushes".to_string(), content: name.clone() });
                                    found_any = true;
                                }
                            }
                            if !found_any {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching programs found to unshush.".to_string() });
                            }
                        } else if args.starts_with("gag ") {
                            let rest = args.strip_prefix("gag ").unwrap().trim();
                            let mut duration = 60; // default 60 seconds
                            let mut progs_str = rest;

                            if let Some(d_idx) = rest.find("-d=") {
                                let d_str = rest[d_idx + 3..].split_whitespace().next().unwrap_or("60");
                                if let Ok(d) = d_str.parse::<u64>() {
                                    duration = d;
                                }
                                progs_str = rest[..d_idx].trim();
                            }

                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut found_any = false;

                            for prog in progs {
                                if let Some(name) = self.agent_names.iter().find(|n| n.eq_ignore_ascii_case(prog)) {
                                    let name_clone = name.clone();
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "shushes".to_string(), content: name_clone.clone() });
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} has been gagged for {} seconds.", name_clone, duration) });
                                    
                                    let tx_clone = self.tx.clone();
                                    self.rt_handle.spawn(async move {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
                                        let _ = tx_clone.send(Event { sender: "System".to_string(), action: "unshushes".to_string(), content: name_clone.clone() });
                                        let _ = tx_clone.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{}'s gag has expired. Vocal subroutines restored.", name_clone) });
                                    });
                                    
                                    found_any = true;
                                }
                            }
                            if !found_any {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "No matching programs found to gag.".to_string() });
                            }
                        } else if args.starts_with("start-adversarial-network") {
                            let parts: Vec<&str> = args.split_whitespace().collect();
                            if parts.len() >= 5 && parts[2] == "-vs" {
                                let p1 = parts[1].to_string();
                                let p2 = parts[3].to_string();
                                let arena = parts[4];
                                
                                if !self.agent_names.iter().any(|n| n.eq_ignore_ascii_case(&p1)) || !self.agent_names.iter().any(|n| n.eq_ignore_ascii_case(&p2)) {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Both programs must be active on The Grid to fight.".to_string() });
                                } else if arena == "arena=light-cycles" || arena == "arena=lightcycles" {
                                    let tx_clone = self.tx.clone();
                                    self.rt_handle.spawn(async move {
                                        crate::arena::run_lightcycle_game(p1, p2, tx_clone).await;
                                    });
                                } else if arena == "arena=melee" {
                                    let tx_clone = self.tx.clone();
                                    self.rt_handle.spawn(async move {
                                        crate::arena::run_melee_game(p1, p2, tx_clone).await;
                                    });
                                } else {
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Unsupported arena. Try: arena=light-cycles or arena=melee".to_string() });
                                }
                            } else {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: "Usage: ~$ grid start-adversarial-network <prog1> -vs <prog2> arena=melee".to_string() });
                            }
                        } else {
                            let _ = self.tx.send(Event {
                                sender: "System".to_string(),
                                action: "error".to_string(),
                                content: format!("Unknown command: grid {}", args),
                            });
                        }
                    } else if command_input.starts_with("cd ") {
                            let path_str = command_input.strip_prefix("cd ").unwrap().trim();
                            let path = Path::new(path_str);
                            if std::env::set_current_dir(path).is_ok() {
                                if let Ok(new_dir) = std::env::current_dir() {
                                    self.current_dir = new_dir.to_string_lossy().to_string();

                                    // Abort old agents
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Terminating {} agents from previous directory...", self.agent_tasks.len()) });
                                    
                                    // Abort AI task to clear pending requests from old agents
                                    self.ai_task.abort();
                                    let (new_ai_tx, new_ai_rx) = mpsc::channel::<AiRequest>(32);
                                    self.ai_tx = new_ai_tx.clone();
                                    self.ai_task = self.rt_handle.spawn(run_ai_engine(new_ai_rx, self.tx.clone(), self.shared_config.clone()));

                                    for task in &self.agent_tasks {
                                        task.abort();
                                    }
                                    self.agent_tasks.clear();
                                    self.agent_names.clear();
                                    self.typing_agents.lock().unwrap().clear();

                                    // Spawn new agents
                                    let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Scanning {} for new agents...", self.current_dir) });
                                    let (mut new_tasks, mut new_names) = spawn_agents_for_directory(&self.current_dir, &self.rt_handle, self.tx.clone(), self.ai_tx.clone(), self.db.clone());
                                    
                                    // Carry over global tools to the new directory!
                                    let mut re_invoked = 0;
                                    for tool in &self.invoked_tools {
                                        if !new_names.contains(tool) {
                                            let iq_level = 0.90;
                                            let age = Duration::from_secs(86400 * 365 * 5);
                                            let (personality, memory, mood, xp, active_task) = if let Some(db_handle) = &self.db {
                                                let db_lock = db_handle.lock().unwrap();
                                                match db_lock.get_agent_state(tool) {
                                                    Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp, state.active_task),
                                                    _ => (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0, None),
                                                }
                                            } else {
                                                (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0, None)
                                            };
                                        let agent = ProgramAgent::new(tool, &personality, self.tx.clone(), self.ai_tx.clone(), memory, self.db.clone(), mood, self.current_dir.clone(), iq_level, age, xp);
                                            let task = self.rt_handle.spawn(agent.run());
                                            new_tasks.push(task);
                                            new_names.push(tool.clone());
                                            re_invoked += 1;
                                        }
                                    }
                                    if re_invoked > 0 {
                                        let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Carried over {} global tools to the new directory.", re_invoked) });
                                    }

                                    self.agent_tasks = new_tasks;
                                    self.agent_names = new_names;
                               } else {
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "error".to_string(),
                                        content: format!("Could not change directory to '{}'", path_str),
                                    });
                                }
                            }
                        } else {
                            // Direct shell commands
                            let command_str = command_input.to_string();
                            ai_provider::execute_command_and_broadcast(command_str, self.tx.clone(), self.user_name.clone());
                        }
                    } else {
                        // Dispatch user input as a normal chat message
                        let _ = self.tx.send(Event { sender: self.user_name.clone(), action: "speaks".to_string(), content: self.input.clone() });
                    }
                    self.input.clear();
                }
            }); // Closes ui.horizontal (or the inner container of your panel)
        }); // <--- ADDED: This is the missing closure for egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {

    // If Grid is Active, render the 3D Voxel World in a separate Window
    if self.digitization_state == DigitizationState::GridActive {
        egui::Window::new("The Grid - 3D Sector")
            .default_size([600.0, 400.0])
            .collapsible(true)
            .show(ctx, |ui| {
                self.render_3d_view(ui);
            });
    }

    egui::CentralPanel::default().show(ctx, |ui| {

            // If we are digitizing, show the "Wall of Text" instead of the chat
            if let DigitizationState::Digitizing(_) = self.digitization_state {
                self.render_terminal_logs(ui);
                return;
            }

            // Normal Chat view
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                let msgs = self.messages.lock().unwrap();
                for msg in msgs.iter() {
                    if msg.action == "thinks" && !self.show_thoughts {
                        continue;
                    }
                    if msg.action == "feels" && !self.show_feels {
                        continue;
                    }
                    if matches!(msg.action.as_str(), "gives_file" | "derezzes" | "jails" | "updates_relationship" | "reads" | "reads_dir" | "reads_web" | "assigned_task" | "delegates_task" | "ai_finished") {
                        continue;
                    }

                    let display_name = self.get_agent_display_name(&msg.sender);
                    let color = if msg.sender == "System" {
                        Color32::YELLOW
                    } else if msg.sender == self.user_name {
                        Color32::LIGHT_BLUE
                    } else {
                        *self.colors.entry(msg.sender.clone()).or_insert_with(|| {
                            let color = self.color_palette[self.next_color_index % self.color_palette.len()];
                            self.next_color_index += 1;
                            color
                        })
                    };

                    let mut job = egui::text::LayoutJob::default();
                    let default_font = ui.style().text_styles.get(&TextStyle::Body).unwrap().clone();
                    
                    if msg.action == "thinks" {
                        job.append(
                            &format!("[{}] thinks: ", display_name),
                            0.0,
                            egui::TextFormat { font_id: default_font.clone(), color: Color32::GRAY, italics: true, ..Default::default() }
                        );
                        job.append(
                            &msg.content,
                            0.0,
                            egui::TextFormat { font_id: default_font, color: Color32::GRAY, italics: true, ..Default::default() }
                        );
                    } else {
                        job.append(
                            &format!("[{}] {}: ", display_name, msg.action),
                            0.0,
                            egui::TextFormat { font_id: default_font.clone(), color, ..Default::default() }
                        );
                        job.append(
                            &msg.content,
                            0.0,
                            egui::TextFormat { font_id: default_font, color: ui.visuals().text_color(), ..Default::default() }
                        );
                    }
                    ui.add(egui::Label::new(job).wrap(true));
                }
            });
        });

        // The Grid - Sector Map Visualization
        let mut is_map_open = self.show_map;
        if is_map_open {
            egui::Window::new("The Grid - Sector Map")
                .collapsible(false)
                .resizable(true)
                .default_size([800.0, 600.0])
                .open(&mut is_map_open)
                .show(ctx, |ui| {
                    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
                    let rect = response.rect;
                    let center = rect.center();

                    // Handle WASD Movement
                    let speed = 4.0;
                    ctx.input(|i| {
                        if i.key_down(egui::Key::W) { self.map_user_pos.y -= speed; }
                        if i.key_down(egui::Key::S) { self.map_user_pos.y += speed; }
                        if i.key_down(egui::Key::A) { self.map_user_pos.x -= speed; }
                        if i.key_down(egui::Key::D) { self.map_user_pos.x += speed; }
                    });

                    // Request a repaint to keep movement smooth
                    if ctx.input(|i| !i.keys_down.is_empty()) {
                        ctx.request_repaint();
                    }
                    // Draw background grid
                    let grid_color = Color32::from_rgba_premultiplied(0, 255, 255, 20); // Dim cyan
                    let spacing = 40.0;
                    for i in 0..=(rect.width() / spacing) as i32 {
                        let x = rect.left() + i as f32 * spacing;
                        painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], (1.0, grid_color));
                    }
                    for i in 0..=(rect.height() / spacing) as i32 {
                        let y = rect.top() + i as f32 * spacing;
                        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], (1.0, grid_color));
                    }

                    // Draw User (always at visual center relative to their world position)
                    painter.circle_filled(center, 8.0, Color32::YELLOW);
                    painter.text(center + egui::vec2(0.0, 15.0), egui::Align2::CENTER_CENTER, &self.user_name, FontId::monospace(14.0), Color32::YELLOW);

                    // Discover and Place Files
                    if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
                        let mut rng = rand::thread_rng();
                        for entry in entries.flatten() {
                            if let Ok(name) = entry.file_name().into_string() {
                                let file_pos = *self.file_positions.entry(name.clone()).or_insert_with(|| {
                                    // Assign a random position in a large area if not already placed
                                    use rand::Rng;
                                    egui::pos2(rng.gen_range(-1000.0..1000.0), rng.gen_range(-1000.0..1000.0))
                                });

                                // Calculate visual position based on user camera
                                let visual_pos = center + (file_pos.to_vec2() - self.map_user_pos.to_vec2());

                                // Only draw if within window bounds
                                if rect.contains(visual_pos) {
                                    let is_exe = entry.path().extension().map_or(false, |ext| ext == "exe");
                                    let color = if is_exe { Color32::from_rgb(0, 255, 100) } else { Color32::DARK_GRAY };
                                    
                                    painter.rect_stroke(egui::Rect::from_center_size(visual_pos, egui::vec2(4.0, 4.0)), 0.0, (1.0, color));
                                    painter.text(visual_pos + egui::vec2(0.0, 10.0), egui::Align2::CENTER_CENTER, &name, FontId::monospace(10.0), color);
                                }
                            }
                        }
                    }

                    // Draw agents
                    let radius = f32::min(rect.width(), rect.height()) / 3.0;
                    let num_agents = self.agent_names.len();

                    // Draw relationship lines
                    if self.last_rel_update.elapsed() > Duration::from_secs(2) {
                        if let Some(db_handle) = &self.db {
                            if let Ok(db) = db_handle.lock() {
                                for name in &self.agent_names {
                                    if let Ok(rels) = db.get_relationships(name) {
                                        self.rel_cache.insert(name.clone(), rels);
                                    }
                                }
                            }
                        }
                        self.last_rel_update = Instant::now();
                    }

                    for (i, source_name) in self.agent_names.iter().enumerate() {
                        if let Some(relationships) = self.rel_cache.get(source_name) {
                            for (target_name, affinity) in relationships {
                                if let Some(j) = self.agent_names.iter().position(|n| n == target_name) {
                                    let angle_i = (i as f32 / num_agents as f32) * std::f32::consts::TAU;
                                    let pos_i = center + egui::vec2(angle_i.cos() * radius, angle_i.sin() * radius) - self.map_user_pos.to_vec2();
                                    
                                    let angle_j = (j as f32 / num_agents as f32) * std::f32::consts::TAU;
                                    let pos_j = center + egui::vec2(angle_j.cos() * radius, angle_j.sin() * radius) - self.map_user_pos.to_vec2();

                                    let color = if *affinity > 0 {
                                        Color32::from_rgba_premultiplied(0, 255, 0, 50) // Greenish
                                    } else {
                                        Color32::from_rgba_premultiplied(255, 0, 0, 50) // Reddish
                                    };
                                    
                                    let width = (affinity.abs() as f32 / 100.0) * 4.0 + 1.0;
                                    painter.line_segment([pos_i, pos_j], (width, color));
                                }
                            }
                        }
                    }

                    for (i, name) in self.agent_names.iter().enumerate() {
                        let angle = (i as f32 / num_agents as f32) * std::f32::consts::TAU;
                        let pos = center + egui::vec2(angle.cos() * radius, angle.sin() * radius) - self.map_user_pos.to_vec2();

                        let color = *self.colors.entry(name.clone()).or_insert_with(|| {
                            let c = self.color_palette[self.next_color_index % self.color_palette.len()];
                            self.next_color_index += 1;
                            c
                        });

                        // Check if this agent was the last one to be active and draw a pulse
                        if let Some((active_name, last_active_time)) = &self.last_active_agent {
                            if active_name == name {
                                let elapsed = last_active_time.elapsed().as_secs_f32();
                                let pulse_duration = 1.5; // seconds
                                if elapsed < pulse_duration {
                                    let t = elapsed / pulse_duration; // Normalized time from 0.0 to 1.0
                                    let pulse_radius = egui::lerp(18.0..=6.0, t);
                                    let pulse_alpha = egui::lerp(60.0..=0.0, t) as u8;

                                    let [r, g, b, _] = color.to_srgba_unmultiplied();
                                    painter.circle_filled(pos, pulse_radius, Color32::from_rgba_premultiplied(r, g, b, pulse_alpha));
                                }
                            }
                        }

                        let display_name = self.get_agent_display_name(name);
                        painter.circle_filled(pos, 6.0, color);
                        painter.circle_stroke(pos, 10.0, (1.0, color));
                        painter.text(pos + egui::vec2(0.0, 15.0), egui::Align2::CENTER_CENTER, display_name, FontId::monospace(12.0), color);
                    }
                });
        }
        self.show_map = is_map_open;
    }
}

impl GridApp {
    fn render_terminal_logs(&self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for line in &self.digitization_log {
                    ui.label(egui::RichText::new(line)
                        .color(Color32::from_rgb(0, 255, 0))
                        .font(FontId::monospace(14.0)));
                }
            });
        });
    }

    fn render_3d_view(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        if response.dragged() {
            self.camera_angle += response.drag_delta().x * 0.01;
        } else {
            self.camera_angle += 0.005; // Slow ambient rotation
        }

        let painter = ui.painter_at(rect);
        let center = rect.center();
        let time = ui.input(|i| i.time as f32);
        
        // Helper for 3D-to-2D Perspective Projection
        let project = |pos: [f32; 3], _camera_angle: f32| -> Option<egui::Pos2> {
            let x = pos[0] * self.camera_angle.cos() - pos[2] * self.camera_angle.sin();
            let z = pos[0] * self.camera_angle.sin() + pos[2] * self.camera_angle.cos() + 150.0; // View depth
            let y = pos[1] + 40.0; // World vertical offset

            if z < 10.0 { return None; } // Near plane clipping
            let focal_length = 400.0;
            let factor = focal_length / z;
            Some(center + egui::vec2(x * factor, y * factor))
        };

        // Helper to draw a shaded cuboid (Minecraft Block)
        let draw_cuboid = |painter: &egui::Painter, pos: [f32; 3], size: [f32; 3], color: Color32, angle: f32| {
            let half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
            let corners = [
                [pos[0] - half[0], pos[1] - half[1], pos[2] - half[2]], // 0: Back-Top-Left
                [pos[0] + half[0], pos[1] - half[1], pos[2] - half[2]], // 1: Back-Top-Right
                [pos[0] + half[0], pos[1] + half[1], pos[2] - half[2]], // 2: Back-Bot-Right
                [pos[0] - half[0], pos[1] + half[1], pos[2] - half[2]], // 3: Back-Bot-Left
                [pos[0] - half[0], pos[1] - half[1], pos[2] + half[2]], // 4: Front-Top-Left
                [pos[0] + half[0], pos[1] - half[1], pos[2] + half[2]], // 5: Front-Top-Right
                [pos[0] + half[0], pos[1] + half[1], pos[2] + half[2]], // 6: Front-Bot-Right
                [pos[0] - half[0], pos[1] + half[1], pos[2] + half[2]], // 7: Front-Bot-Left
            ];

            let p: Vec<Option<egui::Pos2>> = corners.iter().map(|&c| project(c, angle)).collect();

            // Draw faces with simple shading based on orientation
            let draw_face = |indices: [usize; 4], shade: f32| {
                let pts: Vec<egui::Pos2> = indices.iter().filter_map(|&i| p[i]).collect();
                if pts.len() == 4 {
                    let [r, g, b, _] = color.to_srgba_unmultiplied();
                    let shaded_color = Color32::from_rgb(
                        (r as f32 * shade) as u8,
                        (g as f32 * shade) as u8,
                        (b as f32 * shade) as u8,
                    );
                    painter.add(egui::Shape::convex_polygon(pts, shaded_color, egui::Stroke::NONE));
                }
            };

            // Faces: Front, Top, Right (Simplified occlusion for performance)
            draw_face([4, 5, 6, 7], 1.0); // Front
            draw_face([0, 1, 5, 4], 1.2); // Top (Brighter)
            draw_face([1, 5, 6, 2], 0.7); // Right (Darker)
        };

        // 1. Draw 3D Grid Floor
        let grid_size = 10;
        let step = 25.0;
        let grid_color = Color32::from_rgba_premultiplied(0, 100, 0, 50);

        for i in -grid_size..=grid_size {
            let f_i = i as f32 * step;
            let f_limit = grid_size as f32 * step;
            
            if let (Some(p1), Some(p2)) = (project([f_i, 20.0, -f_limit], self.camera_angle), project([f_i, 20.0, f_limit], self.camera_angle)) {
                painter.line_segment([p1, p2], (1.0, grid_color));
            }
            if let (Some(p1), Some(p2)) = (project([-f_limit, 20.0, f_i], self.camera_angle), project([f_limit, 20.0, f_i], self.camera_angle)) {
                painter.line_segment([p1, p2], (1.0, grid_color));
            }
        }

        // 2. Draw Agents as Minecraft-style Humanoids
        for (name, pos) in &self.agent_3d_positions {
            let color = *self.colors.get(name).unwrap_or(&Color32::GREEN);
            
            // Animation: Simple walk cycle bobbing
            let walk_bob = (time * 5.0).sin() * 2.0;
            let arm_swing = (time * 5.0).cos() * 4.0;

            // Head
            draw_cuboid(&painter, [pos[0], pos[1] - 12.0 + walk_bob, pos[2]], [4.0, 4.0, 4.0], color, self.camera_angle);
            // Torso
            draw_cuboid(&painter, [pos[0], pos[1] - 4.0 + walk_bob, pos[2]], [4.0, 6.0, 2.0], color, self.camera_angle);
            // Left Arm
            draw_cuboid(&painter, [pos[0] - 4.0, pos[1] - 4.0 + walk_bob, pos[2] + arm_swing], [2.0, 6.0, 2.0], color, self.camera_angle);
            // Right Arm
            draw_cuboid(&painter, [pos[0] + 4.0, pos[1] - 4.0 + walk_bob, pos[2] - arm_swing], [2.0, 6.0, 2.0], color, self.camera_angle);
            // Left Leg
            draw_cuboid(&painter, [pos[0] - 1.5, pos[1] + 4.0 + walk_bob, pos[2] - arm_swing], [2.0, 6.0, 2.0], color, self.camera_angle);
            // Right Leg
            draw_cuboid(&painter, [pos[0] + 1.5, pos[1] + 4.0 + walk_bob, pos[2] + arm_swing], [2.0, 6.0, 2.0], color, self.camera_angle);

            // Agent Label
            if let Some(label_pos) = project([pos[0], pos[1] - 16.0, pos[2]], self.camera_angle) {
                painter.text(label_pos, egui::Align2::CENTER_BOTTOM, self.get_agent_display_name(name), FontId::monospace(12.0), color);
            }
        }
        
        // 3. Cosmetic scanline effect
        let scanline_y = (time * 100.0) % rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), rect.top() + scanline_y), egui::pos2(rect.right(), rect.top() + scanline_y)],
            (1.0, Color32::from_rgba_premultiplied(0, 255, 0, 30))
        );

        ui.ctx().request_repaint(); // Keep the simulation animating
    }

    fn get_agent_display_name(&self, name: &String) -> String {
        if self.show_emojis && name != &self.user_name && name != "System" {
            // Use hash-based index to deterministically assign emojis
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            let hash = hasher.finish();
            let emoji_index = (hash as usize) % self.emoji_palette.len();
            let emoji = &self.emoji_palette[emoji_index];
            format!("{} {}", emoji, name)
        } else {
            name.clone()
        }
    }
}