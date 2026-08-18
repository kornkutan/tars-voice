use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn clipboard_read() -> Option<Vec<u8>> {
    let out = Command::new("pbpaste").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(out.stdout)
}

fn clipboard_write(data: &[u8]) -> bool {
    let mut child = match Command::new("pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(data);
    }
    child.wait().is_ok()
}

/// Paste text into the focused app: clipboard write + synthetic Cmd+V,
/// then restore the previous clipboard. Unicode-safe (Thai etc.).
pub fn paste(text: &str) {
    let saved = clipboard_read();
    if !clipboard_write(text.as_bytes()) {
        eprintln!("[tars-voice] dictation: pbcopy failed");
        return;
    }
    thread::sleep(Duration::from_millis(80)); // let the pasteboard settle
    use rdev::{simulate, EventType, Key};
    for ev in [
        EventType::KeyPress(Key::MetaLeft),
        EventType::KeyPress(Key::KeyV),
        EventType::KeyRelease(Key::KeyV),
        EventType::KeyRelease(Key::MetaLeft),
    ] {
        let _ = simulate(&ev);
        thread::sleep(Duration::from_millis(20));
    }
    thread::sleep(Duration::from_millis(500)); // let the app consume the paste
    if let Some(prev) = saved {
        clipboard_write(&prev);
    }
}
