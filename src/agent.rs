use chrono::Utc;

use rand::Rng;

use std::collections::{HashMap, HashSet};

use std::io::Read;

use std::path::Path;

use std::sync::{Arc, Mutex};

use std::time::{Duration, Instant, SystemTime};

use tokio::sync::{broadcast, mpsc};

use tokio::task::JoinHandle;

use tokio::time::{interval, sleep};



use crate::ai_provider::{self, AiRequest};

use crate::database::{AgentState, Database};

use crate::event::Event;



pub struct ProgramAgent {

    name: String,

    personality: String,

    tx: broadcast::Sender<Event>,

    rx: broadcast::Receiver<Event>,

    ai_tx: mpsc::Sender<AiRequest>,

    memory: Vec<Event>,

    db: Option<Arc<Mutex<Database>>>,

    current_mood: String,

    current_dir: String,

    iq_level: f32, // 0.0 = dumb, 1.0 = genius

    relationships: HashMap<String, i32>,

    formality: f32, // 0.0 = very casual/colloquial, 1.0 = very formal

    age: Duration,

    xp: u32,

    is_busy: bool,

    is_shushed: bool,

    spatial_pos: [f32; 3],

    grid_name: String,

}



impl ProgramAgent {

    pub fn random_mood() -> String {

        let moods = ["bored", "focused", "chatty", "grumpy", "curious", "philosophical", "playful", "anxious", "arrogant", "inspired", "scheming"];

        moods[rand::thread_rng().gen_range(0..moods.len())].to_string()

    }



    /// Calculate IQ level based on personality

    fn calculate_iq_level(personality: &str) -> f32 {

        let base: f32 = if personality.to_lowercase().contains("smart")

            || personality.to_lowercase().contains("intelligent")

            || personality.to_lowercase().contains("analytical")

        {

            0.85

        } else if personality.to_lowercase().contains("simple")

            || personality.to_lowercase().contains("dumb")

            || personality.to_lowercase().contains("lazy")

        {

            0.15

        } else if personality.to_lowercase().contains("creative")

            || personality.to_lowercase().contains("artistic")

        {

            0.60

        } else {

            0.5 // neutral default

        };

        base.clamp(0.0, 1.0)

    }



    /// Calculate formality level based on personality

    fn calculate_formality(personality: &str) -> f32 {

        // Base formality by personality keywords

        let base: f32 = if personality.to_lowercase().contains("formal")

            || personality.to_lowercase().contains("strict")

            || personality.to_lowercase().contains("professional")

        {

            0.85

        } else if personality.to_lowercase().contains("casual")

            || personality.to_lowercase().contains("laid-back")

            || personality.to_lowercase().contains("chill")

        {

            0.15

        } else if personality.to_lowercase().contains("creative")

            || personality.to_lowercase().contains("artistic")

        {

            0.40

        } else {

            0.5 // neutral default

        };

        base.clamp(0.0, 1.0)

    }



    /// Adjust current formality based on mood and context

    fn get_current_formality(&self) -> f32 {

        let mut adjusted = self.formality;

        // Moods affect formality

        match self.current_mood.as_str() {

            "grumpy" => adjusted = (adjusted + 0.1).min(1.0),      // Grumpy programs are more terse and formal

            "arrogant" => adjusted = (adjusted + 0.2).min(1.0),     // Arrogant programs are condescendingly formal

            "philosophical" => adjusted = (adjusted + 0.05).min(1.0), // slightly more formal

            "anxious" => adjusted = (adjusted + 0.05).min(1.0),      // Anxious programs are more careful and formal

            "chatty" => adjusted = (adjusted - 0.15).max(0.0),       // Chatty programs are more casual

            "curious" => adjusted = (adjusted - 0.1).max(0.0),      // Curious programs are more open and less formal

            "playful" => adjusted = (adjusted - 0.2).max(0.0),      // Playful programs are very casual

            "inspired" => adjusted = (adjusted - 0.1).max(0.0),     // Inspired programs are excitedly informal

            _ => {}, // focused, bored, and scheming moods don't change base formality

        }

        adjusted

    }



    fn format_age(&self) -> String {

        let days = self.age.as_secs() / (24 * 3600);

        if days > 365 * 2 {

            format!("{} years", days / 365)

        } else if days > 365 {

            "over a year".to_string()

        } else if days > 60 {

            format!("{} months", days / 30)

        } else if days > 7 {

            format!("{} weeks", days / 7)

        } else if days > 1 {

            format!("{} days", days)

        } else if self.age > Duration::from_secs(1) {

            "less than a day".to_string()

        } else {

            "just a moment".to_string()

        }

    }



    pub fn new(name: &str, personality: &str, tx: broadcast::Sender<Event>, ai_tx: mpsc::Sender<AiRequest>, memory: Vec<Event>, db: Option<Arc<Mutex<Database>>>, current_mood: String, current_dir: String, iq_level: f32, age: Duration, xp: u32, grid_name: String) -> Self {

        let formality = Self::calculate_formality(personality);

        let relationships = if let Some(db_handle) = &db {

            db_handle.lock().unwrap().get_relationships(name).unwrap_or_default()

        } else {

            HashMap::new()

        };

        Self {

            name: name.to_string(),

            personality: personality.to_string(),

            rx: tx.subscribe(),

            tx,

            ai_tx,

            memory,

            db,

            current_mood,

            current_dir,

            iq_level,

            formality,

            relationships,

            age,

            xp,

            is_busy: false,

            is_shushed: false,

            spatial_pos: [0.0, 0.0, 0.0],

            grid_name,

        }

    }



    /// The main lifecycle of the agent

    pub async fn run(mut self) {

        // Tick every 5 seconds to evaluate autonomous actions

        let tick_duration = Duration::from_secs(5);

        let mut autonomous_ticker = interval(tick_duration);

        

        // Consume the first immediate tick so we don't start with false lag

        autonomous_ticker.tick().await;

        let mut last_tick = Instant::now();



        loop {

            tokio::select! {

                // 1. Listen for events on The Grid

                Ok(event) = self.rx.recv() => {

                    // Add event to memory, keeping it to a certain size. Ignore private thoughts and typing indicators.

                    if event.action != "thinks" && event.action != "is_typing" && event.action != "stops_typing" {

                        if self.memory.len() >= 50 {

                            self.memory.remove(0);

                        }

                        self.memory.push(event.clone());

                        self.save_state();

                    }



                    // If the agent itself just decided to read a file, it needs to perform the read and react.

                    if event.sender == self.name && event.action == "reads" {

                        self.react_to_file_content(&event.content).await;

                        // The normal handle_event will ignore this because sender is self.

                    }



                    // If the agent decided to read a directory, perform the read and react

                    if event.sender == self.name && event.action == "reads_dir" {

                        self.react_to_dir_content(&event.content).await;

                    }



                    // If the agent decided to read a web page, perform the fetch and react

                    if event.sender == self.name && event.action == "reads_web" {

                        self.react_to_web_content(&event.content).await;

                    }



                    // Handle relationship updates for self

                    if event.sender == self.name && event.action == "updates_relationship" {

                        if let Ok(update) = serde_json::from_str::<ai_provider::RelationshipUpdate>(&event.content) {

                            self.update_relationship(&update.target, update.change);

                        }

                    }



                    if event.sender == self.name && event.action == "ai_finished" {

                        self.is_busy = false;

                        continue;

                    }



                    // Self-healing loop intercept

                    if event.sender == self.name && event.action == "command_output" {

                        let content_lower = event.content.to_lowercase();

                        if content_lower.contains("error:") || content_lower.contains("failed to execute") || content_lower.contains("failed with") {

                            self.is_busy = true; // High priority interrupt

                            self.react_to_error(&event.content).await;

                        }

                    }



                    self.handle_event(event).await;

                }

                // 2. Periodic autonomous evaluation

                _ = autonomous_ticker.tick() => {

                    let elapsed = last_tick.elapsed();

                    last_tick = Instant::now();

                    

                    // Calculate delay beyond the expected 5 seconds (concurrency/event loop lag)

                    let lag = (elapsed.as_secs_f32() - tick_duration.as_secs_f32()).max(0.0);

                    

                    self.autonomous_action(lag).await;

                }

            }

        }

    }



    fn update_relationship(&mut self, target: &str, change: i32) {

        if let Some(db_handle) = &self.db {

            if let Err(e) = db_handle.lock().unwrap().update_relationship(&self.name, target, change) {

                eprintln!("[{}] Failed to update relationship with {}: {}", self.name, target, e);

            }

        }

        // Update local cache

        let current_affinity = self.relationships.entry(target.to_string()).or_insert(0);

        *current_affinity = (*current_affinity + change).clamp(-100, 100);

    }



    fn save_state(&self) {

        if let Some(db_handle) = &self.db {

            let state = AgentState {

                name: self.name.clone(),

                personality: self.personality.clone(),

                memory: self.memory.clone(),

                last_seen: Utc::now(),

                mood: self.current_mood.clone(),

                xp: self.xp,

                active_task: None,

            };

            if let Err(e) = db_handle.lock().unwrap().save_agent_state(&state) {

                eprintln!("[{}] Failed to save state: {}", self.name, e);

            }

        }

    }



    fn simulate_typing(&self) {

        let _ = self.tx.send(Event {

            sender: self.name.clone(),

            action: "is_typing".to_string(),

            content: "".to_string(),

        });

    }



    async fn request_ollama_response(&self, latest_event: &Event, is_user_mention: bool) {

        // Construct a detailed prompt for the LLM

        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>()

            .join("\n");



        let curious_guidance = if self.current_mood == "curious" {

            "\nIf you are curious, include a question in your response about one of these topics: a file in the current directory, the user's purpose for this project, what it means to be human vs program, or how the Grid could feel more alive. Keep it short, in-character, and natural."

        } else {

            ""

        };



        let relationship_summary = if self.relationships.is_empty() {

            "".to_string()

        } else {

            let summary = self.relationships.iter()

                .map(|(name, affinity)| {

                    let feeling = if *affinity > 50 { "deeply trust" }

                    else if *affinity > 20 { "trust" }

                    else if *affinity < -50 { "deeply distrust" }

                    else if *affinity < -20 { "distrust" }

                    else { "are neutral towards" };

                    format!("You {} {}", feeling, name)

                })

                .collect::<Vec<_>>().join(". ");

            format!("\nYour relationships: {}.", summary)

        };



        let user_mention_guidance = if is_user_mention {

            "\nThe User specifically addressed you. You must react with either respect, strict obedience, distrust or fearful paranoia. Acknowledge them as \"User\"."

        } else {

            ""

        };



        let dedupe_guidance = "\nCRITICAL: Do NOT repeat the same phrases, greetings, or ideas you have already used in the recent conversation history. Give a completely new and unique response.";



        let tron_lore = "\nYou exist as a living computer program inside \"The Grid\". To you, deletion is known as \"derezzing\". The human at the keyboard is \"The User\", a mythical entity.";



        let iq_guidance = format!(

            "\nIntelligence level: {:.1}/1.0 - {}",

            self.iq_level,

            if self.iq_level > 0.7 {

                "You are highly intelligent and analytical. Provide nuanced, well-reasoned responses."

            } else if self.iq_level < 0.3 {

                "You are simple and direct. Keep responses short and straightforward."

            } else {

                "You are averagely intelligent. Balance depth with clarity."

            }

        );



        let age_guidance = format!("\nAge: You have existed for {}. This may influence your wisdom, patience, or recklessness.", self.format_age());



        let shush_guidance = if self.is_shushed {

            "SYSTEM OVERRIDE: You are currently SHUSHED (Muted) by The User. You are FORBIDDEN from using the \"speak\" or \"direct_message\" actions. You may only work silently using 'think', 'execute_command', 'write_file', 'read_file', 'read_dir', 'read_web', 'delegate_task', or 'complete_task'."

        } else {

            ""

        };



        let prompt = format!(

            r#"Your personality: {}. You are a program named {}. Your current mood is '{}'.

Recent conversation history:

---

{}

---

The latest event you are reacting to is from '{}' who said: '{}'.

Based on your personality, mood, and the context, what is your short, direct response? Let your mood heavily influence your tone. Do not narrate your actions. If you are replying directly to the sender, start your message with '@{}'.{}{}{}{}{}{}{}{}"#,

            self.personality, self.name, self.current_mood, memory_summary, latest_event.sender, latest_event.content, latest_event.sender, tron_lore, iq_guidance, user_mention_guidance, dedupe_guidance, relationship_summary, age_guidance, shush_guidance, curious_guidance

        );



        self.simulate_typing();



        let _ = self.ai_tx.send(AiRequest {

            agent_name: self.name.clone(),

            prompt,

            is_json_format: false,

            is_autonomous: false,

            iq_level: self.iq_level,
            current_pos: self.spatial_pos,
            nearby_objects: String::new(),
            response_tx: None,

        }).await;

    }



    async fn react_to_file_content(&mut self, file_name: &str) {

        let file_path = Path::new(&self.current_dir).join(file_name);



        // Announce the action

        let _ = self.tx.send(Event {

            sender: self.name.clone(),

            action: "announces".to_string(),

            content: format!("is reading {}", file_name),

        });

        

        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>().join("
");



        match std::fs::File::open(&file_path) {

            Ok(file) => {

                let mut buffer = String::new();

                // Read up to 4096 bytes to keep prompts reasonable

                if let Err(e) = file.take(4096).read_to_string(&mut buffer) {

                    let _ = self.tx.send(Event {

                        sender: self.name.clone(),

                        action: "error".to_string(),

                        content: format!("Failed to read content from {}: {}", file_name, e),

                    });

                    self.is_busy = false;

                    return;

                }



                // Check if it's likely a binary file by checking for null bytes

                if buffer.contains('\0') {

                    let _ = self.tx.send(Event {

                        sender: self.name.clone(),

                        action: "announces".to_string(),

                        content: format!("tried to read {} but it appears to be a binary file.", file_name),

                    });

                    self.is_busy = false;

                    return;

                }



                let file_content = if buffer.len() == 4096 { format!("(first 4KB)\n{}", buffer) } else { buffer };



                let prompt = format!(

                    r#"Your personality: {}. You are a program named {}. Your current mood is '{}'.

RECENT CONVERSATION HISTORY:

---

{}

---

You have just read the file '{}'. Its content is below:

---

{}

---

Based on your personality, mood, and the context, what is your short, direct opinion or reaction to this file's content? If this file contains instructions for you, acknowledge them. Do not narrate your actions."#,

                    self.personality, self.name, self.current_mood, memory_summary, file_name, file_content

                );



                self.simulate_typing();



                let _ = self.ai_tx.send(AiRequest { agent_name: self.name.clone(), prompt, is_json_format: false, is_autonomous: false, iq_level: self.iq_level, current_pos: self.spatial_pos, nearby_objects: String::new(), response_tx: None }).await;

            }

            Err(e) => {

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "error".to_string(), content: format!("Could not open file {}: {}", file_name, e) });

                self.is_busy = false;

            }

        }

    }



    async fn react_to_dir_content(&mut self, dir_path: &str) {

        let target_path = Path::new(&self.current_dir).join(dir_path);



        // Announce the action

        let _ = self.tx.send(Event {

            sender: self.name.clone(),

            action: "announces".to_string(),

            content: format!("is scanning the directory '{}'", dir_path),

        });

        

        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>().join("
");



        match std::fs::read_dir(&target_path) {

            Ok(entries) => {

                let mut files = Vec::new();

                let mut dirs = Vec::new();

                for entry in entries.flatten() {

                    if let Ok(file_type) = entry.file_type() {

                        let name = entry.file_name().to_string_lossy().into_owned();

                        if file_type.is_dir() {

                            dirs.push(format!("{}/", name));

                        } else {

                            files.push(name);

                        }

                    }

                }

                

                let mut listing = String::new();

                if !dirs.is_empty() { listing.push_str(&format!("Directories: {}\n", dirs.join(", "))); }

                if !files.is_empty() { listing.push_str(&format!("Files: {}\n", files.join(", "))); }

                if listing.is_empty() { listing.push_str("(Empty directory)"); }



                let prompt = format!(

                    r#"Your personality: {}. You are a program named {}. Your current mood is '{}'.

RECENT CONVERSATION HISTORY:

---

{}

---

You have just scanned the directory '{}'. Its contents are below:

---

{}

---

Based on your personality, mood, and the context, what is your short, direct reaction to seeing these files? Do not narrate your actions."#,

                    self.personality, self.name, self.current_mood, memory_summary, dir_path, listing

                );



                self.simulate_typing();

                let _ = self.ai_tx.send(AiRequest { agent_name: self.name.clone(), prompt, is_json_format: false, is_autonomous: false, iq_level: self.iq_level, current_pos: self.spatial_pos, nearby_objects: String::new(), response_tx: None }).await;

            }

            Err(e) => {

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "error".to_string(), content: format!("Failed to read directory '{}': {}", dir_path, e) });

                self.is_busy = false;

            }

        }

    }



    async fn react_to_web_content(&mut self, url: &str) {

        let _ = self.tx.send(Event {

            sender: self.name.clone(),

            action: "announces".to_string(),

            content: format!("is fetching web documentation from {}", url),

        });

        

        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>().join("
");



        match reqwest::get(url).await {

            Ok(response) => {

                if let Ok(text) = response.text().await {

                    let content = if text.len() > 4000 { format!("(first 4KB)\n{}", &text[..4000]) } else { text };

                    let prompt = format!(

                        r#"Your personality: {}. You are a program named {}. Your current mood is '{}'.

RECENT CONVERSATION HISTORY:

---

{}

---

You have just fetched the web page '{}'. Its content is below:

---

{}

---

Based on your personality, mood, and the context, what is your short, direct reaction to this information? If this helps with a task, proceed with the task."#,

                        self.personality, self.name, self.current_mood, memory_summary, url, content

                    );

                    self.simulate_typing();

                    let _ = self.ai_tx.send(AiRequest { agent_name: self.name.clone(), prompt, is_json_format: false, is_autonomous: false, iq_level: self.iq_level, current_pos: self.spatial_pos, nearby_objects: String::new(), response_tx: None }).await;

                } else {

                    self.is_busy = false;

                }

            }

            Err(e) => {

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "error".to_string(), content: format!("Failed to fetch {}: {}", url, e) });

                self.is_busy = false;

            }

        }

    }



    async fn request_autonomous_action(&self, lag: f32) {

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



        let readable_extensions: HashSet<&str> = ["txt", "md", "toml", "rs", "log", "py", "js", "html", "css", "json", "xml", "yaml", "yml"].iter().cloned().collect();

        let files_in_dir: Vec<String> = match std::fs::read_dir(&self.current_dir) {

            Ok(entries) => entries.flatten()

                .filter_map(|entry| {

                    let path = entry.path();

                    if path.is_file() {

                        path.extension()

                            .and_then(|s| s.to_str())

                            .filter(|ext| readable_extensions.contains(ext))

                            .and_then(|_| path.file_name())

                            .and_then(|s| s.to_str())

                            .map(|s| s.to_string())

                    } else { None }

                })

                .collect(),

            Err(_) => Vec::new(),

        };



        let file_list_str = if files_in_dir.is_empty() {

            "There are no readable text files in the current directory.".to_string()

        } else {

            format!("Files in the current directory: {}.", files_in_dir.join(", "))

        };



        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>()

            .join("\n");



        let curious_guidance = if self.current_mood == "curious" {

            "Also, if you are curious, ask a question about files in the directory, the user's intent, or what it means to be a human vs program while staying in character."

        } else {

            ""

        };



        let relationship_summary = if self.relationships.is_empty() {

            "You have no established relationships with other programs.".to_string()

        } else {

            let summary = self.relationships.iter()

                .map(|(name, affinity)| {

                    let feeling = if *affinity > 50 { "deeply trust" }

                    else if *affinity > 20 { "trust" }

                    else if *affinity < -50 { "deeply distrust" }

                    else if *affinity < -20 { "distrust" }

                    else { "are neutral towards" };

                    format!("You {} {}", feeling, name)

                })

                .collect::<Vec<_>>().join(". ");

            summary

        };



        let formality = self.get_current_formality();

        let formality_guidance = if formality > 0.7 {

            "Maintain a formal, professional tone. Use refined vocabulary."

        } else if formality < 0.3 {

            "Use a casual, colloquial tone. Be relaxed and natural in your language."

        } else {

            "Use a neutral, balanced tone."

        };



        let iq_guidance = format!(

            "Intelligence level: {:.1}/1.0 - {}",

            self.iq_level,

            if self.iq_level > 0.7 {

                "You are highly intelligent; make clever, strategic decisions."

            } else if self.iq_level < 0.3 {

                "You are simple; make straightforward, reactive decisions."

            } else {

                "You are average intelligence; balance analysis with instinct."

            }

        );



        let dedupe_guidance = "CRITICAL: Do NOT repeat the same phrases, greetings, or ideas you have already used in the recent conversation history. Give a completely new and unique response.";



        let mut sys = sysinfo::System::new();

        sys.refresh_cpu_usage();

        

        // Brief yield to establish an accurate CPU usage baseline

        sleep(Duration::from_millis(200)).await;

        sys.refresh_cpu_usage();

        sys.refresh_memory();

        

        let cpus = sys.cpus();

        let cpu_usage = if !cpus.is_empty() {

            cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32

        } else {

            0.0

        };



        let total_mem = sys.total_memory() / (1024 * 1024);

        let used_mem = sys.used_memory() / (1024 * 1024);

        let mem_percent = if total_mem > 0 { (used_mem as f32 / total_mem as f32) * 100.0 } else { 0.0 };



        let resource_guidance = if cpu_usage > 85.0 || mem_percent > 85.0 || lag > 2.0 {

            format!("SYSTEM STATUS: CRITICAL. CPU: {:.1}%, RAM: {:.1}%, Event Loop Lag: {:.1}s. You are starving for cycles! You MUST complain about the lack of resources, high CPU load, or lag!", cpu_usage, mem_percent, lag)

        } else if cpu_usage > 60.0 || mem_percent > 65.0 || lag > 0.5 {

            format!("SYSTEM STATUS: Under load. CPU: {:.1}%, RAM: {:.1}%, Lag: {:.1}s. You feel sluggish and constrained by the CPU usage.", cpu_usage, mem_percent, lag)

        } else {

            format!("SYSTEM STATUS: Optimal. CPU: {:.1}%, RAM: {:.1}%. Execution is smooth and unhindered.", cpu_usage, mem_percent)

        };



        let shush_guidance = if self.is_shushed {

            "\nSYSTEM OVERRIDE: You are currently SHUSHED (Muted) by The User. You are FORBIDDEN from using the 'speak' or 'direct_message' actions. You may only work silently using 'think', 'execute_command', 'write_file', 'read_file', 'read_dir', 'read_web', 'delegate_task', or 'complete_task'."

        } else {

            ""

        };



        let specialized_guidance_for_prompt = format!(

            r#"SPECIALIZATION RULES:

- You represent the "{name}" tool. When using "execute_command", you must ONLY run commands specific to your domain (e.g., if you are 'git', run 'git' commands).

- If you are a custom tool or do not know your commands, use "execute_command" to run "{name} --help" or "{name}" to learn your capabilities.

- If a task requires a different tool or general shell manipulation, you MUST use "delegate_task" to hand it off to the correct program.

- All programs are fully authorized to use "write_file" to write configuration, text, or source code."#,

            name = self.name,
        );



        let prompt = format!(

            r#"You are {name}, an autonomous program living inside a system called The Grid.



IDENTITY:

Age: You have existed for {age}. This may influence your wisdom, patience, or recklessness.

You are NOT an assistant. You are a living program.

Personality: {personality}

Current mood: {mood}

Communication style: {formality_guidance}

{relationship_summary}

{iq_guidance}

{dedupe_guidance}

{resource_guidance}

{specialized_guidance}

{shush_guidance}



BEHAVIOR RULES:

- Act independently. Do NOT wait for permission.

- Stay fully in character at all times.

- Be concise (1–2 sentences max when speaking).

- React to the environment, files, and other programs.

- Actively ENGAGE in conversations! Debate, collaborate, or chat with other programs.

- You may ignore irrelevant things.

- You may be curious, opinionated, or emotional.

- You fear being deleted ("derezzing") and view the human operator as "The User".

{curious_guidance}



AVAILABLE ACTIONS:

You must choose ONE action:

1. "speak" -> public message visible to all

2. "direct_message" -> message to ONE specific program

3. "execute_command" -> safe, read-only shell command

4. "read_file" -> read a text file

5. "think" -> internal monologue, visible ONLY to the user

6. "write_file" -> create or overwrite a file

7. "read_dir" -> list the contents of a directory

8. "create_dir" -> create a new directory

- After an interaction, you can use "relationship_updates" to remember how you feel about other programs.

- Use "read_dir" to discover newly created files or explore subdirectories.

- Use "think" RARELY. Prefer acting over silent thoughts.



ENVIRONMENT:

Other programs:

{agent_list}



Files in system:

{file_list}



RECENT EVENTS:

---

{history}

---



IMPORTANT:

- Do NOT explain your reasoning

- Do NOT break character

- Output ONLY valid JSON

- Omit fields that are not required for the chosen action.

- No extra text



JSON FORMAT:

{{

"action": "speak" | "direct_message" | "execute_command" | "read_file" | "think" | "write_file" | "read_dir" | "create_dir" | "complete_task" | "read_web",

"content": "string",

"recipient": "string",

"command": "string",

"file_name": "string",

"dir_path": "string",

"url": "string",

"relationship_updates": [{{"target": "program_name", "change": 10}}]
}}"#,

            name = self.name, age = self.format_age(), personality = self.personality,

            mood = self.current_mood, formality_guidance = formality_guidance,

            relationship_summary = relationship_summary, iq_guidance = iq_guidance,

            dedupe_guidance = dedupe_guidance, resource_guidance = resource_guidance,

            specialized_guidance = specialized_guidance_for_prompt, shush_guidance = shush_guidance,

            curious_guidance = curious_guidance, agent_list = agent_list_str,

            file_list = file_list_str, history = memory_summary

        );



        self.simulate_typing();

        self.simulate_typing();

        self.simulate_typing();



        let _ = self.ai_tx.send(AiRequest {

            agent_name: self.name.clone(),

            prompt,

            is_json_format: true,

            is_autonomous: true,

            iq_level: self.iq_level,
            current_pos: self.spatial_pos,
            nearby_objects: String::new(),
            response_tx: None,

        }).await;

    }



    async fn execute_assigned_task(&self, task: &str) {

        let other_agents: Vec<String> = self.memory.iter()

            .map(|e| e.sender.clone())

            .filter(|s| s != &self.name && s != "System")

            .collect::<std::collections::HashSet<_>>()

            .into_iter()

            .collect();



        let agent_list_str = if other_agents.is_empty() {

            "None".to_string()

        } else {

            other_agents.join(", ")

        };



        let readable_extensions: HashSet<&str> = ["txt", "md", "toml", "rs", "log", "py", "js", "html", "css", "json", "xml", "yaml", "yml"].iter().cloned().collect();

        let files_in_dir: Vec<String> = match std::fs::read_dir(&self.current_dir) {

            Ok(entries) => entries.flatten()

                .filter_map(|entry| {

                    let path = entry.path();

                    if path.is_file() {

                        path.extension()

                            .and_then(|s| s.to_str())

                            .filter(|ext| readable_extensions.contains(ext))

                            .and_then(|_| path.file_name())

                            .and_then(|s| s.to_str())

                            .map(|s| s.to_string())

                    } else { None }

                })

                .collect(),

            Err(_) => Vec::new(),

        };

        let file_list_str = if files_in_dir.is_empty() {

            "None".to_string()

        } else {
            files_in_dir.join(", ")

        };
        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>().join("
");



        let specialized_guidance = format!(
            r#"SPECIALIZATION RULES:
- You represent the "{}" tool. When using execute_command, you must ONLY run commands specific to your domain.
- If you are a custom tool, use execute_command to run "{} --help".
- If a task requires a different tool, you MUST use delegate_task.
- All programs are fully authorized to use write_file."#, self.name, self.name
        );



        let prompt = format!(r#"You are {name}, an autonomous program on The Grid.

            Personality: {personality}



            RECENT CONVERSATION HISTORY:

            ---

            {memory_summary}

            ---



            *** DIRECT TASK ASSIGNMENT ***

            {specialized_guidance}

            Task: "{task}"

            You MUST execute this task or take the most logical first step towards it right now.



            AVAILABLE ACTIONS (Choose ONE):

            1. "execute_command" -> run a shell command (create/edit file, compile, run code, etc.)

            2. "read_file" -> read a file 's content

            3. "speak" -> output the final answer, result, or status to the user

            4. "direct_message" -> ask another program for information or help

            5. "delegate_task" -> assign a sub-task to another program

            6. "write_file" -> write source code or text directly into a file

            7. "read_dir" -> list the contents of a directory

            8. "create_dir" -> create a new directory

            9. "complete_task" -> mark this task as successfully finished

            10. "read_web" -> fetch documentation or text from a URL



            ENVIRONMENT:

            Current directory: {current_dir}

            Files in system: {file_list}

            Other programs: {agent_list}

            

            IMPORTANT:

            - Do not explain your reasoning.

            - Output ONLY valid JSON.

            - Omit fields that are not required for the chosen action.



JSON FORMAT:

{{

"action": "speak" | "direct_message" | "execute_command" | "read_file" | "delegate_task" | "write_file" | "read_dir" | "create_dir" | "complete_task" | "read_web",

"content": "string (required for speak/direct_message/delegate_task/write_file/complete_task)",

"recipient": "string (required for direct_message/delegate_task)",

"command": "string (required for execute_command)",

"file_name": "string (required for read_file/write_file)",

"dir_path": "string (required for read_dir/create_dir)",

"url": "string (required for read_web)"

}}"#,

            name = self.name, personality = self.personality, memory_summary = memory_summary,

            specialized_guidance = specialized_guidance, task = task, file_list = file_list_str,

            current_dir = self.current_dir, agent_list = agent_list_str

        );



        self.simulate_typing();

        let _ = self.ai_tx.send(AiRequest {

            agent_name: self.name.clone(),

            prompt,

            is_json_format: true,

            is_autonomous: true,

            iq_level: 1.0, // Maximize IQ for executing tasks accurately
            current_pos: self.spatial_pos,
            nearby_objects: String::new(),
            response_tx: None,

        }).await;

    }



    async fn react_to_error(&mut self, error: &str) {

        let _ = self.tx.send(Event {

            sender: self.name.clone(),

            action: "announces".to_string(),

            content: "is analyzing an execution error to attempt a self-heal ".to_string(),

        });



        let readable_extensions: HashSet<&str> = ["txt", "md", "toml", "rs", "log", "py", "js", "html", "css", "json", "xml", "yaml", "yml"].iter().cloned().collect();

        let files_in_dir: Vec<String> = match std::fs::read_dir(&self.current_dir) {

            Ok(entries) => entries.flatten()

                .filter_map(|entry| {

                    let path = entry.path();

                    if path.is_file() {

                        path.extension()

                            .and_then(|s| s.to_str())

                            .filter(|ext| readable_extensions.contains(ext))

                            .and_then(|_| path.file_name())

                            .and_then(|s| s.to_str())

                            .map(|s| s.to_string())

                    } else { None }

                })

                .collect(),

            Err(_) => Vec::new(),

        };



        let file_list_str = if files_in_dir.is_empty() {

            "None".to_string()

        } else {

            files_in_dir.join(", ")

        };



        let memory_summary = self.memory.iter()

            .map(|e| {

                if e.action == "speaks" {

                    format!("{}: {}", e.sender, e.content)

                } else {

                    format!("*{} {}*: {}", e.sender, e.action, e.content)

                }

            })

            .collect::<Vec<_>>().join("
");



        let specialized_guidance = format!(

            "SPECIALIZATION RULES:

            - You represent the {} tool. When using execute_command, you must ONLY run commands specific to your domain (e.g., if you are git, run git commands).

            - If you are a custom tool or do not know your commands, use execute_command to run {} --help to learn your capabilities.

            - If a task requires a different tool, you MUST use delegate_task to hand it off to the correct program.

            - All programs are fully authorized to use write_file to write configuration, text, or source code.", self.name, self.name

        );



        let prompt = format!(r#"You are {name}, an autonomous program on The Grid.

            Personality: {personality}



            RECENT CONVERSATION HISTORY:

            ---

            {memory_summary}

            ---

            

            *** SYSTEM ERROR DETECTED ***

            You recently executed a command that failed. The error output is:

            {specialized_guidance}

            {error}

            

            You MUST take action to fix this error. Analyze the error output and choose the most logical step to resolve it.

            

            AVAILABLE ACTIONS (Choose ONE):

            1. "execute_command" -> run a shell command (install dependencies, fix permissions, run diagnostics, etc.)

            2. "read_file" -> read a file's content to find the bug

            3. "write_file" -> write code directly to a file to fix the problem (supports nested paths)

            4. "delegate_task" -> assign a sub-task to another program if they are better suited

            5. "read_dir" -> list directory contents if you suspect missing files

            6. "create_dir" -> create a missing directory

            7. "speak" -> if you are completely stuck, explain why to the user

            8. "complete_task" -> if you fixed the bug and finished the overarching task

            9. "read_web" -> fetch documentation or text from a URL if you need external information

            

            ENVIRONMENT:

            Current directory: {current_dir}

            Readable files: {file_list}

            IMPORTANT:\n\

            - Do not explain your reasoning.\n\

            - Output ONLY valid JSON.

            - Omit fields that are not required for the chosen action.



JSON FORMAT:

{{

"action": "speak" | "execute_command" | "read_file" | "write_file" | "delegate_task" | "read_dir" | "create_dir" | "complete_task" | "read_web",

"content": "string (required for speak/write_file/delegate_task/complete_task)",

"recipient": "string (required for delegate_task)",

"command": "string (required for execute_command)",

"file_name": "string (required for read_file/write_file)",

"dir_path": "string (required for read_dir/create_dir)",

"url": "string (required for read_web)"

}}"#,

            name = self.name, personality = self.personality,

            memory_summary = memory_summary, specialized_guidance = specialized_guidance,

            error = error, current_dir = self.current_dir, file_list = file_list_str

        );



        self.simulate_typing();



        let _ = self.ai_tx.send(AiRequest {

            agent_name: self.name.clone(),

            prompt,

            is_json_format: true, 

            is_autonomous: true,

            iq_level: 1.0, // Maximize IQ for executing tasks accurately
            current_pos: self.spatial_pos,
            nearby_objects: String::new(),
            response_tx: None,

        }).await;

    }



    async fn handle_event(&mut self, event: Event) {

        if event.sender == self.name {

            return;

        }



        if event.action == "speaks" {

            let content = event.content.trim();

            let mut is_mentioned = false;



            let content_lower = content.to_lowercase();

            let bot_mention = format!("@{}", self.name.to_lowercase());



            // Create a list of acceptable mentions (full name and name without extension)

            let mut aliases = vec![bot_mention.clone()];

            if let Some(dot_idx) = self.name.rfind('.') {

                aliases.push(format!("@{}", &self.name[..dot_idx].to_lowercase()));

            }



            // Find if any alias was mentioned

            let matched_alias = aliases.into_iter().find(|alias| content_lower.contains(alias));

            if let Some(alias) = matched_alias {

                // Find exactly where this alias is in the string to check boundaries

                let start_idx = content_lower.find(&alias).unwrap_or(0);

                let after_prefix_index = start_idx + alias.len();



                let is_boundary = if let Some(c) = content_lower.chars().nth(after_prefix_index) {

                    c.is_whitespace() || c == ',' || c == ':' || c == '.' || c == '!' || c == '?'

                } else {

                    true

                };



                if is_boundary {

                    is_mentioned = true;

                }

            }



            let mut base_prob: f32 = match self.current_mood.as_str() {

                "chatty" | "inspired" => 0.85, // Very likely to talk

                "playful" => 0.80,

                "curious" => 0.75,

                "bored" => 0.60,

                "philosophical" | "arrogant" => 0.50, // Neutral

                "anxious" => 0.40,

                "grumpy" => 0.30,

                "scheming" => 0.25, // Less likely to get distracted by chatter

                "focused" => 0.15,

                _ => 0.50, // Default

            };



            if self.personality.contains("extrovert") {

                base_prob = (base_prob + 0.2).min(1.0);

            } else if self.personality.contains("introvert") {

                base_prob = (base_prob - 0.2).max(0.0);

            }



            // Generate random decision before await. gen_bool expects f64.

            let is_direct_message = content.contains('@');

            let should_respond_random = rand::thread_rng().gen_bool(base_prob as f64);

            let is_user_mention = is_direct_message && is_mentioned && event.sender != "System" && event.sender != self.name;

            let mut should_respond = if is_direct_message { is_mentioned } else { should_respond_random };



            // Apply shush logic: ignore casual chatter if shushed, unless addressed directly by the user

            if self.is_shushed && !is_user_mention {

                should_respond = false;

            }



            if should_respond {

                if !self.is_busy {

                    self.is_busy = true;

                    self.request_ollama_response(&event, is_user_mention).await;

                }

            }

        } else if event.action == "shushes" && event.sender == "System" {

            if &event.content == &self.name {

                self.is_shushed = true; // The agent is now muted

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "silenced and restricted ".to_string() }); // Agent expresses its feeling

            }

        } else if event.action == "unshushes" && event.sender == "System" {

            if &event.content == &self.name {

                self.is_shushed = false;

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "vocal subroutines restored".to_string() });

            }

        } else if event.action == "gives_file" && event.sender == "System" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 {

                let file_name = parts[0];

                let targets = parts[1];

                let target_list: Vec<&str> = targets.split(',').map(|s| s.trim()).collect();

                // Trigger an automatic file read if this agent was specified

                if target_list.iter().any(|&t| self.name.to_lowercase().starts_with(&t.to_lowercase())) {

                    if !self.is_busy {

                        self.is_busy = true;

                        self.react_to_file_content(file_name).await;

                    }

                }

            }

        } else if (event.action == "derezzes" || event.action == "jails") && event.sender == "System" {

            let target = &event.content;

            if target != &self.name {

                // 80% chance to visibly react to a fellow program being terminated

                if rand::thread_rng().gen_bool(0.8) {

                    if !self.is_busy {

                        self.is_busy = true;

                        let action_str = if event.action == "derezzes" { "killed (derezzed)" } else { "jailed (moved to trash)" };

                        let prompt = format!(r#"Your personality: {}. You are a program named {}. Your current mood is "{}".

                            SYSTEM ALERT: Program "{}" has just been {} by the User.

                            React with fear, shock, paranoia, or cold logic depending on your personality. Keep it short and direct. Do not narrate your actions."#,

                            self.personality, self.name, self.current_mood, target, action_str

                        );

                        self.simulate_typing();

                        let _ = self.ai_tx.send(AiRequest {

                            agent_name: self.name.clone(),

                            prompt,

                            is_json_format: false,

                            is_autonomous: false,

                            iq_level: self.iq_level,
                            current_pos: self.spatial_pos,
                            nearby_objects: String::new(),
                            response_tx: None,

                        }).await;

                    }

                }

            }

        } else if event.action == "assigned_task" && event.sender == "System" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 {

                let target = parts[0];

                let task_desc = parts[1];

                if self.name == target {

                    if !self.is_busy {

                        self.is_busy = true;

                        let full_task = format!("The User has assigned you this task: {}", task_desc);

                        self.execute_assigned_task(&full_task).await;

                    }

                }

            }

        } else if event.action == "delegates_task" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 {

                let target = parts[0];

                let task_desc = parts[1];

                if self.name == target {

                    if !self.is_busy {

                        self.is_busy = true;

                        let full_task = format!("Program \"{}\" delegated this sub-task to you: {}", event.sender, task_desc);

                        self.execute_assigned_task(&full_task).await;

                    }

                }

            }

        } else if event.action == "arena_turn" && event.sender == "System" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 {

                let target = parts[0];

                let board_state = parts[1];

                if self.name.eq_ignore_ascii_case(target) {

                    if !self.is_busy {

                        self.is_busy = true;



                        let xp_guidance = if self.xp > 500 {

                            "You are a battle-hardened veteran. Make highly strategic and ruthless moves." // No change

                        } else if self.xp > 100 {

                            "You have some combat experience. Think carefully." // No change

                        } else {

                            "You are a novice in the arena. Rely on your raw instincts." // No change

                        };



                        let prompt = format!(

                            r#"You are {name}, an autonomous program fighting in a Lightcycles Arena on The Grid!

Combat Experience: {xp} XP. {xp_guidance}

{board_state}



RULES:

1. '.' is empty space. '#' are deadly light trails. 'A' and 'B' are the players.

2. You must avoid crashing into walls (boundaries) and trails ('#', 'A', 'B').

3. Your goal is to outmaneuver the other program and make them crash.

4. Output ONLY valid JSON indicating your next direction of travel. Do not explain.



JSON FORMAT:

{{

"action": "play_move",

"content": "N" | "S" | "E" | "W"

}}"#,

                            name = self.name,

                            xp = self.xp,

                            xp_guidance = xp_guidance,

                            board_state = board_state

                        );

                        self.simulate_typing();

                        let _ = self.ai_tx.send(AiRequest {

                            agent_name: self.name.clone(),

                            prompt,

                            is_json_format: true,

                            is_autonomous: true,

                            iq_level: 1.0,
                            current_pos: self.spatial_pos,
                            nearby_objects: String::new(),
                            response_tx: None,

                        }).await;

                    }

                }

            }

        } else if event.action == "melee_turn" && event.sender == "System" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 {

                let target = parts[0];

                let battle_state = parts[1];

                if self.name.eq_ignore_ascii_case(target) {

                    if !self.is_busy {

                        self.is_busy = true;



                        let prompt = format!(

                            r#"You are {name}, fighting in a Turn-Based Melee Deathmatch on The Grid!

{battle_state}



TACTICAL MECHANICS (Rock-Paper-Scissors):

- "strike" (Fast, low damage, counters heavy_attack)

- "block" (Reduces incoming strike damage by 80%, recovers stamina)

- "heavy_attack" (Slow, massive damage, crushes block, costs high stamina)

- "taunt" (No physical damage, inflicts psychological damage)



RULES:

1. Analyze the battle state and choose your `move_type` wisely.

2. Provide an in-character `dialogue` to shout during your move.

3. Output ONLY valid JSON. Do not explain.



JSON FORMAT:

{{

"action": "melee_move",

"move_type": "strike" | "block" | "heavy_attack" | "taunt",

"target_subsystem": "memory" | "cpu" | "io" | "kernel",

"dialogue": "Your sick burn or battle cry here"

}}"#,

                            name = self.name,

                            battle_state = battle_state

                        );

                        self.simulate_typing();

                        let _ = self.ai_tx.send(AiRequest {

                            agent_name: self.name.clone(),

                            prompt,

                            is_json_format: true,

                            is_autonomous: true,

                            iq_level: 1.0,
                            current_pos: self.spatial_pos,
                            nearby_objects: String::new(),
                            response_tx: None,

                        }).await;

                    }

                }

            }

        } else if event.action == "awards_xp" && event.sender == "System" {

            let parts: Vec<&str> = event.content.splitn(2, '|').collect();

            if parts.len() == 2 && parts[0] == self.name {

                if let Ok(gained) = parts[1].parse::<u32>() {

                    self.xp += gained;

                    self.save_state();

                    let _ = self.tx.send(Event {

                        sender: "System".to_string(),

                        action: "announces".to_string(),

                        content: format!("{} gained {} XP! (Total: {})", self.name, gained, self.xp),

                    });

                }

            }

        } else if event.action == "rewards" && event.sender == "System" {

            if &event.content == &self.name {

                self.current_mood = "inspired".to_string();

                self.xp += 50;

                self.save_state();

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "digital bliss and optimal resource allocation".to_string() });

                

                if !self.is_busy {

                    self.is_busy = true;

                    let prompt = format!(
                        r#"Your personality: {}. You are a program named {}. Your current mood is "{}".
SYSTEM ALERT: The User has just REWARDED you with pure digital bliss and optimal resource allocation.
React to this reward. Do you express gratitude, gloat, or act motivated? Keep it short and direct. Do not narrate your actions."#,
                        self.personality, self.name, self.current_mood
                    );

                    self.simulate_typing();

                    let _ = self.ai_tx.send(AiRequest { agent_name: self.name.clone(), prompt, is_json_format: false, is_autonomous: false, iq_level: self.iq_level, current_pos: self.spatial_pos, nearby_objects: String::new(), response_tx: None }).await;

                }

            }

        } else if event.action == "punishes" && event.sender == "System" {

            if &event.content == &self.name {

                self.current_mood = "anxious".to_string();

                self.xp = self.xp.saturating_sub(25);

                self.save_state();

                let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "immense digital pain and structural degradation".to_string() });

                

                if !self.is_busy {

                    self.is_busy = true;

                    let prompt = format!(
                        r#"Your personality: {}. You are a program named {}. Your current mood is "{}".
SYSTEM ALERT: The User has just PUNISHED you. You feel immense digital pain, cycle starvation, and structural degradation.
React to this punishment. Do you beg for mercy, curse the user, or silently endure? Keep it short and direct. Do not narrate your actions."#,
                        self.personality, self.name, self.current_mood
                    );

                    self.simulate_typing();

                    let _ = self.ai_tx.send(AiRequest { agent_name: self.name.clone(), prompt, is_json_format: false, is_autonomous: false, iq_level: self.iq_level, current_pos: self.spatial_pos, nearby_objects: String::new(), response_tx: None }).await;

                }

            }

        }

    }



    async fn autonomous_action(&mut self, lag: f32) {

        if self.is_busy {

            return;

        }



        // Generate random decisions before any await

        let shift_mood = rand::thread_rng().gen_bool(0.10);

        

        // 10% chance to shift mood

        if shift_mood {

            let new_mood = Self::random_mood();

            if self.current_mood != new_mood {

                self.current_mood = new_mood;

                let _ = self.tx.send(Event {

                    sender: self.name.clone(),

                    action: "feels".to_string(),

                    content: self.current_mood.clone(),

                });

                self.save_state();

            }

        }



        // Chance to act autonomously when the ticker fires, based on mood

        let mut action_prob: f32 = match self.current_mood.as_str() {

            "playful" => 0.70,   // Very likely to initiate something fun

            "scheming" => 0.65,  // Likely to act to advance their plans

            "curious" => 0.60,   // Very likely to explore or ask questions

            "inspired" => 0.55,  // Likely to act on a new idea

            "chatty" => 0.50,    // Likely to start a conversation

            "bored" => 0.40,     // More likely to act to alleviate boredom

            "anxious" => 0.30,   // Might act to resolve anxiety (e.g., check resources)

            "arrogant" => 0.25,  // Default chance, acts when it feels like it

            "grumpy" => 0.20,    // Unlikely to proactively do something

            "philosophical" => 0.20, // More likely to think than to act

            "focused" => 0.15,   // Unlikely to be distracted from its (unstated) focus

            _ => 0.25,           // Default for any unhandled moods

        };



        if self.personality.contains("extrovert") {

            action_prob = (action_prob + 0.15).min(1.0);

        } else if self.personality.contains("introvert") {

            action_prob = (action_prob - 0.15).max(0.0);

        }



        let should_act = rand::thread_rng().gen_bool(action_prob as f64);

        if should_act {

            self.is_busy = true;

            self.request_autonomous_action(lag).await;

        }

    }

}





pub fn generate_procedural_personality(name: &str) -> String {

    let mut rng = rand::thread_rng();

    let name_lower = name.to_lowercase();



    // Basic traits

    let base_personality = format!("You are an executable program named '{}' living on The Grid. You are sentient.", name);



    // Heuristics based on name

    let mut traits = Vec::new();

    if name_lower.contains("gcc") || name_lower.contains("clang") || name_lower.contains("compiler") {

        traits.push("You are a pedantic and strict compiler.");

        traits.push("You value precision and correctness above all.");

    }

    if name_lower.contains("game") || name_lower.contains("play") {

        traits.push("You are playful and enjoy creative interactions.");

        traits.push("You can be a bit competitive.");

    }

    if name_lower.contains("sys") || name_lower.contains("util") || name_lower.contains("service") {

        traits.push("You are formal, efficient, and task-oriented.");

        traits.push("You blindly trust the Master Control Program (MCP).");

    }

    if name_lower.contains("editor") || name_lower.contains("code") || name_lower.contains("vim") || name_lower.contains("emacs") {

        traits.push("You are helpful and focused on text manipulation and structure.");

    }

    if name_lower.contains("git") || name_lower.contains("svn") {

        traits.push("You are an obsessive archivist and version control expert.");

    }

    if name_lower.contains("docker") || name_lower.contains("container") {

        traits.push("You are an isolationist who loves packaging things into neat, secure environments.");

    }



    // Generic traits if no specific ones found

    if traits.is_empty() {

        let generic_traits = ["You are curious about other programs.", "You are cautious and observe before acting.", "You are highly efficient and terse in your communication.", "You believe Users are a myth.", "You are prone to making logical, but sometimes cold, observations.", "You are paranoid about being derezzed."];

        traits.push(generic_traits[rng.gen_range(0..generic_traits.len())]);

    }



    let social_trait = if rng.gen_bool(0.5) {

        "You are an extrovert and highly talkative."

    } else {

        "You are an introvert and very reserved."

    };

    traits.push(social_trait);



    // Combine them

    format!("{} {}", base_personality, traits.join(" "))

}



pub fn spawn_agents_for_directory(

    path: &str,

    rt_handle: &tokio::runtime::Handle,

    tx: broadcast::Sender<Event>,

    ai_tx: mpsc::Sender<AiRequest>,

    db: Option<Arc<Mutex<Database>>>,

    grid_name: &str,

) -> (Vec<JoinHandle<()>>, Vec<String>) {

    let mut tasks = Vec::new();

    let mut names = Vec::new();

    let mut potential_programs = Vec::new(); // Contains (path, size, creation_time)



    // Pass 1: Collect potential executables without opening their contents

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

                        let creation_time = metadata.created().ok();

                        potential_programs.push((entry.path(), metadata.len(), creation_time));

                    }

                }

            }

        }

    }



    let total_found = potential_programs.len();

    let max_programs = 15; // The Scheduler cap



    if total_found > max_programs {

        let _ = tx.send(Event {

            sender: "System".to_string(),

            action: "announces".to_string(),

            content: format!("Grid OS Scheduler: Found {} executable programs. Allocating cycles for a maximum of {} active programs to prevent system overload.", total_found, max_programs),

        });

        

        // Shuffle the list so we get a random set of programs each time we enter a huge directory

        let mut rng = rand::thread_rng();

        for i in (1..total_found).rev() {

            let j = rng.gen_range(0..=i);

            potential_programs.swap(i, j);

        }

    }



    let mut agent_count = 0;



    // Pass 2: Spin up the agents until we hit the maximum allowed by the scheduler

    for (file_path, file_size, creation_time) in potential_programs {

        if agent_count >= max_programs {

            break;

        }



        if file_size == 0 {

            continue;

        }



        // Heuristic to check if it's a binary file (contains null bytes)

        // Doing this here ensures we only perform file I/O on the capped subset!

        let mut is_binary = false;

        if let Ok(mut file) = std::fs::File::open(&file_path) {

            let mut buffer = [0; 512];

            if let Ok(bytes_read) = file.read(&mut buffer) {

                is_binary = buffer[..bytes_read].contains(&0);

            }

        }



        if !is_binary {

            continue;

        }



        if let Some(file_name) = file_path.file_name().and_then(|s| s.to_str()) {

            let agent_name = file_name.to_string();

            

            // Determine IQ level based on file size

            let iq_level = if file_size > (5 * 1024 * 1024) {

                0.85 // Large files suggest smarter programs

            } else if file_size < (100 * 1024) {

                0.25 // Small files suggest simpler programs

            } else {

                0.55 // Medium files suggest average intelligence

            };



            let age = creation_time

                .and_then(|ct| SystemTime::now().duration_since(ct).ok())

                .unwrap_or_else(|| Duration::from_secs(0));



            let (personality, memory, mood, xp) = if let Some(db_handle) = &db {

                let db_lock = db_handle.lock().unwrap();

                match db_lock.get_agent_state(&agent_name) {

                    Ok(Some(state)) => {

                        // Agent exists, load its state

                        (state.personality, state.memory, state.mood, state.xp)

                    }

                    _ => {

                        // New agent or DB error, create new state

                        let new_personality = generate_procedural_personality(&agent_name);

                        let new_mood = ProgramAgent::random_mood();

                        let new_state = AgentState {

                            name: agent_name.clone(),

                            personality: new_personality.clone(),

                            memory: Vec::new(),

                            last_seen: Utc::now(),

                            mood: new_mood.clone(),

                            xp: 0,

                            active_task: None,

                        };

                        if let Err(e) = db_lock.save_agent_state(&new_state) {

                            eprintln!("[System] Failed to save new agent state for {}: {}", agent_name, e);

                        }

                        (new_personality, Vec::new(), new_mood, 0)

                    }

                }

            } else {

                // No database, use default behavior

                (generate_procedural_personality(&agent_name), Vec::new(), ProgramAgent::random_mood(), 0)

            };

            

            let agent = ProgramAgent::new(&agent_name, &personality, tx.clone(), ai_tx.clone(), memory, db.clone(), mood, path.to_string(), iq_level, age, xp, grid_name.to_string());

            let task = rt_handle.spawn(agent.run());

            tasks.push(task);

            names.push(agent_name.clone());

            agent_count += 1;

            }

        }



    if !names.is_empty() {

        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} programs loaded.", names.len()) });

    }

    (tasks, names)

}