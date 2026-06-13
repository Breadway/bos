Arch packaging
==============

`PKGBUILD` builds and installs `bos-settings` from source.

## Local build

```bash
makepkg -si
```

## Before publishing to [breadway] repo

1. Tag a release on GitHub.
2. Update `pkgver` to match the tag.
3. Update `source` to the release tarball URL.
4. Run `updpkgsums` (or manually set `sha256sums`).

## Runtime dependencies

| Package | Required | Notes |
|---------|----------|-------|
| `gtk4` | yes | UI toolkit |
| `glib2` | yes | always |
| `snapper` | optional | snapshot management view |
