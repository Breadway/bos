# BOS — Bread Operating System Plan

## Context

The bread ecosystem (bread, breadbar, breadbox, breadcrumbs, breadpad/breadman, bakery) is a cohesive set of Arch/Hyprland-specific tools with a shared theme system, unified package manager, and consistent config conventions. Currently, getting to a working system requires installing Arch, Hyprland, each tool via bakery, and wiring up dotfiles manually. BOS eliminates that — one ISO install produces a fully working desktop with everything preconfigured.

Goals:
- **Install and be done**: Calamares GUI installer → reboot → working Hyprland + full bread stack
- **Rollback safety**: Btrfs subvolumes + snapper + snap-pac; every pacman transaction is snapshotted
- **Unified config**: `bos-settings` GTK4 app surfaces all app configs + snapshot management + bakery updates
- **Future-compatible**: Btrfs layout is designed to allow A/B partition migration later (SteamOS model)

---

## Repo Structure

Single new repo: `Breadway/bos` — a Cargo workspace.

```
bos/
├── Cargo.toml                  # Workspace (members: [bos-settings])
├── bos-settings/               # GTK4 unified settings app
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── state.rs
│       ├── theme.rs
│       ├── ui/
│       │   ├── window.rs       # Sidebar + content shell (port breadman pattern)
│       │   ├── sidebar.rs
│       │   └── views/
│       │       ├── bread.rs
│       │       ├── breadbar.rs
│       │       ├── breadbox.rs
│       │       ├── breadcrumbs.rs
│       │       ├── breadpad.rs
│       │       ├── snapshots.rs
│       │       ├── packages.rs
│       │       └── hyprland.rs
│       └── config/
│           └── mod.rs          # Per-app config loaders
├── iso/                        # archiso profile
│   ├── profiledef.sh
│   ├── packages.x86_64         # Live ISO + installed system package list
│   ├── airootfs/               # Files overlaid onto live ISO root
│   │   └── etc/
│   │       ├── calamares/      # Calamares YAML configuration
│   │       └── skel/           # Default user dotfiles
└── dotfiles/                   # Default configs deployed at install time
    ├── hyprland/               # hyprland.conf, keybinds, autostart
    ├── bread/                  # breadd.toml, init.lua, devices.lua
    ├── breadbar/               # (no config needed; zero-config by default)
    ├── breadbox/               # config.toml with default context priorities
    └── breadcrumbs/            # breadcrumbs.toml with default home profile
```

---

## Component 1: Btrfs Layout + Snapshot Infrastructure

### Partition/subvolume layout (set up by Calamares)

| Subvolume | Mount point | Notes |
|-----------|-------------|-------|
| `@` | `/` | Root — snapshotted by snapper |
| `@home` | `/home` | User data — separate from root snapshots |
| `@snapshots` | `/.snapshots` | Snapper snapshot dir |
| `@log` | `/var/log` | Excluded from root snapshots (prevents bloat) |
| `@cache` | `/var/cache` | Excluded from root snapshots |

Mount options: `noatime,compress=zstd,space_cache=v2` on all subvolumes.

**A/B compatibility note:** The `@` subvolume is self-contained and can be swapped atomically — this is the design property needed for a future A/B upgrade path. The layout does not need to change to adopt it.

### Snapshot tooling (installed + configured during post-install)

- `snapper` — snapshot manager; configured for root (`snapper -c root create-config /`)
- `snap-pac` — pacman hooks that call `snapper pre`/`snapper post` around every transaction
- `grub-btrfs` — regenerates GRUB entries from snapper snapshots; hook runs on `snapper post`

**snapper root config defaults** (written to `/etc/snapper/configs/root`):
```
TIMELINE_CREATE="no"       # timeline snapshots off; snap-pac handles it
NUMBER_CLEANUP="yes"
NUMBER_MIN_AGE="1800"
NUMBER_LIMIT="10"          # keep last 10 pacman snapshots
NUMBER_LIMIT_IMPORTANT="5"
```

No user-facing CLI needed for this component — `bos-settings` is the interface.

---

## Component 2: ISO + Calamares Installer

### archiso profile (`iso/`)

- Derives from `/usr/share/archiso/configs/releng/` (the standard baseline)
- `packages.x86_64` includes: base, linux, grub, btrfs-progs, snapper, snap-pac, grub-btrfs, hyprland, pipewire, wireplumber, networkmanager, gtk4, gtk4-layer-shell, iw, librsvg, libpulse, bluez, bluez-utils, calamares, calamares-qt6
- `airootfs/etc/skel/` contains the default dotfiles (symlinked from `dotfiles/`)
- Live session autologs into a `liveuser` and launches Calamares automatically

### Calamares modules (in order)

1. **welcome** — system checks (RAM ≥ 2GB, internet, disk space)
2. **locale** — timezone + locale selection
3. **keyboard** — layout selection
4. **partition** — custom `btrfs` mode: creates EFI partition + single btrfs pool with the subvolume layout above
5. **users** — create main user, set password
6. **packages** — install package list (reuses `packages.x86_64`)
7. **bootloader** — install GRUB to EFI, `grub-mkconfig` with grub-btrfs hook
8. **shellprocess (post-install)** — runs `iso/post-install.sh`:
   - Configures snapper root config
   - Enables services: `NetworkManager`, `bluetooth`, `breadd` (user), `breadbox-sync` (user)
   - Runs `bakery install bread breadbar breadbox breadcrumbs breadpad` (or `bakery install --all`)
   - Copies `dotfiles/` into `/home/$USER/.config/` (skips any file that already exists)
9. **finished** — reboot prompt

---

## Component 3: `bos-settings` GTK4 App

### Tech choices
- **gtk4-rs** (v0.11, v4_12 feature), no relm4 — plain GTK4 following breadman's pattern
- **bread-theme** for palette + CSS (git dep: `github.com/Breadway/bread-ecosystem`)
- Reads/writes each tool's own config file directly (no unified intermediate config)
- Window: 960×640, sidebar 190px, `gtk4::Stack` for view switching — identical structure to breadman

### Sidebar sections + views

| Section | View | What it does |
|---------|------|--------------|
| **Apps** | bread | Edit `~/.config/bread/breadd.toml` |
| | breadbar | Edit `~/.config/breadbar/` (style.css override, no TOML needed) |
| | breadbox | Edit `~/.config/breadbox/config.toml` (context priority lists) |
| | breadcrumbs | Edit `~/.config/breadcrumbs/breadcrumbs.toml` (profiles, networks) |
| | breadpad | Edit `~/.config/breadpad/breadpad.toml` (model, reminders, calendar) |
| **System** | Snapshots | `snapper list` output; rollback button calls `snapper rollback N` |
| | Packages | `bakery list --installed`; update buttons call `bakery update <pkg>` |
| | Hyprland | "Open config in editor" + monitor list from `bread.state.monitors()` |

### Config loading pattern

Each view has a dedicated `load_config(path) -> Result<T>` and `save_config(path, T) -> Result<()>` using `toml` crate. Config structs mirror each app's existing types (no duplication — import the `*-shared` crate where it exists, e.g. `breadpad-shared`). For apps without a shared crate (breadbox, breadcrumbs), define minimal local structs.

### Snapshots view specifics

- On open: runs `snapper list --output-cols number,date,description,pre-post` via `std::process::Command`, parses into table rows
- Rollback: confirmation dialog → `snapper rollback <N>` → notify user to reboot
- Delete: `snapper delete <N>`
- No write access to `/` needed for list/rollback since snapper is configured with `ALLOW_USERS` for the main user

### Packages view specifics

- On open: reads `~/.local/state/bakery/installed.json` directly (no network)
- "Check for updates": runs `bakery list` (triggers index refresh), compares versions
- "Update all": runs `bakery update --all` in a subprocess, streams stdout to a log TextView

### Distribution

`bos-settings` gets a `bakery.toml` and is added to the `bread-ecosystem` registry — installable standalone on any Arch/Hyprland system via `bakery install bos-settings`, not only as part of a BOS install.

---

## Component 4: Default Dotfiles

Minimal but functional defaults deployed at install time. These are opinionated starting points, not locked configs — users edit freely after install.

| File | Key content |
|------|-------------|
| `dotfiles/hyprland/hyprland.conf` | Monitor auto-detect, default keybinds, `exec-once` for breadd/breadbar/breadbox-sync |
| `dotfiles/hyprland/keybinds.conf` | `$mod+Space` → breadbox, `$mod+N` → breadpad, `$mod+M` → breadman, `$mod+S` → bos-settings |
| `dotfiles/bread/breadd.toml` | All adapters enabled, log_level=info |
| `dotfiles/bread/init.lua` | Minimal: activates "default" profile on startup |
| `dotfiles/breadbox/config.toml` | Single default context with common apps |
| `dotfiles/breadcrumbs/breadcrumbs.toml` | Placeholder home profile (user fills in SSIDs) |

---

## Build Order

1. **Dotfiles** — write default configs; these unblock installer testing immediately
2. **Btrfs + snapper config** — write `post-install.sh`; test in a VM with `archiso` livecdbase
3. **ISO profile** — archiso profiledef + package list + Calamares YAML; iterate in a VM
4. **bos-settings** — start with Snapshots and Packages views (highest value, no app-specific config parsing needed), then add per-app views one at a time

---

## Verification

- **ISO**: Build with `mkarchiso -v -w /tmp/bos-work -o /tmp/bos-out iso/`; boot in QEMU (`qemu-system-x86_64 -cdrom bos.iso -m 4G -enable-kvm`); complete install; reboot into installed system; confirm all services running and bakery packages present
- **btrfs layout**: `btrfs subvolume list /` after install; confirm `@`, `@home`, `@snapshots`, `@log`, `@cache` exist
- **snapper**: `snapper list`; run `pacman -Syu` and confirm two new snapshots appear
- **grub-btrfs**: Reboot and confirm snapshot submenu in GRUB
- **bos-settings**: `cargo build --release`; launch; confirm each view loads its config file; edit a value, save, re-open and confirm persistence; test rollback button in Snapshots view
