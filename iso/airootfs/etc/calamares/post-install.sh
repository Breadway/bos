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
# Initramfs HOOKS: microcode + plymouth. Edit HOOKS first, rebuild once below.
#   microcode — embeds the (autodetect-pruned) CPU microcode into the initramfs
#     so it loads at early boot. The live ISO embeds ucode the same way, so the
#     ISO /boot carries no separate ucode image and bos-copy-kernel stages none
#     onto the target — the installed initramfs must therefore carry it itself.
#     Must sit AFTER `autodetect` so it's pruned to the running CPU's microcode.
#   plymouth — the BOS boot splash. Only the udev `plymouth` hook exists (there
#     is NO `sd-plymouth`), so always insert it after `udev`.
# All best-effort: a failure here still leaves a bootable initramfs.
# ---------------------------------------------------------------------------
if [[ -f /etc/mkinitcpio.conf ]]; then
    if ! grep -qE '^HOOKS=.*\bmicrocode\b' /etc/mkinitcpio.conf; then
        sed -i 's/^\(HOOKS=.*\bautodetect\b\)/\1 microcode/' /etc/mkinitcpio.conf \
            || echo "WARN: adding microcode hook failed"
    fi
    if command -v plymouth-set-default-theme &>/dev/null \
       && ! grep -qE '^HOOKS=.*\bplymouth\b' /etc/mkinitcpio.conf; then
        sed -i 's/^\(HOOKS=.*\budev\b\)/\1 plymouth/' /etc/mkinitcpio.conf \
            || echo "WARN: adding plymouth hook failed"
    fi
fi

# ---------------------------------------------------------------------------
# Boot splash (Plymouth) — BOS logo + spinner instead of kernel text. Set the
# theme + cmdline BEFORE grub so grub.cfg picks up the new cmdline.
# ---------------------------------------------------------------------------
if command -v plymouth-set-default-theme &>/dev/null; then
    # Clean boot: splash activates plymouth; hiding systemd status removes the
    # "[ OK ] Started ..." text (what looked like kernel output) even if the
    # splash itself doesn't grab the display (e.g. in some VMs).
    if ! grep -q 'splash' /etc/default/grub 2>/dev/null; then
        sed -i 's/^\(GRUB_CMDLINE_LINUX_DEFAULT="\)/\1splash quiet vt.global_cursor_default=0 systemd.show_status=false rd.systemd.show_status=false rd.udev.log_level=3 /' \
            /etc/default/grub || echo "WARN: adding splash cmdline failed"
    fi
    plymouth-set-default-theme bos || echo "WARN: plymouth-set-default-theme failed"
fi

# Rebuild every preset (default + fallback that bos-copy-kernel wrote) so the
# microcode + plymouth HOOKS above are actually baked into the initramfs.
mkinitcpio -P || echo "WARN: mkinitcpio -P failed"

# ---------------------------------------------------------------------------
# Install GRUB. /boot now has the kernel + initramfs, and the mount module has
# bind-mounted /proc /sys /dev /run (+ efivars on UEFI) into this chroot, so
# both grub-install passes and grub-mkconfig succeed.
#
# BOS ships a syslinux BIOS boot mode on the ISO (profiledef.sh bootmodes
# includes bios.syslinux), but this only ever ran the UEFI grub-install path —
# a BIOS install would complete successfully and then have no bootloader at
# all. Branch on /sys/firmware/efi (present only when booted UEFI) and install
# the matching GRUB target; on BIOS, grub-install needs the whole disk, not a
# partition, so it's derived from the mounted root via lsblk.
#   UEFI: 1. NVRAM entry (EFI/BOS/grubx64.efi + a firmware boot entry)
#         2. --removable copy to EFI/BOOT/BOOTX64.EFI, so firmware that
#            ignores/loses the NVRAM entry still finds a bootloader.
#   BIOS: MBR install onto the disk hosting /.
# ---------------------------------------------------------------------------
if command -v grub-install &>/dev/null; then
    if [[ -d /sys/firmware/efi ]]; then
        grub-install --target=x86_64-efi --efi-directory=/boot/efi \
            --bootloader-id=BOS --recheck \
            || echo "WARN: grub-install (nvram) failed"
        grub-install --target=x86_64-efi --efi-directory=/boot/efi \
            --removable --recheck \
            || echo "WARN: grub-install (removable) failed"
    else
        ROOT_DEV="$(findmnt -no SOURCE / | sed 's/\[.*\]//')"
        ROOT_DISK="$(lsblk -no pkname "$ROOT_DEV" 2>/dev/null)"
        if [[ -n "$ROOT_DISK" ]]; then
            grub-install --target=i386-pc --recheck "/dev/$ROOT_DISK" \
                || echo "WARN: grub-install (BIOS) failed"
        else
            echo "WARN: could not determine the disk hosting / (root device: ${ROOT_DEV:-unknown}) — BIOS grub-install skipped"
        fi
    fi
fi
if command -v grub-mkconfig &>/dev/null; then
    grub-mkconfig -o /boot/grub/grub.cfg || echo "WARN: grub-mkconfig failed"
fi

# ---------------------------------------------------------------------------
# Create @snapshots, @log, @cache as top-level btrfs subvolumes (peers of @,
# not nested under it — so snapshots of @ don't recursively include
# themselves, and log/cache churn doesn't bloat @'s snapshot history).
#
# iso/partition.conf's `btrfsSubvolumes:` key does NOT do this — verified on
# real hardware that it's not a recognized key in this Calamares version's
# partition module schema at all (same class of bug as the `userShell` fix
# below: Calamares silently ignores unknown top-level keys). Calamares' own
# btrfs support only natively creates @ and @home; everything else has to be
# done by hand, here, after unpackfs has populated / but before anything
# reads/writes /var/log or /var/cache going forward. Existing content in
# those two dirs (real files already unpacked from the squashfs) is migrated
# into the new subvolumes before mounting over them, so nothing is lost —
# just shadowed by the mount, same as any other mountpoint.
# ---------------------------------------------------------------------------
if command -v btrfs &>/dev/null; then
    ROOT_DEV="$(findmnt -no SOURCE / | sed 's/\[.*\]//')"
    ROOT_UUID="$(blkid -s UUID -o value "$ROOT_DEV" 2>/dev/null)"
    BTRFS_TOP=/.btrfs-top-tmp

    if [[ -z "$ROOT_UUID" ]]; then
        echo "WARN: could not determine root filesystem UUID — skipping @snapshots/@log/@cache creation"
    else
        mkdir -p "$BTRFS_TOP"
        if mount -o subvolid=5 "$ROOT_DEV" "$BTRFS_TOP"; then
            for sv in @snapshots @log @cache; do
                if ! btrfs subvolume show "$BTRFS_TOP/$sv" &>/dev/null; then
                    btrfs subvolume create "$BTRFS_TOP/$sv" || echo "WARN: creating $sv failed"
                fi
            done
            rsync -aAX /var/log/ "$BTRFS_TOP/@log/" || echo "WARN: migrating /var/log into @log failed"
            rsync -aAX /var/cache/ "$BTRFS_TOP/@cache/" || echo "WARN: migrating /var/cache into @cache failed"
            umount "$BTRFS_TOP"
            rmdir "$BTRFS_TOP"

            OPTS="noatime,compress=zstd,space_cache=v2"
            grep -q "@snapshots" /etc/fstab || echo "UUID=$ROOT_UUID /.snapshots btrfs subvol=/@snapshots,$OPTS 0 0" >> /etc/fstab
            grep -q "@log"       /etc/fstab || echo "UUID=$ROOT_UUID /var/log    btrfs subvol=/@log,$OPTS 0 0" >> /etc/fstab
            grep -q "@cache"     /etc/fstab || echo "UUID=$ROOT_UUID /var/cache  btrfs subvol=/@cache,$OPTS 0 0" >> /etc/fstab

            mkdir -p /.snapshots
            mount /.snapshots || echo "WARN: mounting /.snapshots failed"
            mount /var/log     || echo "WARN: mounting /var/log failed"
            mount /var/cache   || echo "WARN: mounting /var/cache failed"
        else
            echo "WARN: could not mount btrfs top-level — skipping @snapshots/@log/@cache creation"
        fi
    fi
fi

# ---------------------------------------------------------------------------
# Snapper root config (root is btrfs).
#
# @snapshots is now mounted at /.snapshots (created above) — a dedicated
# top-level subvolume, a peer of @ rather than nested under it, so snapshots
# aren't themselves recursively snapshotted. `snapper create-config` insists
# on creating that subvolume itself and refuses whenever /.snapshots already
# exists, mounted or not — silently, so without this dance every downstream
# sed below is a no-op and BOS ships with its advertised auto-snapshot/
# rollback feature entirely non-functional.
# Unmount the real subvolume, let snapper create + own its nested one, then
# discard that and remount the real @snapshots in its place (fstab entry was
# added above, right after the subvolume was created).
#
# Confirmed on real hardware: a plain `umount /.snapshots` here can
# transiently fail inside the Calamares chroot (the same mount unmounts
# cleanly moments later once booted normally — a chroot-specific busy-mount
# race, not a logic error), which cascades into snapper create-config
# refusing because .snapshots "already exists". Retry a few times, then
# fall back to a lazy unmount (detaches the mountpoint immediately even if
# something still transiently references it) rather than give up.
# ---------------------------------------------------------------------------
if command -v snapper &>/dev/null; then
    unmounted=0
    for _ in 1 2 3 4 5; do
        if umount /.snapshots 2>/dev/null; then
            unmounted=1
            break
        fi
        sleep 1
    done
    if [[ "$unmounted" != "1" ]]; then
        echo "WARN: umount /.snapshots failed after retries, forcing lazy unmount"
        umount -l /.snapshots || echo "WARN: lazy umount /.snapshots also failed"
    fi
    rmdir /.snapshots || echo "WARN: rmdir /.snapshots failed"
    snapper -c root create-config / || echo "WARN: snapper create-config failed"
    if [[ -d /.snapshots ]]; then
        btrfs subvolume delete /.snapshots || echo "WARN: deleting snapper's own .snapshots subvolume failed"
    fi
    mkdir -p /.snapshots
    mount /.snapshots || echo "WARN: remounting the real @snapshots subvolume failed"
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
