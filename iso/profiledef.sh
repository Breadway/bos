#!/usr/bin/env bash
# shellcheck disable=SC2034

iso_name="bos"
iso_label="BOS_$(date +%Y%m)"
iso_publisher="Breadway"
iso_application="Bread Operating System"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=('bios.syslinux' 'uefi.systemd-boot')
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'xz' '-Xbcj' 'x86' '-b' '1M' '-Xdict-size' '1M')
file_permissions=(
    ["/etc/shadow"]="0:0:400"
    ["/etc/sudoers.d/99-bos-live"]="0:0:440"
    ["/etc/calamares/post-install.sh"]="0:0:755"
    ["/usr/local/bin/bos-live-setup"]="0:0:755"
    ["/usr/local/bin/bos-launch-calamares"]="0:0:755"
    ["/usr/local/bin/bos-copy-kernel"]="0:0:755"
    ["/usr/local/bin/bos-session"]="0:0:755"
    ["/usr/local/bin/bos-keybinds"]="0:0:755"
    ["/usr/local/bin/bos-welcome"]="0:0:755"
    ["/usr/local/bin/bos-update"]="0:0:755"
)
