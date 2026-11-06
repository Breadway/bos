#!/usr/bin/env python3
"""Regression test for BOS-97: BAKE_FROM_HOME must bake services from the
installed snapshot, not from a newer cached bakery index.

Stages the shape from the card: an older installed.json without the
breadbug-status services, a newer index-dev.json that has them, an index-only
package that publishes a lockfile binary, and local unit files. Runs the unit
selection stage build-local.sh uses and asserts it exits cleanly, selects only
installed-snapshot services plus the historical local-unit fallback, and warns
about the index-only units it skipped.

A full FAST_BUILD=1 BAKE_FROM_HOME=1 invocation needs root, pacman-key and
mkarchiso, so this test stops at selection. Every selected unit is asserted to
have a unit file, which is the precondition the bake loop checks before it
would abort with "listed as a bakery service but not found".
"""

import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SELECT = ROOT / "scripts/select-bakery-user-units.py"

BINS = [
    "bakery",
    "bread",
    "breadd",
    "bread-emit",
    "bread-module-host",
    "breadman",
    "breadbar",
    "breadbox",
    "breadbox-sync",
    "breadcrumbs",
    "breadpad",
    "breadpaper",
    "bread-theme",
    "breadclip",
    "breadclipd",
    "breadshot",
    "bos-settings",
    "breadhelp",
]


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n")


def write_unit(unit_dir, name, binary):
    unit_dir.mkdir(parents=True, exist_ok=True)
    (unit_dir / name).write_text(
        "[Unit]\n"
        f"Description=test {name}\n"
        "\n"
        "[Service]\n"
        f"ExecStart=%h/.local/bin/{binary}\n"
        "\n"
        "[Install]\n"
        "WantedBy=default.target\n"
    )


with tempfile.TemporaryDirectory(prefix="bos97-bake-test-") as root:
    home = pathlib.Path(root)
    installed_path = home / ".local/state/bakery/installed.json"
    index_path = home / ".cache/bakery/index-dev.json"
    unit_dir = home / ".config/systemd/user"

    # Older installed snapshot: bread has only breadd.service and no
    # breadbug-status units. breadcrumbs is present but omits its service,
    # matching the historical local-unit fallback case.
    write_json(
        installed_path,
        {
            "track": "dev",
            "packages": {
                "bread": {
                    "name": "bread",
                    "version": "0.9.1-dev.20261018010235+90cd7b4",
                    "binaries": [
                        "breadbug",
                        "breadd",
                        "bread-emit",
                        "bread-module-host",
                        "bread",
                    ],
                    "services": ["breadd.service"],
                },
                "breadbox": {
                    "name": "breadbox",
                    "version": "0.4.1-dev.20261020203107+791a42d",
                    "binaries": ["breadbox-sync", "breadbox"],
                    "services": ["breadbox-sync.service"],
                },
                "breadclip": {
                    "name": "breadclip",
                    "version": "0.3.1-dev.20261020204016+04df633",
                    "binaries": ["breadclipd", "breadclip"],
                    "services": ["breadclipd.service"],
                },
                "breadcrumbs": {
                    "name": "breadcrumbs",
                    "version": "2.2.1-dev.20261021030505+df95231",
                    "binaries": ["breadcrumbs"],
                    "services": [],
                },
            },
        },
    )

    # Newer cached index: adds breadbug-status units to bread and a whole
    # package (breadshot) that is not installed. breadsearch is also absent
    # from the snapshot but none of its binaries are in the lockfile, so it
    # must not produce a warning.
    write_json(
        index_path,
        {
            "packages": {
                "bread": {
                    "version": "0.9.1-dev.20261020032941+6e2010a",
                    "binaries": [
                        {"name": "breadbug-x86_64"},
                        {"name": "breadd-x86_64"},
                        {"name": "bread-emit-x86_64"},
                        {"name": "bread-module-host-x86_64"},
                        {"name": "bread-x86_64"},
                    ],
                    "services": [
                        "breadd.service",
                        "breadbug-status.service",
                        "breadbug-status.timer",
                    ],
                },
                "breadbox": {
                    "version": "0.4.1-dev.20261020203107+791a42d",
                    "binaries": [
                        {"name": "breadbox-sync-x86_64"},
                        {"name": "breadbox-x86_64"},
                    ],
                    "services": ["breadbox-sync.service"],
                },
                "breadclip": {
                    "version": "0.3.1-dev.20261020204016+04df633",
                    "binaries": [
                        {"name": "breadclipd-x86_64"},
                        {"name": "breadclip-x86_64"},
                    ],
                    "services": ["breadclipd.service"],
                },
                "breadcrumbs": {
                    "version": "2.2.1-dev.20261021030505+df95231",
                    "binaries": [{"name": "breadcrumbs-x86_64"}],
                    "services": [],
                },
                "breadshot": {
                    "version": "0.1.4-dev.20261020205530+8130ba7",
                    "binaries": [{"name": "breadshot-x86_64"}],
                    "services": ["breadshot.service"],
                },
                "breadsearch": {
                    "version": "0.2.0",
                    "binaries": [
                        {"name": "breadmill-x86_64"},
                        {"name": "breadsearch-x86_64"},
                    ],
                    "services": ["breadmill.service"],
                },
            },
        },
    )

    write_unit(unit_dir, "breadd.service", "breadd")
    write_unit(unit_dir, "breadbox-sync.service", "breadbox-sync")
    write_unit(unit_dir, "breadclipd.service", "breadclipd")
    # installed.json omits this service; the local unit must still be baked.
    write_unit(unit_dir, "breadcrumbs.service", "breadcrumbs")
    # Not in the lockfile: must never be selected.
    write_unit(unit_dir, "llamacpp.service", "llamacpp")

    result = subprocess.run(
        [
            sys.executable,
            str(SELECT),
            str(installed_path),
            str(index_path),
            str(unit_dir),
            *BINS,
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    selected = result.stdout.split()
    expected = [
        "breadbox-sync.service",
        "breadclipd.service",
        "breadcrumbs.service",
        "breadd.service",
    ]
    assert selected == expected, selected
    assert "breadbug-status.service" not in selected
    assert "breadbug-status.timer" not in selected
    assert "breadshot.service" not in selected

    # The bake loop aborts when a selected unit has no file. Every selected
    # unit here has one, so the snapshot-only list cannot trigger that abort.
    for unit in selected:
        assert (unit_dir / unit).is_file(), unit

    warnings = result.stderr
    for skipped in (
        "breadbug-status.service",
        "breadbug-status.timer",
        "breadshot.service",
    ):
        assert f"skipping" in warnings and skipped in warnings, warnings
    assert "breadshot" in warnings and "not installed" in warnings, warnings
    assert "breadbug-status.service" in warnings and "installed snapshot" in warnings, (
        warnings
    )
    assert "breadmill.service" not in warnings, warnings
    assert "ERROR" not in result.stdout + result.stderr, result.stdout + result.stderr

print("BOS-97 BAKE_FROM_HOME service selection uses the installed snapshot only")
