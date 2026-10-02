# Hardware and recovery

## GPUs

BOS ships the generic **Mesa** stack. AMD and Intel work out of the box.

The proprietary NVIDIA driver is **not on the ISO**. NVIDIA firmware is
not on the image either (`linux-firmware-nvidia` stays commented out in
`packages.x86_64`). Default Hyprland env is vendor-neutral.

On first graphical login, `bos-first-boot` probes `lspci` / `/proc` and,
if an NVIDIA GPU is present, writes `~/.local/state/bos/nvidia-offer.json`
and notifies that the proprietary driver is not on the ISO. It does **not**
install anything.

The optional proprietary path is `bos-nvidia-setup` or the Settings →
Updates NVIDIA button. That installs `nvidia` + `nvidia-utils` (never
cuda) and writes `~/.config/hypr/nvidia.lua`. `hyprland.lua` dofiles that
drop-in **only if the file exists**, so Mesa machines stay unchanged.
Reboot after. Installing the packages by hand without the drop-in is not
enough for a working Hyprland session.

The same probe leaves a HiDPI hint at `~/.local/state/bos/hidpi-hint.json`
when scale > 1 or the panel is dense; it never rewrites `monitors.json`.
A VM without `/dev/dri` gets a notification only.

## Compatibility and install limits

BOS is an x86-64 image. It cannot boot on Apple Silicon Macs. Intel Macs with
the T2 security chip need external boot enabled in Recovery settings. Their
keyboard and trackpad also need a patched kernel, which the BOS ISO does not
include, so they are not supported out of the box.

The installer does not provide a supported Windows dual-boot setup. Plan to
use a whole disk for BOS; it will not shrink a Windows installation to make
room alongside it.

Hibernation is not configured by default. BOS enables compressed zram swap in
memory, which does not preserve a session across power loss. The installer
does not create a disk-backed swap area for hibernation.

## Recovery

An update that breaks the system is recovered by **reboot → GRUB
“snapshots” submenu** (grub-btrfs). Snapshot boots are writable through a
temporary overlay, so changes disappear after reboot. `snapper rollback`
will not change what GRUB boots (`rootflags=subvol=@`).

`snapper rollback` swaps the default subvolume; the installed `grub.cfg`
still boots `@`. Pick the grub-btrfs entry so the kernel command line
matches the snapshot you want.

BOS Settings → Snapshots lists snapshot number, date, and description so
you know which GRUB entry to pick. To keep a working snapshot, run
`sudo bos-restore-snapshot <number>` while booted from it. The helper
preserves the current `@` as `@.broken-<date>`, installs the selected
snapshot as `@`, and reboots. Bakery desktop apps live under `/usr/local`
on `@`, so those same snapshots include them.

If the system will not boot (lost EFI entry / broken GRUB), boot the live
ISO and run `sudo bos-rescue`. It mounts `@` + the ESP and offers the same
`grub-install` NVRAM + `--removable` sequence as `post-install.sh`.

See the README Recovery section for the snapshot restore flow.
