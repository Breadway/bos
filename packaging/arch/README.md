Arch packaging
==============

This directory only holds `PKGBUILD`s for third-party AUR packages BOS
republishes to the `[breadway]` pacman repo (`calamares`, `bibata`,
`powerlevel10k`, `yay-bin`), not the user's own code. See each
subdirectory's `.forgejo/workflows/<name>.yml` (in this repo) for how each
one publishes on a push to `packaging/<name>/**`.

Every bread-ecosystem app (bakery, bread, breadbar, breadbox, breadcrumbs,
breadpad, breadpaper, breadmon, breadclip, breadshot,
bos-settings, breadhelp, ...) is bakery-managed, not pacman-packaged; see
`iso/bread-lockfile.toml` (`required_bins` + `optional_bins`), which
`build-local.sh` uses as the name list when baking this machine's bakery
install into the ISO's `/etc/skel`.
`breadlock` is the sole deliberate exception (it needs a root-owned
`/etc/pam.d/breadlock` PAM service file, which bakery (by design) has no
privileged-install path for) and stays on pacman only; see
`bread-ecosystem/docs/release-channels.md` for the full policy.
