use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CloudConfig {
    pub api_key: String,
    pub api_url: String,
    pub smart_model_id: String,
    pub dumb_model_id: String,
    pub protocol: String, // "openai" or "custom"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalConfig {
    pub api_url: String,
    pub smart_model_id: String,
    pub dumb_model_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub mode: String,
    pub cloud: CloudConfig,
    pub local: LocalConfig,
    pub max_retries: u32,
}

impl Config {
    /// Loads the configuration from `config.toml`, or creates a default one if it doesn't exist.
    /// Returns `Ok(Config)` if loaded successfully.
    /// Returns `Err` and exits if the file needs to be created or is malformed.
    pub fn load() -> Result<Self, ()> {
        match fs::read_to_string("config.toml") {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(cfg) => Ok(cfg),
                Err(e) => {
                    eprintln!("Error parsing config.toml: {}. Please fix it or delete it to generate a new one.", e);
                    Err(())
                }
            },
            Err(_) => {
                let default_config = Config {
                    mode: "local".to_string(),
                    cloud: CloudConfig {
                        api_key: "api key".to_string(),
                        api_url: "https://apifreellm.com/api/v1/chat".to_string(),
                        smart_model_id: "gpt-4o".to_string(),
                        dumb_model_id: "gpt-3.5-turbo".to_string(),
                        protocol: "custom".to_string(),
                    },
                    local: LocalConfig {
                        api_url: "http://localhost:11434/api/generate".to_string(),
                        smart_model_id: "qwen2.5:3b-instruct".to_string(),
                        dumb_model_id: "tinyllama".to_string(),
                    },
                    max_retries: 10,
                };
                let toml_string = toml::to_string_pretty(&default_config).expect("Failed to serialize default config.");
                fs::write("config.toml", toml_string).expect("Failed to write default config.toml");
                println!("config.toml not found. A default one has been created. Please review it and restart the application.");
                Err(())
            }
        }
    }
}