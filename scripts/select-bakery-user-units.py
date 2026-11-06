#!/usr/bin/env python3
"""Select bakery systemd user units for the BOS image bake.

The installed bakery snapshot is authoritative. build-local.sh bakes exactly
the services installed.json says are installed, plus local unit files whose
ExecStart is a lockfile binary (installed.json has omitted breadcrumbs.service
before). The cached bakery index may describe package metadata but must never
add packages, binaries or services that are not in the installed snapshot: a
newer cached index next to an older installed snapshot used to inject units
such as breadbug-status.service and then abort the bake because no local unit
file existed.

Index-only candidate units are reported on stderr and skipped. Prints one
selected unit name per line on stdout.
"""

import json
import os
import sys

BLOCKED_PREFIXES = ("breadcast", "breadarr")


def clean_unit(svc):
    name = svc["unit"] if isinstance(svc, dict) else svc
    if not name or str(name).startswith(BLOCKED_PREFIXES):
        return None
    return str(name)


def index_bins(pkg):
    out = []
    for b in pkg.get("binaries") or []:
        name = b["name"] if isinstance(b, dict) else b
        out.append(str(name).removesuffix("-x86_64"))
    return out


def main(argv):
    if len(argv) < 3:
        print(
            "usage: select-bakery-user-units.py INSTALLED INDEX USER_UNIT_DIR [BIN...]",
            file=sys.stderr,
        )
        return 2

    installed_path, index_path, user_dir, *bins = argv
    wanted = set(bins)
    units = set()
    installed_packages = {}

    if os.path.isfile(installed_path):
        with open(installed_path) as f:
            data = json.load(f)
        packages = data.get("packages", data)
        if isinstance(packages, dict):
            installed_packages = packages
            for pkg in packages.values():
                if not isinstance(pkg, dict):
                    continue
                for svc in pkg.get("services") or []:
                    name = clean_unit(svc)
                    if name:
                        units.add(name)

    # Keep the historical fallback: a lockfile binary with a local unit file
    # is baked even when installed.json omitted that service.
    if os.path.isdir(user_dir):
        for fn in sorted(os.listdir(user_dir)):
            if not fn.endswith(".service"):
                continue
            path = os.path.join(user_dir, fn)
            if not os.path.isfile(path):
                continue
            try:
                with open(path) as f:
                    text = f.read()
            except OSError:
                continue
            for line in text.splitlines():
                if line.lstrip().startswith("ExecStart="):
                    argv0 = line.split("=", 1)[1].split()
                    if argv0 and os.path.basename(argv0[0]) in wanted:
                        units.add(fn)
                    break

    # The index only supplies metadata. Report candidate units it would have
    # added so a stale or newer cache cannot silently change the image.
    if os.path.isfile(index_path):
        try:
            with open(index_path) as f:
                idx = json.load(f)
            packages = idx.get("packages") or {}
        except (OSError, json.JSONDecodeError):
            packages = {}
        for name, pkg in packages.items():
            if not isinstance(pkg, dict):
                continue
            pbins = index_bins(pkg)
            if name not in wanted and not any(b in wanted for b in pbins):
                continue
            for svc in pkg.get("services") or []:
                unit = clean_unit(svc)
                if not unit or unit in units:
                    continue
                if name not in installed_packages:
                    print(
                        f"WARN: skipping index service {unit}: package {name} is not installed",
                        file=sys.stderr,
                    )
                else:
                    print(
                        f"WARN: skipping index-only service {unit}: not in installed snapshot for {name}",
                        file=sys.stderr,
                    )

    for unit in sorted(units):
        print(unit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
