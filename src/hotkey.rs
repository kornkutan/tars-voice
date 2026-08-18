use anyhow::Result;
use std::sync::mpsc::Sender;

pub enum HotkeyEvent {
    PttStart,
    PttStop,
    Fatal(String),
}

#[derive(Debug, Clone)]
pub struct Combo {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: rdev::Key,
}

pub fn parse_combo(spec: &str) -> Result<Combo> {
    let mut combo = Combo {
        alt: false,
        ctrl: false,
        shift: false,
        meta: false,
        key: rdev::Key::Space,
    };
    for part in spec.to_lowercase().split('+') {
        match part.trim() {
            "alt" | "opt" | "option" => combo.alt = true,
            "ctrl" | "control" => combo.ctrl = true,
            "shift" => combo.shift = true,
            "meta" | "cmd" | "command" => combo.meta = true,
            "space" => combo.key = rdev::Key::Space,
            "tab" => combo.key = rdev::Key::Tab,
            "escape" => combo.key = rdev::Key::Escape,
            other => {
                if let Some(c) = other.chars().next() {
                    combo.key = key_for_char(c)?;
                } else {
                    anyhow::bail!("invalid key spec: {spec}");
                }
            }
        }
    }
    Ok(combo)
}

fn key_for_char(c: char) -> Result<rdev::Key> {
    use rdev::Key;
    Ok(match c.to_ascii_uppercase() {
        'A' => Key::KeyA, 'B' => Key::KeyB, 'C' => Key::KeyC, 'D' => Key::KeyD,
        'E' => Key::KeyE, 'F' => Key::KeyF, 'G' => Key::KeyG, 'H' => Key::KeyH,
        'I' => Key::KeyI, 'J' => Key::KeyJ, 'K' => Key::KeyK, 'L' => Key::KeyL,
        'M' => Key::KeyM, 'N' => Key::KeyN, 'O' => Key::KeyO, 'P' => Key::KeyP,
        'Q' => Key::KeyQ, 'R' => Key::KeyR, 'S' => Key::KeyS, 'T' => Key::KeyT,
        'U' => Key::KeyU, 'V' => Key::KeyV, 'W' => Key::KeyW, 'X' => Key::KeyX,
        'Y' => Key::KeyY, 'Z' => Key::KeyZ,
        other => anyhow::bail!("unsupported main key: {other}"),
    })
}

/// Spawn the global grab listener. Blocks internally in its own thread;
/// events (and fatal errors) arrive via `tx`.
pub fn spawn(combo: Combo, tx: Sender<HotkeyEvent>) {
    std::thread::spawn(move || {
        use rdev::{grab, Event, EventType, Key};
        use std::cell::Cell;

        let alt_down = Cell::new(false);
        let ctrl_down = Cell::new(false);
        let shift_down = Cell::new(false);
        let meta_down = Cell::new(false);
        let ptt_active = Cell::new(false);

        let tx_fatal = tx.clone();
        let result = grab(move |event: Event| -> Option<Event> {
            match &event.event_type {
                EventType::KeyPress(k) => {
                    match k {
                        Key::Alt | Key::AltGr => alt_down.set(true),
                        Key::ControlLeft | Key::ControlRight => ctrl_down.set(true),
                        Key::ShiftLeft | Key::ShiftRight => shift_down.set(true),
                        Key::MetaLeft | Key::MetaRight => meta_down.set(true),
                        _ => {}
                    }
                    if *k == combo.key
                        && !ptt_active.get()
                        && alt_down.get() == combo.alt
                        && ctrl_down.get() == combo.ctrl
                        && shift_down.get() == combo.shift
                        && meta_down.get() == combo.meta
                    {
                        ptt_active.set(true);
                        let _ = tx.send(HotkeyEvent::PttStart);
                        return None; // swallow: option+space must not type U+00A0
                    }
                }
                EventType::KeyRelease(k) => {
                    match k {
                        Key::Alt | Key::AltGr => alt_down.set(false),
                        Key::ControlLeft | Key::ControlRight => ctrl_down.set(false),
                        Key::ShiftLeft | Key::ShiftRight => shift_down.set(false),
                        Key::MetaLeft | Key::MetaRight => meta_down.set(false),
                        _ => {}
                    }
                    if *k == combo.key && ptt_active.get() {
                        ptt_active.set(false);
                        let _ = tx.send(HotkeyEvent::PttStop);
                        return None;
                    }
                }
                _ => {}
            }
            Some(event)
        });

        if let Err(e) = result {
            let _ = tx_fatal.send(HotkeyEvent::Fatal(format!(
                "global hotkey grab failed: {e:?}. Grant Accessibility permission \
                 to tars-voice in System Settings > Privacy & Security > Accessibility, \
                 then start again."
            )));
        }
    });
}
