Arch packaging
==============

`breadhelp/PKGBUILD` builds and installs `breadhelp` from source — BOS's
onboarding + help center, and the only first-party pacman package still built
from this repo.

`bos-settings` is also pacman-packaged and served from the same [breadway]
repo, but its source lives in its own repo now (`~/Projects/bos-settings`,
`github.com/Breadway/bos-settings`) so a bos-settings release doesn't require
a BOS ISO release.

Everything else the bread ecosystem ships (breadbar, breadbox, breadpad, ...)
is bakery-managed, not pacman-packaged — see `build-local.sh`.

## Local build

```bash
cd breadhelp && makepkg -si
```

## Before publishing to [breadway] repo

CI (`.forgejo/workflows/package.yml`) handles this on tag push (`vX.Y.Z`): it
bumps `pkgver`, archives the repo into the expected tarball name, and runs
`makepkg`. To do it by hand instead:

1. Tag a release on GitHub (`vX.Y.Z`).
2. Update `pkgver` in `breadhelp/PKGBUILD` to match the tag.
3. Update `source` to the release tarball URL.
4. Run `updpkgsums` (or manually set `sha256sums`).

## Runtime dependencies

| Package | Required | Notes |
|---------|----------|-------|
| `gtk4` | yes | UI toolkit |
| `glib2` | yes | always |
| `snapper` | optional | create-backup one-click fix |
