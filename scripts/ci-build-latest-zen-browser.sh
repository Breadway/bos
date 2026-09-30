#!/usr/bin/env bash
set -euo pipefail

build_root="${ZEN_BUILD_ROOT:-/home/builder}"
source_tree="${ZEN_REPO_SOURCE:-$build_root/src}"
output_dir="${ZEN_OUTPUT_DIR:-$build_root/zen-output}"
repo_db="$(mktemp)"
release_json="$(mktemp)"
workdir="$(mktemp -d)"
cleanup() {
  rm -f "$repo_db" "$release_json"
  rm -rf "$workdir"
}
trap cleanup EXIT

curl --fail --location --retry 3 --retry-delay 2 \
  --output "$release_json" \
  https://api.github.com/repos/zen-browser/desktop/releases/latest
release_version="$(jq -er 'select(.draft == false and .prerelease == false) | .tag_name' "$release_json")"
if [[ ! "$release_version" =~ ^[0-9]+(\.[0-9]+){1,2}[a-z]?$ ]]; then
  printf 'invalid stable Zen release tag: %s\n' "$release_version" >&2
  exit 1
fi
asset_digest="$(jq -er '[.assets[] | select(.name == "zen.linux-x86_64.tar.xz" and .state == "uploaded")][0].digest' "$release_json")"
if [[ ! "$asset_digest" =~ ^sha256:[[:xdigit:]]{64}$ ]]; then
  printf 'GitHub did not provide a SHA-256 digest for Zen %s Linux archive\n' "$release_version" >&2
  exit 1
fi
expected_sha256="${asset_digest#sha256:}"

curl --fail --location --retry 3 --retry-delay 2 \
  --output "$repo_db" \
  https://dl.breadway.dev/arch/x86_64/breadway.db
repo_version="$(python3 - "$repo_db" <<'PY'
import sys
import tarfile

with tarfile.open(sys.argv[1], "r:*") as database:
    for member in database.getmembers():
        if not member.isfile() or not member.name.endswith("/desc"):
            continue
        stream = database.extractfile(member)
        if stream is None:
            continue
        fields = stream.read().decode().splitlines()
        pairs = {fields[i][1:-1]: fields[i + 1] for i in range(len(fields) - 1)
                 if fields[i].startswith("%") and fields[i].endswith("%")}
        if pairs.get("NAME") == "zen-browser-bin":
            print(pairs["VERSION"].rsplit("-", 1)[0])
            break
    else:
        print("0")
PY
)"

if (( $(vercmp "$release_version" "$repo_version") <= 0 )); then
  printf 'Zen is current: upstream %s, [breadway] %s\n' "$release_version" "$repo_version"
  exit 0
fi

source_name="zen-browser-${release_version}-1-x86_64.tar.xz"
srcdir="$workdir/sources"
mkdir -p "$srcdir" "$output_dir"
curl --fail --location --retry 3 --retry-delay 2 \
  --output "$srcdir/$source_name" \
  "https://github.com/zen-browser/desktop/releases/download/$release_version/zen.linux-x86_64.tar.xz"
"$source_tree/scripts/ci-verify-zen-release-asset.sh" "$expected_sha256" "$srcdir/$source_name"
export ZEN_PKGVER="$release_version"
export ZEN_SHA256="$expected_sha256"
export SRCDEST="$srcdir"

cd "$source_tree/packaging/zen-browser-bin"
PKGDEST="$output_dir" makepkg --force --noconfirm --nodeps --nocheck --clean
printf 'built Zen %s for publication to [breadway]\n' "$release_version"
