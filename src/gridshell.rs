use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::agent_blueprint::{AgentBlueprint, ToolConfig, AgentEvolution};
use crate::event::Event;
use crate::filesystem::SpatialKnowledgeFS;

#[derive(Debug, Clone)]
pub enum GridShellCommand {
    AgentCall {
        agent: String,
        args: String,
    },
    Pipeline {
        agents: Vec<ParsedAgentCall>,
    },
    VariableAssignment {
        variable: String,
        command: ParsedAgentCall,
    },
    Conditional {
        condition: Condition,
        then_branch: Vec<GridShellCommand>,
        else_branch: Option<Vec<GridShellCommand>>,
    },
    CreateAgent {
        name: String,
        personality: Option<String>,
        specialization: Option<String>,
        tools: Option<Vec<String>>,
    },
    Find {
        tags: Option<Vec<String>>,
        near: Option<[f32; 3]>,
        radius: Option<f32>,
    },
    Move {
        target: String,
        to: Option<[f32; 3]>,
    },
    List {
        tags: Option<Vec<String>>,
        sort_by: Option<String>,
    },
    Status,
    Help,
}

#[derive(Debug, Clone)]
pub struct ParsedAgentCall {
    pub agent: String,
    pub args: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    MoodEquals {
        agent: String,
        mood: String,
    },
    VariableExists {
        variable: String,
    },
    FileExists {
        file: String,
    },
}

pub struct GridShell {
    pub registry: AgentRegistry,
    pub skfs: Arc<Mutex<SpatialKnowledgeFS>>,
    pub variables: HashMap<String, String>,
    pub tx: tokio::sync::broadcast::Sender<Event>,
}

impl GridShell {
    pub fn new(tx: tokio::sync::broadcast::Sender<Event>, skfs: Arc<Mutex<SpatialKnowledgeFS>>) -> Self {
        Self {
            registry: AgentRegistry::new(),
            skfs,
            variables: HashMap::new(),
            tx,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load agent blueprints from sys/
        self.registry.load_from_sbin()?;
        
        let file_count = self.skfs.lock().unwrap().files.len();
        println!("GridShell initialized with {} agents and {} files", self.registry.count(), file_count);
        
        Ok(())
    }

    pub fn execute(&mut self, input: &str) -> Result<String, String> {
        let trimmed = input.trim();
        
        // Handle variable assignment first
        if trimmed.contains('=') && !trimmed.contains('|') {
            if let Ok(assignment) = self.parse_variable_assignment(trimmed) {
                return self.execute_variable_assignment(assignment);
            }
        }
        
        // Parse main command
        let command = self.parse_command(trimmed)?;
        
        match command {
            GridShellCommand::AgentCall { agent, args } => {
                self.execute_agent_call(&agent, &args)
            },
            GridShellCommand::Pipeline { agents } => {
                self.execute_semantic_pipeline(agents)
            },
            GridShellCommand::VariableAssignment { variable, command: cmd } => {
                self.execute_variable_assignment_with_command(variable, cmd)
            },
            GridShellCommand::Conditional { condition, then_branch, else_branch } => {
                self.execute_conditional(condition, then_branch, else_branch)
            },
            GridShellCommand::CreateAgent { name, personality, specialization, tools } => {
                self.create_custom_agent(name, personality, specialization, tools)
            },
            GridShellCommand::Find { tags, near, radius } => {
                self.execute_find(tags, near, radius)
            },
            GridShellCommand::Move { target, to } => {
                self.execute_move(target, to)
            },
            GridShellCommand::List { tags, sort_by } => {
                self.execute_list(tags, sort_by)
            },
            GridShellCommand::Status => {
                self.execute_status()
            },
            GridShellCommand::Help => {
                self.execute_help()
            },
        }
    }

    fn parse_command(&self, input: &str) -> Result<GridShellCommand, String> {
        let input = input.trim();
        
        // Help command
        if input == "help" || input == "?" {
            return Ok(GridShellCommand::Help);
        }
        
        // Status command
        if input == "status" {
            return Ok(GridShellCommand::Status);
        }
        
        // Pipeline detection
        if input.contains('|') {
            return self.parse_pipeline(input);
        }
        
        // Variable assignment
        if input.contains('=') && !input.contains('|') {
            let (variable, command) = self.parse_variable_assignment(input)?;
            return Ok(GridShellCommand::VariableAssignment { variable, command });
        }
        
        // Agent calls with parameters
        if let Some((agent_part, args_part)) = input.split_once(' ') {
            let agent = agent_part.trim().to_string();
            let args = args_part.trim().to_string();
            
            // Parse parameters like --tag="value" --param=value
            let _parameters = self.parse_parameters(&args);
            
            return Ok(GridShellCommand::AgentCall { agent, args });
        }
        
        Err("Unknown command format".to_string())
    }

    fn parse_pipeline(&self, input: &str) -> Result<GridShellCommand, String> {
        let parts: Vec<&str> = input.split('|').collect();
        let mut agents = Vec::new();
        
        for part in parts {
            let agent_call = self.parse_agent_call(part.trim())?;
            agents.push(agent_call);
        }
        
        Ok(GridShellCommand::Pipeline { agents })
    }

    fn parse_agent_call(&self, input: &str) -> Result<ParsedAgentCall, String> {
        let parts: Vec<&str> = input.split(' ').collect();
        if parts.is_empty() {
            return Err("Empty agent call".to_string());
        }
        
        let agent = parts[0].trim().to_string();
        let args = if parts.len() > 1 { parts[1..].join(" ") } else { String::new() };
        let parameters = self.parse_parameters(&args);
        
        Ok(ParsedAgentCall { agent, args, parameters })
    }

    fn parse_parameters(&self, args: &str) -> HashMap<String, String> {
        let mut parameters = HashMap::new();
        let mut current_param = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;
        
        for ch in args.chars() {
            match ch {
                '"' if !escape_next => {
                    in_quotes = !in_quotes;
                }
                '\\' if !escape_next => {
                    escape_next = true;
                }
                ' ' if !in_quotes && !escape_next => {
                    if !current_param.is_empty() {
                        if let Some((key, value)) = current_param.split_once('=') {
                            parameters.insert(key.trim().to_string(), value.trim_matches('"').to_string());
                        }
                        current_param.clear();
                    }
                }
                _ => {
                    if escape_next {
                        escape_next = false;
                    }
                    current_param.push(ch);
                }
            }
        }
        
        if !current_param.is_empty() {
            if let Some((key, value)) = current_param.split_once('=') {
                parameters.insert(key.trim().to_string(), value.trim_matches('"').to_string());
            }
        }
        
        parameters
    }

    fn parse_variable_assignment(&self, input: &str) -> Result<(String, ParsedAgentCall), String> {
        if let Some((var_part, command_part)) = input.split_once('=') {
            let variable = var_part.trim().to_string();
            let command = self.parse_agent_call(command_part.trim())?;
            Ok((variable, command))
        } else {
            Err("Invalid variable assignment".to_string())
        }
    }

    fn execute_agent_call(&mut self, agent: &str, args: &str) -> Result<String, String> {
        // Check if agent exists in registry
        if let Some(_blueprint) = self.registry.get_blueprint(agent) {
            let _ = self.tx.send(Event {
                sender: "GridShell".to_string(),
                action: "spawns_agent".to_string(),
                content: format!("{} with args: {}", agent, args),
            });
            
            Ok(format!("Agent '{}' spawned with args: '{}'", agent, args))
        } else {
            Err(format!("Agent '{}' not found in registry", agent))
        }
    }

    fn execute_semantic_pipeline(&mut self, agents: Vec<ParsedAgentCall>) -> Result<String, String> {
        let mut pipeline_description = String::new();
        
        for (i, agent_call) in agents.iter().enumerate() {
            if i > 0 {
                pipeline_description.push_str(" | ");
            }
            pipeline_description.push_str(&format!("{} {}", agent_call.agent, agent_call.args));
        }
        
        let _ = self.tx.send(Event {
            sender: "GridShell".to_string(),
            action: "creates_pipeline".to_string(),
            content: pipeline_description.clone(),
        });
        
        Ok(format!("Created semantic pipeline: {}", pipeline_description))
    }

    fn execute_variable_assignment(&mut self, assignment: (String, ParsedAgentCall)) -> Result<String, String> {
        // In a full implementation, this would execute the command and store the result
        // For now, just store the command structure
        let variable_value = format!("{} {}", assignment.1.agent, assignment.1.args);
        self.variables.insert(assignment.0.clone(), variable_value.clone());
        
        Ok(format!("Variable '{}' set to command: {}", assignment.0, variable_value))
    }

    fn execute_variable_assignment_with_command(&mut self, variable: String, command: ParsedAgentCall) -> Result<String, String> {
        let variable_value = format!("{} {}", command.agent, command.args);
        self.variables.insert(variable.clone(), variable_value);
        
        Ok(format!("Variable '{}' set", variable))
    }

    fn execute_conditional(&mut self, condition: Condition, then_branch: Vec<GridShellCommand>, else_branch: Option<Vec<GridShellCommand>>) -> Result<String, String> {
        let condition_result = self.evaluate_condition(&condition)?;
        
        let commands_to_execute = if condition_result { then_branch } else { else_branch.unwrap_or_default() };
        
        let mut results = Vec::new();
        for cmd in commands_to_execute {
            results.push(self.execute_gridshell_command(cmd)?);
        }
        
        Ok(format!("Conditional executed. Results: {:?}", results))
    }

    fn evaluate_condition(&self, condition: &Condition) -> Result<bool, String> {
        match condition {
            Condition::MoodEquals { agent: _, mood: _ } => {
                // In a full implementation, we'd query the agent's current mood
                // For now, return a placeholder
                Ok(false) // Placeholder
            },
            Condition::VariableExists { variable } => {
                Ok(self.variables.contains_key(variable))
            },
            Condition::FileExists { file } => {
                // Check if file exists in SKFS
                let skfs = self.skfs.lock().unwrap();
                Ok(skfs.files.values().any(|f| f.name == *file))
            },
        }
    }

    fn execute_find(&mut self, tags: Option<Vec<String>>, near: Option<[f32; 3]>, radius: Option<f32>) -> Result<String, String> {
        let mut found_files = Vec::new();
        
        if let Some(tag_list) = tags {
            let skfs = self.skfs.lock().unwrap();
            for tag in tag_list {
                let files = skfs.find_files_by_tag(&tag);
                found_files.extend(files.iter().map(|f| f.name.clone()));
            }
        }
        
        if let Some(position) = near {
            if let Some(search_radius) = radius {
                let skfs = self.skfs.lock().unwrap();
                let files = skfs.find_files_within_radius(position, search_radius);
                found_files.extend(files.iter().map(|f| f.name.clone()));
            }
        }
        
        Ok(format!("Found {} files: {:?}", found_files.len(), found_files))
    }

    fn execute_move(&mut self, target: String, to: Option<[f32; 3]>) -> Result<String, String> {
        if let Some(position) = to {
            let mut skfs = self.skfs.lock().unwrap();
            // Find file and update its position
            for file in skfs.files.values_mut() {
                if file.name == target {
                    file.position = position;
                    let _ = self.tx.send(Event {
                        sender: "GridShell".to_string(),
                        action: "moves_file".to_string(),
                        content: format!("Moved {} to [{}, {}, {}]", target, position[0], position[1], position[2]),
                    });
                return Ok(format!("Moved '{}' to [{}, {}, {}]", target, position[0], position[1], position[2]));
                }
            }
            Err(format!("File '{}' not found", target))
        } else {
            Err("Target position required".to_string())
        }
    }

    fn execute_list(&mut self, tags: Option<Vec<String>>, sort_by: Option<String>) -> Result<String, String> {
        let mut files = Vec::new();
        let skfs = self.skfs.lock().unwrap();
        
        if let Some(tag_list) = tags {
            for tag in tag_list {
                let tag_files = skfs.find_files_by_tag(&tag);
                files.extend(tag_files);
            }
        } else {
            files = skfs.files.values().collect();
        }
        
        // Sort files
        if let Some(sort_key) = sort_by {
            files.sort_by(|a, b| {
                match sort_key.as_str() {
                    "name" => a.name.cmp(&b.name),
                    "access" => a.access_frequency.partial_cmp(&b.access_frequency).unwrap_or(std::cmp::Ordering::Equal),
                    _ => std::cmp::Ordering::Equal,
                }
            });
        }
        
        let file_list: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
        Ok(format!("Files: {:?}", file_list))
    }

    fn execute_status(&mut self) -> Result<String, String> {
        let agent_count = self.registry.count();
        let file_count = self.skfs.lock().unwrap().files.len();
        let variable_count = self.variables.len();
        
        Ok(format!("GridShell Status:\n  Agents: {}\n  Files: {}\n  Variables: {}", agent_count, file_count, variable_count))
    }

    fn execute_help(&mut self) -> Result<String, String> {
        let help_text = r#"
=== THE GRID SYSTEM COMMANDS ===
  grid init            - Initialize the persistent database
  grid ls              - List all active programs in the current sector
  grid status          - Show detailed system status, resources, and tasks
  grid tasks           - List currently active agent tasks
  grid map             - Toggle the 3D Sector Map visualization
  grid relations       - Show the affinity/relationship graph between agents
  grid reload          - Refresh and respawn agents in current directory
  grid build [target]  - Orchestrate a full project build using available tools
  
  grid invoke [tools]  - Manually spawn system tools (e.g., git, cargo, vim)
  grid revoke [tools]  - Remove manually invoked tools
  grid kill [name]     - Forcefully terminate (derez) a program
  grid jail [name]     - Move an executable to the .jail directory
  
  grid [name] task "[t]" - Assign a direct task to a specific program
  grid give [f] to [n]  - Hand a file to one or more programs
  
  grid reward [names]  - Award digital bliss and XP to programs
  grid punish [names]  - Inflict cycle starvation and structural degradation
  grid shush [names]   - Mute a program's vocal subroutines
  grid unshush [names] - Restore a program's vocal subroutines
  grid gag [names]     - Temporarily mute programs (use -d=seconds)
  
  grid mode [local|cloud] - Switch between local (Ollama) and cloud providers
  grid export [name]   - Save the current conversation log to a file
  grid clear           - Wipe the terminal history
  grid toggle [emojis|thoughts|feels] - Toggle UI metadata overlays
  
  grid start-adversarial-network [p1] -vs [p2] [arena] - Initiation combat
    Arenas: arena=light-cycles, arena=melee

================================

SEMANTIC SHELL COMMANDS:

Agent Calls:
  think "args"           - Spawn analysis agent
  implement "args"        - Spawn implementation agent  
  analyze "args"          - Spawn analysis agent
  create_agent "name"    - Create custom agent

Pipelines:
  think "analyze code" | implement "fix issues" | test "validate"

Variables:
  RESULT = think "analyze data"
  if [condition] then commands else commands

File Operations:
  find --tags "tag1,tag2" --near [x,y,z] --radius R
  move "filename" --to [x,y,z]
  list --tags "tag1,tag2" --sort "name|access"

System:
  status                 - Show semantic shell status
  help                   - Show this help
        "#;
        
        Ok(help_text.to_string())
    }

    fn execute_gridshell_command(&mut self, command: GridShellCommand) -> Result<String, String> {
        match command {
            GridShellCommand::AgentCall { agent, args } => self.execute_agent_call(&agent, &args),
            GridShellCommand::Pipeline { agents } => self.execute_semantic_pipeline(agents),
            GridShellCommand::VariableAssignment { variable, command: cmd } => self.execute_variable_assignment_with_command(variable, cmd),
            GridShellCommand::Conditional { condition, then_branch, else_branch } => self.execute_conditional(condition, then_branch, else_branch),
            GridShellCommand::CreateAgent { name, personality, specialization, tools } => self.create_custom_agent(name, personality, specialization, tools),
            GridShellCommand::Find { tags, near, radius } => self.execute_find(tags, near, radius),
            GridShellCommand::Move { target, to } => self.execute_move(target, to),
            GridShellCommand::List { tags, sort_by } => self.execute_list(tags, sort_by),
            GridShellCommand::Status => self.execute_status(),
            GridShellCommand::Help => self.execute_help(),
        }
    }

    fn create_custom_agent(&mut self, name: String, personality: Option<String>, specialization: Option<String>, tools: Option<Vec<String>>) -> Result<String, String> {
        let blueprint = AgentBlueprint {
            name: name.clone(),
            personality: personality.clone().unwrap_or_else(|| "Balanced, adaptive".to_string()),
            base_iq: 0.7,
            specialization: specialization.clone().unwrap_or_else(|| "General purpose".to_string()),
            permissions: tools.clone().unwrap_or_else(|| vec!["read_file".to_string(), "think".to_string()]),
            system_prompt: format!("You are a custom agent named '{}'. Focus on {} with {} traits.", name, 
                specialization.unwrap_or_else(|| "general tasks".to_string()),
                personality.unwrap_or_else(|| "balanced personality".to_string())),
            tools: {
                let mut tool_map = HashMap::new();
                if let Some(tool_list) = tools {
                    for tool in tool_list {
                        tool_map.insert(tool.clone(), ToolConfig {
                            enabled: true,
                            priority: "medium".to_string(),
                            safety_level: None,
                            max_frequency: None,
                        });
                    }
                }
                tool_map
            },
            evolution: AgentEvolution {
                learning_rate: 0.1,
                feedback_integration: true,
                performance_tracking: true,
                xp_multiplier: 1.0,
            },
        };
        
        self.registry.add_blueprint(name.clone(), blueprint);
        
        Ok(format!("Created custom agent '{}'", name))
    }
}

// Agent Registry implementation
pub struct AgentRegistry {
    blueprints: HashMap<String, AgentBlueprint>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            blueprints: HashMap::new(),
        };
        
        // Add default agents
        registry.blueprints.insert("think".to_string(), AgentBlueprint::create_think_agent());
        registry.blueprints.insert("implement".to_string(), AgentBlueprint::create_implement_agent());
        registry.blueprints.insert("analyze".to_string(), AgentBlueprint::create_analyze_agent());
        
        registry
    }

    pub fn add_blueprint(&mut self, name: String, blueprint: AgentBlueprint) {
        self.blueprints.insert(name, blueprint);
    }

    pub fn get_blueprint(&self, name: &str) -> Option<&AgentBlueprint> {
        self.blueprints.get(name)
    }

    pub fn count(&self) -> usize {
        self.blueprints.len()
    }

    pub fn load_from_sbin(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let sys_path = std::path::Path::new("sys");
        
        if !sys_path.exists() {
            std::fs::create_dir_all(&sys_path)?;
        }
        
        for entry in std::fs::read_dir(&sys_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| Some(s == "ag")) == Some(true) {
                let blueprint = AgentBlueprint::from_file(&path)?;
                self.blueprints.insert(blueprint.name.clone(), blueprint.clone());
                println!("Loaded agent blueprint: {}", blueprint.name);
            }
        }
        
        Ok(())
    }
}
