---
name: project-bos-patterns
description: Recurring patterns and hotspots to watch in the BOS repo across audits
metadata:
  type: project
---

BOS is a rolling Arch Linux OS project combining an archiso installer profile, Calamares configuration, default dotfiles, and a GTK4 Rust settings app (bos-settings).

**Why:** This is a scaffold-originated repo; the initial code was written in one pass and has several recurring anti-patterns worth watching on future audits.

**How to apply:** Check these areas first on any future audit pass.

## Recurring hotspots

- **config_dir() + /home/user fallback**: Multiple views (breadbar, hyprland, packages) originally inlined `std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string())` instead of calling `config::config_dir()`. Watch for new views duplicating this.
- **branding.desc YAML**: The `sidebarTextHighlight` key had no space after the colon, causing a hard YAML parse error. Future edits to this file should be checked with `python3 -c "import yaml; yaml.safe_load(open('branding.desc'))"`.
- **Branding images missing**: `logo.png` and `languages.png` are referenced in branding.desc but do not exist in the repo. These must be created before any ISO build attempt.
- **state.rs is dead code**: The file exists at `bos-settings/src/state.rs` but is not declared as `mod state` in `main.rs`. It is silently ignored by cargo. Either wire it in or delete it.
- **pacman.conf TODO**: `[breadway]` repo URL in `iso/pacman.conf` has a TODO comment — must be a live repo before the ISO can be built.
- **eprintln! in production paths**: packages.rs had `eprintln!("bakery update failed: {e}")` inside a GTK callback. GTK apps don't have useful stderr at runtime. Convert to silent handling or surface errors through the UI.
- **can-you-begin-a-composed-beacon.md**: AI-generated random filename for the project plan doc. Should be renamed to PLAN.md before the repo is shared publicly.
- **skel == dotfiles duplication**: `iso/airootfs/etc/skel/.config/` and `dotfiles/` are byte-for-byte identical. This is by design for now, but risks divergence. The plan calls for post-install.sh to copy skel into user home — skel is the authoritative copy; dotfiles/ is redundant scaffolding. Flag in future audits if they diverge.

## Key architectural constraints

- Calamares shellprocess runs inside chroot (`dont-chroot: false`). `systemctl enable` works, `systemctl start` does not.
- `MAIN_USER` is derived via `getent passwd 1000` — safe only after Calamares `users` step which precedes `shellprocess` in settings.conf.
- The polkit rule grants wheel-group members passwordless `snapper rollback` via pkexec — intentional per design.
- The live session autologs as root (not a "liveuser") — standard archiso releng behavior, not a bug.
