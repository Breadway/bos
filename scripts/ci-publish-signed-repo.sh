#!/usr/bin/env bash
# Collect the current [breadway] ISO packages, detach-sign them with the
# repository-only key on an offline signing host, and publish a signed pacman db
# under /srv/breadway-dl/arch/x86_64/ (https://dl.breadway.dev/arch/x86_64/).
#
# Does not change ISO SigLevel and does not write to the Forgejo Arch
# registry; existing package.yml / packaging/*.yml PUTs stay as they are.
#
# Required env:
#   GNUPGHOME         dedicated offline repository signing keyring
# Private keys are never imported from Actions or environment variables.
# Optional env:
#   BREADWAY_DEST     publish dir (default /srv/breadway-dl/arch/x86_64)
#   BREADWAY_PKG_DIR  extra directory of .pkg.tar.zst to prefer over the registry
#   BREADWAY_REGISTRY Forgejo Arch registry base
#   BREADWAY_SIGN_ONLY=1   skip collect; sign+index BREADWAY_REPO_DIR only
#   BREADWAY_COLLECT_ONLY=1   prepare an unsigned candidate, no keyring needed
set -euo pipefail

PACKAGES=(
  breadlock
  calamares
  zen-browser-bin
  bibata-cursor-theme-bin
  zsh-theme-powerlevel10k
  yay-bin
)

ARCH="${BREADWAY_ARCH:-x86_64}"
REGISTRY="${BREADWAY_REGISTRY:-https://git.breadway.dev/api/packages/Breadway/arch/os}"
DEST="${BREADWAY_DEST:-/srv/breadway-dl/arch/${ARCH}}"
KEY_ID=""
PROVENANCE="${BREADWAY_PROVENANCE_FILE:-$(dirname -- "${BASH_SOURCE[0]}")/../packaging/provenance/approved-packages.json}"
REPO_DIR="${BREADWAY_REPO_DIR:-}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PUBLIC_KEY="${BREADWAY_REPO_PUBLIC_KEY_FILE:-$SCRIPT_DIR/../iso/airootfs/etc/pacman.d/breadway-repo.asc}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

need_key() {
  [[ -n "${GNUPGHOME:-}" && "$GNUPGHOME" == /* && -d "$GNUPGHOME" ]] \
    || die "GNUPGHOME must be an existing dedicated offline signing keyring"
  [[ "${CI:-false}" != true && -z "${GITHUB_ACTIONS:-}" && -z "${FORGEJO_ACTIONS:-}" ]] \
    || die "repository signing is forbidden on an Actions runner"
  KEY_ID="$(python3 "$SCRIPT_DIR/signing-key-policy.py" "$PUBLIC_KEY" --secret)"
}

urlencode() {
  python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe="-._~"))' "$1"
}

pkginfo_name() {
  local pkg="$1" info
  info="$(tar -xOf "$pkg" .PKGINFO 2>/dev/null || zstd -dc "$pkg" | tar -xO .PKGINFO)"
  awk -F ' = ' '$1=="pkgname" {print $2; exit}' <<<"$info"
}


detach_sign_pkgs() {
  local pkg
  shopt -s nullglob
  for pkg in *.pkg.tar.zst; do
    gpg --batch --yes --local-user "$KEY_ID" --detach-sign "$pkg"
  done
  shopt -u nullglob
}

repo_add_signed() {
  local pkgs=()
  shopt -s nullglob
  pkgs=(*.pkg.tar.zst)
  shopt -u nullglob
  (( ${#pkgs[@]} > 0 )) || die "no .pkg.tar.zst files to index"
  rm -f breadway.db breadway.db.tar.gz breadway.db.sig breadway.db.tar.gz.sig \
        breadway.files breadway.files.tar.gz breadway.files.sig breadway.files.tar.gz.sig
  if repo-add --help 2>&1 | grep -q -- '--include-sigs'; then
    repo-add -s -k "$KEY_ID" --include-sigs breadway.db.tar.gz "${pkgs[@]}"
  else
    repo-add -s -k "$KEY_ID" breadway.db.tar.gz "${pkgs[@]}"
  fi
  [[ -e breadway.db.tar.gz.sig || -e breadway.db.sig ]] \
    || die "repo-add -s did not write breadway.db*.sig"
  # gpg writes 0600; nginx and the next publish need world-readable files.
  find . -maxdepth 1 -type f -exec chmod a+r {} + || true
}

sign_and_index() {
  local dir="$1"
  [[ -d "$dir" ]] || die "repo dir missing: $dir"
  command -v repo-add >/dev/null 2>&1 || die "install repo-add on the dedicated signer first"
  python3 "$SCRIPT_DIR/package-provenance.py" verify "$PROVENANCE" "$dir" "${PACKAGES[@]}"
  need_key
  (
    cd "$dir"
    detach_sign_pkgs
    repo_add_signed
  )
}


copy_local_overrides() {
  local dir="$1"
  [[ -n "$dir" && -d "$dir" ]] || return 0
  local pkg name
  shopt -s nullglob
  for pkg in "$dir"/*.pkg.tar.zst "$dir"/*/*.pkg.tar.zst; do
    [[ -f "$pkg" ]] || continue
    name="$(pkginfo_name "$pkg")"
    [[ -n "$name" ]] || continue
    local wanted=0 p
    for p in "${PACKAGES[@]}"; do
      if [[ "$p" == "$name" ]]; then
        wanted=1
        break
      fi
    done
    if (( wanted )); then
      printf 'local override: %s -> %s\n' "$name" "$(basename "$pkg")"
      cp -a "$pkg" "$STAGE/$(basename "$pkg")"
    fi
  done
  shopt -u nullglob
}

has_pkg_named() {
  local name="$1" pkg got
  shopt -s nullglob
  for pkg in "$STAGE"/*.pkg.tar.zst; do
    got="$(pkginfo_name "$pkg")"
    if [[ "$got" == "$name" ]]; then
      shopt -u nullglob
      return 0
    fi
  done
  shopt -u nullglob
  return 1
}

collect_from_registry() {
  local work manifest name filename enc url
  work="$(mktemp -d "${TMPDIR:-/tmp}/breadway-provenance.XXXXXX")"
  manifest="$work/packages.tsv"
  # Abort on invalid policy before requesting registry bytes. Metadata from the
  # registry never chooses the approved version, filename or digest.
  python3 "$SCRIPT_DIR/package-provenance.py" list "$PROVENANCE" "${PACKAGES[@]}" > "$manifest" \
    || { rm -rf "$work"; die "invalid approved package policy"; }
  while IFS=$'\t' read -r name filename; do
    if has_pkg_named "$name"; then
      printf 'using local %s, will verify against approved digest\n' "$name"
      continue
    fi
    enc="$(urlencode "$filename")"
    url="$REGISTRY/$ARCH/$enc"
    printf 'fetch approved %s\n' "$filename"
    curl -fL --retry 3 --retry-delay 2 -o "$STAGE/$filename" "$url" \
      || { rm -rf "$work"; die "failed to fetch $url"; }
  done < "$manifest"
  rm -rf "$work"
  python3 "$SCRIPT_DIR/package-provenance.py" verify "$PROVENANCE" "$STAGE" "${PACKAGES[@]}"
}

publish_tree() {
  local parent dest_name prev
  parent="$(dirname "$DEST")"
  dest_name="$(basename "$DEST")"
  mkdir -p "$parent"
  chmod a+rX "$STAGE" || true
  # gpg --detach-sign often writes 0600 files the runner cannot chmod;
  # do not fail the publish after repo-add -s already succeeded.
  find "$STAGE" -type f -exec chmod a+r {} + || true
  prev="$parent/${dest_name}.prev"
  rm -rf "$prev"
  if [[ -e "$DEST" ]]; then
    mv "$DEST" "$prev"
  fi
  mv "$STAGE" "$DEST"
  rm -rf "$prev"
  STAGE=""
}

if [[ "${BREADWAY_SIGN_ONLY:-0}" == 1 ]]; then
  [[ -n "$REPO_DIR" ]] || die "BREADWAY_SIGN_ONLY requires BREADWAY_REPO_DIR"
  sign_and_index "$REPO_DIR"
  exit 0
fi

if [[ "${BREADWAY_COLLECT_ONLY:-0}" != 1 ]]; then need_key; fi

DEST_PARENT="$(dirname "$DEST")"
mkdir -p "$DEST_PARENT" || die "cannot create $DEST_PARENT (runner must write /srv/breadway-dl)"
STAGE="$(mktemp -d "$DEST_PARENT/.stage-XXXXXX")"
cleanup() {
  if [[ -n "${STAGE:-}" && -d "${STAGE:-}" ]]; then
    rm -rf "$STAGE"
  fi
}
trap cleanup EXIT

copy_local_overrides "${BREADWAY_PKG_DIR:-}"
collect_from_registry

missing=()
for name in "${PACKAGES[@]}"; do
  has_pkg_named "$name" || missing+=("$name")
done
if (( ${#missing[@]} > 0 )); then
  die "missing packages after collect: ${missing[*]}"
fi

if [[ "${BREADWAY_COLLECT_ONLY:-0}" != 1 ]]; then sign_and_index "$STAGE"; fi

# Do not publish helper junk if a container left any.
rm -f "$STAGE/.sign.sh"

publish_tree

printf 'prepared [breadway] tree (collect-only=%s) -> %s\n' "${BREADWAY_COLLECT_ONLY:-0}" "$DEST"
ls -lh "$DEST"
