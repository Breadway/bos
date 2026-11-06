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

## [Unreleased]

- Bluetooth is enabled in the live session, so a machine with an adapter can
  pair without first opening Settings and authenticating. bluez's unit is
  conditional on `/sys/class/bluetooth`, so the setting is a no-op on machines
  with no radio. Installed systems already enabled it. Check with
  `python3 scripts/test-live-bluetooth.py`.
- `hl.dsp.window.float({ action = "set" })` and `{ action = "unset" }` now
  keep a window floating or tiled on repeated calls. Hyprland's Lua binding
  only understands toggle/enable/disable, so the documented set/unset actions
  silently toggled; BOS installs a small action shim at config load. Callers
  that dispatch through the session, including `hyprctl eval`, get the
  corrected behaviour. Check with `scripts/test-window-float.lua` or the
  disposable harness in `scripts/test-window-float-headless.sh`.
- The live medium's writable overlay is now 25% of RAM instead of archiso's
  256 MiB default, the live user's Downloads directory lives on the /tmp tmpfs,
  and the live image disables core dumps. A 256 MiB overlay was exhausted by an
  ordinary session: one download, a browser cache and 192 crash dumps filled it
  in under an hour, after which nothing could be saved, installed or logged.
  Installed systems are unchanged. Check with
  `python3 scripts/test-live-overlay-config.py`.
- The live USB now starts the printing service at boot, so Settings > Printers
  works straight away instead of reporting "Scheduler is not running". Installed
  systems already had this.
- Hyprland now reads its settings, monitors, keybinds, autostart and layer
  rules from `$XDG_CONFIG_HOME/hypr` when that variable is set to an absolute
  path, so a non-default config location is honoured. Users with no
  `XDG_CONFIG_HOME` see no change.
- Idle lock actually locks. `hypridle` used `pidof breadlock`, which is
  always true because `breadlock listen` runs from login, so Super+L, the
  idle lock and the lock-before-suspend never started a lock screen. The
  command now matches only a real locker (`pgrep -xf breadlock`). Existing
  installs pick this up the next time Power settings are saved in
  BOS Settings.
- Report a problem with a bread app: `Super+Shift+B` opens `breadbug-launch`,
  which uses the windowed breadbug reporter when a display is available and
  the terminal client otherwise. It shows exactly what a bug report would
  contain (scrubbed of personal data) before you decide to save it or send
  it. A new `breadbug-notify` module offers to review a report when a bread
  app crashes or freezes, with the same launcher behind its action. Nothing
  is sent unless you choose to, and sending is off by default.
- Encrypted Calamares installs now register a boot keyfile and configure the
  mkinitcpio unlock hook before initramfs generation. GRUB records the LUKS
  container and mapper so the initramfs can unlock root without asking for a
  second passphrase; the smoke test checks the encrypted boot parameters.
- When UEFI firmware is in Setup Mode, the installer now builds GRUB with
  shim-lock disabled, SBAT metadata, and the filesystem, crypto, and boot
  modules it needs. It then enrolls and saves signatures for both GRUB EFI
  files and the kernel so sbctl can re-sign them after package updates.
- Installer finalization now records critical boot failures, returns a failure
  to Calamares instead of showing the success page, and writes a full log to
  `/var/log/bos-install.log`.
- Root Snapper cleanup now keeps 2-20 normal and 4-10 important snapshots,
  while pruning older snapshots when filesystem space thresholds are reached.
- Installer: offer only the no-disk-swap choice. BOS uses zram swap, and
  installer-created swap files or partitions would break btrfs snapshots or
  advertise hibernation without a configured resume path.
- Package cache: include pacman-contrib and enable weekly paccache cleanup,
  retaining the latest three cached versions of each package.
- Graphics: include Intel VA-API decoding and Intel/AMD Vulkan drivers,
  with vainfo and vulkaninfo tools for hardware verification.
- Installer: reserve UID 1000 for the installed account by assigning the
  temporary live user UID 60001, outside normal account allocation.
- Keybinds: use arrows or H/J/K/L for focus, add Shift to move and Ctrl to
  resize. Lock with Super+Escape; logout remains Super+Shift+Escape.
- Repository signing: require a protected exact version/SHA-256 package policy
  before signing registry downloads or local overrides. Packaging workflows emit
  unapproved build provenance for review; registry writers cannot advance trust.

- Signing: Actions now prepares unsigned ISO draft and repository candidates.
  Private signing keys stay on separate offline hosts, with distinct expiring
  identities and pinned-fingerprint checks. New ISO publication requires the
  documented key provisioning and offline approval step.
- NVIDIA setup: classify exact supported PCI IDs, refuse legacy/unknown GPUs,
  choose open modules or DKMS with kernel headers, and expose a read-only
  dry-run plan that first boot records for Settings.
- Updates: pair numbered bakery pre/post Snapper snapshots, including failed
  updates, instead of leaving orphaned pre snapshots. Enable bakery transaction
  snapshots for direct and Packages updates once its Snapper support ships;
  bos-update keeps a paired fallback for older bakery builds.

- Audio: include `sof-firmware` and `alsa-firmware` in the ISO for laptop
  speakers and microphones that require Sound Open Firmware or legacy ALSA
  firmware.
- Installer: require passwords to be at least 8 characters, consistently in
  Calamares and libpwquality. For encrypted installs, use at least 12
  characters; the password page shows a non-blocking warning for shorter
  passwords when LUKS is selected.
- Hypridle now locks the session before turning the display off after eight
  minutes of idle time.
- Installer: stage a signed `linux-lts` package in the ISO for offline installs,
  install it before mkinitcpio and GRUB generation, and save its kernel with
  sbctl when Secure Boot keys are enrolled.
- ISO build: `build-local.sh` now trusts the `[breadway]` repository key in a
  temporary pacman keyring that is removed when the build exits, leaving the
  build host's pacman keyring unchanged.
- Installer firewall setup now persists UFW policy and the enabled setting
  without applying rules to the live session; `ufw.service` applies them on
  the installed system's first boot.
- Skel: move the Hyprland log out shortcut from `SUPER+N` to
  `SUPER+Shift+Escape` so one accidental keypress cannot close the session.
- GRUB snapshot boots now get a temporary writable overlay with both systemd
  and udev initramfs hooks. Added `bos-restore-snapshot` to make a selected
  snapshot the installed `@` root while preserving the previous root.
- Skel: the curated default palette moved from
  `.cache/wal/colors.json` to `.cache/bread/palette.json`; `.zshrc` now
  imports `.cache/bread/sequences` instead of `.cache/wal/sequences`
  (honouring `$XDG_CACHE_HOME` if set). `python-pywal` dropped from
  `iso/packages.x86_64`, its AUR republish packaging/CI workflow, and the
  signed `[breadway]` repo publish script, now that bread-theme extracts
  palettes natively.
- **Upgrade note for existing installs**: the two changes above only affect
  `skel`, the template new user accounts are created from; an existing
  account's own `~/.zshrc` and `~/.cache/wal/` are not touched by a BOS
  package upgrade. If your `.zshrc` predates this change it still does
  `cat ~/.cache/wal/sequences 2>/dev/null` on every new shell: that file is
  no longer written by anything (breadpaper's native extractor writes
  `~/.cache/bread/sequences` instead), so new terminals stop getting live
  OSC colour sequences on wallpaper change until you edit `.zshrc` by hand
  to `cat ~/.cache/bread/sequences 2>/dev/null` (or `$XDG_CACHE_HOME` if
  you set one), matching skel's new line. `python-pywal` itself is not
  removed from an existing install either, and it will not show up as an
  orphan: `iso/packages.x86_64` installed it explicitly (not as a
  dependency of something else), and pacstrap/Calamares preserve that
  install reason, so `pacman -Qtdq` (which only lists packages installed
  as a dependency) will never list it. It simply stays installed, and now
  that `[breadway]` no longer republishes it, it stops receiving updates.
  Remove it yourself once you've confirmed nothing else depends on it
  (check the `Required By` field of `pacman -Qi python-pywal`) with
  `sudo pacman -Rns python-pywal`.
  BOS has no post-upgrade hook mechanism today that could rewrite a user's
  `.zshrc` safely (bakery only exposes a package's own `post_install`,
  which runs the same on every install and has no concept of "this is an
  upgrade of a file the user may have since customised"); a real fix would
  need a one-time, opt-in migration step, e.g. `bos-upgrade-hook` invoked
  from a pacman `post_upgrade` action on the meta-package that only edits
  `.zshrc` if it still contains the exact stock `~/.cache/wal/sequences`
  line (skip silently otherwise, so a customised file is never touched),
  logging what it changed. Not implemented here: flagged for a follow-up
  issue rather than added invasively as part of this change.
- Skel: added `.config/breadpaper/current.json` seeding the `"*"` output
  to the default background, so `breadpaper get` (and bos-settings'
  Wallpaper page) has something to read on a fresh install now that
  `get()` no longer falls back to pywal's `~/.cache/wal/wal`.
- Local ISO builds now stage lockfile-pinned bakery binaries from the signed
  index by default. Set `BAKE_FROM_HOME=1` to use the builder's installed
  bakery versions for development. The build uses a temporary package
  keyring, leaving the host pacman keyring unchanged.
- Updated the bakery, bread-theme, and breadpaper lockfile pins to their
  signed stable-index versions so all three can be verified by SHA-256.

## [0.6.2] - 2026-09-19

### What's different and why

This is a large release: every bakery-channel binary BOS ships got a
real promotion pass (see below), not just a version bump. The
headline changes: breadsearch is no longer installed by default
(still one command away with `bakery install breadsearch`); the
`[breadway]` package repository is cryptographically signed end to
end; boot and session switching no longer flash to a bare text
console; the shell (breadbar, breadbox, bread-theme) got a large
declarative theming overhaul, including a new vertical/side-docked
bar layout, several new built-in themes, and an embedded launcher
capsule in the bar itself; per-monitor theming is now correct
end-to-end, including a bos-settings bug where the accent color could
silently fall back to a stale global palette. Three real security
hardenings landed: breadcrumbs no longer leaks Wi-Fi passwords onto
argv/shell history, bread's module-host canonicalizes path grants and
execs without a shell, and bos-settings hardens privileged command
operands against injection. breadman (the fuller notes manager in the
`breadpad` package) was rebuilt from GTK4 onto Tauri 2 + Svelte.
breadclip gained real clipboard-history pinning, config, and a fix
for the bug where pasting from history had nowhere to paste into.
breadcrumbs moved from shelling out to `nmcli` to talking to
NetworkManager directly over D-Bus, plus a feature batch (captive
portal detection, scheduled profiles, exit-node failover, 802.1x,
per-network DNS). Docking a laptop into external monitors is now
zero-config.

### bread-ecosystem binary version changes (since v0.6.1)

Checked directly against `iso/bread-lockfile.toml` as it stood at the
`v0.6.1` tag (not just the previous bread-ecosystem tag), and against
the live `dl.breadway.dev/index.json` for the versions that actually
shipped, so this reflects what was really promoted to stable, not
what merely got tagged. `breadshot` is unchanged since v0.6.1
(0.1.3); every other pinned binary moved.

- **`bread-theme` 0.7.3 -> 0.7.12.** The largest single change in this
  release. Per-output palettes and window-scoped theme binding (each
  Hyprland/GDK connector can have its own palette and stylesheet, and
  GTK apps bind a widget-level CSS provider so two windows in one
  process can follow different wallpapers) is what exposed the
  bos-settings accent bug below. On top of that: a declarative shell
  theme manifest system (`[[bar.widget]]`, `[panel]`/`[osd]` tables,
  chrome tokens, typography threading), Hyprland layer-rule generation
  from the theme, vertical/horizontal bar orientation support, four
  new built-in themes (Loaf, Daylight, glass-workbench, spotlight),
  a `bread_theme::write_wal_cache` API so consumers stop hand-rolling
  pywal's cache format, and a string of per-monitor correctness fixes
  (bind lifecycle races, hot-reload watching the wrong path, chip
  height token, control-panel radius).
- **`bakery` 0.7.3 -> 0.7.12.** Shares a workspace version with
  `bread-theme` in the bread-ecosystem monorepo, so most of this bump
  is lockstep, not bakery-specific. One real fix: a failed
  data-archive download no longer aborts the whole install.
- **`bread` 0.8.0 -> 0.9.0.** New `bread init` Hyprland compositor
  layer-rule integration. Security fix: `bread-module-host`
  canonicalizes path grants and execs without a shell. `bread-emit`
  and `bread-module-host` are now packaged on all tracks and, in BOS's
  own lockfile, moved from optional to required (bread 0.8.0 already
  shipped both, BOS just was not requiring them).
- **`breadbar` 0.3.3 -> 0.4.0.** Vertical (side-docked) bar layout.
  Consumes the new shell-theme manifest system end to end. Security
  fix: stopped leaking the Wi-Fi password onto `nmcli`/breadcrumbs
  argv. An embedded launcher capsule (phases 6b/6c: keyboard open,
  query modes, search-state geometry). Notification/OSD polish
  (click-through-except-buttons, dismiss button, entrance/exit
  animations, no more focus stealing) and several workspace-pill and
  chip animation fixes. Fixed a subprocess zombie leak on self-restart.
- **`breadbox` 0.3.3 -> 0.4.0.** Adopted the shared `bread-launcher`
  crate for matching/ranking/rows instead of its own copy. Fixed the
  launcher overlay painting opaque instead of transparent. Theme is
  now re-read per popup open instead of once per process. Added a
  tracing subscriber for observability.
- **`breadcrumbs` 2.1.8 -> 2.2.0.** Talks to NetworkManager directly
  over D-Bus instead of shelling out to `nmcli`, tested against an
  in-process D-Bus fake. Security fix: the Wi-Fi PSK no longer reaches
  argv or shell history on `add` (fed to `nmcli --ask` on stdin
  instead) or on the new password-free `join <ssid>` command. Feature
  batch: captive-portal detection, scheduled profiles, exit-node
  failover, 802.1x, per-network DNS. Config/state files gained file
  locking so `watch` no longer races a concurrent one-shot command.
  A single D-Bus device's hiccup no longer aborts the whole scan.
- **`breadclip` 0.2.4 -> 0.3.0.** History DB hardening plus a proper
  user config and pin/primary schema. Watch-driven capture with a
  flock-based singleton and a copied-secret ignore-rule heuristic,
  now user-configurable. Fixed the bug where the history popup stole
  keyboard focus and never gave it back, so pasting from history had
  nowhere to go; fixed the overlay painting opaque instead of
  transparent.
- **`bos-settings` 0.8.2 -> 0.9.0.** Security fix: privileged command
  operands are now hardened against injection. Settings redesign to
  hub-based navigation, plus per-monitor wallpaper and a shell-theme
  picker, canonical accent mapping with live per-output/monitor-move
  updates, and the window titlebar dropped. Fixed the bug where the
  accent color fell back to a stale global pywal palette when Tauri
  could not name the current monitor by its Hyprland connector
  (the same per-output theming change in bread-theme above is what
  exposed this). `cargo fmt --check` is now gated in CI.
- **`breadpad` 0.5.3 -> 0.6.0.** breadman (the fuller notes manager)
  was rebuilt from a bare GTK4 window onto Tauri 2 + Svelte, and both
  `breadpad` and `breadman` windows now bind per-output app CSS
  instead of one shared stylesheet.
- **`breadpaper` 0.1.14 -> 0.1.15.** The global `~/.cache/wal/colors.json`
  and shared `theme.css` now sync from the focused output on a
  per-monitor wallpaper set, and per-output theme state is
  deduplicated and pruned, so non-multi-monitor-aware consumers stop
  drifting from what is actually on screen.
- **`breadmon` 0.1.4 -> 0.1.5.** Unsaved edits are now guarded against
  a background refresh or a failed apply overwriting them, and undo
  steps are coalesced.
- **`breadhelp` 0.2.6 -> 0.2.7.** Help copy now matches BOS's live
  keybinds and reflects breadsearch being opt-in (semantic search).
  The help window is skipped on a silent every-login autostart.
- **`bread-emit`, `bread-module-host`**: see the `bread` entry above,
  version unchanged, requirement status changed.
- **`breadsearch`, `breadmill`**: version unchanged upstream. Dropped
  from BOS's `required_bins` entirely, so neither installs by
  default; still one command away with `bakery install breadsearch`.

### BOS changes (ISO / installer / packaging / CI)

- Dropped breadsearch and its breadmill indexer from the default
  install: lockfile, systemd preset, Hyprland autostart entries, and
  the desktop launcher. Docs updated to point at the opt-in install
  path. (#12)
- Eliminated the console flash across the boot and session handoffs:
  `plymouth-quit` now retains the last splash frame instead of
  clearing the framebuffer, `getty@tty1` is masked so a greetd
  restart cannot flash a login prompt, and the live/session scripts
  blank tty1 around the Hyprland handoff. (#13)
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
- Fixed the live ISO's plymouth splash never rendering on UEFI boots: the
  GRUB menu entries were missing `splash` on the kernel command line, so
  plymouthd silently fell back to text mode while the boot service still
  reported success. Added the same `quiet splash` parameter set already
  used on the BIOS/syslinux boot path. (#17)
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

### Known issues at release time

- The self-hosted CI runner's release-track workflows for several
  repos were found to have a real concurrency bug (a fixed, non-`mktemp`
  temp path gets wiped by a concurrent job's cleanup) during this
  release's promotion pass; tracked as ECO-28, not yet fixed upstream.
  Does not affect what shipped, since every promotion in this release
  was individually verified live before being pinned here.
