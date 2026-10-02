#!/usr/bin/env bash
# Run only on the separate offline ISO signer, after reviewing the candidate.
# Signs SHA256SUMS only. The repository private key must not be on this host.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PUBLIC_KEY="${BOS_ISO_PUBLIC_KEY_FILE:-$SCRIPT_DIR/../KEYS.asc}"
REPO_KEY="${BREADWAY_REPO_PUBLIC_KEY_FILE:-$SCRIPT_DIR/../iso/airootfs/etc/pacman.d/breadway-repo.asc}"
[[ "${CI:-false}" != true && -z "${GITHUB_ACTIONS:-}" && -z "${FORGEJO_ACTIONS:-}" ]] \
  || { echo 'ISO signing is forbidden on an Actions runner' >&2; exit 1; }
KEY_ID="$(python3 "$SCRIPT_DIR/signing-key-policy.py" "$PUBLIC_KEY" --different-from "$REPO_KEY" --secret)"
CANDIDATE="${1:?usage: sign-iso-checksums.sh candidate-directory}"
cd -- "$CANDIDATE"
# Refuse generic checksum lists, paths, extra entries or another kind of payload.
python3 - <<'PY'
import pathlib
import re
line = pathlib.Path("SHA256SUMS").read_text()
match = re.fullmatch(r"[0-9a-f]{64}  (bos-[0-9][A-Za-z0-9.+_-]*-x86_64\.iso)\n", line)
if match is None or not pathlib.Path(match[1]).is_file():
    raise SystemExit("SHA256SUMS must contain exactly one BOS ISO basename")
PY
sha256sum --strict -c SHA256SUMS
gpg --batch --yes --local-user "$KEY_ID" --detach-sign --armor -o SHA256SUMS.asc SHA256SUMS
gpg --batch --verify SHA256SUMS.asc SHA256SUMS
