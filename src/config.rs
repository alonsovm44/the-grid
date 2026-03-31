use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CloudConfig {
    pub api_key: String,
    pub api_url: String,
    pub model_id: String,
    pub protocol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalConfig {
    pub api_url: String,
    pub model_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
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
                    cloud: CloudConfig {
                        api_key: "api key".to_string(),
                        api_url: "https://apifreellm.com/api/v1/chat".to_string(),
                        model_id: "gpt-4o".to_string(),
                        protocol: "openai".to_string(),
                    },
                    local: LocalConfig {
                        api_url: "http://localhost:11434/api/generate".to_string(),
                        model_id: "qwen2.5-coder:3b".to_string(),
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