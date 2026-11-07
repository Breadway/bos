#!/usr/bin/env python3
"""Static check: every live-skel launcher points at a binary the ISO bakes.

No ISO build or boot needed. A stale .desktop entry is invisible to the build:
the launcher lists the app, clicking it closes the launcher, and nothing runs
(BOS-102's breadmon.desktop was exactly that). This walks the shipped skel
entries, resolves each Exec= against the bakery lockfile plus the hand-written
/usr/local/bin scripts, and rejects any reference to a deprecated app.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
AIROOTFS = REPO / "iso" / "airootfs"
SKEL = AIROOTFS / "etc" / "skel"
APPS = SKEL / ".local" / "share" / "applications"
LOCKFILE = REPO / "iso" / "bread-lockfile.toml"

# Deprecated apps must not appear anywhere in the live skel: an entry whose
# Exec is missing, a pinned launcher default, or a stale module comment.
DEPRECATED = {"breadmon"}

problems: list[str] = []


def fail(message: str) -> None:
    problems.append(message)


with LOCKFILE.open("rb") as f:
    lock = tomllib.load(f)
required = lock.get("required_bins") or lock.get("bins") or lock.get("binaries") or []
optional = lock.get("optional_bins") or []
baked = set(required) | set(optional)
if not baked:
    fail(f"{LOCKFILE}: no bakery bins declared")

entries = sorted(APPS.glob("*.desktop"))
if not entries:
    fail(f"{APPS}: no desktop entries found")

for entry in entries:
    text = entry.read_text()
    exec_lines = [line for line in text.splitlines() if line.startswith("Exec=")]
    if len(exec_lines) != 1:
        fail(f"{entry.name}: expected exactly one Exec= line, found {len(exec_lines)}")
        continue
    argv = exec_lines[0][len("Exec=") :].split()
    if not argv:
        fail(f"{entry.name}: empty Exec=")
        continue
    target = argv[0]
    if target.startswith("/"):
        # Absolute command: it must exist in the staged airootfs tree.
        ok = (AIROOTFS / target.lstrip("/")).is_file()
    else:
        # Bare command: a bakery bin baked into /usr/local/bin, or a script
        # committed under airootfs/usr/local/bin.
        ok = target in baked or (AIROOTFS / "usr" / "local" / "bin" / target).is_file()
    if not ok:
        fail(
            f"{entry.name}: Exec target {target!r} is neither a lockfile bin nor in /usr/local/bin"
        )

for path in sorted(SKEL.rglob("*")):
    for bad in DEPRECATED:
        if bad in path.name:
            fail(f"deprecated app name {bad!r} in skel path {path.relative_to(REPO)}")
        if not path.is_file():
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        if bad in text:
            fail(f"deprecated app name {bad!r} referenced in {path.relative_to(REPO)}")

if problems:
    print("FAIL: shipped skel launchers are not all runnable")
    for problem in problems:
        print(f"  - {problem}")
    raise SystemExit(1)

print(
    f"PASS: {len(entries)} skel desktop entries resolve to baked binaries; no deprecated app references"
)
