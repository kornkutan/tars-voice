use crate::{agent, audio, config::Config, dictation, hotkey, state, stt::Stt, tts};
use anyhow::Result;
use std::path::PathBuf;

pub fn run() -> Result<()> {
    let cwd: PathBuf = std::env::var("TARS_VOICE_CWD")
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir().map_err(anyhow::Error::from))
        .map_err(|e| anyhow::anyhow!("no cwd: {e}"))?;
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    let cwd_str = cwd.to_string_lossy().into_owned();

    let mut cfg = Config::load(&cwd)?;
    std::fs::create_dir_all(state::data_dir())?;
    std::fs::write(state::pid_path(), std::process::id().to_string())?;

    let session_id = format!("tars-voice-{}", short_hash(&cwd_str));
    let mut st = state::DaemonState::new(&cwd_str, &session_id);
    state::write(&st);

    eprintln!(
        "[tars-voice] daemon pid {} cwd {} key {} model {}",
        std::process::id(),
        cwd_str,
        cfg.key,
        cfg.model
    );

    // Load whisper first: downloads on first run (large), blocks until ready.
    let stt = match Stt::load(&cfg.model) {
        Ok(s) => s,
        Err(e) => {
            st.state = "error".into();
            st.error = format!("{e:#}");
            st.updated_at = state::now_ms();
            state::write(&st);
            return Err(e);
        }
    };

    let combo = hotkey::parse_combo(&cfg.key)?;
    let (tx, rx) = std::sync::mpsc::channel::<hotkey::HotkeyEvent>();
    hotkey::spawn(combo, tx);

    st.state = "idle".into();
    st.updated_at = state::now_ms();
    state::write(&st);
    eprintln!("[tars-voice] ready - hold {} to talk", cfg.key);

    let mut recorder: Option<audio::Recorder> = None;
    let mut rec_started_at = std::time::Instant::now();

    loop {
        // pick up config edits (mode, say settings) without a daemon restart
        if let Ok(fresh) = Config::load(&cwd) {
            cfg = fresh;
        }
        let ev = match rx.recv() {
            Ok(ev) => ev,
            Err(_) => break, // hotkey thread died
        };
        match ev {
            hotkey::HotkeyEvent::PttStart => {
                if recorder.is_none() {
                    match audio::start() {
                        Ok(rec) => {
                            rec_started_at = std::time::Instant::now();
                            recorder = Some(rec);
                            st.state = "recording".into();
                            st.transcript.clear();
                            st.response.clear();
                            st.updated_at = state::now_ms();
                            state::write(&st);
                        }
                        Err(e) => {
                            eprintln!("[tars-voice] mic error: {e:#}");
                            tts::speak("microphone error", cfg.say_voice.as_deref(), cfg.say_rate);
                        }
                    }
                }
            }
            hotkey::HotkeyEvent::PttStop => {
                let Some(rec) = recorder.take() else {
                    continue;
                };
                st.state = "transcribing".into();
                st.updated_at = state::now_ms();
                state::write(&st);

                eprintln!(
                    "[tars-voice] recording window: {} ms",
                    rec_started_at.elapsed().as_millis()
                );
                let samples = rec.finish();
                let transcript = match stt.transcribe(&samples, &cfg.language) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("[tars-voice] stt error: {e:#}");
                        st.state = "idle".into();
                        st.updated_at = state::now_ms();
                        state::write(&st);
                        continue;
                    }
                };

                if transcript.is_empty() {
                    eprintln!("[tars-voice] empty transcript, ignoring");
                    tts::chirp();
                    st.state = "idle".into();
                    st.updated_at = state::now_ms();
                    state::write(&st);
                    continue;
                }
                eprintln!("[tars-voice] transcript: {transcript}");

                if cfg.mode == "dictate" {
                    st.transcript = transcript.clone();
                    st.state = "dictating".into();
                    st.updated_at = state::now_ms();
                    state::write(&st);
                    dictation::paste(&transcript);
                    st.state = "idle".into();
                    st.updated_at = state::now_ms();
                    state::write(&st);
                    continue;
                }

                st.state = "working".into();
                st.transcript = transcript.clone();

                match agent::run(&transcript, &cwd, &session_id, &cfg) {
                    Ok(response) => {
                        eprintln!("[tars-voice] response: {}", truncate(&response, 200));
                        st.response = response.clone();
                        if cfg.say {
                            tts::speak(&response, cfg.say_voice.as_deref(), cfg.say_rate);
                        }
                    }
                    Err(e) => {
                        eprintln!("[tars-voice] agent error: {e:#}");
                        st.response = format!("error: {e:#}");
                        if cfg.say {
                            tts::speak("agent error", cfg.say_voice.as_deref(), cfg.say_rate);
                        }
                    }
                }
                st.state = "idle".into();
                st.updated_at = state::now_ms();
                state::write(&st);
            }
            hotkey::HotkeyEvent::Fatal(msg) => {
                eprintln!("[tars-voice] fatal: {msg}");
                st.state = "error".into();
                st.error = msg;
                st.updated_at = state::now_ms();
                state::write(&st);
                std::process::exit(1);
            }
        }
    }

    st.state = "stopped".into();
    st.updated_at = state::now_ms();
    state::write(&st);
    Ok(())
}

fn short_hash(s: &str) -> String {
    // FNV-1a 64
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:08x}")
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut end = n;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}
