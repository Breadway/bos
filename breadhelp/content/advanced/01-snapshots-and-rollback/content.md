# Snapshots and rollback

Every package change snapshots your root filesystem automatically (snapper +
snap-pac). Nothing to configure — it just happens.

## Roll back

- From BOS Settings' Snapshots page, or
- Right from the GRUB menu at boot (pick an older snapshot to boot into).

## Take a manual snapshot before something risky

- [Run]snapper -c root create --description "manual snapshot"|Create a snapshot now[/Run]
