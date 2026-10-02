# BOS-47 firewall validation

Validated on 2026-09-30 in an isolated headless Arch Linux KVM guest. A separate Btrfs target root was snapshotted and mounted as a chroot sharing the guest's live network namespace. The test ran the exact firewall block extracted from `post-install.sh`, and enabled `ufw.service` in that target root.

The earlier implementation still called `ufw allow 5353/udp` while UFW was disabled. The real test showed that this command created empty `ip filter` and `ip6 filter` tables in the live kernel. The revised implementation writes both UFW rule files directly, including the tuple metadata used by UFW to display persisted rules.

Before and after the revised installer block, the guest's complete nft ruleset had the same SHA-256:

```text
9f3d6233b0fdfd1d5a961f3ef99c98a0fc1fb14465b964d416fab3afb119d414
```

A byte comparison passed. Running the installer block twice also preserved identical `user.rules` and `user6.rules` checksums, with exactly one IPv4 and one IPv6 mDNS rule.

The guest then rebooted into the prepared target root:

```text
/dev/vda3[/@bos47-installed] btrfs
systemctl is-enabled ufw.service: enabled
systemctl is-active ufw.service: active
ufw status: Status: active
Default: deny (incoming), allow (outgoing), disabled (routed)
5353/udp: ALLOW IN Anywhere
5353/udp (v6): ALLOW IN Anywhere (v6)
```

The test target had an additional temporary SSH rule limited to the guest's QEMU user-network gateway, so its loopback-forwarded administrative connection remained available after reboot. That rule is not part of the installer change.

This was a target-chroot and target-boot test of the exact installer block, not an interactive Calamares run. No host firewall, display, desktop session, or personal configuration was exposed to the guest.
