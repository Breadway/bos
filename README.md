# BOS — Bread Operating System

An Arch-based, Hyprland desktop distribution that ships the [bread
ecosystem](https://github.com/Breadway) preconfigured. One Calamares install
produces a themed, bootable Wayland desktop — no manual Arch bootstrap, no
wiring up dotfiles, no per-tool bakery installs.

> Design rationale and the btrfs/A-B roadmap live in [DESIGN.md](DESIGN.md).
> This file is the practical overview: what's in the image, how to build it,
> and how to test it.

## What you get

- **Compositor**: Hyprland with a native-Lua config (`hyprland.lua`), curated
  keybinds, snappy animations, blur, and pywal-driven colours on a black base.
- **bread ecosystem**, baked into `/etc/skel` from bakery-managed binaries
  (no network needed at install time): `bread`/`breadd`, `breadbar` (status bar
  + notification daemon), `breadbox` (launcher), `breadcrumbs` (Wi-Fi profiles),
  `breadpad` (notes/reminders), `breadman`, and the `bakery` package manager.
- **bos-settings**: a GTK4 control panel that configures every bread\* app's
  config from a GUI (non-destructively), plus snapshot rollback and bakery
  updates. See below.
- **Login**: greetd + tuigreet → Hyprland session.
- **Boot splash**: Plymouth `bos` theme (logo + spinner, black background).
- **Theming**: global dark across GTK3 (Adwaita-dark), GTK4/libadwaita
  (`color-scheme: prefer-dark`), and Qt (qt5ct/qt6ct Fusion dark); Papirus-Dark
  icons; Bibata cursor.
- **Apps**: kitty, nautilus (+ gvfs), Zen browser, VLC, loupe, gnome-text-editor,
  gnome-calculator, file-roller, with file associations wired in `mimeapps.list`.
- **Hardware**: pipewire audio, NetworkManager, BlueZ + blueman, CUPS printing
  with avahi mDNS discovery, TLP power management, fwupd firmware updates.
- **Resilience**: btrfs + snapper + snap-pac + grub-btrfs snapshots on every
  pacman transaction; zram swap; ufw firewall (deny-incoming, mDNS allowed).

## Repo layout

```
bos/
├── Cargo.toml                     # workspace (members: bos-settings)
├── bos-settings/                  # GTK4 unified settings app (Rust)
│   └── src/
│       ├── config/mod.rs          # non-destructive toml_edit config layer
│       └── ui/{widgets,window,sidebar}.rs, ui/views/*.rs
├── iso/                           # archiso profile
│   ├── profiledef.sh
│   ├── packages.x86_64            # live + installed package set
│   └── airootfs/                  # files overlaid onto the image
│       └── etc/
│           ├── skel/              # default user dotfiles (hypr, kitty, gtk, …)
│           └── calamares/         # installer config + post-install.sh
├── packaging/                     # in-house PKGBUILDs for AUR-only deps
│   ├── arch/                      # bos-settings
│   ├── calamares/
│   └── bibata/
├── .forgejo/workflows/            # CI: build + publish packages to [breadway]
├── build-local.sh                 # native ISO build for this machine
└── DESIGN.md
```

## Building the ISO

`build-local.sh` builds the image natively (no container) and bakes this
machine's bakery-installed bread binaries into `/etc/skel`:

```sh
sudo ./build-local.sh              # release-quality (xz squashfs)
sudo FAST_BUILD=1 ./build-local.sh # fast dev iteration (zstd squashfs)
```

The ISO lands in `out/bos-<date>-x86_64.iso`. The script pins
`SOURCE_DATE_EPOCH` (reproducible UUIDs) and rewrites the `[breadway]` repo URL
to the Tailscale-reachable Forgejo registry for the build.

### Why some packages are in-house

`calamares`, `zen-browser-bin`, and `bibata-cursor-theme` are AUR-only. BOS
keeps a PKGBUILD for each under `packaging/` and republishes the built package
to the `[breadway]` repo via a Forgejo Actions workflow (built on the hestia
self-hosted runner, published with a scoped registry token). `bos-settings`
itself publishes the same way on a `v*` tag.

## Testing in a VM

A reusable, GPU-accelerated launcher lives at `~/bos-vm/run.sh`:

```sh
~/bos-vm/run.sh install   # boot the ISO installer (target disk attached)
~/bos-vm/run.sh           # boot the installed system from the disk
```

It uses KVM + `-cpu host`, 8 GiB / 8 vCPU, and `virtio-vga-gl` with
`-display gtk,gl=on` (virgl) — 3D acceleration is essential for a smooth
Hyprland session in QEMU. The disk lives on NVMe (not the tmpfs `/tmp`) to
avoid memory pressure.

## bos-settings

`bos-settings` edits each bread\* app's TOML **non-destructively**: it parses
the file with `toml_edit`, changes only the keys a view exposes, and writes it
back — preserving comments and any keys the UI doesn't model (calendar
passwords, saved-network passwords, model paths). Views:

| View | Config |
|------|--------|
| bread | `bread/breadd.toml` — daemon, lua, modules, all adapters, events, notifications |
| breadbar | `breadbar/style.css` override |
| breadbox | `breadbox/config.toml` — launcher contexts |
| breadcrumbs | `breadcrumbs/breadcrumbs.toml` — settings, saved networks, profiles |
| breadpad | `breadpad/breadpad.toml` — settings, model + ollama, reminders, calendar |
| Snapshots | `snapper` list / rollback / delete |
| Packages | `bakery` installed list + updates |
| Hyprland | open config in editor + monitor list |

Build standalone:

```sh
cargo build --release -p bos-settings
cargo test -p bos-settings        # includes config round-trip tests
```

## The bread ecosystem at a glance

| Tool | Role | Launch |
|------|------|--------|
| `bread` / `breadd` | Reactive automation daemon — normalises hardware/compositor signals into events dispatched to Lua modules | runs at login |
| `breadbar` | Top status bar (workspaces, clock, stats, tray) **and** the notification daemon | runs at login |
| `breadbox` | Application launcher | `SUPER+Space` |
| `breadpad` | Notes & reminders (AI-classified, optional CalDAV sync) | `SUPER+U` |
| `breadman` | Package-manager UI | `SUPER+M` |
| `breadcrumbs` | Wi-Fi profile state machine (location-aware) | CLI / BOS Settings |
| `bakery` | CLI package manager for the ecosystem | `bakery` |
| `bos-settings` | Unified GTK4 control panel for all of the above + snapshots + updates | `SUPER+,` |

## Keyboard shortcuts

`SUPER` is the Windows/Cmd key. Press **`SUPER+/`** at any time for this
cheatsheet in-session; first boot shows a short welcome (once).

| Keys | Action |
|------|--------|
| `SUPER+Return` | Terminal (kitty) |
| `SUPER+Space` | App launcher (breadbox) |
| `SUPER+E` / `SUPER+B` | Files (nautilus) / Browser (zen) |
| `SUPER+U` / `SUPER+M` | breadpad / breadman |
| `SUPER+,` / `SUPER+/` | BOS Settings / keybind cheatsheet |
| `SUPER+L` / `SUPER+N` | Lock / log out |
| `SUPER+Backspace` | Close window |
| `SUPER+F` / `SUPER+V` / `SUPER+T` | Fullscreen / float / toggle split |
| `SUPER+Shift+V` | Clipboard history |
| `SUPER+Tab` | Last window |
| `SUPER+Shift+S/C/P` | Screenshot region→file / region→clipboard / screen→file |
| `SUPER+arrows` | Move focus |
| `SUPER+Shift+h/j/k/l` | Move window |
| `SUPER+Shift+arrows` | Resize window |
| `SUPER+1..0` | Switch to workspace 1–10 |
| `SUPER+Shift+1..0` | Move window to workspace |
| `SUPER+[ / ]` | Previous / next workspace |
| `SUPER+left/right-drag` | Move / resize window with the mouse |

## Known limitations

- **GPUs**: ships the generic Mesa stack — AMD and Intel work out of the box.
  The **NVIDIA proprietary driver is not included**; NVIDIA users must install
  `nvidia`/`nvidia-utils` and set the usual Hyprland env vars after install.
- **Virtual machines**: Hyprland needs GPU acceleration to be smooth. Use
  `virtio-vga-gl` + `-display gtk,gl=on` (virgl); plain software rendering is
  noticeably laggy.
- **Wayland-first**: X11-only apps run through XWayland; a few may misbehave.
- **Secure Boot**: not configured. Boot with Secure Boot disabled, or enroll
  your own keys. The installer writes both an NVRAM entry and the removable
  `EFI/BOOT/BOOTX64.EFI` fallback.
- **Snapshots assume btrfs**: the snapper/grub-btrfs tooling expects the default
  btrfs subvolume layout the installer creates.

## Recovery

**An update broke something (system still boots):** open BOS Settings →
Snapshots and roll back, or pick a pre-update snapshot from the **GRUB
“snapshots” submenu** at boot, then run `snapper rollback` from the booted
snapshot.

**The system won't boot (broken GRUB / lost EFI entry):**

1. Boot the BOS ISO and open a terminal (`SUPER+Return`).
2. Mount the installed root and EFI, then chroot:
   ```sh
   mount -o subvol=@ /dev/sdXN /mnt
   mount /dev/sdXP /mnt/boot/efi      # the EFI partition
   arch-chroot /mnt
   ```
3. Reinstall the bootloader (the same sequence the installer uses):
   ```sh
   grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=BOS --recheck
   grub-install --target=x86_64-efi --efi-directory=/boot/efi --removable --recheck
   grub-mkconfig -o /boot/grub/grub.cfg
   ```

**Firmware shows “no boot device”:** select `EFI/BOOT/BOOTX64.EFI` from the
firmware boot menu — the installer always writes that removable fallback.

## Boot architecture notes

archiso keeps the kernel and initramfs outside the squashfs, so the installer
stages them explicitly: a `shellprocess@kernel` step copies the kernel + ucode
into the target `/boot` and writes a stock mkinitcpio preset before the native
`initcpio` module builds the initramfs. GRUB is **not** installed by Calamares'
`bootloader`/`grubcfg` modules (they leave the ESP empty in this layout) —
`post-install.sh` runs `grub-install` (NVRAM **and** `--removable`) +
`grub-mkconfig` instead, which is the sequence verified to boot.
