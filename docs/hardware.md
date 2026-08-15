# Hardware and recovery

## GPUs

BOS ships the generic **Mesa** stack. AMD and Intel work out of the box.

**NVIDIA is unsupported.** The proprietary driver is not included, NVIDIA
firmware is not on the image, and there is no Hyprland NVIDIA env wiring.
Installing `nvidia` / `nvidia-utils` after the fact is not a product path.

On first graphical login, `bos-first-boot` probes `lspci` / `/proc` and,
if an NVIDIA GPU is present, writes `~/.local/state/bos/nvidia-offer.json`
and notifies that the proprietary driver is not on the ISO. It does **not**
install anything. bos-settings can grow a panel that reads that file later.

The same probe leaves a HiDPI hint at `~/.local/state/bos/hidpi-hint.json`
when scale > 1 or the panel is dense; it never rewrites `monitors.json`.
A VM without `/dev/dri` gets a notification only.

## Recovery

An update that breaks the system is recovered from the **GRUB "snapshots"
submenu** (grub-btrfs), not `snapper rollback`.

BOS GRUB pins `rootflags=subvol=@`. `snapper rollback` swaps the default
subvolume; the installed `grub.cfg` will still boot `@`. Pick the grub-btrfs
entry so the kernel command line matches the snapshot you want.

If the system will not boot (lost EFI entry / broken GRUB), boot the live
ISO and run `sudo bos-rescue`. It mounts `@` + the ESP and offers the same
`grub-install` NVRAM + `--removable` sequence as `post-install.sh`.

A/B root swapping is not implemented. See the README Recovery section.
