#!/usr/bin/env bash
# Run by mkarchiso inside the airootfs chroot, after packages are installed
# and before the squashfs is built. (archiso prints a deprecation warning for
# this hook, but there is no non-deprecated replacement for "trust an extra
# pacman repo key in the image keyring", and BOS ships no pacman-init.service.)
#
# Purpose: trust the BOS release key (56203B86…) in the image's pacman
# keyring so the signed [breadway] repo (SigLevel = Required,
# https://dl.breadway.dev/arch) verifies both on the live medium and — via
# calamares' unpackfs, which copies this squashfs to the target — on the
# installed system. calamares/post-install.sh re-does this in the target
# chroot as a fallback (unpackfs can skip /etc/pacman.d/gnupg).
set -euo pipefail

BREADWAY_KEY_FPR="56203B86A110695AE7F310934AF3323D678EB5E2"
KEY_FILE="/etc/pacman.d/breadway-repo.asc"

pacman-key --init
pacman-key --populate archlinux

if [[ -f "$KEY_FILE" ]]; then
    pacman-key --add "$KEY_FILE"
    pacman-key --lsign-key "$BREADWAY_KEY_FPR"
    echo "customize_airootfs: trusted [breadway] repo key $BREADWAY_KEY_FPR"
else
    echo "customize_airootfs: WARNING $KEY_FILE missing; [breadway] will not verify" >&2
fi
