# `dotfiles/` is not the live skel

These files are a leftover from an earlier design (Hyprland `.conf` binds,
including grimblast). They are **not** copied into the ISO or the installed
system.

Live user defaults live in [`iso/airootfs/etc/skel`](../iso/airootfs/etc/skel)
(`hyprland.lua` + `binds.json`, breadlock/`loginctl lock-session`, breadshot,
breadpad, …). Edit that tree.
