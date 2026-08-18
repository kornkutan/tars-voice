mod agent;
mod audio;
mod config;
mod daemon;
mod hotkey;
mod state;
mod stt;
mod tts;

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use state::DaemonState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("start") => cmd_start(args.get(2).map(|s| s.as_str())),
        Some("stop") => cmd_stop(),
        Some("status") => cmd_status(),
        Some("run-daemon") => {
            if let Err(e) = daemon::run() {
                eprintln!("[tars-voice] daemon exiting: {e:#}");
                std::process::exit(1);
            }
        }
        _ => help(),
    }
}

fn help() {
    println!(
        "tars-voice - push-to-talk voice control for the Pi Coding Agent (TARS)

Usage:
  tars-voice start [cwd]   Start the voice daemon (default cwd: here)
  tars-voice status        Show daemon state, transcript and response
  tars-voice stop          Stop the daemon

Config: <cwd>/.pi/tars-voice.json (falls back to ~/.pi/tars-voice.json)
     \"say\": true, \"language\": \"auto\" }}

State:  ~/.pi-agent/tars-voice/state.json (read by the Pi status bar)
Log:    ~/.pi-agent/tars-voice/daemon.log"
    );
}

fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_pid() -> Option<u32> {
    std::fs::read_to_string(state::pid_path())
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn running_pid() -> Option<u32> {
    let pid = read_pid()?;
    pid_alive(pid).then_some(pid)
}

fn cmd_start(cwd_arg: Option<&str>) {
    let cwd = match cwd_arg {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().expect("cannot read cwd"),
    };
    let cwd = cwd.canonicalize().unwrap_or(cwd);

    if let Some(pid) = running_pid() {
        println!("tars-voice already running (pid {pid})");
        return;
    }

    std::fs::create_dir_all(state::data_dir()).expect("cannot create data dir");
    let log = state::data_dir().join("daemon.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .expect("cannot open daemon log");

    let exe = std::env::current_exe().expect("cannot resolve own path");
    let child = Command::new(exe)
        .arg("run-daemon")
        .env("TARS_VOICE_CWD", &cwd)
        .current_dir(&cwd)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(log_file)
        .spawn()
        .expect("failed to spawn daemon");

    // give it a moment to write state so we can surface immediate errors
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let mut hint = String::new();
    if let Some(st) = state::read() {
        if st.state == "error" {
            hint = st.error;
        }
    }
    if hint.is_empty() {
        println!(
            "tars-voice started (daemon pid {}, cwd {}, log {})",
            child.id(),
            cwd.display(),
            log.display()
        );
    } else {
        println!("tars-voice failed to start: {hint}");
        let _ = Command::new("kill").arg(child.id().to_string()).status();
    }
}

fn cmd_stop() {
    match running_pid() {
        Some(pid) => {
            let _ = Command::new("kill").arg(pid.to_string()).status();
            if let Some(mut st) = state::read() {
                st.state = "stopped".into();
                st.updated_at = state::now_ms();
                state::write(&st);
            }
            let _ = std::fs::remove_file(state::pid_path());
            println!("tars-voice stopped (pid {pid})");
        }
        None => println!("tars-voice not running"),
    }
}

fn cmd_status() {
    match running_pid() {
        Some(pid) => {
            let st: DaemonState = state::read().unwrap_or_else(|| {
                DaemonState::new("", "")
            });
            println!("running: pid {pid} state {}", st.state);
            println!("  cwd:       {}", st.cwd);
            println!("  session:   {}", st.session_id);
            if !st.transcript.is_empty() {
                println!("  last said: {}", st.transcript);
            }
            if !st.response.is_empty() {
                println!("  reply:     {}", truncate_oneline(&st.response));
            }
            if !st.error.is_empty() {
                println!("  error:     {}", st.error);
            }
        }
        None => {
            if let Some(st) = state::read() {
                if st.state == "error" {
                    println!("stopped (last error: {})", st.error);
                    return;
                }
            }
            println!("not running");
        }
    }
}

fn truncate_oneline(s: &str) -> String {
    let one_line: String = s.replace('\n', " ");
    if one_line.len() > 120 {
        let mut end = 120;
        while end > 0 && !one_line.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &one_line[..end])
    } else {
        one_line
    }
}
