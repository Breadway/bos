#!/usr/bin/env bash
# Disposable headless Hyprland harness for the window.float action shim
# (BOS-91). It never talks to the live session: Mutter runs a private headless
# Wayland display in its own runtime directory, Hyprland runs nested inside
# that display, and the test window is a Kitty owned by the nested instance.
#
#     scripts/test-window-float-headless.sh
#
# Requires Hyprland, hyprctl, mutter (51 or newer), kitty, jq and
# dbus-run-session. Prints SKIP and exits 0 when one is missing, so it is safe
# to call from a plain checkout or a machine without a compositor.
#
# Exit status is non-zero if a measured floating state or dispatch result is
# wrong.
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
SHIM="$REPO/iso/airootfs/etc/skel/.config/hypr/scripts/lib/window.lua"

for bin in Hyprland hyprctl mutter kitty jq dbus-run-session; do
    if ! command -v "$bin" >/dev/null 2>&1; then
        echo "SKIP: $bin is not installed"
        exit 0
    fi
done

if [ ! -f "$SHIM" ]; then
    echo "FAIL: shim not found at $SHIM"
    exit 1
fi

# Keep the runtime path short: the nested Hyprland IPC socket appends
# /hypr/<signature>/.socket.sock and Unix socket paths are length limited.
S=$(mktemp -d "${TMPDIR:-/tmp}/bos91.XXXXXX") || exit 1
RUN="$S/run"
mkdir -p "$RUN" "$S/home"
chmod 700 "$RUN"

LIVE_SIG=${HYPRLAND_INSTANCE_SIGNATURE:-}
unset HYPRLAND_INSTANCE_SIGNATURE HYPRLAND_CMD WAYLAND_DISPLAY DISPLAY SWAYSOCK
unset DBUS_SESSION_BUS_ADDRESS KITTY_LISTEN_ON KITTY_PID KITTY_WINDOW_ID

PIDS=()
cleanup() {
    local status=$?
    set +e
    for pid in "${PIDS[@]}"; do
        kill -- "-$pid" 2>/dev/null
        kill "$pid" 2>/dev/null
    done
    for pid in "${PIDS[@]}"; do
        wait "$pid" 2>/dev/null
    done
    rm -rf "$S"
    return $status
}
trap cleanup EXIT INT TERM

# Start a command in its own process group and record the group leader pid in
# a file. Killing the negative pid later removes every process it spawned.
start_group() {
    local pidfile=$1 logfile=$2
    shift 2
    setsid bash -c 'echo $$ > "$1"; shift; exec "$@"' bash "$pidfile" "$@" >"$logfile" 2>&1 &
    local i
    for i in $(seq 1 50); do
        [ -s "$pidfile" ] && break
        sleep 0.1
    done
    if [ ! -s "$pidfile" ]; then
        echo "FAIL: could not start $* (see $logfile)"
        cat "$logfile" 2>/dev/null
        return 1
    fi
    PIDS+=("$(cat "$pidfile")")
}

wait_for_path() {
    local path=$1 i
    for i in $(seq 1 100); do
        [ -e "$path" ] && return 0
        sleep 0.1
    done
    return 1
}

echo "== disposable headless Hyprland =="
start_group "$S/mutter.pid" "$S/mutter.log" \
    env XDG_RUNTIME_DIR="$RUN" HOME="$S/home" XDG_CONFIG_HOME="$S/home/.config" \
    dbus-run-session -- mutter --headless --wayland --wayland-display=bos91 \
    --virtual-monitor 1600x1000
if ! wait_for_path "$RUN/bos91"; then
    echo "FAIL: Mutter did not create the bos91 Wayland socket"
    cat "$S/mutter.log"
    exit 1
fi

cat >"$S/hypr.lua" <<EOF
hl.config({ misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })
dofile("$SHIM")(hl)
EOF

start_group "$S/hypr.pid" "$S/hypr.log" \
    env XDG_RUNTIME_DIR="$RUN" HOME="$S/home" XDG_CONFIG_HOME="$S/home/.config" \
    WAYLAND_DISPLAY=bos91 XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=Hyprland \
    Hyprland -c "$S/hypr.lua"
if ! wait_for_path "$RUN/hypr"; then
    echo "FAIL: nested Hyprland did not start"
    cat "$S/hypr.log"
    exit 1
fi

SIG=""
for i in $(seq 1 100); do
    SIG=$(find "$RUN/hypr" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %f\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)
    if [ -n "$SIG" ] && [ -S "$RUN/hypr/$SIG/.socket.sock" ]; then
        break
    fi
    SIG=""
    sleep 0.1
done
if [ -z "$SIG" ]; then
    echo "FAIL: nested Hyprland did not create an instance socket"
    cat "$S/hypr.log"
    exit 1
fi
if [ -n "$LIVE_SIG" ] && [ "$SIG" = "$LIVE_SIG" ]; then
    echo "FAIL: refusing to use the live Hyprland instance"
    exit 1
fi
echo "nested signature: $SIG"

hypr() {
    XDG_RUNTIME_DIR="$RUN" HYPRLAND_INSTANCE_SIGNATURE="$SIG" hyprctl "$@"
}

NESTED_SOCKET=""
for i in $(seq 1 100); do
    NESTED_SOCKET=$(find "$RUN" -maxdepth 1 -type s -name 'wayland-*' ! -name 'bos91' 2>/dev/null | head -1)
    [ -n "$NESTED_SOCKET" ] && break
    sleep 0.1
done
if [ -z "$NESTED_SOCKET" ]; then
    echo "FAIL: nested Hyprland did not create a Wayland socket"
    exit 1
fi
NESTED_WAYLAND=$(basename "$NESTED_SOCKET")
echo "nested wayland:  $NESTED_WAYLAND"

start_group "$S/kitty.pid" "$S/kitty.log" \
    env XDG_RUNTIME_DIR="$RUN" WAYLAND_DISPLAY="$NESTED_WAYLAND" HOME="$S/home" \
    XDG_CONFIG_HOME="$S/home/.config" kitty --config NONE --title bos91-float-test \
    -o font_family=monospace -o font_size=11

ADDR=""
for i in $(seq 1 100); do
    ADDR=$(hypr clients -j 2>/dev/null | jq -r '.[] | select(.class == "kitty" and .title == "bos91-float-test") | .address' | head -1)
    [ -n "$ADDR" ] && break
    sleep 0.1
done
if [ -z "$ADDR" ]; then
    echo "FAIL: test kitty did not map"
    hypr clients -j 2>/dev/null
    cat "$S/kitty.log"
    exit 1
fi
echo "test window:     $ADDR"

FLOAT=$(hypr clients -j | jq -r --arg a "$ADDR" '.[] | select(.address == $a) | .floating')

pass=0
fail=0
ok() { printf '  PASS  %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf '  FAIL  %s\n' "$1"; fail=$((fail + 1)); }

# Leave the window tiled before the measured sequence.
hypr eval "hl.dispatch(hl.dsp.window.float({ action = 'unset', window = 'address:$ADDR' }))" >/dev/null 2>&1
FLOAT=$(hypr clients -j | jq -r --arg a "$ADDR" '.[] | select(.address == $a) | .floating')
if [ "$FLOAT" = "false" ]; then ok "window starts tiled"; else bad "window starts tiled (floating=$FLOAT)"; fi

measure() {
    local label=$1 code=$2 want=$3
    local out float
    out=$(hypr eval "$code" 2>&1)
    float=$(hypr clients -j | jq -r --arg a "$ADDR" '.[] | select(.address == $a) | .floating')
    printf '  %-24s return=%-4s floating=%s\n' "$label" "$out" "$float"
    if [ "$out" = "ok" ] && [ "$float" = "$want" ]; then
        ok "$label"
    else
        bad "$label (return=$out floating=$float, wanted $want)"
    fi
}

echo "== measured states =="
measure "set #1"   "hl.dispatch(hl.dsp.window.float({ action = 'set', window = 'address:$ADDR' }))"   "true"
measure "set #2"   "hl.dispatch(hl.dsp.window.float({ action = 'set', window = 'address:$ADDR' }))"   "true"
measure "unset #1" "hl.dispatch(hl.dsp.window.float({ action = 'unset', window = 'address:$ADDR' }))" "false"
measure "unset #2" "hl.dispatch(hl.dsp.window.float({ action = 'unset', window = 'address:$ADDR' }))" "false"
measure "toggle #1" "hl.dispatch(hl.dsp.window.float({ action = 'toggle', window = 'address:$ADDR' }))" "true"
measure "toggle #2" "hl.dispatch(hl.dsp.window.float({ action = 'toggle', window = 'address:$ADDR' }))" "false"

printf '\nResult: %d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
