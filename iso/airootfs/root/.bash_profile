# Auto-start Hyprland on tty1 in the live session
if [[ "$(tty)" == "/dev/tty1" ]] && [[ -z "$WAYLAND_DISPLAY" ]]; then
    # Allow a software-rendering fallback so the live session comes up even
    # without a GPU (VMs, headless, exotic hardware). On real hardware wlroots
    # still selects the hardware renderer; this only permits llvmpipe when no
    # GPU renderer is available. Must be exported before Hyprland starts —
    # wlroots reads it at renderer init, earlier than any Hyprland `env=` line.
    export WLR_RENDERER_ALLOW_SOFTWARE=1
    # Software cursors: hardware-cursor planes are often unusable in VMs and
    # show as invisible/garbled; this is the reliable choice for a live medium.
    export WLR_NO_HARDWARE_CURSORS=1
    exec Hyprland
fi
