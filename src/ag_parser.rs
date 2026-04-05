use std::collections::HashMap;

use crate::agent_blueprint::{AgentBlueprint, AgentEvolution, ToolConfig};

/// Parser for .ag files — The Grid's Agent Definition Language.
///
/// Format:
/// ```text
/// # Comments start with #
///
/// agent <name>
///   personality  "Quoted string value"
///   iq           0.8
///   spec         "Quoted string value"
///
/// permit
///   action1 action2 action3
///
/// prompt ---
/// Multi-line system prompt text.
/// Supports {specialization} and {personality} interpolation.
/// ---
///
/// tools
///   tool_name  priority:high safety:read_only
///   other_tool depth:deep
///
/// evolve
///   rate     0.1
///   feedback on
///   tracking on
///   xp       1.0
/// ```

#[derive(Debug)]
enum Section {
    None,
    Agent,
    Permit,
    Prompt,
    Tools,
    Evolve,
}

pub fn parse_ag_file(content: &str) -> Result<AgentBlueprint, String> {
    let mut name = String::new();
    let mut personality = String::new();
    let mut base_iq: f32 = 0.5;
    let mut specialization = String::new();
    let mut permissions: Vec<String> = Vec::new();
    let mut system_prompt = String::new();
    let mut tools: HashMap<String, ToolConfig> = HashMap::new();
    let mut evolution = AgentEvolution {
        learning_rate: 0.1,
        feedback_integration: true,
        performance_tracking: true,
        xp_multiplier: 1.0,
    };

    let mut section = Section::None;
    let mut in_heredoc = false;
    let mut prompt_lines: Vec<String> = Vec::new();

    for (line_num, raw_line) in content.lines().enumerate() {
        let line_num = line_num + 1; // 1-indexed for error messages

        // Inside a prompt heredoc block
        if in_heredoc {
            if raw_line.trim() == "---" {
                in_heredoc = false;
                system_prompt = prompt_lines.join("\n").trim().to_string();
                section = Section::None;
            } else {
                prompt_lines.push(raw_line.to_string());
            }
            continue;
        }

        let trimmed = raw_line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section headers (non-indented lines)
        if !raw_line.starts_with(' ') && !raw_line.starts_with('\t') {
            if trimmed.starts_with("agent ") {
                name = trimmed.strip_prefix("agent ").unwrap().trim().to_string();
                section = Section::Agent;
                continue;
            } else if trimmed == "permit" {
                section = Section::Permit;
                continue;
            } else if trimmed.starts_with("prompt") {
                // Check for heredoc opener: "prompt ---"
                let after = trimmed.strip_prefix("prompt").unwrap().trim();
                if after == "---" {
                    in_heredoc = true;
                    prompt_lines.clear();
                    section = Section::Prompt;
                } else if after.starts_with('"') {
                    // Single-line prompt: prompt "text"
                    system_prompt = unquote(after);
                    section = Section::None;
                } else {
                    return Err(format!("line {}: expected '---' or quoted string after 'prompt'", line_num));
                }
                continue;
            } else if trimmed == "tools" {
                section = Section::Tools;
                continue;
            } else if trimmed == "evolve" {
                section = Section::Evolve;
                continue;
            } else {
                return Err(format!("line {}: unknown section '{}'", line_num, trimmed));
            }
        }

        // Indented content belongs to the current section
        match section {
            Section::Agent => {
                let (key, value) = parse_kv(trimmed)
                    .ok_or_else(|| format!("line {}: expected 'key value' in agent block", line_num))?;
                match key {
                    "personality" => personality = unquote(value),
                    "iq" => base_iq = value.parse::<f32>()
                        .map_err(|_| format!("line {}: invalid float for iq: '{}'", line_num, value))?,
                    "spec" | "specialization" => specialization = unquote(value),
                    other => return Err(format!("line {}: unknown agent field '{}'", line_num, other)),
                }
            }
            Section::Permit => {
                // Space-separated list of permissions on one or more lines
                for token in trimmed.split_whitespace() {
                    permissions.push(token.to_string());
                }
            }
            Section::Tools => {
                // Format: tool_name  key:value key:value ...
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let tool_name = parts[0].to_string();
                let mut priority = "medium".to_string();
                let mut safety_level: Option<String> = None;
                let mut max_frequency: Option<u32> = None;

                for &prop in &parts[1..] {
                    if let Some((k, v)) = prop.split_once(':') {
                        match k {
                            "priority" => priority = v.to_string(),
                            "safety" => safety_level = Some(v.to_string()),
                            "frequency" => max_frequency = v.parse::<u32>().ok(),
                            "depth" | "mode" => { /* informational, stored as priority alias */ }
                            _ => return Err(format!("line {}: unknown tool property '{}'", line_num, k)),
                        }
                    } else {
                        return Err(format!("line {}: expected 'key:value' property, got '{}'", line_num, prop));
                    }
                }

                tools.insert(tool_name, ToolConfig {
                    enabled: true,
                    priority,
                    safety_level,
                    max_frequency,
                });
            }
            Section::Evolve => {
                let (key, value) = parse_kv(trimmed)
                    .ok_or_else(|| format!("line {}: expected 'key value' in evolve block", line_num))?;
                match key {
                    "rate" | "learning_rate" => evolution.learning_rate = value.parse::<f32>()
                        .map_err(|_| format!("line {}: invalid float for rate: '{}'", line_num, value))?,
                    "feedback" => evolution.feedback_integration = parse_bool(value),
                    "tracking" => evolution.performance_tracking = parse_bool(value),
                    "xp" | "xp_multiplier" => evolution.xp_multiplier = value.parse::<f32>()
                        .map_err(|_| format!("line {}: invalid float for xp: '{}'", line_num, value))?,
                    other => return Err(format!("line {}: unknown evolve field '{}'", line_num, other)),
                }
            }
            Section::Prompt => {
                // Shouldn't reach here outside heredoc
                return Err(format!("line {}: unexpected content in prompt section", line_num));
            }
            Section::None => {
                return Err(format!("line {}: content outside any section: '{}'", line_num, trimmed));
            }
        }
    }

    if in_heredoc {
        return Err("unterminated prompt heredoc (missing closing '---')".to_string());
    }

    if name.is_empty() {
        return Err("missing 'agent <name>' declaration".to_string());
    }

    Ok(AgentBlueprint {
        name,
        personality,
        base_iq,
        specialization,
        permissions,
        system_prompt,
        tools,
        evolution,
    })
}

/// Serialize an AgentBlueprint back to .ag format
pub fn serialize_ag(blueprint: &AgentBlueprint) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {} — Cognitive Blueprint for The Grid\n\n", blueprint.name));

    out.push_str(&format!("agent {}\n", blueprint.name));
    out.push_str(&format!("  personality  \"{}\"\n", blueprint.personality));
    out.push_str(&format!("  iq           {}\n", blueprint.base_iq));
    out.push_str(&format!("  spec         \"{}\"\n", blueprint.specialization));
    out.push('\n');

    out.push_str("permit\n");
    out.push_str(&format!("  {}\n", blueprint.permissions.join(" ")));
    out.push('\n');

    out.push_str("prompt ---\n");
    out.push_str(&blueprint.system_prompt);
    out.push_str("\n---\n");
    out.push('\n');

    if !blueprint.tools.is_empty() {
        out.push_str("tools\n");
        for (name, config) in &blueprint.tools {
            let mut props = vec![format!("priority:{}", config.priority)];
            if let Some(ref safety) = config.safety_level {
                props.push(format!("safety:{}", safety));
            }
            if let Some(freq) = config.max_frequency {
                props.push(format!("frequency:{}", freq));
            }
            out.push_str(&format!("  {}  {}\n", name, props.join(" ")));
        }
        out.push('\n');
    }

    out.push_str("evolve\n");
    out.push_str(&format!("  rate     {}\n", blueprint.evolution.learning_rate));
    out.push_str(&format!("  feedback {}\n", if blueprint.evolution.feedback_integration { "on" } else { "off" }));
    out.push_str(&format!("  tracking {}\n", if blueprint.evolution.performance_tracking { "on" } else { "off" }));
    out.push_str(&format!("  xp       {}\n", blueprint.evolution.xp_multiplier));

    out
}

// ── Helpers ──────────────────────────────────────────────

/// Split a trimmed line into (key, rest) on the first whitespace boundary.
fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let mut iter = line.splitn(2, |c: char| c.is_whitespace());
    let key = iter.next()?;
    let value = iter.next()?.trim();
    if value.is_empty() {
        None
    } else {
        Some((key, value))
    }
}

/// Strip surrounding double quotes if present.
fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse "on"/"true"/"yes"/"1" as true, everything else as false.
fn parse_bool(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "on" | "true" | "yes" | "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let input = r#"
agent test
  personality  "Bold and fearless"
  iq           0.9
  spec         "Testing"

permit
  read_file write_file

prompt ---
You are a test agent.
---

evolve
  rate     0.2
  feedback on
  tracking off
  xp       1.5
"#;
        let bp = parse_ag_file(input).unwrap();
        assert_eq!(bp.name, "test");
        assert_eq!(bp.personality, "Bold and fearless");
        assert_eq!(bp.base_iq, 0.9);
        assert_eq!(bp.permissions, vec!["read_file", "write_file"]);
        assert_eq!(bp.system_prompt, "You are a test agent.");
        assert_eq!(bp.evolution.learning_rate, 0.2);
        assert!(bp.evolution.feedback_integration);
        assert!(!bp.evolution.performance_tracking);
        assert_eq!(bp.evolution.xp_multiplier, 1.5);
    }

    #[test]
    fn test_parse_tools() {
        let input = r#"
agent tooled
  personality  "Efficient"
  iq           0.7
  spec         "Tools"

permit
  read_file

prompt ---
A prompt.
---

tools
  read_file      priority:high
  execute_command safety:read_only priority:medium

evolve
  rate 0.1
  feedback on
  tracking on
  xp 1.0
"#;
        let bp = parse_ag_file(input).unwrap();
        assert_eq!(bp.tools.len(), 2);
        assert_eq!(bp.tools["read_file"].priority, "high");
        assert_eq!(bp.tools["execute_command"].safety_level, Some("read_only".to_string()));
    }

    #[test]
    fn test_roundtrip() {
        let input = r#"
agent roundtrip
  personality  "Careful and precise"
  iq           0.75
  spec         "Serialization testing"

permit
  read_file think

prompt ---
You test serialization roundtrips.
---

tools
  read_file  priority:high

evolve
  rate     0.1
  feedback on
  tracking on
  xp       1.0
"#;
        let bp = parse_ag_file(input).unwrap();
        let serialized = serialize_ag(&bp);
        let bp2 = parse_ag_file(&serialized).unwrap();
        assert_eq!(bp.name, bp2.name);
        assert_eq!(bp.personality, bp2.personality);
        assert_eq!(bp.base_iq, bp2.base_iq);
        assert_eq!(bp.permissions, bp2.permissions);
    }
}
