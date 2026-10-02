#!/usr/bin/env python3
"""Check pinned offline signing identities without modifying any keyring."""
import argparse
import os
import subprocess
import tempfile
import time


def records(args, home):
    result = subprocess.run(
        ["gpg", "--homedir", home, "--batch", "--with-colons", *args],
        check=True, capture_output=True, text=True,
    )
    return [line.split(":") for line in result.stdout.splitlines()]


def identity(rows, kind):
    keys = [row for row in rows if row[0] == kind]
    if len(keys) != 1:
        raise ValueError("public key must contain exactly one primary identity")
    key = keys[0]
    if key[1] in {"r", "e", "d"} or not key[6] or int(key[6]) <= time.time():
        raise ValueError("signing identity must be valid and have a future expiry")
    if "s" not in key[11].lower():
        raise ValueError("identity has no signing capability")
    position = rows.index(key)
    if position + 1 >= len(rows) or rows[position + 1][0] != "fpr":
        raise ValueError("primary fingerprint missing")
    return rows[position + 1][9]


def fingerprint(path):
    with tempfile.TemporaryDirectory(prefix="bos-public-key-") as home:
        return identity(records(["--import-options", "show-only", "--import", path], home), "pub")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("public_key")
    parser.add_argument("--different-from")
    parser.add_argument("--secret", action="store_true")
    args = parser.parse_args()
    expected = fingerprint(args.public_key)
    if args.different_from and expected == fingerprint(args.different_from):
        raise ValueError("ISO and repository signing identities must be different")
    if args.secret:
        home = os.environ.get("GNUPGHOME", "")
        if not os.path.isabs(home) or not os.path.isdir(home):
            raise ValueError("GNUPGHOME must name the dedicated offline keyring")
        found = identity(records(["--list-secret-keys", expected], home), "sec")
        if found != expected:
            raise ValueError("secret signing identity does not match pinned public key")
    print(expected)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Signing policy rejected: {error}") from error
