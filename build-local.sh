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

# --- Bake this laptop's bakery-installed bread ecosystem into /etc/skel -------
# The bread apps are managed by bakery (which fetches release binaries from
# GitHub), not pacman. bakery needs DNS at install time, which the live/installed
# image doesn't have — so instead of running bakery on the target, we copy the
# exact binaries + bakery manifest this laptop already has into skel. Every user
# created from skel (the live user and the installed user) then gets the same
# versions `bakery list` reports here, fully offline. Copied at build time so the
# binaries never bloat the git repo and always track the current bakery state.
BREAD_BINS=(bakery bread breadd breadman breadbar breadbox breadbox-sync breadcrumbs breadpad breadpaper bread-theme)
LAPTOP_HOME="${LAPTOP_HOME:-$(getent passwd "${SUDO_USER:-$USER}" | cut -d: -f6)}"
BAKERY_BIN="$LAPTOP_HOME/.local/bin"
BAKERY_STATE="$LAPTOP_HOME/.local/state/bakery"
BAKERY_CACHE="$LAPTOP_HOME/.cache/bakery"
SKEL="$STAGE/airootfs/etc/skel"
echo "=== baking bakery bread ecosystem from $LAPTOP_HOME ==="
install -d -m 0755 "$SKEL/.local/bin" "$SKEL/.local/state/bakery" "$SKEL/.cache/bakery"
for b in "${BREAD_BINS[@]}"; do
  install -m 0755 "$BAKERY_BIN/$b" "$SKEL/.local/bin/$b"
done
install -m 0644 "$BAKERY_STATE/installed.json" "$SKEL/.local/state/bakery/installed.json"
# bakery fetches its package index from dl.breadway.dev (then a GitHub fallback),
# but falls back to a cached index when both are unreachable. With no network/DNS
# in the live/installed image, even `bakery list` errors unless that cache exists,
# so bake it in too — then bakery works fully offline (list/info from cache;
# install/update still need network, as expected).
install -m 0644 "$BAKERY_CACHE/index.json" "$SKEL/.cache/bakery/index.json"
echo "baked: $(ls "$SKEL/.local/bin")"

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
