#!/usr/bin/env bash
# Read-only checks that a builder home (and optionally a staged skel) has
# everything build-local.sh needs before mkarchiso. Exit non-zero on failure.
#
#   LAPTOP_HOME=/build-home ./scripts/ci-verify-bake.sh
#   SKEL=/tmp/bos-iso-stage/airootfs/etc/skel ./scripts/ci-verify-bake.sh
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
LOCKFILE="${LOCKFILE:-$REPO/iso/bread-lockfile.toml}"
LAPTOP_HOME="${LAPTOP_HOME:-/build-home}"
SKEL="${SKEL:-}"

pass=0
fail=0
ok()   { printf '  PASS  %s\n' "$1"; pass=$((pass + 1)); }
bad()  { printf '  FAIL  %s\n' "$1" >&2; fail=$((fail + 1)); }

if [[ ! -f "$LOCKFILE" ]]; then
  echo "ERROR: lockfile missing: $LOCKFILE" >&2
  exit 1
fi

eval "$(python3 - "$LOCKFILE" <<'PY'
import sys, tomllib
path = sys.argv[1]
with open(path, "rb") as f:
    data = tomllib.load(f)
required = data.get("required_bins")
optional = data.get("optional_bins") or []
if required is None:
    required = data.get("bins") or data.get("binaries") or []
def emit(name, values):
    print(f"{name}=(")
    for v in values:
        print(f"  {v!r}")
    print(")")
emit("REQUIRED_BINS", required)
emit("OPTIONAL_BINS", optional)
PY
)"

echo "== lockfile $LOCKFILE =="
echo "   ${#REQUIRED_BINS[@]} required, ${#OPTIONAL_BINS[@]} optional"
echo "== builder home $LAPTOP_HOME =="

check_exec() {
  local path="$1" label="$2"
  if [[ -x "$path" && -f "$path" ]]; then
    ok "$label executable: $path"
  else
    bad "$label missing or not executable: $path"
  fi
}

check_dir() {
  local path="$1" label="$2"
  if [[ -d "$path" ]]; then
    ok "$label: $path"
  else
    bad "$label missing: $path"
  fi
}

check_file() {
  local path="$1" label="$2"
  if [[ -f "$path" ]]; then
    ok "$label: $path"
  else
    bad "$label missing: $path"
  fi
}

for b in "${REQUIRED_BINS[@]}"; do
  check_exec "$LAPTOP_HOME/.local/bin/$b" "required bin $b"
done
for b in "${OPTIONAL_BINS[@]}"; do
  if [[ -x "$LAPTOP_HOME/.local/bin/$b" ]]; then
    ok "optional bin $b present"
  else
    printf '  ----  optional bin %s not staged (ok until bread ships it)\n' "$b"
  fi
done

check_dir "$LAPTOP_HOME/.local/share/breadhelp/content" "breadhelp content"
check_file "$LAPTOP_HOME/.cache/bakery/index.json" "bakery index cache"
check_file "$LAPTOP_HOME/.local/state/bakery/installed.json" "bakery installed.json"

mapfile -t UNITS < <(python3 - "$LAPTOP_HOME/.local/state/bakery/installed.json" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    data = json.load(f)
pkgs = data.get("packages", data)
for pkg in pkgs.values():
    for s in pkg.get("services", []):
        print(s["unit"] if isinstance(s, dict) else s)
PY
)
if [[ ${#UNITS[@]} -eq 0 ]]; then
  bad "installed.json lists no service units"
else
  for unit in "${UNITS[@]}"; do
    [[ -n "$unit" ]] || continue
    check_file "$LAPTOP_HOME/.config/systemd/user/$unit" "unit $unit"
  done
fi

if [[ -n "$SKEL" ]]; then
  echo "== staged skel $SKEL =="
  for b in "${REQUIRED_BINS[@]}"; do
    check_exec "$SKEL/.local/bin/$b" "skel required bin $b"
  done
  check_dir "$SKEL/.local/share/breadhelp/content" "skel breadhelp content"
  check_file "$SKEL/.cache/bakery/index.json" "skel bakery index cache"
  for unit in "${UNITS[@]}"; do
    [[ -n "$unit" ]] || continue
    if [[ -f "$SKEL/.config/systemd/user/$unit" ]]; then
      ok "skel unit $unit"
    else
      bad "skel unit missing: $SKEL/.config/systemd/user/$unit"
    fi
  done
fi

echo
printf 'Result: %d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
