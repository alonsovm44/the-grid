use crate::config::Config;
use crate::Event;
use reqwest;
use serde::{Deserialize, Serialize};
use std::process::Command;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;

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

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Serialize)]
struct CustomCloudRequest<'a> {
    message: String,
    model: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RelationshipUpdate {
    pub target: String,
    pub change: i32,
}

#[derive(Deserialize, Debug)]
pub struct AutonomousAction {
    pub action: String,
    pub content: Option<String>,
    pub recipient: Option<String>,
    pub command: Option<String>,
    pub file_name: Option<String>,
    pub dir_path: Option<String>,
    pub url: Option<String>,
    pub relationship_updates: Option<Vec<RelationshipUpdate>>,
}

/// Represents a request from an agent to the central AI Engine
pub struct AiRequest {
    pub agent_name: String,
    pub prompt: String,
    pub is_json_format: bool,
    pub is_autonomous: bool, // true if deciding an action, false if just conversing
    pub iq_level: f32, // 0.0 = dumb, 1.0 = smart
}

/// Executes a shell command in a separate thread and broadcasts the result.
pub fn execute_command_and_broadcast(command_str: String, tx: broadcast::Sender<Event>, sender: String) {
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

        let output = Command::new(shell).arg(arg).arg(&command_str).output();

        let response_content = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                
                let mut out_str = stdout.to_string();
                let mut err_str = stderr.to_string();
                
                let limit = 2000;
                if out_str.len() > limit { out_str.truncate(limit); out_str.push_str("\n...[OUTPUT TRUNCATED]..."); }
                if err_str.len() > limit { err_str.truncate(limit); err_str.push_str("\n...[OUTPUT TRUNCATED]..."); }

                if out.status.success() {
                    if out_str.trim().is_empty() {
                        format!("Command '{}' executed successfully with no output.", command_str)
                    } else {
                        out_str
                    }
                } else if err_str.trim().is_empty() {
                    format!("Command '{}' failed with no error message.\nOutput: {}", command_str, out_str)
                } else {
                    format!("Error:\n{}\nOutput:\n{}", err_str, out_str)
                }
            }
            Err(e) => format!("Failed to execute command '{}': {}", command_str, e),
        };

        // Broadcast the result
        let _ = tx.send(Event { sender, action: "command_output".to_string(), content: response_content });
    });
}

/// Central AI Engine Task (Processes LLM requests sequentially)
pub async fn run_ai_engine(
    mut ai_rx: mpsc::Receiver<AiRequest>,
    tx_for_ai: broadcast::Sender<Event>,
    config_for_ai: Arc<Mutex<Config>>,
) {
    let client = reqwest::Client::new();
    while let Some(request) = ai_rx.recv().await {
        // Reset retry logic for each new request
        let mut retries = 0;
        let mut current_delay = Duration::from_secs(1);
        loop { // Retry loop
            // Lock, copy data, and immediately unlock before any .await calls.
            let (mode, local_config, cloud_config, max_retries) = {
                let config_guard = config_for_ai.lock().unwrap();
                (
                    config_guard.mode.clone(),
                    config_guard.local.clone(),
                    config_guard.cloud.clone(),
                    config_guard.max_retries,
                )
            }; // The lock guard is dropped here as it goes out of scope.

            let model_id = if mode == "local" {
                &local_config.smart_model_id
            } else { // "cloud"
                &cloud_config.smart_model_id
            };

            let response_result = if mode == "local" {
                let ollama_req = OllamaRequest { model: model_id, prompt: request.prompt.clone(), stream: false, format: if request.is_json_format { Some("json") } else { None } };
                client.post(&local_config.api_url).json(&ollama_req).send().await
            } else { // "cloud"
                if cloud_config.protocol == "openai" {
                    let open_ai_req = OpenAiRequest { model: model_id, messages: vec![OpenAiMessage { role: "user", content: request.prompt.clone() }] };
                    client.post(&cloud_config.api_url)
                        .bearer_auth(&cloud_config.api_key)
                        .json(&open_ai_req)
                        .send()
                        .await
                } else { // "custom" protocol for apifreellm
                    let custom_req = CustomCloudRequest { model: model_id, message: request.prompt.clone() };
                    client.post(&cloud_config.api_url)
                        .bearer_auth(&cloud_config.api_key)
                        .json(&custom_req)
                        .send()
                        .await
                }
            };

            match response_result {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await.unwrap_or_default();
                        
                        let text_result: Result<String, serde_json::Error> = if mode == "local" {
                            serde_json::from_str::<OllamaResponse>(&text).map(|r| r.response)
                        } else {
                            if cloud_config.protocol == "openai" {
                                serde_json::from_str::<OpenAiResponse>(&text).map(|r| r.choices.first().map_or("".to_string(), |c| c.message.content.clone()))
                            } else { // "custom" protocol
                                serde_json::from_str::<serde_json::Value>(&text).map(|v| {
                                    // Dynamically attempt to extract the text from common fields
                                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                                        content.to_string()
                                    } else if let Some(message) = v.get("message").and_then(|m| m.as_str()) {
                                        message.to_string()
                                    } else if let Some(response) = v.get("response").and_then(|r| r.as_str()) {
                                        response.to_string()
                                    } else if let Some(data) = v.get("data").and_then(|d| d.as_str()) {
                                        data.to_string()
                                    } else if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
                                        choices.first().and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()).unwrap_or("").to_string()
                                    } else {
                                        text.clone() // Fallback to raw text if no standard fields match
                                    }
                                })
                            }
                        };

                        match text_result {
                            Ok(generated_text) => {
                                if request.is_autonomous {
                                    let clean_json = generated_text.trim().trim_start_matches("```json").trim_end_matches("```").trim();
                                    match serde_json::from_str::<AutonomousAction>(clean_json) {
                                        Ok(action) => {
                                            match action.action.as_str() {
                                                "speak" => {
                                                    if let Some(content) = action.content {
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content });
                                                    }
                                                },
                                                "execute_command" => {
                                                    if let Some(command) = action.command {
                                                        execute_command_and_broadcast(command, tx_for_ai.clone(), request.agent_name.clone());
                                                    }
                                                },
                                                "direct_message" => {
                                                    if let (Some(recipient), Some(content)) = (action.recipient, action.content) {
                                                        if !recipient.is_empty() {
                                                            let dm_content = format!("@{}, {}", recipient, content);
                                                            let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content: dm_content });
                                                        }
                                                    }
                                                },
                                                "think" => {
                                                    if let Some(content) = action.content {
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "thinks".to_string(), content: format!("*{}*", content) });
                                                    }
                                                },
                                                "read_file" => {
                                                    if let Some(file_name) = action.file_name {
                                                        // Broadcast the intent to read, the agent will handle the rest
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "reads".to_string(), content: file_name });
                                                    }
                                                },
                                                "write_file" => {
                                                    if let (Some(file_name), Some(content)) = (action.file_name, action.content) {
                                                        match std::fs::write(&file_name, &content) {
                                                            Ok(_) => { let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "writes_file".to_string(), content: format!("Successfully wrote to {}", file_name) }); },
                                                            Err(e) => { let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "error".to_string(), content: format!("Failed to write {}: {}", file_name, e) }); }
                                                        }
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "announces".to_string(), content: format!("has written code to '{}'", file_name) });
                                                    }
                                                },
                                                "read_dir" => {
                                                    // Default to current directory if not specified
                                                    let path = action.dir_path.unwrap_or_else(|| ".".to_string());
                                                    // Broadcast the intent to read a directory, the agent will handle the rest
                                                    let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "reads_dir".to_string(), content: path });
                                                },
                                                "read_web" => {
                                                    if let Some(url) = action.url {
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "reads_web".to_string(), content: url });
                                                    }
                                                },
                                                "delegate_task" => {
                                                    if let (Some(recipient), Some(content)) = (action.recipient, action.content) {
                                                        let _ = tx_for_ai.send(Event {
                                                            sender: request.agent_name.clone(),
                                                            action: "delegates_task".to_string(),
                                                            content: format!("{}|{}", recipient, content),
                                                        });
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content: format!("@{}, I need you to handle this sub-task: {}", recipient, content) });
                                                    }
                                                },
                                                "complete_task" => {
                                                    if let Some(content) = action.content {
                                                        let _ = tx_for_ai.send(Event {
                                                            sender: request.agent_name.clone(),
                                                            action: "completes_task".to_string(),
                                                            content,
                                                        });
                                                    }
                                                },
                                                "play_move" => {
                                                    if let Some(content) = action.content {
                                                        // Extract just the first letter (N, S, E, W) in case LLM gets wordy
                                                        let dir = content.trim().chars().next().unwrap_or(' ').to_string().to_uppercase();
                                                        let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "plays_move".to_string(), content: dir });
                                                    }
                                                },
                                                _ => {}
                                            }
                                            // Also process any relationship updates
                                            if let Some(updates) = action.relationship_updates {
                                                for update in updates {
                                                    if let Ok(update_content) = serde_json::to_string(&update) {
                                                        let _ = tx_for_ai.send(Event {
                                                            sender: request.agent_name.clone(),
                                                            action: "updates_relationship".to_string(),
                                                            content: update_content,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => { eprintln!("[{}] Autonomous Action LLM Error: Failed to parse JSON from {} provider: {}\nRaw Text: {}", request.agent_name, mode, e, clean_json); }
                                    }
                                } else {
                                    let _ = tx_for_ai.send(Event { sender: request.agent_name.clone(), action: "speaks".to_string(), content: generated_text.trim().to_string() });
                                }
                            }
                            Err(e) => {
                                eprintln!("[{}] Error decoding JSON response from {} provider: {}. Raw response: {}", request.agent_name, mode, e, text);
                            }
                        }
                        break; // Success, exit retry loop
                    } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        if retries >= max_retries {
                            eprintln!("[{}] Max retries reached for rate limit. Giving up on this request.", request.agent_name);
                            break;
                        }
                        retries += 1;
                        let text = response.text().await.unwrap_or_default();
                        let wait_seconds = text.split("wait ").nth(1).and_then(|s| s.split(' ').next()).and_then(|n| n.parse::<u64>().ok());
                        let delay_duration = if let Some(seconds) = wait_seconds {
                            Duration::from_secs(seconds + 2) // Wait requested time + 2s buffer
                        } else {
                            let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..1000));
                            current_delay *= 2;
                            current_delay + jitter
                        };
                        eprintln!("[{}] Rate limited. Retrying in {:?}... (Attempt {}/{})", request.agent_name, delay_duration, retries, max_retries);
                        sleep(delay_duration).await;
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        eprintln!("[{}] API Error from {} provider: {} - {}", request.agent_name, mode, status, text);
                        break; // Unrecoverable API error, exit retry loop
                    }
                }
                Err(e) => {
                    if retries >= max_retries {
                        eprintln!("[{}] Max retries reached for connection error. Giving up on this request.", request.agent_name);
                        break;
                    }
                    retries += 1;
                    let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..1000));
                    current_delay *= 2;
                    let delay_duration = current_delay + jitter;
                    eprintln!("[{}] Connection Error to {} provider: {}. Retrying in {:?}... (Attempt {}/{})", request.agent_name, mode, e, delay_duration, retries, max_retries);
                    sleep(delay_duration).await;
                }
            }
        }

        // Small delay to let the system breathe between LLM generation calls
        sleep(Duration::from_millis(500)).await;
        
        // Signal the agent that its AI generation cycle is complete to release its busy lock
        let _ = tx_for_ai.send(Event {
            sender: request.agent_name.clone(),
            action: "ai_finished".to_string(),
            content: "".to_string(),
        });
    }
}