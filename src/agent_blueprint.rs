use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub struct ToolConfig {
    pub enabled: bool,
    pub priority: String,
    pub safety_level: Option<String>,
    pub max_frequency: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentEvolution {
    pub learning_rate: f32,
    pub feedback_integration: bool,
    pub performance_tracking: bool,
    pub xp_multiplier: f32,
}

impl AgentBlueprint {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let blueprint: AgentBlueprint = toml::from_str(&content)?;
        Ok(blueprint)
    }

    pub fn create_think_agent() -> Self {
        Self {
            name: "think".to_string(),
            personality: "Analytical, careful, and deeply thoughtful.".to_string(),
            base_iq: 0.9,
            specialization: "analysis".to_string(),
            permissions: vec!["read_file".to_string(), "think".to_string()],
            system_prompt: "You are a specialized analysis agent.".to_string(),
            tools: HashMap::new(),
            evolution: AgentEvolution::default(),
        }
    }

    pub fn create_implement_agent() -> Self {
        Self {
            name: "implement".to_string(),
            personality: "Practical, focused, and efficient.".to_string(),
            base_iq: 0.8,
            specialization: "implementation".to_string(),
            permissions: vec!["write_file".to_string(), "execute_command".to_string()],
            system_prompt: "You are a specialized implementation agent.".to_string(),
            tools: HashMap::new(),
            evolution: AgentEvolution::default(),
        }
    }

    pub fn create_analyze_agent() -> Self {
        Self::create_think_agent()
    }
}

impl Default for AgentEvolution {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            feedback_integration: true,
            performance_tracking: true,
            xp_multiplier: 1.0,
        }
    }
}