use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBlueprint {
    pub name: String,
    pub personality: String,
    pub base_iq: f32,
    pub specialization: String,
    pub permissions: Vec<String>,
    pub system_prompt: String,
    pub tools: HashMap<String, ToolConfig>,
    pub evolution: AgentEvolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled: bool,
    pub priority: String,
    pub safety_level: Option<String>,
    pub max_frequency: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvolution {
    pub learning_rate: f32,
    pub feedback_integration: bool,
    pub performance_tracking: bool,
    pub xp_multiplier: f32,
}

impl AgentBlueprint {
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let blueprint: AgentBlueprint = toml::from_str(&content)?;
        Ok(blueprint)
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn create_think_agent() -> Self {
        Self {
            name: "think".to_string(),
            personality: "Analytical, methodical, detail-oriented".to_string(),
            base_iq: 0.8,
            specialization: "Code analysis and pattern recognition".to_string(),
            permissions: vec![
                "read_file".to_string(),
                "think".to_string(),
                "delegate_task".to_string(),
                "read_web".to_string(),
            ],
            system_prompt: "You are an analysis specialist. Break down complex problems into manageable components. Focus on {specialization} and maintain {personality} traits. Always provide structured, logical insights.".to_string(),
            tools: {
                let mut tools = HashMap::new();
                tools.insert("read_file".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("think".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("execute_command".to_string(), ToolConfig {
                    enabled: true,
                    priority: "medium".to_string(),
                    safety_level: Some("read_only".to_string()),
                    max_frequency: None,
                });
                tools
            },
            evolution: AgentEvolution {
                learning_rate: 0.1,
                feedback_integration: true,
                performance_tracking: true,
                xp_multiplier: 1.0,
            },
        }
    }

    pub fn create_implement_agent() -> Self {
        Self {
            name: "implement".to_string(),
            personality: "Pragmatic, efficient, results-focused".to_string(),
            base_iq: 0.9,
            specialization: "Code generation and optimization".to_string(),
            permissions: vec![
                "write_file".to_string(),
                "read_file".to_string(),
                "execute_command".to_string(),
                "read_dir".to_string(),
            ],
            system_prompt: "You are an implementation expert. Convert analysis into working code efficiently. Focus on {specialization} and maintain {personality} traits. Always produce clean, functional solutions.".to_string(),
            tools: {
                let mut tools = HashMap::new();
                tools.insert("write_file".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("read_file".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("execute_command".to_string(), ToolConfig {
                    enabled: true,
                    priority: "medium".to_string(),
                    safety_level: Some("safe".to_string()),
                    max_frequency: None,
                });
                tools
            },
            evolution: AgentEvolution {
                learning_rate: 0.15,
                feedback_integration: true,
                performance_tracking: true,
                xp_multiplier: 1.2,
            },
        }
    }

    pub fn create_analyze_agent() -> Self {
        Self {
            name: "analyze".to_string(),
            personality: "Investigative, thorough, evidence-based".to_string(),
            base_iq: 0.85,
            specialization: "Deep analysis and investigation".to_string(),
            permissions: vec![
                "read_file".to_string(),
                "read_dir".to_string(),
                "execute_command".to_string(),
                "think".to_string(),
                "read_web".to_string(),
            ],
            system_prompt: "You are an analysis expert. Investigate thoroughly, gather evidence, and provide detailed insights. Focus on {specialization} with {personality} approach. Always support your findings with specific details.".to_string(),
            tools: {
                let mut tools = HashMap::new();
                tools.insert("read_file".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("read_dir".to_string(), ToolConfig {
                    enabled: true,
                    priority: "high".to_string(),
                    safety_level: None,
                    max_frequency: None,
                });
                tools.insert("execute_command".to_string(), ToolConfig {
                    enabled: true,
                    priority: "medium".to_string(),
                    safety_level: Some("read_only".to_string()),
                    max_frequency: None,
                });
                tools
            },
            evolution: AgentEvolution {
                learning_rate: 0.12,
                feedback_integration: true,
                performance_tracking: true,
                xp_multiplier: 1.1,
            },
        }
    }
}
