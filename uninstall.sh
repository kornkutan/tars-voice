#!/bin/bash
# tars-voice uninstaller. Removes binary, Pi extension, config, runtime state.
# Asks before deleting the whisper model cache (~550 MB).

set -euo pipefail

BIN_DIR="$HOME/bin"
BIN_PATH="$BIN_DIR/tars-voice"
PI_EXT_DIR="$HOME/.pi/agent/extensions/voice"
PI_GLOBAL_CFG="$HOME/.pi/tars-voice.json"
DATA_DIR="$HOME/.pi/voice"
WHISPER_DIR="$HOME/Library/Caches/tars-voice/whisper"

say() { printf '%s\n' "$*"; }
ask() {
    # ask <prompt> ; returns 0 for yes
    local reply
    printf '%s [y/N] ' "$1" >&2
    read -r reply </dev/tty 2>/dev/null || return 1
    [[ "$reply" =~ ^[Yy]([Ee][Ss])?$ ]]
}

say "==> stopping daemon (if running)"
"$BIN_PATH" stop >/dev/null 2>&1 || true

say "==> removing binary $BIN_PATH"
rm -f "$BIN_PATH"

say "==> removing Pi extension $PI_EXT_DIR"
rm -rf "$PI_EXT_DIR"

if [ -f "$PI_GLOBAL_CFG" ]; then
    if ask "remove global config $PI_GLOBAL_CFG?"; then
        rm -f "$PI_GLOBAL_CFG"
        say "    removed"
    else
        say "    kept $PI_GLOBAL_CFG"
    fi
fi

if [ -d "$DATA_DIR" ]; then
    if ask "remove state/log dir $DATA_DIR?"; then
        rm -rf "$DATA_DIR"
        say "    removed"
    else
        say "    kept $DATA_DIR"
    fi
fi

if [ -d "$WHISPER_DIR" ] && [ -n "$(ls -A "$WHISPER_DIR" 2>/dev/null)" ]; then
    if ask "remove whisper model cache $WHISPER_DIR (~550 MB)?"; then
        rm -rf "$WHISPER_DIR"
        say "    removed"
    else
        say "    kept $WHISPER_DIR"
    fi
fi

say ""
say "uninstalled. two manual notes:"
say "  - restart Pi so the voice extension unloads"
say "  - macOS Accessibility list (System Settings > Privacy & Security >"
say "    Accessibility) may still show tars-voice; remove the entry by hand."
