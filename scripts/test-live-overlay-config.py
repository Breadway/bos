#!/usr/bin/env python3
"""Regression checks for the live-medium writable overlay (BOS-99).

The live ISO used a 256 MiB archiso cow space. Ordinary use (downloads,
browser cache, a crash loop) filled it. This test pins the mitigations:

1. Every active boot entry passes a cow_spacesize large enough to be useful.
2. The live image disables core dumps.
3. The live user's Downloads directory lives on the tmpfs.

Run from the repository root:
    python3 scripts/test-live-overlay-config.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ACTIVE_CMDLINE_FILES = (
    "iso/grub/grub.cfg",
    "iso/grub/loopback.cfg",
    "iso/syslinux/archiso_sys-linux.cfg",
)

MIN_COW_PERCENT = 20


def failures() -> list[str]:
    problems: list[str] = []

    for relative in ACTIVE_CMDLINE_FILES:
        text = (ROOT / relative).read_text(encoding="utf-8")
        entries = [
            line
            for line in text.splitlines()
            if "archisosearchuuid" in line or "img_loop=" in line
        ]
        if not entries:
            problems.append(f"{relative}: no live boot entries found")
            continue
        for line in entries:
            match = re.search(r"cow_spacesize=(\d+)%", line)
            if not match:
                problems.append(
                    f"{relative}: live entry without cow_spacesize: {line.strip()}"
                )
            elif int(match.group(1)) < MIN_COW_PERCENT:
                problems.append(
                    f"{relative}: cow_spacesize={match.group(1)}% is below {MIN_COW_PERCENT}%"
                )

    coredump = ROOT / "iso/airootfs/etc/systemd/coredump.conf.d/90-bos-live.conf"
    if not coredump.exists():
        problems.append(f"{coredump.relative_to(ROOT)}: missing")
    elif not re.search(
        r"^Storage=none$", coredump.read_text(encoding="utf-8"), re.MULTILINE
    ):
        problems.append(f"{coredump.relative_to(ROOT)}: Storage=none is not set")

    live_setup = ROOT / "iso/airootfs/usr/local/bin/bos-live-setup"
    if "liveuser-downloads" not in live_setup.read_text(encoding="utf-8"):
        problems.append(
            f"{live_setup.relative_to(ROOT)}: Downloads is not redirected to tmpfs"
        )

    return problems


def main() -> int:
    problems = failures()
    for problem in problems:
        print(f"FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(
        "PASS: live overlay size, coredump cap and Downloads redirection are configured"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
