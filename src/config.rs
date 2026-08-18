use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Push-to-talk combo: modifiers + main key, e.g. "alt+space", "ctrl+shift+v"
    pub key: String,
    /// Whisper ggml model name (large-v3-turbo-q5_0, medium-q5_0, base, ...)
    pub model: String,
    /// Optional pi --model pattern; empty = pi's configured default
    pub agent_model: String,
    /// Speak the agent's response with macOS `say`
    pub say: bool,
    /// Optional `say -v` voice name
    pub say_voice: Option<String>,
    /// Whisper language: "auto" or ISO code ("en", "th", ...)
    pub language: String,
    /// Ephemeral commands (no persistent session)
    pub no_session: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            key: "alt+space".into(),
            model: "large-v3-turbo-q5_0".into(),
            agent_model: String::new(),
            say: true,
            say_voice: None,
            language: "auto".into(),
            no_session: false,
        }
    }
}

impl Config {
    pub fn load(cwd: &Path) -> anyhow::Result<Config> {
        let candidates = [
            Some(cwd.join(".pi").join("tars-voice.json")),
            home_config_path(),
        ];
        for path in candidates.iter().flatten() {
            if path.exists() {
                let raw = std::fs::read_to_string(path)?;
                let mut cfg: Config = serde_json::from_str(&raw)?;
                // allow partial overrides on top of defaults
                if raw.contains("\"key\"") == false && cfg.key.is_empty() {
                    cfg.key = "alt+space".into();
                }
                return Ok(cfg);
            }
        }
        Ok(Config::default())
    }
}

fn home_config_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".pi").join("tars-voice.json"))
}
