#!/usr/bin/env python3
"""Regression checks for Bluetooth in the live image (BOS-100).

A fresh live session reported "The Bluetooth service is off" even with an
hci0 controller present: the live airootfs never enabled bluetooth.service,
although the installer enabled it on the target. bluez's unit carries
ConditionPathIsDirectory=/sys/class/bluetooth, so enabling it by default is
safe on machines without a radio: systemd skips the unit instead of failing.

Pins:
1. The live airootfs enables bluetooth.service in multi-user.target.wants.
2. post-install.sh still enables it on the installed system.

Run from the repository root:
    python3 scripts/test-live-bluetooth.py
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

WANTS_LINK = (
    ROOT / "iso/airootfs/etc/systemd/system/multi-user.target.wants/bluetooth.service"
)
EXPECTED_TARGET = "/usr/lib/systemd/system/bluetooth.service"
POST_INSTALL = ROOT / "iso/airootfs/etc/calamares/post-install.sh"


def failures() -> list[str]:
    problems: list[str] = []

    if not WANTS_LINK.is_symlink():
        problems.append(
            f"{WANTS_LINK.relative_to(ROOT)}: missing or not a symlink; the live "
            "session starts with bluetooth.service disabled"
        )
    else:
        target = os.readlink(WANTS_LINK)
        if target != EXPECTED_TARGET:
            problems.append(
                f"{WANTS_LINK.relative_to(ROOT)}: points at {target!r}, expected "
                f"{EXPECTED_TARGET!r}"
            )

    post_install = POST_INSTALL.read_text(encoding="utf-8")
    enable_loop = re.search(
        r"for unit in\s+(?P<units>.*?);\s*do", post_install, re.DOTALL
    )
    if not enable_loop or "bluetooth.service" not in enable_loop.group("units"):
        problems.append(
            f"{POST_INSTALL.relative_to(ROOT)}: the installed-system enable loop "
            "no longer includes bluetooth.service"
        )

    return problems


def main() -> int:
    problems = failures()
    for problem in problems:
        print(f"FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(
        "PASS: live image and installed system both enable bluetooth.service "
        "(conditional on /sys/class/bluetooth in bluez's unit)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
