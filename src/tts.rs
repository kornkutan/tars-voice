use std::process::Command;

pub fn speak(text: &str, voice: Option<&str>, rate: Option<u32>) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let mut cmd = Command::new("say");
    if let Some(v) = voice {
        cmd.arg("-v").arg(v);
    }
    if let Some(r) = rate {
        cmd.arg("-r").arg(r.to_string());
    }
    cmd.arg(text);
    if let Err(e) = cmd.spawn() {
        eprintln!("[tars-voice] say failed: {e}");
    }
}
/// Short audible cue so the user knows the command was heard but was empty.
pub fn chirp() {
    let _ = Command::new("afplay")
        .arg("/System/Library/Sounds/Tink.aiff")
        .spawn();
}
