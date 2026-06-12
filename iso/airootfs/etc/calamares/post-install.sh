#!/bin/bash
set -euo pipefail

# --- Snapper root config ---
snapper -c root create-config /
sed -i 's/TIMELINE_CREATE="yes"/TIMELINE_CREATE="no"/' /etc/snapper/configs/root
sed -i 's/NUMBER_CLEANUP="no"/NUMBER_CLEANUP="yes"/' /etc/snapper/configs/root
sed -i 's/NUMBER_MIN_AGE="[^"]*"/NUMBER_MIN_AGE="1800"/' /etc/snapper/configs/root
sed -i 's/NUMBER_LIMIT="[^"]*"/NUMBER_LIMIT="10"/' /etc/snapper/configs/root
sed -i 's/NUMBER_LIMIT_IMPORTANT="[^"]*"/NUMBER_LIMIT_IMPORTANT="5"/' /etc/snapper/configs/root

# Allow main user to list/create/delete snapshots without sudo
MAIN_USER=$(getent passwd 1000 | cut -d: -f1)
sed -i "s/ALLOW_USERS=\"\"/ALLOW_USERS=\"$MAIN_USER\"/" /etc/snapper/configs/root

# --- System services ---
systemctl enable NetworkManager
systemctl enable bluetooth
systemctl enable snapper-cleanup.timer
systemctl enable grub-btrfs.path

# --- Bakery: install bread ecosystem ---
# Requires [breadway] repo in /etc/pacman.conf — see iso/pacman.conf
if command -v bakery &>/dev/null; then
    sudo -u "$MAIN_USER" bakery install bread breadbar breadbox breadcrumbs breadpad bos-settings
fi

# --- Deploy dotfiles into user home (skip any file that already exists) ---
SKEL_SRC="/etc/skel/.config"
DOTFILES_DEST="/home/$MAIN_USER/.config"

if [[ -d "$SKEL_SRC" ]]; then
    mkdir -p "$DOTFILES_DEST"
    cp -rn "$SKEL_SRC/." "$DOTFILES_DEST/"
    chown -R "$MAIN_USER:$MAIN_USER" "$DOTFILES_DEST"
fi

# --- XDG user dirs ---
sudo -u "$MAIN_USER" xdg-user-dirs-update

echo "BOS post-install complete. Reboot to start your system."
