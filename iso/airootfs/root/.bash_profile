# Auto-start Hyprland on tty1 in the live session
if [[ "$(tty)" == "/dev/tty1" ]] && [[ -z "$WAYLAND_DISPLAY" ]]; then
    exec Hyprland
fi
