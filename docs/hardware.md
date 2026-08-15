# Hardware and recovery

## GPUs

BOS ships the generic **Mesa** stack. AMD and Intel work out of the box.

**NVIDIA is unsupported.** The proprietary driver is not included, NVIDIA
firmware is not on the image, and there is no Hyprland NVIDIA env wiring.
Installing `nvidia` / `nvidia-utils` after the fact is not a product path.

## Recovery

An update that breaks the system is recovered from the **GRUB "snapshots"
submenu** (grub-btrfs), not `snapper rollback`.

BOS GRUB pins `rootflags=subvol=@`. `snapper rollback` swaps the default
subvolume; the installed `grub.cfg` will still boot `@`. Pick the grub-btrfs
entry so the kernel command line matches the snapshot you want.

A/B root swapping is not implemented. See the README Recovery section for
the "system will not boot" GRUB/EFI repair path.
