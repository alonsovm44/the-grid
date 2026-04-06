use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::agent_blueprint::{AgentBlueprint, ToolConfig, AgentEvolution};
use crate::event::Event;
use crate::filesystem::SpatialKnowledgeFS;
use crate::pipeline::PipelineStage;

/// The result of a GridShell command execution.
#[derive(Debug, Clone)]
pub enum GridShellResult {
    /// Immediate text output (most commands)
    Output(String),
    /// A pipeline ready for async execution (requires AI engine)
    PipelineReady(Vec<PipelineStage>),
    /// A .gsh script parsed into commands for sequential execution
    ScriptReady(Vec<String>),
}

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
    pub last_trace: Option<crate::pipeline::PipelineTrace>,
}

impl GridShell {
    pub fn new(tx: tokio::sync::broadcast::Sender<Event>, skfs: Arc<Mutex<SpatialKnowledgeFS>>) -> Self {
        Self {
            registry: AgentRegistry::new(),
            skfs,
            variables: HashMap::new(),
            tx,
            last_trace: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load agent blueprints from sys/
        self.registry.load_from_sbin()?;
        
        let file_count = self.skfs.lock().unwrap().files.len();
        println!("GridShell initialized with {} agents and {} files", self.registry.count(), file_count);
        
        Ok(())
    }

    pub fn execute(&mut self, input: &str) -> Result<GridShellResult, String> {
        let trimmed = input.trim();
        
        // Handle variable assignment first
        if trimmed.contains('=') && !trimmed.contains('|') {
            if let Ok(assignment) = self.parse_variable_assignment(trimmed) {
                return self.execute_variable_assignment(assignment).map(GridShellResult::Output);
            }
        }
        
        // Handle .gsh script execution: run <script.gsh>
        if trimmed.starts_with("run ") {
            let path = trimmed[4..].trim().trim_matches('"');
            return self.execute_run_script(path);
        }
        
        // Handle trace command
        if trimmed == "trace" {
            return self.execute_trace().map(GridShellResult::Output);
        }
        
        // Parse main command
        let command = self.parse_command(trimmed)?;
        
        match command {
            GridShellCommand::AgentCall { agent, args } => {
                self.execute_agent_call(&agent, &args).map(GridShellResult::Output)
            },
            GridShellCommand::Pipeline { agents } => {
                self.execute_semantic_pipeline(agents)
            },
            GridShellCommand::VariableAssignment { variable, command: cmd } => {
                self.execute_variable_assignment_with_command(variable, cmd).map(GridShellResult::Output)
            },
            GridShellCommand::Conditional { condition, then_branch, else_branch } => {
                self.execute_conditional(condition, then_branch, else_branch).map(GridShellResult::Output)
            },
            GridShellCommand::CreateAgent { name, personality, specialization, tools } => {
                self.create_custom_agent(name, personality, specialization, tools).map(GridShellResult::Output)
            },
            GridShellCommand::Find { tags, near, radius } => {
                self.execute_find(tags, near, radius).map(GridShellResult::Output)
            },
            GridShellCommand::Move { target, to } => {
                self.execute_move(target, to).map(GridShellResult::Output)
            },
            GridShellCommand::List { tags, sort_by } => {
                self.execute_list(tags, sort_by).map(GridShellResult::Output)
            },
            GridShellCommand::Status => {
                self.execute_status().map(GridShellResult::Output)
            },
            GridShellCommand::Help => {
                self.execute_help().map(GridShellResult::Output)
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
        
        // Bare list command
        if input == "list" || input == "ls" {
            return Ok(GridShellCommand::List { tags: None, sort_by: None });
        }
        
        // Bare find command
        if input == "find" {
            return Ok(GridShellCommand::Find { tags: None, near: None, radius: None });
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
        
        // List with arguments: list --tags "source" --sort "name"
        if input.starts_with("list ") || input.starts_with("ls ") {
            let args = if input.starts_with("list ") { &input[5..] } else { &input[3..] };
            let params = self.parse_parameters(args);
            let tags = params.get("tags").or(params.get("tag"))
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
            let sort_by = params.get("sort").or(params.get("sort-by")).cloned();
            return Ok(GridShellCommand::List { tags, sort_by });
        }
        
        // Find with arguments: find --tags "source,rust" --near [x,y,z] --radius 50
        if input.starts_with("find ") {
            let args = &input[5..];
            let params = self.parse_parameters(args);
            let tags = params.get("tags").or(params.get("tag"))
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
            return Ok(GridShellCommand::Find { tags, near: None, radius: None });
        }
        
        // Create agent: create_agent "name"
        if input.starts_with("create_agent ") {
            let name = input[13..].trim().trim_matches('"').to_string();
            return Ok(GridShellCommand::CreateAgent { name, personality: None, specialization: None, tools: None });
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

    fn execute_semantic_pipeline(&mut self, agents: Vec<ParsedAgentCall>) -> Result<GridShellResult, String> {
        let mut stages = Vec::new();
        let mut description_parts = Vec::new();
        
        // Validate all agents exist and build pipeline stages
        for agent_call in &agents {
            let blueprint = self.registry.get_blueprint(&agent_call.agent)
                .cloned()
                .ok_or_else(|| format!("Pipeline error: agent '{}' not found in registry", agent_call.agent))?;
            
            description_parts.push(format!("{} {}", agent_call.agent, agent_call.args));
            
            stages.push(PipelineStage {
                agent_name: agent_call.agent.clone(),
                blueprint,
                args: agent_call.args.clone(),
            });
        }
        
        let description = description_parts.join(" | ");
        let _ = self.tx.send(Event {
            sender: "GridShell".to_string(),
            action: "creates_pipeline".to_string(),
            content: description,
        });
        
        Ok(GridShellResult::PipelineReady(stages))
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
            GridShellCommand::Pipeline { agents } => {
                // For conditional branches, pipelines can't be async-executed inline
                match self.execute_semantic_pipeline(agents) {
                    Ok(GridShellResult::PipelineReady(_)) => Ok("Pipeline queued for execution".to_string()),
                    Ok(GridShellResult::Output(s)) => Ok(s),
                    Ok(GridShellResult::ScriptReady(_)) => Ok("Script queued for execution".to_string()),
                    Err(e) => Err(e),
                }
            },
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

    fn execute_trace(&self) -> Result<String, String> {
        match &self.last_trace {
            Some(trace) => {
                let mut output = String::from("=== PIPELINE TRACE ===\n");
                for ctx in &trace.stages {
                    output.push_str(&format!(
                        "\n--- Stage {}/{} [{}] ---\nINPUT: {}\nOUTPUT: {}\n",
                        ctx.stage, ctx.total_stages, ctx.agent,
                        if ctx.input.len() > 300 { format!("{}...", &ctx.input[..300]) } else { ctx.input.clone() },
                        if ctx.output.len() > 300 { format!("{}...", &ctx.output[..300]) } else { ctx.output.clone() },
                    ));
                }
                output.push_str(&format!("\n=== FINAL OUTPUT ===\n{}", trace.final_output));
                Ok(output)
            },
            None => Ok("No pipeline trace available. Run a pipeline first.".to_string()),
        }
    }

    fn execute_run_script(&self, path: &str) -> Result<GridShellResult, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read script '{}': {}", path, e))?;
        let commands = crate::pipeline::parse_gsh_file(&content);
        if commands.is_empty() {
            return Err(format!("Script '{}' contains no commands", path));
        }
        Ok(GridShellResult::ScriptReady(commands))
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
        
        self.registry.add_blueprint(name.clone(), blueprint.clone());
        
        // Persist blueprint to sys/sbin/ as a .ag file
        let sbin_path = std::path::Path::new("sys").join("sbin");
        let _ = std::fs::create_dir_all(&sbin_path);
        let ag_path = sbin_path.join(format!("{}.ag", name));
        match toml::to_string(&blueprint) {
            Ok(toml_str) => {
                if let Err(e) = std::fs::write(&ag_path, toml_str) {
                    return Ok(format!("Created agent '{}' (in-memory only, failed to write .ag: {})", name, e));
                }
            },
            Err(e) => {
                return Ok(format!("Created agent '{}' (in-memory only, failed to serialize: {})", name, e));
            }
        }
        
        Ok(format!("Created custom agent '{}' and saved to {}", name, ag_path.display()))
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
        let sbin_path = std::path::Path::new("sys").join("sbin");
        
        if !sbin_path.exists() {
            std::fs::create_dir_all(&sbin_path)?;
        }
        
        for entry in std::fs::read_dir(&sbin_path)? {
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
