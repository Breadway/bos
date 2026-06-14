#!/bin/bash
# Runs inside the installed-system chroot (Calamares shellprocess, after the
# bootloader step). Best-effort: a single failure must not abort the rest, so
# we deliberately do NOT use `set -e`.
set -uo pipefail

MAIN_USER="$(getent passwd 1000 | cut -d: -f1 || true)"

# ---------------------------------------------------------------------------
# Strip live-only bits that unpackfs copied verbatim from the live medium.
# ---------------------------------------------------------------------------
rm -f /etc/systemd/system/getty@tty1.service.d/autologin.conf
rm -f /etc/systemd/system/bos-live-setup.service \
      /etc/systemd/system/multi-user.target.wants/bos-live-setup.service
rm -f /usr/local/bin/bos-live-setup /usr/local/bin/bos-launch-calamares
rm -f /etc/sudoers.d/99-bos-live
userdel -r liveuser 2>/dev/null || true

# Root used a passwordless entry on the live medium; lock it (sudo model).
passwd -l root || true

# ---------------------------------------------------------------------------
# Rebuild the initramfs for a real install — the live image ships the archiso
# hooks, which would send the installed system into the live-boot path.
# ---------------------------------------------------------------------------
rm -f /etc/mkinitcpio.conf.d/archiso.conf
cat >/etc/mkinitcpio.d/linux.preset <<'PRESET'
# mkinitcpio preset file for the 'linux' package
ALL_config="/etc/mkinitcpio.conf"
ALL_kver="/boot/vmlinuz-linux"
PRESETS=('default' 'fallback')
default_image="/boot/initramfs-linux.img"
fallback_image="/boot/initramfs-linux-fallback.img"
fallback_options="-S autodetect"
PRESET
mkinitcpio -P || echo "WARN: mkinitcpio regeneration failed"

# ---------------------------------------------------------------------------
# Install GRUB ourselves. Calamares' bootloader module runs before the kernel
# and initramfs exist (archiso keeps them out of the squashfs; shellprocess
# @kernel only lays vmlinuz down just beforehand), so its grub-install/config
# leaves the ESP empty. Redo it here, now that /boot is fully populated.
#
# Two passes: the standard NVRAM entry, plus a --removable copy to
# EFI/BOOT/BOOTX64.EFI so firmware that lost/never wrote an NVRAM entry (the
# "no boot device / screen just refreshes" failure) still finds a bootloader.
# ---------------------------------------------------------------------------
if command -v grub-install &>/dev/null; then
    grub-install --target=x86_64-efi --efi-directory=/boot/efi \
        --bootloader-id=BOS --recheck \
        || echo "WARN: grub-install (nvram) failed"
    grub-install --target=x86_64-efi --efi-directory=/boot/efi \
        --removable --recheck \
        || echo "WARN: grub-install (removable) failed"
fi

# Refresh GRUB so it references the kernel + rebuilt initramfs.
if command -v grub-mkconfig &>/dev/null; then
    grub-mkconfig -o /boot/grub/grub.cfg || echo "WARN: grub-mkconfig failed"
fi

# ---------------------------------------------------------------------------
# Snapper root config (root is btrfs).
# ---------------------------------------------------------------------------
if command -v snapper &>/dev/null; then
    snapper -c root create-config / || echo "WARN: snapper create-config failed"
    if [[ -f /etc/snapper/configs/root ]]; then
        sed -i 's/TIMELINE_CREATE="yes"/TIMELINE_CREATE="no"/' /etc/snapper/configs/root
        sed -i 's/NUMBER_CLEANUP="no"/NUMBER_CLEANUP="yes"/' /etc/snapper/configs/root
        sed -i 's/NUMBER_MIN_AGE="[^"]*"/NUMBER_MIN_AGE="1800"/' /etc/snapper/configs/root
        sed -i 's/NUMBER_LIMIT="[^"]*"/NUMBER_LIMIT="10"/' /etc/snapper/configs/root
        sed -i 's/NUMBER_LIMIT_IMPORTANT="[^"]*"/NUMBER_LIMIT_IMPORTANT="5"/' /etc/snapper/configs/root
        [[ -n "$MAIN_USER" ]] && \
            sed -i "s/ALLOW_USERS=\"\"/ALLOW_USERS=\"$MAIN_USER\"/" /etc/snapper/configs/root
    fi
fi

# ---------------------------------------------------------------------------
# System services.
# ---------------------------------------------------------------------------
systemctl enable NetworkManager bluetooth snapper-cleanup.timer grub-btrfs.path \
    || echo "WARN: enabling some services failed"

# ---------------------------------------------------------------------------
# Bread ecosystem via bakery (needs network — non-fatal so an offline install
# still completes; the user can run it after first boot).
# ---------------------------------------------------------------------------
if [[ -n "$MAIN_USER" ]] && command -v bakery &>/dev/null; then
    sudo -u "$MAIN_USER" bakery install bread breadbar breadbox breadcrumbs breadpad bos-settings \
        || echo "WARN: bakery install failed (no network during install?); run it after first boot"
fi

# ---------------------------------------------------------------------------
# Deploy dotfiles into the user's home (don't clobber existing files).
# ---------------------------------------------------------------------------
if [[ -n "$MAIN_USER" && -d /etc/skel/.config ]]; then
    mkdir -p "/home/$MAIN_USER/.config"
    cp -rn /etc/skel/.config/. "/home/$MAIN_USER/.config/" || true
    chown -R "$MAIN_USER:$MAIN_USER" "/home/$MAIN_USER/.config" || true
    sudo -u "$MAIN_USER" xdg-user-dirs-update || true
fi

echo "BOS post-install complete."
