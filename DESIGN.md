# BOS — historical design plan

## Current architecture

**Read [README.md](README.md) for how this repo actually ships.** This file
is the original plan. Several sections below are historical and must not be
taken as current:

| Plan said | What the tree does now |
|-----------|------------------------|
| Cargo workspace with a `bos-settings/` member | This repo is ISO + Calamares + skel only. No Cargo workspace. |
| `bos-settings` as an in-tree GTK4 app | Standalone bakery product, **Tauri 2 + Svelte**. |
| bakery install in Calamares post-install | bakery binaries + breadhelp content are **baked into `/etc/skel` at ISO build time** from `iso/bread-lockfile.toml`. Missing bins fail the bake. |
| `dotfiles/` is the live skel | Live defaults are `iso/airootfs/etc/skel`. `dotfiles/` is stale. |
| A/B root swapping | **Future.** Today: btrfs + snapper + **grub-btrfs**. GRUB pins `rootflags=subvol=@`, so `snapper rollback` is not the user-facing recovery path. |
| Work on `dev`; origin = GitHub | Single-trunk `main`; `stable` is a CI marker. `origin` = Forgejo, `github` = GitHub. |
| `[breadway]` provides bakery/breadbar/bos-settings | `[breadway]` is breadlock + AUR republishes. Desktop apps are bakery. **Not shipped:** breadcast, breadarr. |
| NVIDIA / A/B / Secure Boot / LUKS2 | NVIDIA proprietary is **unsupported**. A/B root swapping is **not implemented**. Secure Boot is **Setup Mode only** (self-signed `sbctl`). Disk encryption is **LUKS1** because GRUB cannot unlock LUKS2+Argon2id. |
| `SigLevel = Required` on `[breadway]` | **Yes, as of the signed repo.** `[breadway]` points at `https://dl.breadway.dev/arch` where `scripts/ci-publish-signed-repo.sh` detach-signs every `.pkg.tar.zst` and the db with the BOS release key (`56203B86…`, `KEYS.asc`). That key is trusted in the pacman keyring at build time, on the live medium, and on the installed target. |

---

# Original plan (kept for history)

## Context

The bread ecosystem (bread, breadbar, breadbox, breadcrumbs, breadpad/breadman, bakery) is a cohesive set of Arch/Hyprland-specific tools with a shared theme system, unified package manager, and consistent config conventions. Currently, getting to a working system requires installing Arch, Hyprland, each tool via bakery, and wiring up dotfiles manually. BOS eliminates that — one ISO install produces a fully working desktop with everything preconfigured.

Goals:
- **Install and be done**: Calamares GUI installer → reboot → working Hyprland + full bread stack
- **Rollback safety**: Btrfs subvolumes + snapper + snap-pac; every pacman transaction is snapshotted
- **Unified config**: `bos-settings` surfaces all app configs + snapshot management + bakery updates
- **Future-compatible**: Btrfs layout is designed to allow A/B partition migration later (SteamOS model)

---

## Repo Structure

Single new repo: `Breadway/bos` — *planned as* a Cargo workspace. **That is
not what landed**; see Current architecture.

```
bos/
├── Cargo.toml                  # Workspace (members: [bos-settings])  — NOT in tree
├── bos-settings/               # planned GTK4 app — now its own bakery repo
├── iso/                        # archiso profile (this is the repo)
│   ├── profiledef.sh
│   ├── packages.x86_64
│   └── airootfs/
└── dotfiles/                   # planned install-time configs — NOT the live skel
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

**A/B compatibility note (future):** The `@` subvolume is self-contained and
could be swapped atomically. This is a design property for a later upgrade
path. It is **not** implemented. Recovery today is reboot into a grub-btrfs
snapshot; GRUB's `rootflags=subvol=@` means a raw `snapper rollback` is the
wrong instruction to give users.

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
- `packages.x86_64` is the live + installed pacman set (Hyprland, Calamares,
  breadlock, WebKitGTK 4.1 for Tauri bos-settings, …). bakery apps are not
  listed here.
- `airootfs/etc/skel/` contains the default user configs (this is the live
  skel — not `dotfiles/`).
- Live session autologs into a `liveuser` and launches Calamares automatically

### Calamares modules (in order)

The historical list below included a post-install `bakery install` and
Calamares `bootloader`/`grubcfg` installing GRUB. What shipped instead:
binaries are already in skel; `post-install.sh` runs `grub-install` +
`grub-mkconfig` (Calamares' bootloader modules leave the ESP empty here).

1. **welcome** — system checks (RAM ≥ 2GB, internet, disk space)
2. **locale** — timezone + locale selection
3. **keyboard** — layout selection
4. **partition** — custom `btrfs` mode: creates EFI partition + single btrfs pool with the subvolume layout above
5. **users** — create main user, set password
6. **packages** — install package list (reuses `packages.x86_64`)
7. **bootloader** — *planned*; actual GRUB install is in `post-install.sh`
8. **shellprocess (post-install)** — snapper, services, copy skel; does **not** run bakery
9. **finished** — reboot prompt

---

## Component 3: `bos-settings` (planned as GTK4)

### Tech choices (original)

- **gtk4-rs** (v0.11, v4_12 feature), no relm4 — plain GTK4 following breadman's pattern

**What shipped:** Tauri 2 + Svelte in its own repo
(`git.breadway.dev/Breadway/bos-settings`), distributed by bakery. This
repo does not build it.

### Sidebar sections + views

The panel list is still roughly accurate; see README. Snapshots recovery
should send users through **grub-btrfs reboot**, not `snapper rollback N`.

### Distribution

`bos-settings` has its own `bakery.toml` and is installable via
`bakery install bos-settings` on any Arch/Hyprland system, not only as part
of a BOS install.

---

## Component 4: Default Dotfiles

Minimal but functional defaults. These live in `iso/airootfs/etc/skel`
(`hyprland.lua` + JSON binds, not `dotfiles/hyprland/*.conf`).

Zero-config bakery apps survive with no extra skel files. breadcrumbs
networks are user-filled after install — do not invent a full
`breadcrumbs.toml` in-tree.

---

## Build Order

Historical. The ISO profile + skel + Calamares path is what this repo
iterates on. bos-settings is developed in its own repo.

---

## Verification

- **ISO**: `sudo ./build-local.sh` (not a raw `mkarchiso iso/` — the bake
  step is required). Boot in QEMU; complete install; confirm bakery bins and
  `~/.local/share/breadhelp/content`.
- **btrfs layout**: `btrfs subvolume list /` after install; confirm `@`, `@home`, `@snapshots`, `@log`, `@cache` exist
- **snapper**: `snapper list`; run `pacman -Syu` and confirm two new snapshots appear
- **grub-btrfs**: Reboot and confirm snapshot submenu in GRUB
- **bos-settings**: built and tested in the bos-settings repo, not here
