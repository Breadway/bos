#!/bin/bash
# BOS-specific finalization, run inside the installed-system chroot (Calamares
# shellprocess), AFTER the native initcpio module has built the initramfs. The
# kernel (shellprocess@kernel) and initramfs (initcpio) are in place by now, so
# this script installs GRUB and does the rest of setup. Calamares' own
# `bootloader`/`grubcfg` modules are NOT used — in this archiso layout they leave
# the ESP empty and abort; the explicit grub-install below is verified to boot.
# Best-effort: do NOT use `set -e`; a single failure here must not abort the rest.
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
# Pacman keyring. The live medium's /etc/pacman.d/gnupg doesn't reliably carry
# over to the target (unpackfs may skip it / perms differ), leaving the installed
# system unable to verify package signatures — the first `pacman -Syu` then dies
# with "keyring is not writable / required key missing". Initialise it here so a
# fresh install can update out of the box. archlinux-keyring is already present;
# [breadway] is SigLevel=Never so it needs no key.
# ---------------------------------------------------------------------------
if command -v pacman-key &>/dev/null; then
    pacman-key --init || echo "WARN: pacman-key --init failed"
    pacman-key --populate archlinux || echo "WARN: pacman-key --populate failed"
fi

# ---------------------------------------------------------------------------
# Boot splash (Plymouth) — BOS logo + spinner instead of kernel text. Done
# BEFORE grub so grub.cfg picks up the new cmdline and the rebuilt initramfs.
# All best-effort: if anything here fails the system still boots (just without
# the splash) — the initramfs the initcpio module already built stays valid.
# ---------------------------------------------------------------------------
if command -v plymouth-set-default-theme &>/dev/null; then
    # Ensure the plymouth hook is in HOOKS (plymouthcfg/initcpiocfg usually add it;
    # this is the belt). Handle both the udev and systemd initramfs styles.
    if ! grep -q 'plymouth' /etc/mkinitcpio.conf 2>/dev/null; then
        if grep -qE '^HOOKS=.*\bsystemd\b' /etc/mkinitcpio.conf; then
            sed -i 's/^\(HOOKS=.*\bsystemd\b\)/\1 sd-plymouth/' /etc/mkinitcpio.conf \
                || echo "WARN: adding sd-plymouth hook failed"
        else
            sed -i 's/^\(HOOKS=.*\budev\b\)/\1 plymouth/' /etc/mkinitcpio.conf \
                || echo "WARN: adding plymouth hook failed"
        fi
    fi
    # Clean boot: splash activates plymouth; hiding systemd status removes the
    # "[ OK ] Started ..." text (what looked like kernel output) even if the
    # splash itself doesn't grab the display (e.g. in some VMs).
    if ! grep -q 'splash' /etc/default/grub 2>/dev/null; then
        sed -i 's/^\(GRUB_CMDLINE_LINUX_DEFAULT="\)/\1splash quiet vt.global_cursor_default=0 systemd.show_status=false rd.systemd.show_status=false rd.udev.log_level=3 /' \
            /etc/default/grub || echo "WARN: adding splash cmdline failed"
    fi
    # Set the BOS theme and rebuild the initramfs (-R) with the plymouth hook.
    plymouth-set-default-theme -R bos || echo "WARN: plymouth-set-default-theme failed"
fi

# ---------------------------------------------------------------------------
# Install GRUB (UEFI). /boot now has the kernel + initramfs, and the mount
# module has bind-mounted /proc /sys /dev /run + efivars into this chroot, so
# both grub-install passes and grub-mkconfig succeed.
#   1. NVRAM entry (EFI/BOS/grubx64.efi + a firmware boot entry)
#   2. --removable copy to EFI/BOOT/BOOTX64.EFI, so firmware that ignores/loses
#      the NVRAM entry (the "no boot device / PXE fallback" failure) still finds
#      a bootloader.
# ---------------------------------------------------------------------------
if command -v grub-install &>/dev/null; then
    grub-install --target=x86_64-efi --efi-directory=/boot/efi \
        --bootloader-id=BOS --recheck \
        || echo "WARN: grub-install (nvram) failed"
    grub-install --target=x86_64-efi --efi-directory=/boot/efi \
        --removable --recheck \
        || echo "WARN: grub-install (removable) failed"
fi
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
# System services. Enable each one INDEPENDENTLY: `systemctl enable a b c`
# resolves every unit first and enables NONE if any one can't be loaded, so a
# single wrong/absent unit name would silently leave NetworkManager (etc.)
# disabled. The loop isolates failures to the offending unit.
#   greetd       — graphical login (shipped disabled; live uses tty autologin)
#   grub-btrfsd  — regenerates GRUB snapshot entries (the unit is grub-btrfsd.service,
#                  NOT grub-btrfs.path, which no longer exists)
# ---------------------------------------------------------------------------
for unit in NetworkManager.service bluetooth.service systemd-timesyncd.service \
            tlp.service greetd.service snapper-cleanup.timer grub-btrfsd.service \
            fstrim.timer cups.socket avahi-daemon.service ufw.service \
            fwupd-refresh.timer reflector.timer; do
    systemctl enable "$unit" || echo "WARN: failed to enable $unit"
done
systemctl set-default graphical.target || echo "WARN: set-default graphical failed"

# ---------------------------------------------------------------------------
# mDNS resolution (nss-mdns): insert mdns_minimal into the hosts: line so the
# resolver answers *.local (network printers, other hosts) via avahi. Idempotent.
# ---------------------------------------------------------------------------
if [[ -f /etc/nsswitch.conf ]] && ! grep -q 'mdns_minimal' /etc/nsswitch.conf; then
    sed -i 's/^\(hosts:[[:space:]]*\)/\1mdns_minimal [NOTFOUND=return] /' \
        /etc/nsswitch.conf || echo "WARN: wiring nss-mdns failed"
fi

# ---------------------------------------------------------------------------
# Firewall: deny inbound by default, allow outbound, and permit inbound mDNS so
# avahi printer/service discovery keeps working. Best-effort — rule application
# happens at boot; here we only persist the policy + enable the unit.
# ---------------------------------------------------------------------------
if command -v ufw &>/dev/null; then
    ufw default deny incoming  || echo "WARN: ufw default deny incoming failed"
    ufw default allow outgoing || echo "WARN: ufw default allow outgoing failed"
    ufw allow 5353/udp         || echo "WARN: ufw allow mDNS failed"
    ufw --force enable         || echo "WARN: ufw enable failed"
fi

# The bread ecosystem (bakery + bread, breadbar, breadbox, breadcrumbs, breadpad)
# is bakery-managed, not pacman: the binaries and bakery manifest live in
# /etc/skel/.local (baked in at ISO build time) and are copied into the user's
# home below, so the install works fully offline with no DNS for bakery/GitHub.
# bos-settings is the only pacman bread package and was installed by unpackfs.

# ---------------------------------------------------------------------------
# Deploy dotfiles + the bakery bread ecosystem into the user's home (Calamares
# already seeds from /etc/skel, but copy explicitly too so a fresh install is
# self-contained even if the users module skips skel). Don't clobber existing.
# ---------------------------------------------------------------------------
if [[ -n "$MAIN_USER" && -d /etc/skel ]]; then
    for d in .config .local .cache; do
        [[ -d "/etc/skel/$d" ]] || continue
        mkdir -p "/home/$MAIN_USER/$d"
        cp -rn "/etc/skel/$d/." "/home/$MAIN_USER/$d/" || true
        chown -R "$MAIN_USER:$MAIN_USER" "/home/$MAIN_USER/$d" || true
    done
    sudo -u "$MAIN_USER" xdg-user-dirs-update || true
fi

echo "BOS post-install complete."
