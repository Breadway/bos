# Keep ~/.local/bin on PATH for per-user tools. Arch already includes
# /usr/local/bin (where bakery desktop apps live on BOS). The Hyprland
# session resolves exec-once against the PATH it inherits from the login
# shell; Arch's stock /etc/profile does not add ~/.local/bin, so do it
# here for every login shell (live user and installed user alike).
case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) export PATH="$HOME/.local/bin:$PATH" ;;
esac
