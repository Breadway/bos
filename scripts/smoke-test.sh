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
        note "couldn't list subvolumes (need root): skipping"
    else
        for sv in @ @home @snapshots @log @cache; do
            if grep -qx "$sv" <<<"$paths"; then ok "subvolume $sv present"; else bad "subvolume $sv missing"; fi
        done
    fi
else
    note "btrfs not installed (not a btrfs root?): skipping subvolume checks"
fi

echo "== snapshot tooling =="
check "snapper root config exists" "[ -f /etc/snapper/configs/root ]"
check "snap-pac hook present"      "pacman -Qq snap-pac"
check "grub-btrfs present"         "pacman -Qq grub-btrfs"

echo "== audio firmware =="
check "sof-firmware installed" "pacman -Qq sof-firmware"

echo "== enabled system services =="
for unit in NetworkManager.service greetd.service bluetooth.service tlp.service \
            cups.socket avahi-daemon.socket ufw.service systemd-timesyncd.service; do
    check "$unit enabled" "systemctl is-enabled $unit"
done
check "graphical.target is default" "[ \"\$(systemctl get-default)\" = graphical.target ]"

echo "== bread ecosystem on PATH =="
for bin in bakery bread breadd bread-emit bread-module-host breadbar breadbox breadbox-sync breadcrumbs breadpad breadman; do
    check "$bin found" "command -v $bin"
done
if command -v breadbug >/dev/null 2>&1; then
    check "breadbug found" "command -v breadbug"
else
    note "breadbug not installed: breadbug feature check skipped (bread packaging not in iso/bread-lockfile.toml yet); not a pass"
fi

echo "== bos-settings =="
check "bos-settings installed" "command -v bos-settings"

echo "== breadhelp =="
check "breadhelp installed" "command -v breadhelp"
check "breadhelp content installed" \
    "[ -d /usr/local/share/breadhelp/content ] || [ -d \"\$HOME/.local/share/breadhelp/content\" ]"
check "bos-netcheck present" "command -v bos-netcheck"
check "bos-rescue present" "command -v bos-rescue"
check "bos-first-boot present" "command -v bos-first-boot"
check "bos-nvidia-setup present" "command -v bos-nvidia-setup"
if pacman -Qq nvidia >/dev/null 2>&1; then
    note "nvidia installed (optional proprietary path)"
    check "nvidia env drop-in present" "[ -f \"\$HOME/.config/hypr/nvidia.lua\" ]"
else
    check "nvidia not on the default image" "! pacman -Qq nvidia"
fi

echo "== bakery user units (global enable) =="
# A later useradd does not enable --user units unless they were enabled
# --global (or the user enables them). post-install + live-setup + bake
# write /etc/systemd/user/<target>.wants/ and a preset listing the set.
check "bakery user preset present" \
    "[ -f /usr/lib/systemd/user-preset/90-bos-bakery.preset ]"
check "bos-enable-bakery-user-units present" \
    "command -v bos-enable-bakery-user-units"
check "breadd.service globally enabled" \
    "systemctl --global is-enabled breadd.service || [ -L /etc/systemd/user/default.target.wants/breadd.service ]"
if [[ -f /usr/lib/systemd/user-preset/90-bos-bakery.preset ]]; then
    while read -r verb unit; do
        [[ "$verb" == enable && -n "$unit" ]] || continue
        [[ -f /usr/lib/systemd/user/$unit ]] || continue
        check "$unit globally enabled" \
            "systemctl --global is-enabled $unit || [ -L /etc/systemd/user/default.target.wants/$unit ] || [ -L /etc/systemd/user/graphical-session.target.wants/$unit ]"
    done < /usr/lib/systemd/user-preset/90-bos-bakery.preset
fi
check "skel hyprland.lua present" "[ -f /etc/skel/.config/hypr/hyprland.lua ]"
check "skel bakery installed.json present" \
    "[ -f /etc/skel/.local/state/bakery/installed.json ]"
check "skel bakery index cache present" \
    "[ -f /etc/skel/.cache/bakery/index.json ]"
check "skel has no bakery binaries" \
    "! [ -e /etc/skel/.local/bin/bakery ] && ! [ -e /etc/skel/.local/bin/breadd ]"
check "useradd SKEL is /etc/skel" \
    "grep -q '^SKEL=/etc/skel' /etc/default/useradd"

echo "== default dotfiles =="
check "hyprland.lua present"  "[ -f \"\$HOME/.config/hypr/hyprland.lua\" ]"
check "hyprland.lua includes nvidia.lua only if present" \
    "grep -q 'nvidia.lua' \"\$HOME/.config/hypr/hyprland.lua\""
check "binds.json present" "[ -f \"\$HOME/.config/hypr/binds.json\" ]"
check "monitors.json present" "[ -f \"\$HOME/.config/hypr/monitors.json\" ]"
check "settings.json present" "[ -f \"\$HOME/.config/hypr/settings.json\" ]"
check "autostart.json present" "[ -f \"\$HOME/.config/hypr/autostart.json\" ]"
check "autostart includes first-boot probe" "grep -q bos-first-boot \"\$HOME/.config/hypr/autostart.json\""
check "hypr scripts/lib present" "[ -f \"\$HOME/.config/hypr/scripts/lib/json.lua\" ]"
check "window.float action shim present" "[ -f \"\$HOME/.config/hypr/scripts/lib/window.lua\" ]"
check "mimeapps.list present" "[ -f \"\$HOME/.config/mimeapps.list\" ]"
check "kitty config present"  "[ -f \"\$HOME/.config/kitty/kitty.conf\" ]"
check "breadbug-notify module present" \
    "[ -f \"\$HOME/.config/bread/modules/breadbug-notify.lua\" ]"
check "breadbug keybind present" "grep -q 'breadbug' \"\$HOME/.config/hypr/binds.json\""
check "breadbug sends nothing by default" \
    "! grep -qs '^upload_enabled *= *true' \"\$HOME/.config/bread/breadbug.toml\""

echo "== bootloader (EFI) =="
check "GRUB EFI binary present" \
    "[ -f /boot/efi/EFI/BOS/grubx64.efi ] || [ -f /boot/efi/EFI/BOOT/BOOTX64.EFI ]"
check "grub.cfg present" "[ -f /boot/grub/grub.cfg ]"
ROOT_DEVICE="$(findmnt -no SOURCE / | sed 's/\[.*\]//')"
if [[ "$(lsblk -no TYPE "$ROOT_DEVICE" 2>/dev/null)" == "crypt" ]]; then
    check "encrypted root has an initramfs unlock parameter" \
        "grep -Eq '(^| )(cryptdevice=|rd.luks.name=)' /proc/cmdline"
else
    note "root is not encrypted: skipping initramfs unlock parameter check"
fi

echo
printf 'Result: \033[32m%d passed\033[0m, \033[31m%d failed\033[0m\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
