#!/usr/bin/env bash
# BOS post-install smoke test.
#
# Run this INSIDE a freshly installed BOS (as the main user) to assert the
# install's core invariants. It is read-only and safe to run any time.
#
#   ./smoke-test.sh
#
# Exit status is non-zero if any check fails, so it can gate CI / manual QA.
set -uo pipefail

pass=0 fail=0
ok()   { printf '  \033[32mPASS\033[0m  %s\n' "$1"; pass=$((pass+1)); }
bad()  { printf '  \033[31mFAIL\033[0m  %s\n' "$1"; fail=$((fail+1)); }
note() { printf '  \033[33m----\033[0m  %s\n' "$1"; }

check() { if eval "$2" >/dev/null 2>&1; then ok "$1"; else bad "$1"; fi; }

echo "== btrfs subvolume layout =="
if command -v btrfs >/dev/null; then
    # `btrfs subvolume list` needs root; try unprivileged, fall back to non-
    # interactive sudo (no hang if creds aren't cached).
    paths="$(btrfs subvolume list / 2>/dev/null || sudo -n btrfs subvolume list / 2>/dev/null)"
    paths="$(awk '{print $NF}' <<<"$paths")"
    if [ -z "$paths" ]; then
        note "couldn't list subvolumes (need root) — skipping"
    else
        for sv in @ @home @snapshots @log @cache; do
            if grep -qx "$sv" <<<"$paths"; then ok "subvolume $sv present"; else bad "subvolume $sv missing"; fi
        done
    fi
else
    note "btrfs not installed (not a btrfs root?) — skipping subvolume checks"
fi

echo "== snapshot tooling =="
check "snapper root config exists" "[ -f /etc/snapper/configs/root ]"
check "snap-pac hook present"      "pacman -Qq snap-pac"
check "grub-btrfs present"         "pacman -Qq grub-btrfs"

echo "== enabled system services =="
for unit in NetworkManager.service greetd.service bluetooth.service tlp.service \
            cups.socket avahi-daemon.service ufw.service systemd-timesyncd.service; do
    check "$unit enabled" "systemctl is-enabled $unit"
done
check "graphical.target is default" "[ \"\$(systemctl get-default)\" = graphical.target ]"

echo "== bread ecosystem on PATH =="
for bin in bakery bread breadd breadbar breadbox breadbox-sync breadcrumbs breadpad breadman; do
    check "$bin found" "command -v $bin"
done

echo "== bos-settings =="
check "bos-settings installed" "command -v bos-settings"

echo "== default dotfiles =="
check "hyprland.lua present"  "[ -f \"\$HOME/.config/hypr/hyprland.lua\" ]"
check "mimeapps.list present" "[ -f \"\$HOME/.config/mimeapps.list\" ]"
check "kitty config present"  "[ -f \"\$HOME/.config/kitty/kitty.conf\" ]"

echo "== bootloader (EFI) =="
check "GRUB EFI binary present" \
    "[ -f /boot/efi/EFI/BOS/grubx64.efi ] || [ -f /boot/efi/EFI/BOOT/BOOTX64.EFI ]"
check "grub.cfg present" "[ -f /boot/grub/grub.cfg ]"

echo
printf 'Result: \033[32m%d passed\033[0m, \033[31m%d failed\033[0m\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
