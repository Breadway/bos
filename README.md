# BOS — Bread Operating System

An Arch-based, Hyprland desktop distribution that ships the [bread
ecosystem](https://git.breadway.dev/Breadway) preconfigured. One Calamares install
produces a themed, bootable Wayland desktop — no manual Arch bootstrap, no
wiring up dotfiles, no per-tool bakery installs.

> Design rationale and the btrfs/A-B roadmap live in [DESIGN.md](DESIGN.md).
> This file is the practical overview: what's in the image, how to build it,
> and how to test it.

## What you get

- **Compositor**: Hyprland with a native-Lua config (`hyprland.lua`), curated
  keybinds, snappy animations, blur, and pywal-driven colours on a black base.
- **bread ecosystem**, baked into `/etc/skel` from bakery-managed binaries
  (no network needed at install time): the `bread`/`breadd` automation daemon,
  `breadbar` (status bar + notifications), `breadbox` (launcher), `breadclip`
  (clipboard history), `breadcrumbs` (Wi-Fi profiles), `breadpad`/`breadman`
  (notes), `breadpaper` (wallpaper + theme), `breadsearch` (system search),
  `breadmon` (monitor layout TUI), `breadshot` (screenshots), `bread-theme`
  (the shared palette engine), and the `bakery` package manager. `breadlock`
  (lock screen + greeter) ships as its own pacman package alongside
  `bos-settings`, not through bakery. See [below](#the-bread-ecosystem) for
  what each one actually does.
- **bos-settings**: a GTK4 control panel that configures every bread\* app's
  config from a GUI (non-destructively), plus snapshot rollback and bakery
  updates. See below.
- **Login**: greetd + breadgreet (bread-ecosystem's own greeter, under `cage`)
  → Hyprland session.
- **Boot splash**: Plymouth `bos` theme (logo + spinner, black background).
- **Theming**: global dark across GTK3 (Adwaita-dark), GTK4/libadwaita
  (`color-scheme: prefer-dark`), and Qt (qt5ct/qt6ct Fusion dark); Papirus-Dark
  icons; Bibata cursor.
- **Apps**: kitty, nautilus (+ gvfs), Zen browser, VLC, loupe, gnome-text-editor,
  gnome-calculator, file-roller, with file associations wired in `mimeapps.list`.
  `yay` ships for AUR access beyond bakery's bread ecosystem + `[breadway]`.
- **Hardware**: pipewire audio, NetworkManager, BlueZ + blueman, CUPS printing
  with avahi mDNS discovery, TLP power management, fwupd firmware updates.
- **Resilience**: btrfs + snapper + snap-pac + grub-btrfs snapshots on every
  pacman transaction; zram swap; ufw firewall (deny-incoming, mDNS allowed).
- **Security**: full-disk encryption (LUKS, via Calamares' built-in support —
  cryptsetup + the matching mkinitcpio/GRUB wiring ship so an encrypted
  install actually boots) and self-signed Secure Boot (via `sbctl`, enrolled
  automatically at install time when the firmware is in Setup Mode).

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

`calamares`, `zen-browser-bin`, `bibata-cursor-theme`, and `yay-bin` are
AUR-only. BOS keeps a PKGBUILD for each under `packaging/` and republishes the
built package to the `[breadway]` repo via a Forgejo Actions workflow (built
on the hestia self-hosted runner, published with a scoped registry token).
`bos-settings` itself publishes the same way on a `v*` tag.

### Verifying a release

Every tagged release ISO on the [Forgejo releases
page](https://git.breadway.dev/Breadway/bos/releases) ships alongside a
`SHA256SUMS` file and a detached signature `SHA256SUMS.asc`, signed by a
dedicated release-signing key (not reused from anything else):

```
5620 3B86 A110 695A E7F3  1093 4AF3 323D 678E B5E2
```

The public half is committed at [`KEYS.asc`](KEYS.asc). To verify a download:

```sh
gpg --import KEYS.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
```

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

A GTK4 settings app aiming for GNOME-Settings-style parity: not just editing
config files, but live system state and control, so day-to-day machine
administration doesn't require a terminal.

Bread-ecosystem TOML configs are edited **non-destructively**: `toml_edit`
parses the file, changes only the keys a view exposes, and writes it back —
preserving comments and any keys the UI doesn't model (calendar passwords,
saved-network passwords, model paths). Panels with a daemon behind them
(bread, breadbox, breadcrumbs, breadsearch, breadclip) also get live
systemd status + Start/Stop/Restart/Logs via a shared `service_control`
widget, not just the config file.

| Panel | What it does |
|-------|--------------|
| About | System info (OS/kernel/CPU/GPU/memory/disk/uptime) + hostname |
| Network | Wi-Fi scan/connect, Ethernet status, radio toggle |
| Wi-Fi Profiles (breadcrumbs) | `breadcrumbs.toml` — settings, saved networks, profiles |
| Firewall | ufw rules: enable/disable, add/remove, view active rules |
| Sound | PipeWire output/input device + volume via `pactl` |
| Power | Battery status/health, brightness, charge limits (hardware-dependent), TLP profile (read-only) |
| Date & Time | Timezone, NTP sync toggle |
| Display (Hyprland) | Connected monitors + open `hyprland.lua` in editor |
| Users | Add/remove accounts, change passwords |
| Wallpaper (breadpaper) | Set wallpaper, drives the pywal-derived accent palette |
| Bar (breadbar) | `breadbar/style.css` override, live-reloads on save |
| Launcher (breadbox) | `breadbox/config.toml` — launcher contexts |
| Clipboard (breadclip) | breadclipd service control + "open history" |
| Notes (breadpad) | `breadpad/breadpad.toml` — settings, model + ollama, reminders, calendar |
| File Search (breadsearch) | `breadsearch/config.toml` — index/search/model + breadmill service |
| Daemon (bread) | `breadd.toml` — daemon, lua, modules, adapters, events, notifications |
| Packages | `bakery` installed list + updates, pacman system update |
| AUR | Search via `yay`; installing opens a terminal (AUR build scripts need review) |
| Firmware | `fwupd` device list + updates |
| Snapshots | `snapper` list / boot-into (grub-btrfs) / delete |

Build standalone:

```sh
cargo build --release -p bos-settings
cargo test -p bos-settings        # includes config round-trip tests
```

## The bread ecosystem

Everything below is a separate bakery-distributed project with its own repo
and release cadence, baked into `/etc/skel` at ISO build time so a fresh
install has them all with no network round-trip. Some ship more than one
binary from a single package — that's noted where it applies. Most have a
corresponding **bos-settings** panel for configuration; this table is about
*using* the app directly.

**Desktop shell**

| Tool | Role | Launch |
|------|------|--------|
| `bread` / `breadd` | Reactive automation daemon — normalises hardware/compositor/power/network signals into events dispatched to Lua modules (`~/.config/bread/`). Everything else in the ecosystem can subscribe to its events. | runs at login (`breadd.service`) |
| `breadbar` | Top status bar: workspaces, clock, system stats, tray, **and** the notification daemon — one process, not two | runs at login |
| `breadbox` | Application launcher (fuzzy search, per-context results via `breadbox-sync`) | `SUPER+Space` |
| `breadlock` | Idle lock screen. Also provides `breadgreet`, the login greeter hosted under `cage` via greetd — same project, two binaries, one visual identity from login to lock | `SUPER+L` (via `loginctl lock-session`, picked up by `hypridle`); `breadgreet` runs automatically at boot |
| `bread-theme` | The shared palette engine every bread app renders through: fixed dark base colors, with only the accent slots following the current wallpaper's pywal palette. `bread-theme generate` regenerates the stylesheet; hyprland.lua calls it automatically on wallpaper change. | invoked automatically, rarely run by hand |

**Productivity**

| Tool | Role | Launch |
|------|------|--------|
| `breadpad` | Quick-capture scratchpad/notes popup with AI classification and optional CalDAV calendar sync | `SUPER+U` |
| `breadman` | The fuller notes manager view (browse/organize) — ships from the same `breadpad` package as a second binary | `SUPER+M` |
| `breadclip` | Clipboard history. `breadclipd` is the background daemon that actually records history; `breadclip` is the GTK4 popup that browses it | `SUPER+V` / `SUPER+Shift+V` |
| `breadsearch` | Semantic system-wide search (indexes files/notes, embeds locally — CPU/ROCm/CUDA backend configurable). `breadmill` is its indexing daemon. | via breadbox, or BOS Settings → File Search |

**System**

| Tool | Role | Launch |
|------|------|--------|
| `breadcrumbs` | Location-aware Wi-Fi profile state machine, with optional Tailscale integration — switches network behavior based on which saved network you're on | CLI, or BOS Settings → Wi-Fi Profiles |
| `breadpaper` | Wallpaper manager — sets the wallpaper via `awww`, generates the pywal accent palette from it, and reloads every bread-theme app | BOS Settings → Wallpaper |
| `breadmon` | TUI monitor layout manager (resolution/position/scaling) — the interactive counterpart to BOS Settings' read-only Display panel | `breadmon` in a terminal |
| `breadshot` | Screenshot utility wrapping `grim`/`slurp`/`wl-copy` with Hyprland-aware geometry (multi-monitor-safe region select) | `breadshot`, or `SUPER+Shift+S/C/P` |

**Tooling**

| Tool | Role | Launch |
|------|------|--------|
| `bakery` | CLI package manager for the whole ecosystem — install/update/list, tracks installed binaries + versions independently of pacman | `bakery` |
| `bos-settings` | Unified GTK4 control panel: live system state + control (network, power, firewall, users, packages, firmware, AUR, snapshots) plus non-destructive config editing for every app above | `SUPER+,` |

## Keyboard shortcuts

`SUPER` is the Windows/Cmd key. Press **`SUPER+/`** at any time for this
cheatsheet in-session; first boot shows a short welcome (once).

| Keys | Action |
|------|--------|
| `SUPER+Return` | Terminal (kitty) |
| `SUPER+Space` | App launcher (breadbox) |
| `SUPER+E` / `SUPER+B` | Files (nautilus) / Browser (zen) |
| `SUPER+U` / `SUPER+M` | Notes (breadpad) / notes manager (breadman) |
| `SUPER+,` / `SUPER+/` | BOS Settings / keybind cheatsheet |
| `SUPER+L` / `SUPER+N` | Lock / log out |
| `SUPER+Backspace` | Close window |
| `SUPER+F` | Fullscreen |
| `SUPER+I` | Toggle floating |
| `SUPER+P` | Toggle pseudotile |
| `SUPER+R` | Resize mode |
| `SUPER+T` | Toggle split direction |
| `SUPER+V` / `SUPER+Shift+V` | Clipboard history (breadclip) |
| `SUPER+Tab` | Last window |
| `SUPER+Shift+S/C/P` | Screenshot region→file / region→clipboard / screen→file |
| `SUPER+arrows` | Move focus |
| `SUPER+Shift+h/j/k/l` | Move window |
| `SUPER+Shift+arrows` | Resize window |
| `SUPER+1..0` | Switch to workspace 1–10 |
| `SUPER+Shift+1..0` | Move window to workspace |
| `SUPER+[ / ]` | Previous / next workspace |
| `SUPER+Shift+[ / ]` | Move window to previous / next workspace |
| `SUPER+scroll` | Cycle workspaces |
| `SUPER+left/right-drag` | Move / resize window with the mouse |
| Volume / brightness / play-pause / next / prev | Media keys — work even on the lock screen |
| Calculator key | Opens gnome-calculator |

## Known limitations

- **GPUs**: ships the generic Mesa stack — AMD and Intel work out of the box.
  The **NVIDIA proprietary driver is not included**; NVIDIA users must install
  `nvidia`/`nvidia-utils` and set the usual Hyprland env vars after install.
- **Virtual machines**: Hyprland needs GPU acceleration to be smooth. Use
  `virtio-vga-gl` + `-display gtk,gl=on` (virgl); plain software rendering is
  noticeably laggy.
- **Wayland-first**: X11-only apps run through XWayland; a few may misbehave.
- **Secure Boot**: self-signed only, via `sbctl` — BOS can't ship a
  Microsoft-signed shim (that needs going through Microsoft's own paid UEFI
  CA process). Post-install enrolls BOS's own keys automatically, but only
  when the firmware is already in Setup Mode (no vendor keys installed yet);
  otherwise it's skipped and you can run
  `sudo sbctl enroll-keys --microsoft && sudo sbctl sign-all -g` yourself
  later (after clearing your firmware's existing keys, if any). The installer
  writes both an NVRAM entry and the removable `EFI/BOOT/BOOTX64.EFI` fallback
  either way.
- **Disk encryption**: full-disk LUKS is available on the installer's "Erase
  disk" page (Calamares' own checkbox) and on manually-created partitions —
  BOS ships the matching `cryptsetup`/mkinitcpio/GRUB wiring so an encrypted
  install actually boots (LUKS1, since GRUB doesn't support LUKS2 + Argon2id).
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
