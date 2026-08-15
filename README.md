# BOS — Bread Operating System

An Arch-based, Hyprland desktop distribution that ships the [bread
ecosystem](https://git.breadway.dev/Breadway) preconfigured. One Calamares install
produces a themed, bootable Wayland desktop — no manual Arch bootstrap, no
wiring up dotfiles, no per-tool bakery installs.

> This file is the product as the tree ships it. [DESIGN.md](DESIGN.md) is the
> original plan, kept as history — several of its sections (in-tree GTK
> bos-settings, bakery-at-post-install, A/B as if it were current) are not
> how the ISO works today.

## What you get

- **Compositor**: Hyprland with a native-Lua config (`hyprland.lua`), curated
  keybinds, snappy animations, blur, and pywal-driven colours on a black base.
- **bread ecosystem**, baked into `/usr/local` from bakery-managed binaries
  (no network needed at install time; per-user bakery state is seeded in
  `/etc/skel`): the `bread`/`breadd` automation daemon
  (`bread-emit` / `bread-module-host` when the stable bread release publishes
  them), `breadbar` (status bar + notifications), `breadbox` (launcher),
  `breadclip` (clipboard history), `breadcrumbs` (Wi-Fi profiles),
  `breadpad`/`breadman` (notes), `breadpaper` (wallpaper + theme),
  `breadsearch` (system search), `breadmon` (monitor layout TUI),
  `breadshot` (screenshots), `bread-theme` (the shared palette engine),
  `breadhelp` (onboarding + cheatsheet), `bos-settings` (control panel),
  and the `bakery` package manager. Most of those apps are zero-config on
  first boot; breadcrumbs networks are user-filled after install. See
  [below](#the-bread-ecosystem).
- **breadlock** (lock screen + greeter) is the one bread\* app that ships as
  **pacman**, not bakery — it needs a root-owned PAM service.
- **bos-settings**: a **Tauri 2 + Svelte** control panel (standalone bakery
  product, not a member of this repo). Configures every bread\* app
  non-destructively, plus snapshots, bakery/pacman updates, and day-to-day
  machine administration.
- **Login**: greetd + breadgreet (under `cage`) → Hyprland session.
- **Boot splash**: Plymouth `bos` theme (logo + spinner, black background).
- **Theming**: global dark across GTK3 (Adwaita-dark), GTK4/libadwaita
  (`color-scheme: prefer-dark`), and Qt (qt5ct/qt6ct Fusion dark); Papirus-Dark
  icons; Bibata cursor.
- **Apps**: kitty, nautilus (+ gvfs), Zen browser, VLC, loupe, gnome-text-editor,
  gnome-calculator, file-roller, with file associations wired in `mimeapps.list`.
  `yay` ships for AUR access beyond bakery + `[breadway]`.
- **Hardware**: pipewire audio, NetworkManager, BlueZ + blueman, CUPS printing
  with avahi mDNS discovery, TLP power management, fwupd firmware updates.
  Mesa only — **NVIDIA proprietary drivers are not included** and NVIDIA is
  unsupported out of the box (see [docs/hardware.md](docs/hardware.md)).
- **Resilience**: btrfs + snapper + snap-pac + grub-btrfs snapshots on every
  pacman transaction; zram swap; ufw firewall (deny-incoming, mDNS allowed).
  A/B root swapping is **not** implemented. Recovery is a grub-btrfs reboot,
  not `snapper rollback` (GRUB pins `rootflags=subvol=@`). See
  [docs/hardware.md](docs/hardware.md).
- **Security**: optional full-disk encryption is **LUKS1** (GRUB cannot unlock
  LUKS2 + Argon2id). Secure Boot is **self-signed Setup Mode only** via
  `sbctl` — not a Microsoft-signed shim; enrollment is skipped unless the
  firmware is already in Setup Mode.

## What ships vs what does not

| Channel | What |
|---------|------|
| **Bakery, required** | `bakery`, `bread` / `breadd`, `breadbar`, `breadbox` / `breadbox-sync`, `breadcrumbs`, `breadpad` / `breadman`, `breadpaper`, `bread-theme`, `breadmon`, `breadsearch` / `breadmill`, `breadclip` / `breadclipd`, `breadshot`, `bos-settings`, `breadhelp` (+ breadhelp content under `/usr/local/share/breadhelp/`) |
| **Bakery, optional** | `bread-emit`, `bread-module-host` — baked when the verified stable index publishes them; skipped (not a failed bake) until bread ships them |
| **pacman (`packages.x86_64`)** | `breadlock`, plus the rest of the distro (Hyprland, Calamares, Zen, …) |
| **Not shipped** | `breadcast`, `breadarr` |

The baked name list is [`iso/bread-lockfile.toml`](iso/bread-lockfile.toml)
(plus optional `[versions]` / `[[pin]]` so CI fetches
`https://dl.breadway.dev/<pkg>/<ver>/...`). `build-local.sh` fails if any
**required** binary is missing on the builder.

## Repo layout

This is an **ISO + Calamares + skel** repo. There is no Cargo workspace and
no `bos-settings/` member — bos-settings and breadhelp live in their own
repos and arrive via bakery.

```
bos/
├── iso/                           # archiso profile
│   ├── bread-lockfile.toml        # bakery bins + optional version pins
│   ├── profiledef.sh
│   ├── packages.x86_64            # live + installed pacman set
│   └── airootfs/                  # files overlaid onto the image
│       └── etc/
│           ├── skel/              # live user defaults (hypr, kitty, gtk, …)
│           └── calamares/         # installer config + post-install.sh
├── packaging/                     # in-house PKGBUILDs for AUR-only deps
│   ├── calamares/
│   ├── bibata/
│   ├── powerlevel10k/
│   └── yay-bin/
├── dotfiles/                      # STALE — not the live skel; see its README
├── scripts/
│   ├── ci-stage-bakery.py     # CI: minisign-verified index → $LAPTOP_HOME
│   ├── ci-verify-bake.sh      # CI: read-only checks before mkarchiso
│   └── smoke-test.sh
├── docs/
│   ├── hardware.md                # Mesa only, NVIDIA, grub-btrfs recovery
│   └── signed-repo.md             # future dl.breadway.dev/arch signing
├── .forgejo/workflows/            # CI: AUR republish + tagged ISO release
├── build-local.sh                 # native ISO build for this machine
├── README.md
└── DESIGN.md                      # historical plan
```

Live binds are `iso/airootfs/etc/skel/.config/hypr/binds.json` (`Super+L` →
`loginctl lock-session`, breadshot on `Super+Shift+S/C/P`, `Super+U`
breadpad). Do not treat `dotfiles/hypr/keybinds.conf` as current.

## Branches and remotes

Single-trunk: work on **`main`** via short-lived `feature/*` / `fix/*`
branches. **`stable`** is a marker branch CI fast-forwards to the latest
non-RC release tag — do not land work there by hand.

Dual remotes:

- **`origin`** — Forgejo (`ssh://git@100.66.238.26:2222/Breadway/bos.git`),
  authoritative
- **`github`** — GitHub (`https://github.com/Breadway/bos.git`) mirror

Push `origin` (and `github` when mirroring). Do not treat origin as GitHub.

## Building the ISO

`build-local.sh` builds the image natively (no container) and copies this
machine's bakery-installed bread binaries + breadhelp content from the
builder's `~/.local` into the image at `/usr/local` (bins, share/data,
desktop files, licenses) and `/usr/lib/systemd/user` (units). Per-user
bakery state (`installed.json` + index cache) is seeded in `/etc/skel`.
User units are `systemctl --global enable`'d so a later `useradd -m`
starts them on first login. BOS opts in via `/etc/bakery/config.toml`
(`prefix = "/usr/local"`); default bakery without that file is still
`~/.local`. Snapper `@` snapshots include `/usr/local`; recovery is
still grub-btrfs, not `snapper rollback`.

```sh
sudo ./build-local.sh              # release-quality (xz squashfs)
sudo FAST_BUILD=1 ./build-local.sh # fast dev iteration (zstd squashfs)
```

The ISO lands in `out/bos-<date>-x86_64.iso`. The script pins
`SOURCE_DATE_EPOCH` (reproducible UUIDs), rewrites the `[breadway]` repo URL
to the Tailscale-reachable Forgejo registry for the build, and **exits
non-zero** if any **required** lockfile binary (or breadhelp content) is
missing. Optional bins are skipped with a warning.

CI stages the builder from the **minisign-verified** stable bakery index
(`index.json` + `index.json.minisig`) and prefers lockfile `[versions]`
URLs (`https://dl.breadway.dev/<pkg>/<ver>/...`) when set, so two bakes
of the same commit fetch the same bits. Local builds still snapshot the
builder.

### Why some packages are in-house

`calamares`, `zen-browser-bin`, `bibata-cursor-theme`, and `yay-bin` are
AUR-only. BOS keeps a PKGBUILD for each under `packaging/` and republishes
the built package to the `[breadway]` repo via a Forgejo Actions workflow
(built on the hestia self-hosted runner, published with a scoped registry
token). `[breadway]` is **not** where bakery/breadbar/bos-settings live.

### Verifying a release

Every tagged release ISO on the [Forgejo releases
page](https://git.breadway.dev/Breadway/bos/releases) ships alongside a
`SHA256SUMS` file and a detached signature `SHA256SUMS.asc`, signed by a
dedicated release-signing key (not reused from anything else):

```
5620 3B86 A110 695A E7F3  1093 4AF3 323D 678E B5E2
```

The public half is committed at [`KEYS.asc`](KEYS.asc). That key signs
**ISO checksums only** — it does not sign the `[breadway]` pacman repo
(Forgejo's Arch registry has no pacman-compatible db signatures; that
section stays `SigLevel = Never` until a signed repo exists — see
[docs/signed-repo.md](docs/signed-repo.md)). To verify a download:

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

Post-install, `scripts/smoke-test.sh` (run as the installed user) checks
subvolumes, services, bakery bins on PATH, breadhelp content under
`/usr/local/share/breadhelp/content`, and that bakery user units are
`--global` enabled (or the preset / wants files exist).

## Second account

Bakery desktop apps live in `/usr/local` — shared, already on PATH. A later
account does **not** get a private copy of those binaries.

`/etc/default/useradd` keeps `SKEL=/etc/skel`. Stock `useradd -m` is enough:

```sh
sudo useradd -m alice
sudo passwd alice
```

- **Apps**: `/usr/local/bin` (and `/usr/local/share`) — already there.
- **Session files**: `useradd -m` copies `/etc/skel` (Hyprland, bread
  config, bakery `installed.json` + index cache) so first login has a
  session. Skel does not contain bakery binaries.
- **Daemons**: `breadd`, `breadbox-sync`, `breadclipd`, `breadcrumbs`,
  `breadmill`, … are `systemctl --global enable`'d at install (and on
  the live image). Creating a user starts them on first login.
- **Login**: greetd/breadgreet lists any local user with a login shell
  (`SHELL=/usr/bin/zsh` is the useradd default).

`breadclipd` is WantedBy=`graphical-session.target`. BOS does not activate
that target (no uwsm), so Hyprland still `systemctl --user start`s it after
the compositor is up. `--global enable` still records it for every account.

Rollback is still the GRUB snapshots submenu (grub-btrfs), not
`snapper rollback`. `/usr/local` rides the `@` snapshot.

## bos-settings

Standalone bakery product: **Tauri 2 + Svelte**, not GTK4, and not built
from this repo. Install/update with `bakery`; the ISO just bakes whatever
binary the builder has.

It aims for GNOME-Settings-style parity: live system state and control, so
day-to-day administration doesn't require a terminal. Bread-ecosystem
configs are edited **non-destructively** (comments and unmodeled keys stay).
Panels with a daemon (bread, breadbox, breadcrumbs, breadsearch, breadclip)
also get live systemd status + Start/Stop/Restart/Logs.

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
| Snapshots | `snapper` list (number / date / description); reboot to pick in GRUB (grub-btrfs); delete |

Source and build live in the [bos-settings](https://git.breadway.dev/Breadway/bos-settings)
repo, not here.

## The bread ecosystem

Everything below is a separate bakery-distributed project with its own repo
and release cadence, baked into `/usr/local` at ISO build time so a fresh
install has them all with no network round-trip. Some ship more than one
binary from a single package — that's noted where it applies. Most have a
corresponding **bos-settings** panel; this table is about *using* the app
directly.

**Desktop shell**

| Tool | Role | Launch |
|------|------|--------|
| `bread` / `breadd` | Reactive automation daemon — normalises hardware/compositor/power/network signals into events dispatched to Lua modules (`~/.config/bread/`). `bread-emit` is the fire-and-forget helper hooks/CLIs use; `bread-module-host` is the sandboxed out-of-process module runtime breadd spawns. Both extra bins are **optional** on the ISO until a stable bread release publishes them. | runs at login (`breadd.service`) |
| `breadbar` | Top status bar: workspaces, clock, system stats, tray, **and** the notification daemon — one process, not two | runs at login |
| `breadbox` | Application launcher (fuzzy search, per-context results via `breadbox-sync`) | `SUPER+Space` |
| `breadlock` | Idle lock screen. Also provides `breadgreet`, the login greeter hosted under `cage` via greetd — same project, two binaries, one visual identity from login to lock. **pacman**, not bakery. | `SUPER+L` (via `loginctl lock-session`, picked up by `hypridle`); `breadgreet` runs automatically at boot |
| `bread-theme` | The shared palette engine every bread app renders through: fixed dark base colors, with only the accent slots following the current wallpaper's pywal palette. `bread-theme generate` regenerates the stylesheet; hyprland.lua calls it automatically on wallpaper change. | invoked automatically, rarely run by hand |

**Productivity**

| Tool | Role | Launch |
|------|------|--------|
| `breadpad` | Quick-capture scratchpad/notes popup with AI classification and optional CalDAV calendar sync | `SUPER+U` |
| `breadman` | The fuller notes manager view (browse/organize) — ships from the same `breadpad` package as a second binary | `SUPER+M` |
| `breadclip` | Clipboard history. `breadclipd` is the background daemon that actually records history; `breadclip` is the GTK4 popup that browses it | `SUPER+V` / `SUPER+Shift+V` |
| `breadsearch` | Semantic system-wide search (indexes files/notes, embeds locally — CPU/ROCm/CUDA backend configurable). `breadmill` is its indexing daemon. | via breadbox, or BOS Settings → File Search |
| `breadhelp` | Onboarding + in-session help/cheatsheet. Content lives at `/usr/local/share/breadhelp/content` (bakery `content.tar.gz`, baked into the image). | `SUPER+/` |

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
| `bos-settings` | Unified Tauri 2 + Svelte control panel: live system state + control (network, power, firewall, users, packages, firmware, AUR, snapshots) plus non-destructive config editing for every app above | `SUPER+,` |

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

See [docs/hardware.md](docs/hardware.md) (GPUs, NVIDIA, recovery) and
[docs/signed-repo.md](docs/signed-repo.md) (`[breadway]` stays unsigned
until `dl.breadway.dev/arch` exists).

- **GPUs**: ships the generic Mesa stack — AMD and Intel work out of the box.
  NVIDIA is **unsupported** (no proprietary driver, no NVIDIA firmware). See
  [docs/hardware.md](docs/hardware.md).
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
  btrfs subvolume layout the installer creates. Recovery is the GRUB
  snapshots submenu, not `snapper rollback` — [docs/hardware.md](docs/hardware.md).
- **`[breadway]` signatures**: `SigLevel = Never` until a signed repo is
  stood up at `dl.breadway.dev/arch`. See [docs/signed-repo.md](docs/signed-repo.md).

## Recovery

**An update broke something (system still boots):** reboot → **GRUB
“snapshots” submenu** (grub-btrfs), then boot that entry.

BOS Settings → Snapshots lists each snapshot’s number, date, and
description so you know which GRUB entry to pick. It does not roll the
running root back in place.

Do **not** run `snapper rollback`. BOS GRUB pins `rootflags=subvol=@`, so
a snapper-swapped default subvolume is not what the installed grub.cfg
will boot next. Details: [docs/hardware.md](docs/hardware.md).

A/B root swapping (SteamOS-style) is a **future** idea in DESIGN.md — it is
not shipped.

**The system won't boot (broken GRUB / lost EFI entry):**

1. Boot the BOS ISO and open a terminal (`SUPER+Return`).
2. Run `sudo bos-rescue`. It finds the installed btrfs `@` and the ESP,
   prints the devices it will use, and asks `YES` before writing. It can
   `arch-chroot` and/or reinstall GRUB with the same sequence the
   installer uses (NVRAM + `--removable` + `grub-mkconfig`).
3. Manual equivalent, if you would rather type it:
   ```sh
   mount -o subvol=@ /dev/sdXN /mnt
   mount /dev/sdXP /mnt/boot/efi      # the EFI partition
   arch-chroot /mnt
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
