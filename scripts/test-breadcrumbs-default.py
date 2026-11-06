#!/usr/bin/env python3
"""Static check: the shipped breadcrumbs default is a clean fresh install.

No ISO build or boot needed. breadcrumbs creates home/work/away places when it
generates its own skeleton; bos-settings then calls the empty home/work pair
"left over from an older version" on a brand-new machine (BOS-103). BOS ships
a default that avoids that shape, and this checks it stays that way.

The leftover predicate mirrors bos-settings' Places card: a core place with no
networks, detection, bootstrap, exit node or Tailscale use that is neither the
fallback nor the current place is shown as a leftover.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT = (
    REPO
    / "iso"
    / "airootfs"
    / "etc"
    / "skel"
    / ".config"
    / "breadcrumbs"
    / "breadcrumbs.toml"
)
CORE_PLACES = {"home", "work", "away"}

# breadcrumbs' Settings::default() / documented fresh-install defaults.
EXPECTED_SETTINGS = {
    "default_profile": "away",
    "dns": "1.1.1.1",
    "connect_wait": 8,
    "exit_node": "",
    "watch_interval": 12,
    "connectivity_url": "http://connectivitycheck.gstatic.com/generate_204",
    "ping_host": "1.1.1.1",
}

problems: list[str] = []


def fail(message: str) -> None:
    problems.append(message)


if not DEFAULT.is_file():
    print(f"FAIL: shipped breadcrumbs default missing: {DEFAULT.relative_to(REPO)}")
    raise SystemExit(1)

with DEFAULT.open("rb") as f:
    cfg = tomllib.load(f)

settings = cfg.get("settings") or {}
places = cfg.get("profiles") or {}

# Fresh installs must not be re-backfilled with empty core places. breadcrumbs
# only skips ensure_core_profiles() once this flag is set.
if settings.get("core_profiles_initialized") is not True:
    fail(
        "settings.core_profiles_initialized must be true, or breadcrumbs backfills empty home/work places"
    )

for key, want in EXPECTED_SETTINGS.items():
    if settings.get(key) != want:
        fail(
            f"settings.{key} = {settings.get(key)!r}, expected documented default {want!r}"
        )

default_place = settings.get("default_profile")
if default_place not in places:
    fail(f"settings.default_profile {default_place!r} is not one of the shipped places")


# Mirror bos-settings' Places card: name a place only if it has a real purpose.
def is_leftover(name: str, place: dict) -> bool:
    return (
        name in CORE_PLACES
        and not place.get("networks")
        and not place.get("detect_ssids")
        and not place.get("bootstrap")
        and not place.get("exit_node")
        and not place.get("tailscale")
        and name != default_place
        and name != cfg.get("current")  # no current place in a shipped default
    )


for name, place in places.items():
    if is_leftover(name, place):
        fail(
            f"shipped place {name!r} has no purpose and would show as a leftover in bos-settings"
        )

away = places.get("away") or {}
if away.get("include_all_known") is not True:
    fail(
        "the away fallback place must keep include_all_known = true so it joins saved networks"
    )
if away.get("networks"):
    fail("the away fallback place must not pin SSIDs in a fresh-install default")

# The old stale shape used [[profile]]/ssids, which breadcrumbs does not read.
if "profile" in cfg:
    fail("legacy [[profile]] table found; breadcrumbs reads [profiles.<name>]")

if problems:
    print("FAIL: shipped breadcrumbs default is not a clean fresh install")
    for problem in problems:
        print(f"  - {problem}")
    raise SystemExit(1)

print(
    f"PASS: breadcrumbs default has {len(places)} place(s) and no empty core-place leftovers"
)
