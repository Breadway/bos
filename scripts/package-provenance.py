#!/usr/bin/env python3
"""Record build candidates or verify packages against a protected digest policy."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def package_info(path):
    raw = subprocess.run(["tar", "-xOf", str(path), ".PKGINFO"], check=True, capture_output=True, text=True).stdout
    fields = {}
    for line in raw.splitlines():
        if " = " in line:
            key, value = line.split(" = ", 1)
            if key in {"pkgname", "pkgver", "arch"}:
                if key in fields:
                    raise ValueError(f"duplicate {key} in package metadata")
                fields[key] = value
    if set(fields) != {"pkgname", "pkgver", "arch"}:
        raise ValueError("package metadata is missing identity fields")
    return fields


def load_policy(path, names):
    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate policy field: {key}")
            result[key] = value
        return result
    policy = json.loads(path.read_text(), object_pairs_hook=unique_pairs)
    if policy.get("schema") != 1 or not isinstance(policy.get("packages"), dict):
        raise ValueError("unknown package provenance schema")
    packages = policy["packages"]
    if set(packages) != set(names):
        raise ValueError("approved package names must exactly match the publisher allowlist")
    filenames = set()
    for name, entry in packages.items():
        filename = entry.get("filename", "")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_:~-]*\.pkg\.tar\.zst", filename) or filename in filenames:
            raise ValueError(f"unsafe or duplicate filename for {name}")
        filenames.add(filename)
        if not re.fullmatch(r"[0-9a-f]{64}", entry.get("sha256", "")):
            raise ValueError(f"missing approved SHA256 for {name}")
        if not entry.get("pkgver") or entry.get("arch") not in {"x86_64", "any"}:
            raise ValueError(f"missing version or unsupported architecture for {name}")
        if not isinstance(entry.get("source"), dict) or not entry["source"].get("kind"):
            raise ValueError(f"missing provenance source for {name}")
    return packages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    record = commands.add_parser("record", help="emit an unapproved candidate, never change the allowlist")
    record.add_argument("package", type=Path)
    record.add_argument("--source-repo", required=True)
    record.add_argument("--source-commit", required=True)
    record.add_argument("--workflow", required=True)
    record.add_argument("--run-id", required=True)
    for command in ("list", "verify"):
        child = commands.add_parser(command)
        child.add_argument("policy", type=Path)
        if command == "verify":
            child.add_argument("directory", type=Path)
        child.add_argument("names", nargs="+")
    args = parser.parse_args()
    if args.command == "record":
        info = package_info(args.package)
        if not re.fullmatch(r"[0-9a-f]{40,64}", args.source_commit):
            raise ValueError("build source commit must be the complete Git object ID")
        candidate = {
            "filename": args.package.name, "pkgver": info["pkgver"], "arch": info["arch"], "sha256": digest(args.package),
            "source": {"kind": "build-candidate", "repository": args.source_repo, "commit": args.source_commit, "workflow": args.workflow, "run_id": args.run_id},
        }
        print(json.dumps({"schema": 1, "packages": {info["pkgname"]: candidate}}, indent=2, sort_keys=True))
        return
    packages = load_policy(args.policy, args.names)
    if args.command == "list":
        for name in args.names:
            print(f"{name}\t{packages[name]['filename']}")
        return
    paths = sorted(args.directory.glob("*.pkg.tar.zst"))
    if {path.name for path in paths} != {entry["filename"] for entry in packages.values()}:
        raise ValueError("candidate package files do not exactly match the approved set")
    for name, expected in packages.items():
        path = args.directory / expected["filename"]
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"package is not a regular file: {name}")
        if digest(path) != expected["sha256"]:
            raise ValueError(f"approved SHA256 mismatch: {name}, refusing to sign")
        actual = package_info(path)
        if actual != {"pkgname": name, "pkgver": expected["pkgver"], "arch": expected["arch"]}:
            raise ValueError(f"approved package name/version/architecture mismatch: {name}")
    print("verified approved package set, versions and SHA256 digests")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Package provenance rejected: {error}") from error
