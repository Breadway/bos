# Signed `[breadway]` repo

Today the ISO's `[Breadway.os.git.breadway.dev]` section is
`SigLevel = Never`. That is TLS-only integrity: packages come from Forgejo's
Arch registry, which does **not** serve pacman-compatible database
signatures. `KEYS.asc` signs **ISO `SHA256SUMS` only** — it is not imported
as a pacman repo key. Do not flip `SigLevel` to `Required` on that section
until a signed repo exists and has been verified; Required without
signatures breaks the ISO and every installed system.

The signed repo belongs at `https://dl.breadway.dev/arch`, not on Forgejo's
registry.

## Stand up `dl.breadway.dev/arch`

Use the same release-signing key already in CI:

- Public half: [`KEYS.asc`](../KEYS.asc)
  (`5620 3B86 A110 695A E7F3  1093 4AF3 323D 678E B5E2`,
  `releases@breadway.dev`)
- Private half: the `GPG_PRIVATE_KEY` Forgejo secret (armoured secret key,
  no passphrase). Same secret `release-iso.yml` uses to sign `SHA256SUMS`.

Layout (example for `x86_64`):

```
https://dl.breadway.dev/arch/x86_64/
  breadlock-<ver>-1-x86_64.pkg.tar.zst
  breadlock-<ver>-1-x86_64.pkg.tar.zst.sig
  breadway.db
  breadway.db.sig
  breadway.files
  breadway.files.sig
```

Build the database **and sign it** with `repo-add -s`:

```sh
export GNUPGHOME=/tmp/gnupg-breadway-repo
mkdir -m 700 -p "$GNUPGHOME"
printf '%s\n' "$GPG_PRIVATE_KEY" | gpg --batch --import

cd /srv/dl.breadway.dev/arch/x86_64
repo-add -s -k releases@breadway.dev breadway.db.tar.gz *.pkg.tar.zst
```

`repo-add -s` writes `breadway.db.tar.gz.sig` (and the `.files` pair).
Pacman fetches `<section>.db` + `<section>.db.sig` from `Server`.

Package signatures are separate from the database signature. Detach-sign
each `.pkg.tar.zst` as a **binary** sidecar (pacman wants `.sig`, not
armoured `.asc`):

```sh
gpg --batch --yes --local-user releases@breadway.dev \
    --detach-sign breadlock-<ver>-1-x86_64.pkg.tar.zst
# → breadlock-<ver>-1-x86_64.pkg.tar.zst.sig
```

## breadlock `package.yml` sidecar

[`breadlock` `package.yml`](https://git.breadway.dev/Breadway/breadlock/src/branch/main/.forgejo/workflows/package.yml)
already `makepkg`s and PUTs the archive at Forgejo's registry. When the
signed repo exists, that job can also emit the sidecar and publish both
files to `dl.breadway.dev/arch`:

```sh
PKG=$(find packaging/arch -name '*.pkg.tar.zst' | head -1)
printf '%s\n' "$GPG_PRIVATE_KEY" | gpg --batch --import
gpg --batch --yes --local-user releases@breadway.dev --detach-sign "$PKG"
# upload "$PKG" and "${PKG}.sig" to dl.breadway.dev/arch/x86_64/
# then repo-add -s as above
```

Keep publishing to Forgejo until installs have been switched. The ISO
section stays `SigLevel = Never` until the signed tree is live.

## After the signed repo exists

1. Import `KEYS.asc` into the ISO keyring (`pacman-key --add` + `--lsign-key`).
2. Point `[breadway]` `Server` at `https://dl.breadway.dev/arch/$arch`.
3. Only then flip that section to `SigLevel = Required`.

Do not do those three steps against Forgejo's registry. See
`iso/pacman.conf` and `iso/airootfs/etc/pacman.conf`.
