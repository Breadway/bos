#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  echo "usage: $0 <upstream-sha256> <archive>" >&2
  exit 2
fi

expected="${1#sha256:}"
archive="$2"
if [[ ! "$expected" =~ ^[[:xdigit:]]{64}$ ]]; then
  echo 'invalid upstream SHA-256 digest' >&2
  exit 2
fi
if [[ ! -f "$archive" ]]; then
  printf 'release archive not found: %s\n' "$archive" >&2
  exit 2
fi

actual="$(sha256sum "$archive" | awk '{print $1}')"
if [[ "${actual,,}" != "${expected,,}" ]]; then
  printf 'release archive digest mismatch: expected %s, got %s\n' "$expected" "$actual" >&2
  exit 1
fi
