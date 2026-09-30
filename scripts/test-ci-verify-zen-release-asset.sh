#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
verify="$script_dir/ci-verify-zen-release-asset.sh"
expected='sha256:0aaee1b3f67f074aebf7fc6c2fe824441bcafd431e9991336479c16370519dd0'
asset_url='https://github.com/zen-browser/desktop/releases/download/1.22.3b/zen.linux-x86_64.tar.xz'
workdir="$(mktemp -d)"
cleanup() { rm -rf "$workdir"; }
trap cleanup EXIT

archive="${1:-$workdir/zen.linux-x86_64.tar.xz}"
if (( $# == 0 )); then
  curl --fail --location --retry 3 --retry-delay 2 --output "$archive" "$asset_url"
fi
"$verify" "$expected" "$archive"

modified="$workdir/modified-zen.linux-x86_64.tar.xz"
cp -- "$archive" "$modified"
python3 - "$modified" <<'PY'
import sys

with open(sys.argv[1], "r+b") as archive:
    first = archive.read(1)
    if not first:
        raise SystemExit("test archive is empty")
    archive.seek(0)
    archive.write(bytes([first[0] ^ 1]))
PY

if "$verify" "$expected" "$modified"; then
  echo 'modified release archive passed upstream digest verification' >&2
  exit 1
fi
echo 'pinned Zen release digest accepted the official asset and rejected a modified copy'
