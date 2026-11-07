#!/usr/bin/env python3
"""Static check: breadhelp-tour is selected from the bake manifest when due.

No ISO build, no network. Loads the real lockfile and the real
ci-stage-bakery.py selection helpers, then exercises the two index shapes:
breadhelp 0.2.7 (only the help window) and 0.3.0 (help window plus tour).
BOS-105 regressed silently whenever the tour was absent, so the manifest
stays optional until the pin reaches 0.3.0 and then the guard must fail.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LOCKFILE = REPO / "iso" / "bread-lockfile.toml"
STAGER = REPO / "scripts" / "ci-stage-bakery.py"

spec = importlib.util.spec_from_file_location("ci_stage_bakery", STAGER)
stager = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(stager)

problems: list[str] = []


def check(condition: bool, message: str) -> None:
    if not condition:
        problems.append(message)


required, optional, versions = stager.load_lockfile(LOCKFILE)
check("breadhelp-tour" in optional, "breadhelp-tour must be in optional_bins")
check(
    "breadhelp-tour" not in required,
    "breadhelp-tour must not be required while 0.2.7 is the stable release",
)
check(
    not (set(required) & set(optional)), "a bin must not be both required and optional"
)
check("breadhelp" in required, "breadhelp itself stays required")


def fake_package(version: str, bins: list[str]) -> dict:
    return {
        "version": version,
        "binaries": [
            {
                "name": f"{name}-x86_64",
                "dl_url": f"https://dl.breadway.dev/breadhelp/{version}/{name}-x86_64",
                "sha256": "0" * 64,
            }
            for name in bins
        ],
    }


# What the stager sees in the signed stable index today.
old = stager.bin_index({"breadhelp": fake_package("0.2.7", ["breadhelp"])})
check("breadhelp" in old, "0.2.7 index must publish the help window")
check(
    "breadhelp-tour" not in old,
    "0.2.7 index has no tour, so the optional bin must be skippable",
)

# What it sees once breadhelp 0.3.0 is published: both bins belong to breadhelp.
new = stager.bin_index(
    {"breadhelp": fake_package("0.3.0", ["breadhelp", "breadhelp-tour"])}
)
check("breadhelp-tour" in new, "0.3.0 index must publish the tour")
check(
    new.get("breadhelp-tour", (None,))[0] == "breadhelp",
    "the tour must select the breadhelp package",
)

# The guard: dormant below 0.3.0, fatal at or above it when the tour is absent.
check(
    stager.breadhelp_tour_error("0.2.7", ["breadhelp"]) is None,
    "0.2.7 without tour must not fail",
)
check(
    stager.breadhelp_tour_error(None, ["breadhelp"]) is None,
    "an unpinned unknown version must not fail",
)
check(
    stager.breadhelp_tour_error("0.3.0", ["breadhelp"]) is not None,
    "0.3.0 without tour must fail",
)
check(
    stager.breadhelp_tour_error("0.3.0", ["breadhelp", "breadhelp-tour"]) is None,
    "0.3.0 with tour must pass",
)
check(
    stager.breadhelp_tour_error("0.10.0", ["breadhelp"]) is not None,
    "numeric compare: 0.10.0 is newer than 0.3.0",
)
check(
    stager.breadhelp_tour_error("1.0.0", ["breadhelp", "breadhelp-tour"]) is None,
    "1.0.0 with tour must pass",
)

# The pin may lag the release while the signed index catches up, but if it is
# already 0.3.0 or newer the manifest must be able to satisfy the guard.
pin = versions.get("breadhelp")
if pin:
    check(
        stager.breadhelp_tour_error(pin, ["breadhelp", "breadhelp-tour"]) is None,
        f"lockfile pin {pin} must pass the guard with the tour staged",
    )

# breadhelp 0.3.0's runtime deps: Tauri's GTK3/WebKitGTK side plus the GTK4
# tour overlay. webkit2gtk-4.1 pulls gtk3, libsoup3 and glib2 transitively,
# so the direct pacman entries below are what the image must keep.
packages = set((REPO / "iso" / "packages.x86_64").read_text().split())
for package in ("webkit2gtk-4.1", "gtk4", "gtk4-layer-shell", "hicolor-icon-theme"):
    check(
        package in packages,
        f"iso/packages.x86_64 must keep {package} for breadhelp 0.3.0",
    )

for script in ("build-local.sh", "scripts/ci-verify-bake.sh"):
    result = subprocess.run(
        ["bash", "-n", str(REPO / script)], capture_output=True, text=True
    )
    check(result.returncode == 0, f"bash -n {script}: {result.stderr.strip()}")

if problems:
    print("FAIL: breadhelp-tour bake manifest is not consistent")
    for problem in problems:
        print(f"  - {problem}")
    raise SystemExit(1)

print(
    "PASS: breadhelp-tour is optional in the manifest, selected from a 0.3.0 index, "
    "and a missing tour fails the bake once the pin is 0.3.0+"
)
