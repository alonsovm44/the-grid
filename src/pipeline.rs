use crate::agent_blueprint::AgentBlueprint;
use crate::ai_provider::AiRequest;
use crate::Event;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Represents the context flowing between pipeline stages.
/// This is the "mental state" that gets piped from one agent to the next.
#[derive(Debug, Clone)]
pub struct PipelineContext {
    pub input: String,
    pub output: String,
    pub agent: String,
    pub stage: usize,
    pub total_stages: usize,
}

/// A single stage in a semantic pipeline, ready for execution.
#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub agent_name: String,
    pub blueprint: AgentBlueprint,
    pub args: String,
}

/// The result of a full pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineTrace {
    pub stages: Vec<PipelineContext>,
    pub final_output: String,
}

/// Executes a semantic pipeline asynchronously.
///
/// Each stage:
/// 1. Builds a prompt from the blueprint's system_prompt + user args + previous output
/// 2. Sends an AiRequest with a oneshot response channel
/// 3. Awaits the response
/// 4. Passes the output as context to the next stage
pub async fn execute_pipeline(
    stages: Vec<PipelineStage>,
    ai_tx: mpsc::Sender<AiRequest>,
    event_tx: broadcast::Sender<Event>,
) -> Result<PipelineTrace, String> {
    let total = stages.len();
    let mut trace = PipelineTrace {
        stages: Vec::with_capacity(total),
        final_output: String::new(),
    };
    let mut previous_output = String::new();

    let _ = event_tx.send(Event {
        sender: "Pipeline".to_string(),
        action: "announces".to_string(),
        content: format!("Starting semantic pipeline with {} stages...", total),
    });

    for (i, stage) in stages.iter().enumerate() {
        let stage_num = i + 1;

        // Broadcast progress
        let _ = event_tx.send(Event {
            sender: "Pipeline".to_string(),
            action: "announces".to_string(),
            content: format!("[{}/{}] Invoking agent '{}'...", stage_num, total, stage.agent_name),
        });

        // Build the prompt
        let prompt = build_stage_prompt(stage, &previous_output, stage_num, total);

        // Create oneshot channel for synchronous response
        let (resp_tx, resp_rx) = oneshot::channel::<String>();

        let request = AiRequest {
            agent_name: format!("pipeline:{}", stage.agent_name),
            prompt,
            is_json_format: false,
            is_autonomous: false,
            iq_level: stage.blueprint.base_iq,
            current_pos: [0.0, 0.0, 0.0],
            nearby_objects: String::new(),
            response_tx: Some(resp_tx),
        };

        // Send to AI engine
        if ai_tx.send(request).await.is_err() {
            return Err(format!("Pipeline stage {}: Failed to send request to AI engine", stage_num));
        }

        // Await the response (with timeout)
        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(120),
            resp_rx,
        ).await {
            Ok(Ok(text)) => text,
            Ok(Err(_)) => return Err(format!("Pipeline stage {}: AI engine dropped the response channel", stage_num)),
            Err(_) => return Err(format!("Pipeline stage {}: Timed out after 120s", stage_num)),
        };

        // Broadcast stage completion
        let preview = if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output.clone()
        };
        let _ = event_tx.send(Event {
            sender: format!("pipeline:{}", stage.agent_name),
            action: "speaks".to_string(),
            content: preview,
        });

        // Record in trace
        let ctx = PipelineContext {
            input: if previous_output.is_empty() { stage.args.clone() } else { previous_output.clone() },
            output: output.clone(),
            agent: stage.agent_name.clone(),
            stage: stage_num,
            total_stages: total,
        };
        trace.stages.push(ctx);

        previous_output = output;
    }

    trace.final_output = previous_output;

    // Broadcast pipeline completion
    let _ = event_tx.send(Event {
        sender: "Pipeline".to_string(),
        action: "announces".to_string(),
        content: format!("Pipeline complete. {} stages executed.", total),
    });

    Ok(trace)
}

/// Builds the prompt for a pipeline stage, injecting the blueprint's system prompt,
/// the user's arguments, and the previous stage's output as context.
fn build_stage_prompt(
    stage: &PipelineStage,
    previous_output: &str,
    stage_num: usize,
    total: usize,
) -> String {
    let mut prompt = String::new();

    // System identity from blueprint
    prompt.push_str(&format!("SYSTEM: {}\n\n", stage.blueprint.system_prompt));

    // Pipeline context
    prompt.push_str(&format!(
        "You are stage {}/{} in a semantic pipeline. ",
        stage_num, total
    ));

    if !previous_output.is_empty() {
        prompt.push_str("The previous agent produced the following output. Use it as your primary input context:\n\n");
        prompt.push_str("--- PREVIOUS AGENT OUTPUT ---\n");
        prompt.push_str(previous_output);
        prompt.push_str("\n--- END PREVIOUS OUTPUT ---\n\n");
    }

    if !stage.args.is_empty() {
        prompt.push_str(&format!("USER REQUEST: {}\n\n", stage.args));
    }

    prompt.push_str("Respond with your analysis/output directly. Be thorough but concise. Do not wrap in JSON.");

    prompt
}

/// Executes a .gsh (GridShell script) file.
/// Each line is a GridShell command executed sequentially.
/// Lines starting with # are comments. Empty lines are skipped.
/// Variables set with = are available in subsequent lines via $VAR syntax.
pub fn parse_gsh_file(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}
