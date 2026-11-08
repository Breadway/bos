#!/usr/bin/env bash
# Static + dry-run regression test for BOS-121 signal-safe cleanup in
# build-local.sh. No ISO build, no root, no real mounts: mount tools are
# stubbed and everything runs in a scratch dir.
#
#   scripts/test-build-cleanup.sh
#
# Checks:
#   1. build-local.sh installs EXIT, INT and TERM traps before anything that
#      can mount (before STAGE setup and before mkarchiso runs).
#   2. The EXIT handler unmounts before it removes (unmount_tree precedes
#      safe_rmtree inside cleanup_build), and no bare rm -rf of WORK remains.
#   3. Dry run with stub findmnt/umount/rm: while a mount is reported under a
#      scratch root, safe_rmtree refuses and nothing is deleted; after the
#      stub unmount, unmount_tree + safe_rmtree removes the tree using
#      rm --one-file-system; when unmounting fails, cleanup_build keeps the
#      tree instead of deleting across the mount.
#   4. The SIGKILL residual (untrappable, needs manual unmount) is documented.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/.." && pwd)"
SCRIPT="$REPO/build-local.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }

[ -f "$SCRIPT" ] || fail "build-local.sh not found at $SCRIPT"

# --- 1. traps cover EXIT, INT and TERM, installed up top --------------------
if ! grep -qE '^trap +cleanup_build +EXIT' "$SCRIPT"; then
  fail "no EXIT trap installing cleanup_build"
fi
if ! grep -qE "^trap .*INT" "$SCRIPT"; then
  fail "no INT trap"
fi
if ! grep -qE "^trap .*TERM" "$SCRIPT"; then
  fail "no TERM trap"
fi
trap_line="$(grep -nE '^trap +cleanup_build +EXIT' "$SCRIPT" | head -1 | cut -d: -f1)"
stage_line="$(grep -n 'STAGE=/tmp/bos-iso-stage' "$SCRIPT" | head -1 | cut -d: -f1)"
iso_line="$(grep -n '^mkarchiso ' "$SCRIPT" | head -1 | cut -d: -f1)"
if [ "$trap_line" -gt "$stage_line" ]; then
  fail "EXIT trap (line $trap_line) must precede STAGE setup (line $stage_line)"
fi
if [ "$trap_line" -gt "$iso_line" ]; then
  fail "EXIT trap (line $trap_line) must precede mkarchiso (line $iso_line)"
fi

# --- 2. unmount-before-remove ordering --------------------------------------
body="$(awk '/^cleanup_build\(\) \{/,/^\}/' "$SCRIPT")"
[ -n "$body" ] || fail "cleanup_build body not found"
umount_pos="$(printf '%s\n' "$body" | grep -n 'unmount_tree' | head -1 | cut -d: -f1)"
remove_pos="$(printf '%s\n' "$body" | grep -n 'safe_rmtree' | head -1 | cut -d: -f1)"
if [ -z "$umount_pos" ] || [ -z "$remove_pos" ]; then
  fail "cleanup_build must call unmount_tree and safe_rmtree"
fi
if [ "$umount_pos" -gt "$remove_pos" ]; then
  fail "cleanup_build must unmount before removing"
fi
if grep -qE '^[[:space:]]*rm -rf "\$WORK"' "$SCRIPT"; then
  fail 'bare rm -rf "$WORK" must be replaced by the mount-checked removal'
fi

# --- 4. SIGKILL residual documented ------------------------------------------
if ! grep -q 'SIGKILL' "$SCRIPT"; then
  fail "SIGKILL residual must be documented in build-local.sh"
fi

# --- 3. dry run with stubbed mount tools ------------------------------------
T="$(mktemp -d)"
test_cleanup() { rm -rf "$T"; }
trap test_cleanup EXIT

mkdir -p "$T/stubs"
touch "$T/state" "$T/rm.log" "$T/umount.log"

cat > "$T/stubs/findmnt" <<'STUB'
#!/usr/bin/env bash
# Stub: reports the recorded mount when the queried root contains it.
root="${@: -1}"
mount="$(cat "$STATE_FILE")"
[ -n "$mount" ] || exit 0
case "$mount" in
  "$root"|"$root"/*) printf '%s\n' "$mount" ;;
esac
exit 0
STUB

cat > "$T/stubs/umount" <<'STUB'
#!/usr/bin/env bash
# Stub: logs the call; clears the recorded mount unless UMOUNT_SHOULD_FAIL=1.
printf '%s\n' "$*" >> "$UMOUNT_LOG"
if [ "${UMOUNT_SHOULD_FAIL:-0}" = "1" ]; then
  exit 1
fi
for arg in "$@"; do
  case "$arg" in -*) continue ;; esac
  mount="$(cat "$STATE_FILE")"
  case "$mount" in
    "$arg"|"$arg"/*) : > "$STATE_FILE" ;;
  esac
done
exit 0
STUB

cat > "$T/stubs/rm" <<'STUB'
#!/usr/bin/env bash
# Stub: logs the call, then performs the real removal with /bin/rm.
printf '%s\n' "$*" >> "$RM_LOG"
exec /bin/rm "$@"
STUB
chmod +x "$T/stubs/findmnt" "$T/stubs/umount" "$T/stubs/rm"

awk '/BOS-121-CLEANUP-BEGIN/,/BOS-121-CLEANUP-END/' "$SCRIPT" > "$T/cleanup.funcs"
if ! grep -q 'unmount_tree()' "$T/cleanup.funcs"; then
  fail "could not extract unmount_tree from build-local.sh"
fi
if ! grep -q 'safe_rmtree()' "$T/cleanup.funcs"; then
  fail "could not extract safe_rmtree from build-local.sh"
fi
if ! grep -q 'cleanup_build()' "$T/cleanup.funcs"; then
  fail "could not extract cleanup_build from build-local.sh"
fi

export STATE_FILE="$T/state" UMOUNT_LOG="$T/umount.log" RM_LOG="$T/rm.log"
export PATH="$T/stubs:$PATH"
# Functions under test, exactly as shipped in build-local.sh.
# shellcheck disable=SC1090
source "$T/cleanup.funcs"

# 3a. Mount present: safe_rmtree refuses and nothing is deleted.
mkdir -p "$T/root/proc"
echo sentinel > "$T/root/proc/sentinel"
echo "$T/root/proc" > "$STATE_FILE"
: > "$RM_LOG"
if safe_rmtree "$T/root"; then
  fail "safe_rmtree removed a tree containing a mount"
fi
if [ -s "$RM_LOG" ]; then
  fail "rm must not run while a mount is present (rm log: $(cat "$RM_LOG"))"
fi
if [ ! -f "$T/root/proc/sentinel" ]; then
  fail "sentinel file was deleted despite the active mount"
fi

# 3b. After unmount: tree is removed, via rm --one-file-system.
unmount_tree "$T/root"
if [ -s "$STATE_FILE" ]; then
  fail "stub unmount did not clear the mount"
fi
if ! safe_rmtree "$T/root"; then
  fail "safe_rmtree should succeed once unmounted"
fi
if ! grep -q -- "--one-file-system" "$RM_LOG"; then
  fail "removal must use rm --one-file-system (rm log: $(cat "$RM_LOG"))"
fi
if [ -e "$T/root" ]; then
  fail "tree should be gone after unmount+remove"
fi

# 3c. Unmount fails: cleanup_build keeps the tree, never deletes across mounts.
mkdir -p "$T/root2/proc"
echo sentinel > "$T/root2/proc/sentinel"
echo "$T/root2/proc" > "$STATE_FILE"
export UMOUNT_SHOULD_FAIL=1
: > "$UMOUNT_LOG"
: > "$RM_LOG"
WORK="$T/root2" CLEANUP_DONE=0 cleanup_build
export UMOUNT_SHOULD_FAIL=0
if [ ! -s "$UMOUNT_LOG" ]; then
  fail "cleanup_build should have attempted to unmount first"
fi
if [ -s "$RM_LOG" ]; then
  fail "rm must not run when unmounting fails (rm log: $(cat "$RM_LOG"))"
fi
if [ ! -f "$T/root2/proc/sentinel" ]; then
  fail "cleanup_build deleted a tree whose unmount failed"
fi

# 3d. Unmount succeeds: cleanup_build removes WORK after unmounting it.
mkdir -p "$T/root3/proc"
echo sentinel > "$T/root3/proc/sentinel"
echo "$T/root3/proc" > "$STATE_FILE"
: > "$RM_LOG"
WORK="$T/root3" BAKERY_TMP_HOME="" BUILD_GPG_DIR="" CLEANUP_DONE=0 cleanup_build
if [ -e "$T/root3" ]; then
  fail "cleanup_build should remove WORK after unmounting it"
fi

echo "PASS: build cleanup is signal-safe (trap, ordering, fail-closed removal)"
