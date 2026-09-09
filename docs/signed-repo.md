# Signed `[breadway]` repo

**Status: live.** The ISO's `[breadway]` section is `SigLevel = Required`
and points at `https://dl.breadway.dev/arch/$arch`, where every
`.pkg.tar.zst` and the db carry a detached `.sig` from the BOS release key
(`56203B86…`, `KEYS.asc`, `releases@breadway.dev`). That key is trusted in
the pacman keyring at build time (`build-local.sh`), on the live medium
(`iso/airootfs/root/customize_airootfs.sh`), and on the installed target
(`iso/airootfs/etc/calamares/post-install.sh`).

Forgejo publishing is unchanged: `package.yml` / packaging workflows still
PUT unsigned `.pkg.tar.zst` to Forgejo's Arch registry. The signed tree at
`dl.breadway.dev/arch` is rebuilt from that registry by
`.forgejo/workflows/signed-repo.yml` + `scripts/ci-publish-signed-repo.sh`.

The rest of this doc is the original stand-up / verification procedure,
kept for reference and for re-verifying after key rotation.

## Stand up `dl.breadway.dev/arch`

CI job: **Publish signed `[breadway]` repo**
(`.forgejo/workflows/signed-repo.yml`), host runner: **no
container**, so it can write `/srv/breadway-dl` like bakery releases.
breadlock `package.yml` uses `archlinux:latest` and cannot see host `/srv`.

Use the same release-signing key already in CI:

- Public half: [`KEYS.asc`](../KEYS.asc)
  (`5620 3B86 A110 695A E7F3  1093 4AF3 323D 678E B5E2`,
  `releases@breadway.dev`)
- Private half: the `GPG_PRIVATE_KEY` Forgejo secret (armoured secret key,
  no passphrase). Same secret `release-iso.yml` uses to sign `SHA256SUMS`.
  The workflow **fails** if this secret is missing.

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

On disk: `/srv/breadway-dl/arch/x86_64/` (nginx already serves
`/srv/breadway-dl` as `https://dl.breadway.dev/`).

The job collects the current ISO `[breadway]` set from the Forgejo Arch
registry (breadlock + calamares, zen-browser-bin, bibata-cursor-theme-bin,
zsh-theme-powerlevel10k, yay-bin, python-pywal). Leftover bakery-channel pacman packages
still sitting in that registry are **not** copied. Optional
`BREADWAY_PKG_DIR` on the runner overrides individual files.

Then it detach-signs each `.pkg.tar.zst` as a **binary** sidecar (pacman
wants `.sig`, not armoured `.asc`) and builds the database with
`repo-add -s`:

```sh
export GNUPGHOME=/tmp/gnupg-breadway-repo
mkdir -m 700 -p "$GNUPGHOME"
printf '%s\n' "$GPG_PRIVATE_KEY" | gpg --batch --import

gpg --batch --yes --local-user releases@breadway.dev \
    --detach-sign breadlock-<ver>-1-x86_64.pkg.tar.zst
# → breadlock-<ver>-1-x86_64.pkg.tar.zst.sig

cd /srv/breadway-dl/arch/x86_64
repo-add -s -k releases@breadway.dev breadway.db.tar.gz *.pkg.tar.zst
```

`repo-add -s` writes `breadway.db.tar.gz.sig` (and the `.files` pair).
Pacman fetches `<section>.db` + `<section>.db.sig` from `Server`.

## Dispatch the workflow

Forgejo UI: **Actions → "Publish signed [breadway] repo" → Run workflow**.
Select `main`.

API (`workflow_dispatch`):

```sh
curl -fsS -X POST \
  -H "Authorization: token ${RELEASE_TOKEN}" \
  -H "Content-Type: application/json" \
  "https://git.breadway.dev/api/v1/repos/Breadway/bos/actions/workflows/signed-repo.yml/dispatches" \
  -d '{"ref":"main"}'
```

It also runs after the in-repo AUR republish workflows complete
(`calamares` / `bibata` / `powerlevel10k` / `yay-bin` / `python-pywal`). breadlock lives in
another repo; that job can fire this one with `repository_dispatch` event
`publish-signed-repo` (or dispatch from the UI after a breadlock tag).

## Verify

Confirm the signed db is actually served **before** touching ISO
`SigLevel` or `Server`:

```sh
curl -fsSIL https://dl.breadway.dev/arch/x86_64/breadway.db
curl -fsSIL https://dl.breadway.dev/arch/x86_64/breadway.db.sig
```

Both must be HTTP 200. A 404 on `breadway.db.sig` means do **not** flip
`SigLevel` to `Required`.

Import `KEYS.asc` and check the detached signatures:

```sh
gpg --import KEYS.asc
curl -fsSL -o /tmp/breadway.db https://dl.breadway.dev/arch/x86_64/breadway.db
curl -fsSL -o /tmp/breadway.db.sig https://dl.breadway.dev/arch/x86_64/breadway.db.sig
gpg --verify /tmp/breadway.db.sig /tmp/breadway.db
```

On a throwaway Arch box (not the ISO tree):

```sh
sudo pacman-key --add KEYS.asc
sudo pacman-key --lsign-key 56203B86A110695AE7F310934AF3323D678EB5E2

# Temporary /etc/pacman.conf snippet; do not commit this to the ISO:
# [breadway]
# SigLevel = Required
# Server = https://dl.breadway.dev/arch/$arch

sudo pacman -Sy
```

`pacman -Sy` must fetch `breadway.db` + `breadway.db.sig` without
"missing or invalid signature". Then `pacman -Si breadlock` (and the AUR
republishes) should list the `[breadway]` section.

## breadlock `package.yml` sidecar

[`breadlock` `package.yml`](https://git.breadway.dev/Breadway/breadlock/src/branch/main/.forgejo/workflows/package.yml)
still `makepkg`s and PUTs the archive at Forgejo's registry. That path
stays; Never installs keep working. The signed tree is rebuilt by the bos
workflow above (registry fetch + sign + `repo-add -s`), not by writing
`/srv` from breadlock's container.

## The ISO flip (done)

All three steps have landed:

1. **Key trusted.** The public key is committed at
   `iso/airootfs/etc/pacman.d/breadway-repo.asc`. `build-local.sh`
   `pacman-key --add` + `--lsign-key`s it into the build host keyring;
   `customize_airootfs.sh` does the same in the airootfs;
   `calamares/post-install.sh` re-does it in the target chroot.
2. **`Server`** in `iso/pacman.conf` and `iso/airootfs/etc/pacman.conf`
   points at `https://dl.breadway.dev/arch/$arch`, section renamed to
   `[breadway]` (matching `breadway.db`).
3. **`SigLevel = Required`** on that section.

### Re-verify after any build

In a VM booted from a fresh ISO:

```sh
sudo pacman -Sy            # must fetch breadway.db + .sig, no signature error
sudo pacman -Si breadlock  # lists the [breadway] section
sudo pacman -S --noconfirm yay-bin   # installs with no key prompt
```

Then run the installer and, on the installed system, `sudo pacman -Sy`
again; the target keyring must already trust `56203B86…`.
