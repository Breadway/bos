# Put the per-user bakery bin dir on PATH. The bread ecosystem (breadd, breadbar,
# breadbox, …) is installed there by bakery, and the Hyprland session launches
# them via `exec-once`, which resolves against the PATH it inherits from the
# login shell. Arch's stock /etc/profile does not add ~/.local/bin, so do it here
# for every login shell (live user and installed user alike).
case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) export PATH="$HOME/.local/bin:$PATH" ;;
esac
