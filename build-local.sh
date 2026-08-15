#!/usr/bin/env bash
# Local BOS ISO build for hermes (native Arch — no container needed).
#
# Builds straight from the working tree in ./iso. Two speedups vs the hestia
# container build:
#   * runs natively (hermes is Arch; hestia needed a dockerised Arch)
#   * no 2 GB scp afterwards — the ISO lands here, where we test it
#
#   FAST_BUILD=1   zstd squashfs instead of xz -9e: compresses many times
#                  faster at the cost of a slightly larger image. Dev only.
#
# Usage:  sudo ./build-local.sh            # release-quality xz
#         sudo FAST_BUILD=1 ./build-local.sh   # fast dev iteration
set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"
# WORK defaults to /tmp, but on hermes /tmp is a 16 GB tmpfs — a full xz build
# (uncompressed rootfs + squashfs + work copies) can exhaust it mid-build. Allow
# pointing it at the NVMe instead: WORK=/home/.../bos-work sudo ./build-local.sh
WORK="${WORK:-/tmp/bos-work}"
OUT="${OUT:-$REPO/out}"

# Build against a throwaway copy of the profile so the working tree stays clean
# when FAST_BUILD / the registry rewrite mutate profile files.
STAGE=/tmp/bos-iso-stage
rm -rf "$STAGE" && cp -a "$REPO/iso" "$STAGE"

# Rewrite the [breadway] pacman repo URL to the fastest reachable address.
#   CI_BUILD=1  — container runs on hestia with --network=host; localhost:3002 is direct
#   default     — building on hermes; git.breadway.dev is flaky from there, use Tailscale
# Only ever rewrites the staged copy, never the committed pacman.conf.
if [ "${CI_BUILD:-0}" = "1" ]; then
  sed -i 's#https://git.breadway.dev/api/packages/Breadway/arch/os#http://localhost:3002/api/packages/Breadway/arch/os#' "$STAGE/pacman.conf"
else
  sed -i 's#https://git.breadway.dev/api/packages/Breadway/arch/os#http://100.66.238.26:3002/api/packages/Breadway/arch/os#' "$STAGE/pacman.conf"
fi

if [ "${FAST_BUILD:-0}" = "1" ]; then
  echo "=== FAST_BUILD: squashfs -> zstd level 6 ==="
  sed -i "s#^airootfs_image_tool_options=.*#airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '6' '-b' '1M')#" "$STAGE/profiledef.sh"
fi
grep airootfs_image_tool_options "$STAGE/profiledef.sh"

# --- Bake this machine's bakery-installed bread ecosystem into /etc/skel ------
# The bread desktop apps are bakery-managed (release binaries from
# dl.breadway.dev / GitHub), not pacman. bakery needs DNS at install time,
# which the live/installed image doesn't have — so instead of running bakery
# on the target, we copy the binaries + bakery manifest this builder already
# has into skel. Every user created from skel then gets those versions fully
# offline. Copied at build time so the binaries never bloat the git repo.
#
# CI should prefer the stable bakery index when populating the builder home.
# Local builds still snapshot the builder. required_bins fail the bake if
# missing; optional_bins are skipped with a warning (a hollow ISO is worse
# than a failed build). A flat `bins` list is treated as all-required.
LOCKFILE="$REPO/iso/bread-lockfile.toml"
if [[ ! -f "$LOCKFILE" ]]; then
  echo "ERROR: bakery lockfile missing: $LOCKFILE" >&2
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
    required = data.get("bins") or data.get("binaries")
if not isinstance(required, list) or not required:
    sys.exit(f"{path}: missing non-empty required_bins (or bins) list")
if not isinstance(optional, list):
    sys.exit(f"{path}: optional_bins must be a list")
blocked = {"breadcast", "breadarr"}
for label, names in (("required_bins", required), ("optional_bins", optional)):
    for b in names:
        if not isinstance(b, str) or not b or "/" in b or b in (".", ".."):
            sys.exit(f"{path}: invalid {label} name {b!r}")
        if b in blocked:
            sys.exit(f"{path}: {b} is not shipped on the ISO")
def emit(name, values):
    print(f"{name}=(")
    for v in values:
        print(f"  {v!r}")
    print(")")
emit("REQUIRED_BINS", required)
emit("OPTIONAL_BINS", optional)
PY
)"
if [[ ${#REQUIRED_BINS[@]} -eq 0 ]]; then
  echo "ERROR: $LOCKFILE produced an empty required bins list" >&2
  exit 1
fi

LAPTOP_HOME="${LAPTOP_HOME:-$(getent passwd "${SUDO_USER:-$USER}" | cut -d: -f6)}"
BAKERY_BIN="$LAPTOP_HOME/.local/bin"
BAKERY_STATE="$LAPTOP_HOME/.local/state/bakery"
BAKERY_CACHE="$LAPTOP_HOME/.cache/bakery"
BAKERY_SHARE="$LAPTOP_HOME/.local/share"
SKEL="$STAGE/airootfs/etc/skel"
echo "=== baking bakery bread ecosystem from $LAPTOP_HOME ==="
echo "lockfile: $LOCKFILE (${#REQUIRED_BINS[@]} required, ${#OPTIONAL_BINS[@]} optional)"

missing=()
for b in "${REQUIRED_BINS[@]}"; do
  if [[ ! -x "$BAKERY_BIN/$b" ]]; then
    missing+=("$BAKERY_BIN/$b")
  fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
  echo "ERROR: bakery lockfile requires binaries that are missing on the builder:" >&2
  printf '  %s\n' "${missing[@]}" >&2
  echo "Install them with bakery (or stage them under $BAKERY_BIN) before baking." >&2
  echo "A hollow ISO is worse than a failed build." >&2
  exit 1
fi

BREAD_BINS=("${REQUIRED_BINS[@]}")
for b in "${OPTIONAL_BINS[@]}"; do
  if [[ -x "$BAKERY_BIN/$b" ]]; then
    BREAD_BINS+=("$b")
  else
    echo "WARN: optional lockfile bin missing, skipping: $BAKERY_BIN/$b" >&2
  fi
done

install -d -m 0755 "$SKEL/.local/bin" "$SKEL/.local/state/bakery" "$SKEL/.cache/bakery"
for b in "${BREAD_BINS[@]}"; do
  install -m 0755 "$BAKERY_BIN/$b" "$SKEL/.local/bin/$b"
done

# Drop packages that are not in the lockfile (breadcast/breadarr must not
# appear installed when their binaries were deliberately left out).
python3 - "$BAKERY_STATE/installed.json" "$SKEL/.local/state/bakery/installed.json" "${BREAD_BINS[@]}" <<'PY'
import json, sys
src, dest, *bins = sys.argv[1:]
wanted = set(bins)
with open(src) as f:
    data = json.load(f)
pkgs = data.get("packages", data)
if not isinstance(pkgs, dict):
    sys.exit(f"{src}: expected packages object")
kept = {}
for name, pkg in pkgs.items():
    pbins = pkg.get("binaries") or []
    if name in wanted or any(b in wanted for b in pbins):
        kept[name] = pkg
out = {"packages": kept}
if "track" in data:
    out["track"] = data["track"]
with open(dest, "w") as f:
    json.dump(out, f, indent=2)
    f.write("\n")
print("installed.json packages:", ", ".join(sorted(kept)) or "(none)")
PY

# bakery fetches its package index from dl.breadway.dev (then a GitHub fallback),
# but falls back to a cached index when both are unreachable. With no network/DNS
# in the live/installed image, even `bakery list` errors unless that cache exists,
# so bake it in too — then bakery works fully offline (list/info from cache;
# install/update still need network, as expected).
if [[ ! -f "$BAKERY_CACHE/index.json" ]]; then
  echo "ERROR: bakery index cache missing: $BAKERY_CACHE/index.json" >&2
  exit 1
fi
install -m 0644 "$BAKERY_CACHE/index.json" "$SKEL/.cache/bakery/index.json"
echo "baked bins: $(ls "$SKEL/.local/bin")"

# --- Bake bakery data dirs the apps need offline ------------------------------
# bakery extracts data_archive (breadhelp's content.tar.gz) to
# ~/.local/share/<pkg>/ and writes desktop entries + licenses next to it.
# Copy those — never laptop-local state (clipboard history, WebKit cache,
# bread sync-repo, models).
echo "=== baking bakery share/data into skel ==="
BREADHELP_CONTENT="$BAKERY_SHARE/breadhelp/content"
if [[ ! -d "$BREADHELP_CONTENT" ]]; then
  echo "ERROR: breadhelp content missing: $BREADHELP_CONTENT" >&2
  echo "bakery installs this from content.tar.gz into ~/.local/share/breadhelp/content" >&2
  echo "A breadhelp binary without content is a hollow ISO." >&2
  exit 1
fi
install -d -m 0755 "$SKEL/.local/share"
cp -a "$BAKERY_SHARE/breadhelp" "$SKEL/.local/share/breadhelp"
echo "  baked $SKEL/.local/share/breadhelp/content"

python3 - "$BAKERY_CACHE/index.json" "$BAKERY_SHARE" "$SKEL/.local/share" "${BREAD_BINS[@]}" <<'PY'
import json, os, shutil, sys
index_path, src_share, dest_share, *bins = sys.argv[1:]
wanted = set(bins)
try:
    with open(index_path) as f:
        idx = json.load(f)
    packages = idx.get("packages", {})
except (OSError, json.JSONDecodeError):
    packages = {}

# Package names we ship: lockfile bin names plus index packages that
# publish at least one of those bins.
pkg_names = set(wanted)
for name, pkg in packages.items():
    pbins = []
    for b in pkg.get("binaries") or []:
        n = b["name"] if isinstance(b, dict) else b
        pbins.append(str(n).removesuffix("-x86_64"))
    if name in wanted or any(b in wanted for b in pbins):
        pkg_names.add(name)

os.makedirs(os.path.join(dest_share, "applications"), exist_ok=True)
os.makedirs(os.path.join(dest_share, "licenses"), exist_ok=True)

for name in sorted(pkg_names):
    pkg = packages.get(name) or {}
    if pkg.get("data_archive"):
        src = os.path.join(src_share, name)
        dest = os.path.join(dest_share, name)
        if name == "breadhelp":
            continue  # already copied above, required
        if os.path.isdir(src):
            if os.path.exists(dest):
                shutil.rmtree(dest)
            shutil.copytree(src, dest, symlinks=True)
            print(f"  baked data dir {dest}")
        else:
            sys.exit(f"ERROR: bakery data_archive for {name} missing at {src}")

    desktop_src = os.path.join(src_share, "applications", f"{name}.desktop")
    desktop_dest = os.path.join(dest_share, "applications", f"{name}.desktop")
    if os.path.isfile(desktop_src) and not os.path.isfile(desktop_dest):
        shutil.copy2(desktop_src, desktop_dest)
        print(f"  baked desktop {desktop_dest}")

    lic_src = os.path.join(src_share, "licenses", name)
    lic_dest = os.path.join(dest_share, "licenses", name)
    if os.path.isdir(lic_src) and not os.path.isdir(lic_dest):
        shutil.copytree(lic_src, lic_dest, symlinks=True)
        print(f"  baked license {lic_dest}")
PY

# --- Bake systemd user services for bakery-managed bread packages -----------
# Historically only breadd.service was hand-committed to skel; every other
# bakery package's service (breadbox-sync, breadmill, breadclipd, ...) was
# silently left out, so those daemons never start on a fresh install/live
# boot until the user re-runs `bakery install` (which needs network).
# Source of truth is the *filtered* installed.json we just wrote: only
# lockfile packages. Copy each unit with ExecStart rewritten from this
# laptop's literal home path to the portable `%h` specifier, and recreate
# whichever *.target.wants enable symlink bakery created locally. Units
# already committed by hand (breadd.service carries a
# RuntimeDirectoryPreserve=yes fix not yet upstreamed) are left alone.
echo "=== baking bakery service units into skel ==="
SYSTEMD_USER_DIR="$LAPTOP_HOME/.config/systemd/user"
SKEL_SYSTEMD="$SKEL/.config/systemd/user"
mapfile -t SERVICE_UNITS < <(python3 - "$SKEL/.local/state/bakery/installed.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
for pkg in d.get("packages", d).values():
    for s in pkg.get("services", []):
        print(s["unit"] if isinstance(s, dict) else s)
PY
)
for unit in "${SERVICE_UNITS[@]}"; do
  [[ -n "$unit" ]] || continue
  if [[ -f "$SKEL_SYSTEMD/$unit" ]]; then
    echo "  $unit already committed in skel, leaving as-is"
    continue
  fi
  src="$SYSTEMD_USER_DIR/$unit"
  if [[ ! -f "$src" ]]; then
    echo "ERROR: $unit listed in bakery installed.json but not found at $src" >&2
    echo "Refusing to bake a skel whose daemons will never start." >&2
    exit 1
  fi
  install -d -m 0755 "$SKEL_SYSTEMD"
  sed "s#ExecStart=$LAPTOP_HOME/.local/bin/#ExecStart=%h/.local/bin/#" "$src" > "$SKEL_SYSTEMD/$unit"
  for wants_dir in "$SYSTEMD_USER_DIR"/*.target.wants; do
    [[ -L "$wants_dir/$unit" ]] || continue
    target_name="$(basename "$wants_dir")"
    install -d -m 0755 "$SKEL_SYSTEMD/$target_name"
    ln -sf "../$unit" "$SKEL_SYSTEMD/$target_name/$unit"
  done
  echo "  baked $unit"
done

# mkarchiso resets every airootfs file to 0644, so executables must be declared
# in profiledef.sh's file_permissions array or they ship non-executable and the
# exec-once launches fail with "permission denied". Inject a 0755 entry for each
# baked binary right after the array opener (keeps the binary list in one place).
perm_file="$(mktemp)"
for b in "${BREAD_BINS[@]}"; do
  printf '    ["/etc/skel/.local/bin/%s"]="0:0:755"\n' "$b" >>"$perm_file"
done
sed -i "/^file_permissions=(/r $perm_file" "$STAGE/profiledef.sh"
rm -f "$perm_file"
echo "=== file_permissions after injection ==="; grep -A14 '^file_permissions=(' "$STAGE/profiledef.sh"

# Pin one timestamp for the whole build. Without this, mkarchiso derives the
# boot-config UUID (%ARCHISO_UUID%) when it starts and the iso9660 volume UUID
# when xorriso writes the image at the end — on a slow build these diverge by
# the build duration, so the initramfs searches /dev/disk/by-uuid/<wrong-uuid>,
# never finds the medium, and drops to a recovery shell. Fixing the epoch makes
# both derive from the same instant (and makes builds reproducible).
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(date +%s)}"
echo "SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH ($(date -u -d "@$SOURCE_DATE_EPOCH" +%Y-%m-%d-%H-%M-%S-00))"

echo "=== running mkarchiso ==="
rm -rf "$WORK" && mkdir -p "$OUT"
mkarchiso -v -w "$WORK" -o "$OUT" "$STAGE"

echo "=== RESULT ==="
if ls -lh "$OUT"/*.iso 2>/dev/null; then echo "ISO BUILT OK -> $OUT"; else echo "ISO BUILD FAILED"; exit 1; fi
