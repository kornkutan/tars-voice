# tars-voice

Push-to-talk voice control for the [Pi Coding Agent](https://github.com/badlogic/pi-mono)
(TARS fork: `@earendil-works/pi-coding-agent`). Hold a key, speak, and your
installed `pi` runs the command with full TARS config (extensions, models,
skills, AGENTS.md). The agent's reply is spoken back via macOS `say`.

Single static Rust binary (~2 MB). No Node runtime for the daemon, no
bundled agent SDK (it shells out to your installed `pi`, so it always runs
your version with your full config).

## Architecture

```
tars-voice (Rust binary)
  hotkey    rdev global grab (macOS CGEventTap) -> hold-to-record
  audio     cpal CoreAudio capture -> 16 kHz mono (downmix + resample)
  stt       whisper-rs (whisper.cpp, Metal) -> ggml model transcribe
  agent     spawn `pi -p --mode json --session-id tars-voice-<cwd-hash> "<text>"`
  tts       pipe final assistant text to macOS `say`
```

State is written to `~/.pi-agent/tars-voice/state.json`, read by the Pi
status-bar extension (`~/.pi/agent/extensions/voice/`) so you get a `VOICE:`
indicator and `/voice start|stop|status` slash commands inside Pi.

## Build

Requires Xcode command line tools (clang, cmake) and Rust. Build natively on
Apple Silicon (the whisper.cpp CMake step needs a consistent arm64 environment;
if your shell runs under Rosetta, prefix commands with `arch -arm64`).

```sh
cargo build --release
cp target/release/tars-voice ~/bin/tars-voice
```

The `metal` feature is on by default (Apple Silicon GPU). First run downloads
the whisper model to `~/.pi-agent/whisper/`.

## Usage

```sh
tars-voice start [cwd]   # start the daemon (default cwd: current dir)
tars-voice status        # state, last transcript, last reply
tars-voice stop
```

Config lives at `<cwd>/.pi/tars-voice.json` (falls back to `~/.pi/tars-voice.json`):

```json
{
  "key": "alt+space",
  "model": "large-v3-turbo-q5_0",
  "agent_model": "*haiku*",
  "say": true,
  "say_voice": null,
  "language": "auto",
  "no_session": false
}
```

- `key` — push-to-talk combo: modifiers (`ctrl`/`shift`/`alt`|`opt`/`meta`|`cmd`)
  + a main key (`space`, letters, `tab`, `escape`). Default `alt+space`.
- `model` — whisper ggml model name (e.g. `large-v3-turbo-q5_0`, `medium-q5_0`,
  `base`). Bigger = more accurate, slower.
- `agent_model` — `pi --model` pattern for the agent. Voice wants fast + cheap,
  so `*haiku*` by default.
- `say` — speak the agent's reply.
- `say_voice` — optional `say -v` voice name.
- `language` — `auto` or an ISO code (`en`, `th`, ...).
- `no_session` — ephemeral commands (no persistent `--session-id`).

## macOS permission (one-time)

Global hotkeys need Accessibility. If `tars-voice status` shows
`global hotkey grab failed: EventTapError`, open
**System Settings > Privacy & Security > Accessibility** and add
`~/bin/tars-voice` (or your terminal app), then `tars-voice start` again.

## Pi integration

The companion extension at `~/.pi/agent/extensions/voice/` is auto-discovered.
Restart Pi after adding it. It adds:

- a `VOICE:` status-bar line (idle / REC / working / error)
- the `/voice start|stop|status` slash command

Its sibling `config.json` can override `binaryPath`, `statePath`,
`pollIntervalMs`.

## Notes

- Sessions persist per project via `--session-id tars-voice-<hash-of-cwd>`, so
  multi-turn voice commands share context across daemon restarts. Set
  `no_session: true` for one-shot commands.
- The grab callback swallows the push-to-talk key, so `alt+space` won't type a
  non-breaking space into the focused app.
- The daemon speaks only the final assistant message (parsed from `pi -p` JSON
  `agent_end`), not tool output.