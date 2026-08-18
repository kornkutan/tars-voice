#!/bin/bash
# tars-voice installer: build the binary and deploy the full system.
#   - binary           -> ~/bin/tars-voice
#   - Pi extension     -> ~/.pi/agent/extensions/voice/
#   - default config   -> ~/.pi/tars-voice.json (only if missing)
#   - runtime dirs     -> ~/.pi-agent/tars-voice, ~/.pi-agent/whisper

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$HOME/bin"
BIN_PATH="$BIN_DIR/tars-voice"
PI_EXT_DIR="$HOME/.pi/agent/extensions/voice"
PI_GLOBAL_CFG="$HOME/.pi/tars-voice.json"
DATA_DIR="$HOME/.pi-agent/tars-voice"
WHISPER_DIR="$HOME/.pi-agent/whisper"

say() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# -- build ------------------------------------------------------------------

command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust: https://rustup.rs"

say "==> building (release, arm64 native)..."
(
    cd "$REPO_ROOT"
    if command -v arch >/dev/null 2>&1 && arch -arm64 true 2>/dev/null; then
        arch -arm64 cargo build --release
    else
        cargo build --release
    fi
)
[ -f "$REPO_ROOT/target/release/tars-voice" ] || die "build produced no binary"

say "==> installing binary to $BIN_PATH"
mkdir -p "$BIN_DIR"
cp "$REPO_ROOT/target/release/tars-voice" "$BIN_PATH"
chmod +x "$BIN_PATH"

# -- Pi extension -----------------------------------------------------------

say "==> deploying Pi extension to $PI_EXT_DIR"
mkdir -p "$PI_EXT_DIR"
cp "$REPO_ROOT/pi-extension/index.ts" "$PI_EXT_DIR/index.ts"
cp "$REPO_ROOT/pi-extension/config.json" "$PI_EXT_DIR/config.json"

# -- config + runtime dirs --------------------------------------------------

mkdir -p "$DATA_DIR" "$WHISPER_DIR"

if [ ! -f "$PI_GLOBAL_CFG" ]; then
    say "==> writing default config $PI_GLOBAL_CFG"
    cat > "$PI_GLOBAL_CFG" << 'EOF'
{
  "key": "alt+space",
  "model": "large-v3-turbo-q5_0",
  "agent_model": "",
  "say": true,
  "say_voice": null,
  "language": "auto",
  "no_session": false
}
EOF
else
    say "==> keeping existing config $PI_GLOBAL_CFG"
fi

# -- done -------------------------------------------------------------------

say ""
say "installed:"
say "  binary:      $BIN_PATH"
say "  extension:   $PI_EXT_DIR (restart Pi to load)"
say "  config:      $PI_GLOBAL_CFG (or <project>/.pi/tars-voice.json)"
say "  state/log:   $DATA_DIR"
say ""
say "next steps:"
say "  1. If not done yet: System Settings > Privacy & Security > Accessibility"
say "     -> add $BIN_PATH"
say "  2. From a project dir: $BIN_PATH start"
say "  3. Hold alt+space, speak, release."
say ""
say "note: first run downloads the whisper model (~550 MB) to $WHISPER_DIR"
