use crate::config::Config;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn pi_bin() -> String {
    if let Ok(bin) = std::env::var("TARS_VOICE_PI_BIN") {
        return bin;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let candidate = PathBuf::from(&home).join(".bun").join("bin").join("pi");
    if candidate.exists() {
        return candidate.to_string_lossy().into_owned();
    }
    "pi".into()
}

/// Run one voice command through `pi -p` in the project cwd.
/// Returns the final assistant text.
pub fn run(transcript: &str, cwd: &Path, session_id: &str, cfg: &Config) -> Result<String> {
    let mut cmd = Command::new(pi_bin());
    cmd.args(["-p", "--mode", "json"]);
    if !cfg.agent_model.is_empty() {
        cmd.arg("--model").arg(&cfg.agent_model);
    }
    if cfg.no_session {
        cmd.arg("--no-session");
    } else {
        cmd.arg("--session-id").arg(session_id);
    }
    cmd.arg(transcript);
    cmd.current_dir(cwd);

    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn {}", pi_bin()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut final_text = String::new();

    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v["type"] == "agent_end" {
            if let Some(messages) = v["messages"].as_array() {
                for msg in messages.iter().rev() {
                    if msg["role"] != "assistant" {
                        continue;
                    }
                    let text = assistant_text(msg);
                    if !text.is_empty() {
                        final_text = text;
                        break;
                    }
                }
            }
        }
    }

    if final_text.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
        anyhow::bail!(
            "pi exited {} without assistant text{}",
            output.status,
            if tail.is_empty() { String::new() } else { format!(": {tail}") }
        );
    }
    Ok(final_text)
}

fn assistant_text(msg: &Value) -> String {
    msg["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}
