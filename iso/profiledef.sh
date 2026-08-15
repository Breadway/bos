#!/usr/bin/env bash
# shellcheck disable=SC2034

iso_name="bos"
iso_label="BOS_$(date +%Y%m)"
iso_publisher="Breadway"
iso_application="Bread Operating System"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
# systemd-boot can only read files from the ESP it was launched from, so
# mkarchiso's _make_bootmode_uefi.systemd-boot copies vmlinuz + initramfs
# INTO the FAT efiboot.img on top of the copy already on ISO9660 (~244 MiB
# duplicate). uefi.grub's ESP is only EFI + shell*.efi — GRUB reads ISO9660
# directly. iso/grub/{grub,loopback}.cfg are already BOS-branded.
bootmodes=('bios.syslinux' 'uefi.grub')
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
    ["/usr/local/bin/bos-resolve-airootfs"]="0:0:755"
    ["/usr/local/bin/bos-session"]="0:0:755"
    ["/usr/local/bin/bos-netcheck"]="0:0:755"
    ["/usr/local/bin/bos-update"]="0:0:755"
)
