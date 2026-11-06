#!/usr/bin/env bash
# Read-only checks that a builder home (and optionally a staged image) has
# everything build-local.sh needs before mkarchiso. Exit non-zero on failure.
#
# Builder home stays user-layout (~/.local). The image is system-prefix
# /usr/local; pass SKEL and/or AIROOTFS to check those destinations.
#
#   LAPTOP_HOME=/build-home ./scripts/ci-verify-bake.sh
#   SKEL=/tmp/bos-iso-stage/airootfs/etc/skel ./scripts/ci-verify-bake.sh
#   AIROOTFS=/tmp/bos-iso-stage/airootfs ./scripts/ci-verify-bake.sh
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
LOCKFILE="${LOCKFILE:-$REPO/iso/bread-lockfile.toml}"
LAPTOP_HOME="${LAPTOP_HOME:-/build-home}"
SKEL="${SKEL:-}"
AIROOTFS="${AIROOTFS:-}"

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
if grep -qE '^nvidia(-utils|-dkms|-open)?$' "$REPO/iso/packages.x86_64"; then
  bad "iso/packages.x86_64 lists an nvidia driver package"
else
  ok "iso/packages.x86_64 has no nvidia driver package"
fi
echo "== host tools =="
if command -v grub-install >/dev/null 2>&1; then
  ok "grub-install (uefi.grub bootmode)"
else
  bad "grub-install missing: mkarchiso uefi.grub will abort (install grub on the builder)"
fi
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

if [[ -n "$SKEL" && -z "$AIROOTFS" ]]; then
  if [[ -d "$SKEL/usr/local/bin" ]]; then
    AIROOTFS="$SKEL"
    SKEL="$AIROOTFS/etc/skel"
  elif [[ -d "$SKEL/../../usr/local" ]]; then
    AIROOTFS="$(cd "$SKEL/../.." && pwd)"
  fi
elif [[ -n "$AIROOTFS" && -z "$SKEL" ]]; then
  SKEL="$AIROOTFS/etc/skel"
fi

if [[ -n "$AIROOTFS" || -n "$SKEL" ]]; then
  if [[ -n "$AIROOTFS" ]]; then
    echo "== staged image $AIROOTFS =="
    check_file "$AIROOTFS/etc/bakery/config.toml" "bakery prefix config"
    if [[ -f "$AIROOTFS/etc/bakery/config.toml" ]] && grep -q 'prefix[[:space:]]*=[[:space:]]*"/usr/local"' "$AIROOTFS/etc/bakery/config.toml"; then
      ok "bakery prefix = /usr/local"
    else
      bad "bakery prefix is not /usr/local in $AIROOTFS/etc/bakery/config.toml"
    fi
    for b in "${REQUIRED_BINS[@]}"; do
      check_exec "$AIROOTFS/usr/local/bin/$b" "image required bin $b"
    done
    check_exec "$AIROOTFS/usr/local/bin/bos-nvidia-setup" "image bos-nvidia-setup"
    check_dir "$AIROOTFS/usr/local/share/breadhelp/content" "image breadhelp content"
    # Keep the directory check above, but also resolve content the way the
    # packaged runtime does: breadhelp 0.2.7 reads
    # /usr/share/breadhelp/content and merges the user root. A baked image
    # once passed on directory existence alone while the Learn tab was empty
    # (BOS-112), so require a non-empty guide list and a readable body.
    if [[ -L "$AIROOTFS/usr/share/breadhelp/content" ]]; then
      ok "image breadhelp content compatibility path (usr/share symlink)"
    else
      bad "image breadhelp content path missing: usr/share/breadhelp/content"
    fi
    if python3 "$REPO/scripts/check-breadhelp-content.py" "$AIROOTFS"; then
      ok "breadhelp resolves guides from the baked layout"
    else
      bad "breadhelp resolves no guides from the baked layout (hollow Learn tab)"
    fi
    # Live medium: CUPS is socket-activated, so the socket must be enabled
    # in the image or Printers reports "Scheduler is not running" (BOS-93).
    # -L, not -e: the symlink target is absolute and only resolves on the
    # booted system, not inside the staging tree.
    if [[ -L "$AIROOTFS/etc/systemd/system/sockets.target.wants/cups.socket" ]]; then
      ok "cups.socket enabled in live image (sockets.target.wants)"
    else
      bad "cups.socket not enabled in live image (missing sockets.target.wants/cups.socket)"
    fi
    # Bluetooth must be enabled in the live session, not only after install
    # (BOS-100). The canonical enable path is bluetooth.target.wants, which
    # udev's Bluetooth device tag pulls in only when a controller exists.
    if [[ -L "$AIROOTFS/etc/systemd/system/bluetooth.target.wants/bluetooth.service" ]]; then
      ok "bluetooth.service enabled in live image (bluetooth.target.wants)"
    else
      bad "bluetooth.service not enabled in live image (missing bluetooth.target.wants/bluetooth.service)"
    fi
  fi
  if [[ -n "$SKEL" ]]; then
    echo "== staged skel $SKEL =="
    check_file "$SKEL/.cache/bakery/index.json" "skel bakery index cache"
    check_file "$SKEL/.local/state/bakery/installed.json" "skel bakery installed.json"
    for b in "${REQUIRED_BINS[@]}"; do
      if [[ -e "$SKEL/.local/bin/$b" ]]; then
        bad "skel still has bakery bin $b (belongs in /usr/local/bin)"
      fi
    done
    check_file "$SKEL/.config/hypr/hyprland.lua" "skel hyprland.lua"
    if grep -q 'nvidia.lua' "$SKEL/.config/hypr/hyprland.lua"; then
      ok "skel hyprland.lua includes nvidia.lua only if present"
    else
      bad "skel hyprland.lua does not mention nvidia.lua"
    fi
  fi
  image_units_json=""
  if [[ -n "$SKEL" && -f "$SKEL/.local/state/bakery/installed.json" ]]; then
    image_units_json="$SKEL/.local/state/bakery/installed.json"
  fi
  if [[ -n "$image_units_json" ]]; then
    mapfile -t IMAGE_UNITS < <(python3 - "$image_units_json" <<'PY'
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
  else
    IMAGE_UNITS=("${UNITS[@]}")
  fi
  if [[ -n "$AIROOTFS" ]]; then
    for unit in "${IMAGE_UNITS[@]}"; do
      [[ -n "$unit" ]] || continue
      check_file "$AIROOTFS/usr/lib/systemd/user/$unit" "image unit $unit"
      if [[ -f "$AIROOTFS/usr/lib/systemd/user/$unit" ]]; then
        if grep -q '^ExecStart=/usr/local/bin/' "$AIROOTFS/usr/lib/systemd/user/$unit"; then
          ok "image unit $unit ExecStart uses /usr/local/bin"
        elif grep -q '^ExecStart=' "$AIROOTFS/usr/lib/systemd/user/$unit"; then
          bad "image unit $unit ExecStart is not /usr/local/bin: $(grep '^ExecStart=' "$AIROOTFS/usr/lib/systemd/user/$unit")"
        fi
      fi
    done
    check_file "$AIROOTFS/usr/lib/systemd/user-preset/90-bos-bakery.preset" \
      "bakery user preset"
    if [[ -L "$AIROOTFS/etc/systemd/user/default.target.wants/breadd.service" ]] \
       || [[ -f "$AIROOTFS/etc/systemd/user/default.target.wants/breadd.service" ]]; then
      ok "breadd.service globally enabled (etc wants)"
    else
      bad "breadd.service missing from /etc/systemd/user/default.target.wants"
    fi
    # After bake the image has /usr/local/bin/breadd and every preset unit.
    # The committed airootfs only has the preset + breadd wants.
    if [[ -f "$AIROOTFS/usr/lib/systemd/user-preset/90-bos-bakery.preset" ]] \
       && [[ -x "$AIROOTFS/usr/local/bin/breadd" ]]; then
      while read -r verb unit; do
        [[ "$verb" == enable && -n "$unit" ]] || continue
        check_file "$AIROOTFS/usr/lib/systemd/user/$unit" "preset unit $unit"
        if [[ -L "$AIROOTFS/etc/systemd/user/default.target.wants/$unit" ]] \
           || [[ -L "$AIROOTFS/etc/systemd/user/graphical-session.target.wants/$unit" ]]; then
          ok "$unit globally enabled (etc wants)"
        else
          bad "$unit missing from /etc/systemd/user/*.target.wants"
        fi
      done < "$AIROOTFS/usr/lib/systemd/user-preset/90-bos-bakery.preset"
    fi
    if [[ -x "$AIROOTFS/usr/local/bin/bos-enable-bakery-user-units" ]]; then
      ok "bos-enable-bakery-user-units executable"
    else
      bad "bos-enable-bakery-user-units missing or not executable"
    fi
  fi
fi

echo
printf 'Result: %d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
