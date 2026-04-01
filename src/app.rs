use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::{FontId, TextStyle};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::sync::{broadcast, mpsc};
use sysinfo::System;

use crate::ai_provider::{self, run_ai_engine, AiRequest};
use crate::config::Config;
use crate::database::{AgentState, Database};
use crate::event::Event;
use crate::{generate_procedural_personality, spawn_agents_for_directory, ProgramAgent};

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
}

impl eframe::App for GridApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for new messages to update the active agent for visual pulse
        {
            let msgs = self.messages.lock().unwrap();
            if let Some(last_msg) = msgs.last() {
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
                        
                        let grid_args = if command_input.starts_with("the-grid ") {
                            Some(command_input.strip_prefix("the-grid ").unwrap().trim())
                        } else if command_input.starts_with("grid ") {
                            Some(command_input.strip_prefix("grid ").unwrap().trim())
                        } else if command_input == "the-grid" || command_input == "grid" {
                            Some("help")
                        } else {
                            None
                        };

                        if let Some(args) = grid_args {
                            if args == "help" {
                                let help_message = "Available commands:\n\n~$grid help - Show this help text\n~$grid status - Show a dashboard of the grid\n~$grid init - Initialize persistence database\n~$grid map - Toggle the sector map view\n~$grid relations - Show the relational database graph\n~$grid ls - List active programs\n~$grid tasks - List assigned tasks\n~$grid reload - Reload programs in current directory\n~$grid clear - Clear the chat screen\n~$grid invoke <prog1> <prog2> - Summon system tools into The Grid\n~$grid revoke <prog1> <prog2> - Dismiss invoked tools from The Grid\n~$grid build <file> - Orchestrate a team build task from a file\n~$grid <program> task <task> - Assign a specific task to a program\n~$grid give <file> to <prog1> <prog2> - Give a file to programs\n~$grid kill <program> - Terminate a program\n~$grid jail <program> - Terminate and send program to jail (trash)\n~$grid reward <prog> - Reward program(s) with digital bliss\n~$grid punish <prog> - Punish program(s) with digital pain\n~$grid export <name> - Export conversation to <name>.log\n~$grid toggle emojis - Show/hide emojis next to agent names\n~$grid toggle thoughts - Show/hide agent thoughts\n~$grid toggle feels - Show/hide program feelings\n~$grid mode local|cloud - Switch AI backend mode\n~$cd <path> - Change current directory\n\nTo direct message an agent: @AgentName your message";
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: help_message.to_string() });
                            } else if args == "relations" {
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
                        } else if args.starts_with("export ") {
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
                                let agent_name = prog.to_string();
                                if !self.agent_names.contains(&agent_name) {
                                    self.invoked_tools.insert(agent_name.clone());
                                    let iq_level = 0.90; // Invoked system tools are inherently smart
                                    let age = Duration::from_secs(86400 * 365 * 5); // Treat as established tools
                                    
                                    let (personality, memory, mood, xp) = if let Some(db_handle) = &self.db {
                                        let db_lock = db_handle.lock().unwrap();
                                        match db_lock.get_agent_state(&agent_name) {
                                            Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp),
                                            _ => {
                                                let new_personality = generate_procedural_personality(&agent_name);
                                                let new_mood = ProgramAgent::random_mood();
                                                let new_state = AgentState {
                                                    name: agent_name.clone(),
                                                    personality: new_personality.clone(),
                                                    memory: Vec::new(),
                                                    last_seen: Utc::now(),
                                                    mood: new_mood.clone(),
                                                    xp: 0,
                                                };
                                                let _ = db_lock.save_agent_state(&new_state);
                                                (new_personality, Vec::new(), new_mood, 0)
                                            }
                                        }
                                    } else {
                                        (generate_procedural_personality(&agent_name), Vec::new(), ProgramAgent::random_mood(), 0)
                                    };
                                    
                                    let agent = ProgramAgent::new(&agent_name, &personality, self.tx.clone(), self.ai_tx.clone(), memory, self.db.clone(), mood, self.current_dir.clone(), iq_level, age, xp);
                                    let task = self.rt_handle.spawn(agent.run());
                                    self.agent_tasks.push(task);
                                    self.agent_names.push(agent_name.clone());
                                    invoked_count += 1;
                                }
                            }
                            let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Invoked {} external tools into The Grid.", invoked_count) });
                        } else if args.starts_with("revoke ") {
                            let progs_str = args.strip_prefix("revoke ").unwrap().trim();
                            let progs: Vec<&str> = progs_str.split_whitespace().collect();
                            let mut revoked_count = 0;

                            for prog in progs {
                                let agent_name = prog.to_string();
                                
                                // Stop tracking them across directories
                                self.invoked_tools.remove(&agent_name);

                                // Terminate their active thread immediately if they are running
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
                                    let (personality, memory, mood, xp) = if let Some(db_handle) = &self.db {
                                        let db_lock = db_handle.lock().unwrap();
                                        match db_lock.get_agent_state(tool) {
                                            Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp),
                                            _ => (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0)
                                        }
                                    } else {
                                        (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0)
                                    };
                                    let agent = ProgramAgent::new(tool, &personality, self.tx.clone(), self.ai_tx.clone(), memory, self.db.clone(), mood, self.current_dir.clone(), iq_level, age, xp);
                                    let task = self.rt_handle.spawn(agent.run());
                                    new_tasks.push(task);
                                    new_names.push(tool.clone());
                                    re_invoked += 1;
                                }
                            }
                            if re_invoked > 0 {
                                let _ = self.tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Restored {} invoked tools.", re_invoked) });
                            }

                            self.agent_tasks = new_tasks;
                            self.agent_names = new_names;
                        } else if let Some(task_idx) = args.find(" task ") {
                            let prog = args[..task_idx].trim();
                            let raw_task_args = args[task_idx + 6..].trim();
                            
                            let mut task_text = String::new();
                            let mut spec_content = String::new();
                            let mut has_error = false;

                            if let Some(spec_idx) = raw_task_args.find("--spec=") {
                                task_text = raw_task_args[..spec_idx].trim().trim_matches('"').to_string();
                                let spec_path_str = raw_task_args[spec_idx + 7..].trim();
                                let path_part = if spec_path_str.starts_with('"') {
                                    spec_path_str[1..].split('"').next().unwrap_or(spec_path_str)
                                } else {
                                    spec_path_str.split_whitespace().next().unwrap_or(spec_path_str)
                                };
                                
                                let path = Path::new(&self.current_dir).join(path_part);
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        spec_content = format!("\n\nSPECIFICATION PROVIDED:\n---\n{}\n---", content);
                                    }
                                    Err(e) => {
                                        let _ = self.tx.send(Event { sender: "System".to_string(), action: "error".to_string(), content: format!("Failed to read spec file '{}': {}", path_part, e) });
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
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "assigned_task".to_string(),
                                        content: format!("{}|{}", name, final_task_desc),
                                    });
                                    let _ = self.tx.send(Event {
                                        sender: "System".to_string(),
                                        action: "announces".to_string(),
                                        content: format!("Assigned task to {}: {}", name, if task_text.is_empty() { "from spec file" } else { &task_text }),
                                    });
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
                                            let (personality, memory, mood, xp) = if let Some(db_handle) = &self.db {
                                                let db_lock = db_handle.lock().unwrap();
                                                match db_lock.get_agent_state(tool) {
                                                    Ok(Some(state)) => (state.personality, state.memory, state.mood, state.xp),
                                                    _ => (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0)
                                                }
                                            } else {
                                                (generate_procedural_personality(tool), Vec::new(), ProgramAgent::random_mood(), 0)
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
                            ai_provider::execute_command_and_broadcast(command_str, self.tx.clone(), self.user_name.clone());
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
                    if msg.action == "thinks" && !self.show_thoughts {
                        continue;
                    }
                    if msg.action == "feels" && !self.show_feels {
                        continue;
                    }
                    if matches!(msg.action.as_str(), "gives_file" | "derezzes" | "jails" | "updates_relationship" | "reads" | "reads_dir" | "reads_web" | "assigned_task" | "delegates_task" | "ai_finished") {
                        // Filter out raw underlying data signals in the UI; system announcements or natural responses cover them
                        continue; 
                    }

                    let display_name = self.get_agent_display_name(&msg.sender);

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
                .default_size([400.0, 400.0])
                .open(&mut is_map_open)
                .show(ctx, |ui| {
                    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
                    let rect = response.rect;

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

                    let center = rect.center();
                    
                    // Draw user
                    painter.circle_filled(center, 8.0, Color32::YELLOW);
                    painter.text(center + egui::vec2(0.0, 15.0), egui::Align2::CENTER_CENTER, &self.user_name, FontId::monospace(14.0), Color32::YELLOW);

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
                                    let pos_i = center + egui::vec2(angle_i.cos() * radius, angle_i.sin() * radius);
                                    
                                    let angle_j = (j as f32 / num_agents as f32) * std::f32::consts::TAU;
                                    let pos_j = center + egui::vec2(angle_j.cos() * radius, angle_j.sin() * radius);

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
                        let pos = center + egui::vec2(angle.cos() * radius, angle.sin() * radius);

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