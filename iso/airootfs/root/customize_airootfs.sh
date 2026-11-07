#!/usr/bin/env bash
# Run by mkarchiso inside the airootfs chroot, after packages are installed
# and before the squashfs is built. (archiso prints a deprecation warning for
# this hook, but there is no non-deprecated replacement for "trust an extra
# pacman repo key in the image keyring", and BOS ships no pacman-init.service.)
#
# Purpose: trust the BOS release key (56203B86…) in the image's pacman
# keyring so the signed [breadway] repo (SigLevel = Required,
# https://dl.breadway.dev/arch) verifies both on the live medium and, via
# calamares' unpackfs, which copies this squashfs to the target, on the
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

# Generate the locales listed in etc/locale.gen. glibc's package ships every
# entry commented out and mkarchiso never runs locale-gen, so a stock live
# medium has only C/POSIX: Language and Formats then offer nothing but
# "Computer default (C)" and apps format dates and numbers in the C locale.
locale-gen

# Keep an installer-only copy of linux-lts. It must be available to
# Calamares when the target is offline, but it is not installed on the live
# medium. The unpackfs module copies this file into the target, and
# bos-copy-kernel installs it before mkinitcpio and GRUB are configured.
LTS_CACHE_DIR="/usr/share/bos/installer-packages"
LTS_DOWNLOAD_DIR="$(mktemp -d)"
trap 'rm -rf "$LTS_DOWNLOAD_DIR"' EXIT
pacman -Swy --noconfirm --cachedir "$LTS_DOWNLOAD_DIR" linux-lts
mapfile -t LTS_PACKAGES < <(find "$LTS_DOWNLOAD_DIR" -maxdepth 1 -type f \
    -name 'linux-lts-*-x86_64.pkg.tar.zst' -print)
if [[ "${#LTS_PACKAGES[@]}" != 1 || ! -f "${LTS_PACKAGES[0]}.sig" ]]; then
    echo "customize_airootfs: expected one signed linux-lts package in $LTS_DOWNLOAD_DIR" >&2
    exit 1
fi
install -d -m 0755 "$LTS_CACHE_DIR"
install -m 0644 "${LTS_PACKAGES[0]}" "${LTS_PACKAGES[0]}.sig" "$LTS_CACHE_DIR/"
echo "customize_airootfs: staged installer-only linux-lts package"
