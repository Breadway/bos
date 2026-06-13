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
    ["/etc/calamares/post-install.sh"]="0:0:755"
)
