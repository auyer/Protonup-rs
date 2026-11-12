#!/usr/bin/env bash
# Drive the real GUI window with synthetic X11 key events and follow its state
# through the window title.
#
# Usage:
#   scripts/gui_e2e_interactive.sh [path/to/protonup-rsui]
#
# Runs the GUI in an isolated, throwaway HOME (your real ~/.steam is never
# touched), seeds a fake Steam install there, presses Enter twice to start
# "Quick Update", then waits for the title to report the download finished and
# checks the tool really landed on disk.
#
# The window title mirrors the app state, which is what makes this test
# deterministic:
#   "Protonup-rs"                          initial menu
#   "Protonup-rs - Quick Update - Checking"
#   "Protonup-rs - Quick Update - Downloading"
#   "Protonup-rs - Quick Update done!"     finished
#
# The GE-Proton archive is a few hundred MB, so the first run takes a while.
#
# It works in two environments without any extra flags:
#   * desktop session  -> reuses your existing DISPLAY (X11 or XWayland)
#   * headless / CI    -> starts its own Xvfb server
#
# Requirements (Debian/Ubuntu):
#   sudo apt-get install xvfb xdotool libxcursor1 libxi6 libxfixes3 \
#       libxcb-xkb1 libxkbcommon-x11-0
# Missing libraries are reported by name, with the exact install command.
#
# Environment variables:
#   DISPLAY        reuse this X11 display instead of starting Xvfb
#   DISPLAY_NUM    Xvfb display number when one is started (default :99)
#   E2E_HOME       use this directory as the isolated HOME (default: a fresh
#                  temp dir); useful to inspect artifacts after a run
#   E2E_KEEP       if set, do not delete the temporary HOME after the run
#   WAIT_SECONDS   how long to wait for the download (default 600)
#   TITLE          window-title prefix to match (default "Protonup-rs")
#   KEY_DELAY      pause between synthetic keystrokes (default 0.3)
#   ICED_BACKEND   iced renderer (default tiny-skia)
set -euo pipefail

BIN="${1:-${PROTONUP_BIN:-./target/release/protonup-rsui}}"

# Matches `ProtonupGui::window_title()` in protonup-rsui/src/state.rs.
TITLE="${TITLE:-Protonup-rs}"

log() { printf '%s\n' "$*" >&2; }
die() {
    log "ERROR: $*"
    exit 1
}

need() {
    # $1 = command, $2 = Debian/Ubuntu package name.
    command -v "$1" > /dev/null 2>&1 ||
        die "missing '$1'. Install it with: sudo apt-get install $2"
}

if [ ! -x "$BIN" ]; then
    die "binary not found at '$BIN'. Build it first (cargo build --release -p protonup-rsui), or pass its path as the first argument."
fi

need xdotool xdotool

# ---- Runtime library preflight ----------------------------------------------
# winit loads the X11 stack with dlopen() at startup, so `ldd` reports nothing
# even when these are absent: the app aborts while creating the event loop.
# Probe the loader directly instead, and name the Debian packages.
X11_RUNTIME_LIBS=(
    "libX11.so.6:libx11-6"
    "libXcursor.so.1:libxcursor1"
    "libXrandr.so.2:libxrandr2"
    "libXi.so.6:libxi6"
    "libXinerama.so.1:libxinerama1"
    "libXfixes.so.3:libxfixes3"
    "libxcb-xkb.so.1:libxcb-xkb1"
    "libxkbcommon.so.0:libxkbcommon0"
    "libxkbcommon-x11.so.0:libxkbcommon-x11-0"
)

lib_is_present() {
    local lib="$1"
    if command -v ldconfig > /dev/null 2>&1; then
        ldconfig -p 2>/dev/null | grep -Fq "${lib} "
        return
    fi
    # No ldconfig: actually try to load it.
    python3 -c 'import ctypes,sys; ctypes.CDLL(sys.argv[1])' "$lib" > /dev/null 2>&1
}

check_runtime_libs() {
    local entry lib pkg
    local missing_pkgs=()
    for entry in "${X11_RUNTIME_LIBS[@]}"; do
        lib="${entry%%:*}"
        pkg="${entry##*:}"
        if ! lib_is_present "$lib"; then
            log "missing runtime library: $lib"
            missing_pkgs+=("$pkg")
        fi
    done

    if [ "${#missing_pkgs[@]}" -gt 0 ]; then
        log ""
        log "The GUI loads the X11 stack at runtime, so these are needed even"
        log "though the binary links fine. On Debian/Ubuntu:"
        log "    sudo apt-get install ${missing_pkgs[*]}"
        exit 1
    fi
}

check_runtime_libs

# ---- Isolated HOME -----------------------------------------------------------
CREATED_HOME=0
if [ -n "${E2E_HOME:-}" ]; then
    HOME_DIR="$E2E_HOME"
    mkdir -p "$HOME_DIR"
else
    HOME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/protonup-gui-e2e.XXXXXX")"
    CREATED_HOME=1
fi
export HOME="$HOME_DIR"

compat_dir="$HOME/.steam/steam/compatibilitytools.d"
mkdir -p "$compat_dir"
# This home is throwaway: clear any GE-Proton from a previous run so the GUI
# really downloads rather than short-circuiting to "up to date".
rm -rf "$compat_dir"/GE-Proton* 2>/dev/null || true

# ---- Display -----------------------------------------------------------------
APP_PID=""
XVFB_PID=""

# Invoked indirectly through the EXIT trap.
# shellcheck disable=SC2329
cleanup() {
    if [ -n "$APP_PID" ]; then
        kill "$APP_PID" 2>/dev/null || true
    fi
    if [ -n "$XVFB_PID" ]; then
        kill "$XVFB_PID" 2>/dev/null || true
    fi
    if [ "$CREATED_HOME" -eq 1 ] && [ -z "${E2E_KEEP:-}" ]; then
        rm -rf "$HOME_DIR"
    fi
}
# Invoked indirectly through the EXIT trap.
trap cleanup EXIT

if [ -n "${DISPLAY:-}" ] && xdotool getdisplaygeometry > /dev/null 2>&1; then
    log "Using existing X11 display $DISPLAY"
else
    need Xvfb xvfb
    DISPLAY_NUM="${DISPLAY_NUM:-:99}"
    Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 -nolisten tcp \
        > "$HOME_DIR/xvfb.log" 2>&1 &
    XVFB_PID=$!
    export DISPLAY="$DISPLAY_NUM"

    for _ in $(seq 1 100); do
        xdotool getdisplaygeometry > /dev/null 2>&1 && break
        sleep 0.2
    done
    if ! xdotool getdisplaygeometry > /dev/null 2>&1; then
        cat "$HOME_DIR/xvfb.log" >&2
        die "Xvfb did not start"
    fi
fi

# Force the X11 backend (xdotool's XTEST only works against X11/XWayland) and a
# CPU renderer so no GPU/Vulkan driver is required.
export WINIT_UNIX_BACKEND=x11
export ICED_BACKEND="${ICED_BACKEND:-tiny-skia}"
export LIBGL_ALWAYS_SOFTWARE=1

# ---- Helpers -----------------------------------------------------------------
app_log() {
    log "--- gui.log ---"
    cat "$HOME_DIR/gui.log" >&2 || true
}

# Current window title, or empty while the window does not exist yet.
window_title() {
    [ -n "$window_id" ] &&
        xdotool getwindowname "$window_id" 2>/dev/null ||
        true
}

send_key() {
    xdotool key --clearmodifiers "$1"
    sleep "${KEY_DELAY:-0.3}"
}

# Waits until the window title matches the given extended regex.
# $1 = pattern, $2 = timeout seconds, $3 = human description.
wait_for_title() {
    local pattern="$1" timeout_s="$2" desc="$3"
    local deadline=$((SECONDS + timeout_s))
    local current=""

    while [ "$SECONDS" -lt "$deadline" ]; do
        if ! kill -0 "$APP_PID" 2>/dev/null; then
            log "The GUI exited while waiting for ${desc}."
            app_log
            exit 1
        fi
        current=$(window_title)
        if printf '%s' "$current" | grep -Eq "$pattern"; then
            log "Title: $current"
            return 0
        fi
        if printf '%s' "$current" | grep -Eq -- "- Error$"; then
            log "The GUI reported an error while waiting for ${desc}."
            app_log
            exit 1
        fi
        sleep 0.5
    done

    log "Timed out after ${timeout_s}s waiting for ${desc} (last title: '${current}')."
    app_log
    exit 1
}

# ---- Start the GUI -----------------------------------------------------------
log "Isolated HOME: $HOME_DIR"
"$BIN" > "$HOME_DIR/gui.log" 2>&1 &
APP_PID=$!

# Wait for the window to map, bailing out as soon as the app dies so the real
# error (a panic in gui.log) is reported immediately instead of after a timeout.
window_id=""
deadline=$((SECONDS + 60))
while [ "$SECONDS" -lt "$deadline" ]; do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
        log "The GUI exited before its window appeared."
        app_log
        exit 1
    fi
    window_ids=$(xdotool search --onlyvisible --name "$TITLE" 2>/dev/null || true)
    if [ -n "$window_ids" ]; then
        window_id=${window_ids%%$'\n'*}
        break
    fi
    sleep 0.5
done

if [ -z "$window_id" ]; then
    log "The GUI window never appeared within 60s."
    app_log
    exit 1
fi

# Bare Xvfb runs no window manager, so input focus is PointerRoot. Focus the
# window explicitly (best effort) and park the pointer inside it so XTEST key
# events land on the application. With a real window manager the window can be
# placed anywhere, so aim at its center instead of a fixed screen position.
timeout 10 xdotool windowfocus --sync "$window_id" 2>/dev/null || true
if geometry=$(xdotool getwindowgeometry --shell "$window_id" 2>/dev/null); then
    eval "$geometry" # sets X, Y, WIDTH and HEIGHT
    xdotool mousemove "$((X + WIDTH / 2))" "$((Y + HEIGHT / 2))"
else
    xdotool mousemove 640 400
fi

# ---- Drive the app -----------------------------------------------------------
wait_for_title "^${TITLE}\$" 60 "the initial menu"

# The title also tracks the shortcut help panels, which is a cheap way to prove
# that key events really reach the application.
send_key question
wait_for_title "Keyboard Shortcuts" 30 "the keyboard shortcut panel"
send_key Escape
wait_for_title "^${TITLE}\$" 30 "the initial menu again"

# First Enter only engages the focus highlight; the second one activates the
# focused sidebar entry ("Quick Update"). The title then reports the flow.
send_key Return
send_key Return
wait_for_title "${TITLE} - Quick Update" 120 "the Quick Update screen"
wait_for_title "${TITLE} - Quick Update done!" "${WAIT_SECONDS:-600}" "the download to finish"

# ---- Verify the result on disk ----------------------------------------------
installed=$(compgen -G "$compat_dir/GE-Proton*" || true)
if [ -z "$installed" ]; then
    log "The GUI reported success but nothing was installed in $compat_dir"
    ls -R "$HOME/.steam" >&2 || true
    app_log
    exit 1
fi

echo "Installed via keyboard events:"
# shellcheck disable=SC2086  # word splitting is intentional: one path per match
ls -d $installed

found_proton=$(find "$compat_dir" -maxdepth 5 -name proton -print -quit)
if [ -z "$found_proton" ]; then
    log "The installed tree looks empty."
    ls -R "$compat_dir" >&2
    exit 1
fi

echo "Found: $found_proton"
if [ -n "${E2E_KEEP:-}" ]; then
    echo "Artifacts kept at: $HOME_DIR"
fi
exit 0
