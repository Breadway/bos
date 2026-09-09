# `dotfiles/` is not the live skel

These files are a leftover from an earlier design (Hyprland `.conf` binds).
They are **not** copied into the ISO or the installed system.

`dotfiles/hypr/keybinds.conf` still mentions `grimblast`. That is not the
screenshot tool BOS ships; live binds use **breadshot**
(`iso/airootfs/etc/skel/.config/hypr/binds.json`). Do not treat grimblast
as current.

Live user defaults live in [`iso/airootfs/etc/skel`](../iso/airootfs/etc/skel)
(`hyprland.lua` + `binds.json`, breadlock/`loginctl lock-session`, breadshot,
breadpad, …). Edit that tree.
