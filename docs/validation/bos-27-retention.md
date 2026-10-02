# BOS-27 snapshot retention validation

Validated on 2026-09-30 in an ephemeral Arch Linux KVM guest with a Btrfs root filesystem. The guest ran inside a container limited to two CPUs and 3 GiB RAM, with no host display, desktop session, or personal configuration mounts. The official Arch cloud image checksum was verified before boot.

The guest used Snapper 0.13.2-1, snap-pac 3.0.1-3, and btrfs-progs 7.1-1. The root filesystem had 17 GiB available out of 18 GiB after the transaction run.

The root configuration was created with Snapper's normal Btrfs template, then configured using the installer commands in `post-install.sh`:

```sh
snapper -c root setup-quota
snapper -c root set-config \
  NUMBER_LIMIT=2-20 NUMBER_LIMIT_IMPORTANT=4-10 \
  FREE_LIMIT=0.2 SPACE_LIMIT=0.5
snapper -c root set-config \
  TIMELINE_CREATE=no NUMBER_CLEANUP=yes NUMBER_MIN_AGE=1800
for transaction in $(seq 1 10); do
  pacman -S --noconfirm nano
done
snapper -c root cleanup number
```

All ten real pacman transactions completed. The installed snap-pac hooks created ten pre snapshots numbered 1, 3, 5, 7, 9, 11, 13, 15, 17, and 19, and ten post snapshots numbered 2, 4, 6, 8, 10, 12, 14, 16, 18, and 20. Each post snapshot referenced its matching pre snapshot. All twenty remained after number cleanup.

The test also temporarily set `NUMBER_MIN_AGE=0` and repeated number cleanup. All ten complete pairs remained, confirming retention was not merely protected by the 1800-second age guard. The configured age was restored afterwards.

The resulting configuration contained:

```ini
QGROUP="1/0"
SPACE_LIMIT="0.5"
FREE_LIMIT="0.2"
NUMBER_CLEANUP="yes"
NUMBER_MIN_AGE="1800"
NUMBER_LIMIT="2-20"
NUMBER_LIMIT_IMPORTANT="4-10"
TIMELINE_CREATE="no"
```

This validates the requested free-space transaction retention. It does not claim a low-space stress test or a fresh Calamares installation. The Settings copy is tracked separately in BSET-20 and PR #22.
