#!/usr/bin/env bash
# Runs the shipped breadbug-notify.lua module under a real breadd in a
# throwaway HOME, with stub notify-send/kitty, and checks what it does when
# breadbug markers arrive. Needs a breadd that has the bug.* IPC methods
# (bread >= 0.9.x with breadbug); skips otherwise.
#
#   BREADD=/path/to/breadd scripts/test-breadbug-notify.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULE="${HERE}/iso/airootfs/etc/skel/.config/bread/modules/breadbug-notify.lua"
BREADD="${BREADD:-breadd}"
command -v "$BREADD" >/dev/null || { echo "skip: no breadd ($BREADD)"; exit 0; }

T="$(mktemp -d)"
PID=""
cleanup() { [ -n "$PID" ] && kill "$PID" 2>/dev/null || true; rm -rf "$T"; }
trap cleanup EXIT
fail() { echo "FAIL: $*" >&2; echo "--- notify log:" >&2; cat "$T/notify.log" 2>/dev/null >&2 || true; exit 1; }

mkdir -p "$T/home" "$T/rt" "$T/cfg/bread/modules" "$T/stubs"
cp "$MODULE" "$T/cfg/bread/modules/"
echo 'return bread' > "$T/cfg/bread/init.lua"
cat > "$T/cfg/bread/breadd.toml" <<'TOML'
[daemon]
log_level = "error"
[lua]
entry_point = "~/.config/bread/init.lua"
module_path = "~/.config/bread/modules"
[adapters.hyprland]
enabled = false
[adapters.udev]
enabled = false
[adapters.power]
enabled = false
[adapters.network]
enabled = false
[adapters.podman]
enabled = false
TOML
# Stubs: record the notification, never open a window, never wait.
cat > "$T/stubs/notify-send" <<STUB
#!/bin/sh
echo "notify-send \$*" >> "$T/notify.log"
# Pretend the user clicked "Review report".
echo review
exit 0
STUB
cat > "$T/stubs/kitty" <<STUB
#!/bin/sh
echo "kitty \$*" >> "$T/notify.log"
STUB
chmod +x "$T/stubs/"*

export HOME="$T/home" XDG_CONFIG_HOME="$T/cfg" XDG_RUNTIME_DIR="$T/rt" XDG_STATE_HOME="$T/home/.local/state"
export PATH="$T/stubs:$PATH"
"$BREADD" >"$T/breadd.log" 2>&1 &
PID=$!
SOCK="$T/rt/bread/breadd.sock"
for _ in $(seq 1 100); do [ -S "$SOCK" ] && break; sleep 0.1; done
[ -S "$SOCK" ] || fail "breadd did not start: $(cat "$T/breadd.log")"

rpc() {
python3 - "$SOCK" "$1" "$2" <<'PY'
import json, socket, sys
sock, method, params = sys.argv[1], sys.argv[2], json.loads(sys.argv[3])
s = socket.socket(socket.AF_UNIX); s.connect(sock)
s.sendall((json.dumps({"id": "1", "method": method, "params": params}) + "\n").encode())
print(s.makefile().readline().strip())
PY
}

out="$(rpc bug.status '{}')"
case "$out" in *'"available"'*) ;; *) echo "skip: this breadd has no bug.* IPC ($out)"; exit 0;; esac

count() { grep -c "$1" "$T/notify.log" 2>/dev/null || true; }
wait_for() { for _ in $(seq 1 60); do [ "$(count "$1")" -ge "$2" ] && return 0; sleep 0.1; done; return 1; }

# 1. a panic produces one notification naming the app and the shortcut
rpc bug.marker '{"app":"clip","marker":{"kind":"panic","message":"boom"}}' >/dev/null
wait_for "clip crashed" 1 || fail "no crash notification"
wait_for "kitty" 1 || fail "clicking Review should open a terminal"
grep -q "SUPER + SHIFT + B" "$T/notify.log" || fail "notification should name the shortcut"
grep -q "Nothing is sent" "$T/notify.log" || fail "notification should say nothing is sent"
grep -q "breadbug --app clip" "$T/notify.log" || fail "review action should open breadbug for the app"

# 2. a second crash within the cooldown does not notify again
rpc bug.marker '{"app":"clip","marker":{"kind":"crash","signal":"SIGSEGV"}}' >/dev/null
sleep 1
[ "$(count "clip crashed")" -eq 1 ] || fail "crash loop should be rate limited"

# 3. a different app is not affected by clip's cooldown
rpc bug.marker '{"app":"pad","marker":{"kind":"panic"}}' >/dev/null
wait_for "pad crashed" 1 || fail "other apps should still notify"

# 4. a freeze notifies once until it recovers
rpc bug.marker '{"app":"bar","marker":{"kind":"freeze"}}' >/dev/null
wait_for "bar looks frozen" 1 || fail "no freeze notification"
rpc bug.marker '{"app":"bar","marker":{"kind":"freeze"}}' >/dev/null
sleep 1
[ "$(count "bar looks frozen")" -eq 1 ] || fail "a repeated freeze should not notify"
rpc bug.marker '{"app":"bar","marker":{"kind":"recovered"}}' >/dev/null
rpc bug.marker '{"app":"bar","marker":{"kind":"freeze"}}' >/dev/null
wait_for "bar looks frozen" 2 || fail "a new freeze after recovery should notify"

# 5. breadd itself is still healthy and the module loaded cleanly
grep -qi "error\|panic" "$T/breadd.log" && fail "breadd logged errors: $(cat "$T/breadd.log")"

echo "breadbug-notify: all checks passed"
