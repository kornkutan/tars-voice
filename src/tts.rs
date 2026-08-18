use std::process::Command;

/// Speak text via macOS `say`. Fire and forget.
pub fn speak(text: &str, voice: Option<&str>) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let mut cmd = Command::new("say");
    if let Some(v) = voice {
        cmd.arg("-v").arg(v);
    }
    // say handles arbitrarily long input; responses are short enough anyway
    cmd.arg(text);
    if let Err(e) = cmd.spawn() {
        eprintln!("[tars-voice] say failed: {e}");
    }
}

/// Short audible cue so the user knows the command was heard but was empty.
pub fn chirp() {
    let _ = Command::new("say")
        .args(["-v", "Bell"])
        .arg("boop")
        .spawn();
}
