use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonState {
    /// idle | starting | recording | transcribing | working | error | stopped
    pub state: String,
    pub transcript: String,
    pub response: String,
    pub updated_at: u64,
    pub pid: u32,
    pub cwd: String,
    pub session_id: String,
    /// Populated when state == "error"
    pub error: String,
}

impl DaemonState {
    pub fn new(cwd: &str, session_id: &str) -> Self {
        DaemonState {
            state: "starting".into(),
            transcript: String::new(),
            response: String::new(),
            updated_at: now_ms(),
            pid: std::process::id(),
            cwd: cwd.into(),
            session_id: session_id.into(),
            error: String::new(),
        }
    }
}

pub fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".pi").join("voice")
}

pub fn pid_path() -> PathBuf {
    data_dir().join("pid")
}

pub fn write(state: &DaemonState) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("state.json.tmp");
    if let Ok(s) = serde_json::to_string_pretty(state) {
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, state_path());
        }
    }
}

pub fn read() -> Option<DaemonState> {
    serde_json::from_str(&std::fs::read_to_string(state_path()).ok()?).ok()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
