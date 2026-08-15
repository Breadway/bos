# AGENTS.md — Repo hygiene

Scope: this file covers *repo hygiene* — branching, remotes, CI, cleanup. It is not project documentation. Product shape is [README.md](README.md).

## What this repo is

ISO + Calamares + skel. **No Cargo workspace. No `bos-settings/` member.**
bos-settings (Tauri 2 + Svelte) and breadhelp are standalone bakery
products. breadlock is the only bread\* pacman package.

Live skel is `iso/airootfs/etc/skel`. `dotfiles/` is stale.

## Branch model

Single-trunk:

- `main` — integration + release trunk. Land work via short-lived
  `feature/*` / `fix/*` branches, then merge.
- `stable` — marker only. CI fast-forwards it to the latest non-RC release
  tag. Never merge into it by hand.

There is no `dev` integration branch.

## Remotes

- `origin` — Forgejo (`ssh://git@100.66.238.26:2222/Breadway/bos.git`) — authoritative.
- `github` — GitHub (`https://github.com/Breadway/bos.git`) mirror. Push
  origin (and github when mirroring).

`origin` is **not** GitHub.

## CI

- `.forgejo/workflows/*.yml` trigger on `push: tags: ['v*']` (or
  path-scoped packaging triggers) — ordinary pushes to `main` run nothing
  except those path filters. Tag a release to build the ISO.
- `stable` is moved by the release-iso workflow, not by humans.
- No build/lint/test CI runs on ordinary commits or PRs — test locally
  before merging to `main`.

## Cleanup

- Delete feature/fix branches (local + remote) once merged. Check with
  `git branch --merged main`.
- Don't let merged branches accumulate.

## Don't

- Don't embed credentials in remote URLs — SSH or a credential helper only.
- Don't leave the default branch pointed at a feature branch on
  GitHub/Forgejo.
- Don't bake an ISO (`sudo ./build-local.sh`) unless asked — lockfile/docs
  work does not require it.
- Don't tell users to `snapper rollback` blindly; GRUB pins
  `rootflags=subvol=@`. Recovery is grub-btrfs reboot.
