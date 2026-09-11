# Changelog

All notable changes to Bread OS (BOS) are documented in this file.

This file starts at 0.6.2; earlier releases are not backfilled. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), one
section per BOS release tag, newest first. Each section has:

1. A technical list: every bread-ecosystem binary whose pinned version
   changed since the previous BOS release (sourced from
   `iso/bread-lockfile.toml`), plus BOS-level changes (ISO, installer,
   packaging, CI) from that release's own commit and PR history.
2. A short, plain-language "what's different and why, and how it
   affects you" summary, distilled from the technical list rather than
   a restatement of it.

Updating this file is part of the BOS release process: refresh the
`[Unreleased]` section as work lands, then rename it to the release
tag (with date) at tag time.

## [Unreleased] - targeting 0.6.2

### What's different and why

Breadsearch (semantic file search) is no longer installed by default.
It was a memory-heavy background service most people never used; it
is still one command away with `bakery install breadsearch`. Boot and
session switching are visibly smoother: the splash screen now stays
on screen until the desktop actually paints, instead of flashing to a
bare text console, most noticeable right after boot and when greetd
restarts the login screen. The `[breadway]` package repository is now
cryptographically signed and verified end to end, closing a
supply-chain gap where the ISO trusted an unsigned build artifact
tree. A GTK4 rendering bug that could make the whole shell (launcher,
clipboard popup, bar, settings) render solid black on virtual
machines and some real hardware is fixed, and shell blur and
transparency effects now work for the first time. Docking a laptop
into external monitors is zero-config. Assorted packaging and CI
hardening keeps ISO builds reproducible.

### bread-ecosystem binary version changes (since v0.6.1)

- `bakery`: 0.7.3 -> 0.7.4
- `bread-theme`: 0.7.3 -> 0.7.4
- `bread-emit`, `bread-module-host`: moved from optional to required
  (bread 0.8.0 ships both)
- `breadsearch`, `breadmill`: dropped from the default install (still
  installable with `bakery install breadsearch`)

### BOS changes (ISO / installer / packaging / CI)

- Dropped breadsearch and its breadmill indexer from the default
  install: lockfile, systemd preset, Hyprland autostart entries, and
  the desktop launcher. Docs updated to point at the opt-in install
  path. (#12)
- Eliminated the console flash across the boot and session handoffs:
  `plymouth-quit` now retains the last splash frame instead of
  clearing the framebuffer, `getty@tty1` is masked so a greetd
  restart cannot flash a login prompt, and the live/session scripts
  blank tty1 around the Hyprland handoff. (#13, pending merge)
- Flipped the `[breadway]` pacman repository to the signed,
  GPG-verified `dl.breadway.dev/arch` endpoint; the ISO, the live
  medium, and Calamares installs all trust the repo key instead of
  pulling from the unsigned Forgejo registry.
- Republished `python-pywal` to `[breadway]` after Arch dropped it
  from `[extra]`, which had been breaking every ISO build
  (`pacstrap: target not found: python-pywal`). `bread-theme` shells
  out to `wal` for wallpaper palette extraction.
- Fixed a GTK4 renderer bug (`GSK_RENDERER=cairo`) causing the shell
  to draw transparent layer-shell surfaces as opaque black on
  wlroots, most reliably reproduced on virtio-gpu VMs but also seen
  on real hardware. Wired up the previously-unread shell layer-rule
  JSON (blur, ignore-alpha, per-namespace motion) that bread-theme
  had been generating but BOS never applied.
- Added an external-monitors bread module for zero-config docking.
- Post-install audit sweep: resolve the target user's UID after
  `liveuser` is deleted so Snapper and skel target the right account,
  wrap `bakery update` with `sudo -n`/`pkexec` for `/usr/local`, make
  the Calamares internet check non-blocking, autostart
  `breadlock listen`, and smoke-test `avahi-daemon.socket`.
- CI hardening: sign the `[breadway]` repo as the runner user instead
  of root (fixes an unreadable-signature publish failure), don't fail
  signed-repo publish on a signature chmod, and scrub the internal
  runner hostname from public workflow files.
