#!/usr/bin/env python3
"""Regression checks for the live image's locales (BOS-101).

The live ISO shipped only C, C.utf8 and POSIX because glibc's locale.gen is
entirely commented out and mkarchiso never runs locale-gen. Language and
Formats then offered nothing but "Computer default (C)" and applications
formatted dates and numbers in the C locale.

Pins:
1. iso/airootfs/etc/locale.gen generates at least en_US.UTF-8.
2. iso/airootfs/etc/locale.conf LANG matches a locale generated there.
3. The airootfs build hook actually runs locale-gen.
4. Calamares still generates the locale chosen at install time on the target.

Run from the repository root:
    python3 scripts/test-live-locale.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

REQUIRED_LOCALE = "en_US.UTF-8"
LOCALE_GEN = ROOT / "iso/airootfs/etc/locale.gen"
LOCALE_CONF = ROOT / "iso/airootfs/etc/locale.conf"
CUSTOMIZE = ROOT / "iso/airootfs/root/customize_airootfs.sh"
CALAMARES_SETTINGS = ROOT / "iso/airootfs/etc/calamares/settings.conf"
CALAMARES_LOCALE = ROOT / "iso/airootfs/etc/calamares/modules/locale.conf"


def active_locales(text: str) -> list[str]:
    locales = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 2:
            locales.append(parts[0])
    return locales


def failures() -> list[str]:
    problems: list[str] = []

    if not LOCALE_GEN.is_file():
        problems.append(f"{LOCALE_GEN.relative_to(ROOT)}: missing")
        locales: list[str] = []
    else:
        locales = active_locales(LOCALE_GEN.read_text(encoding="utf-8"))
        if REQUIRED_LOCALE not in locales:
            problems.append(
                f"{LOCALE_GEN.relative_to(ROOT)}: {REQUIRED_LOCALE} is not generated "
                f"(active: {', '.join(locales) or 'none'})"
            )

    lang = None
    if not LOCALE_CONF.is_file():
        problems.append(f"{LOCALE_CONF.relative_to(ROOT)}: missing")
    else:
        match = re.search(
            r"^LANG=([^\s#]+)$",
            LOCALE_CONF.read_text(encoding="utf-8"),
            re.MULTILINE,
        )
        if not match:
            problems.append(f"{LOCALE_CONF.relative_to(ROOT)}: no LANG= line")
        else:
            lang = match.group(1)
            if lang not in locales:
                problems.append(
                    f"{LOCALE_CONF.relative_to(ROOT)}: LANG={lang} is not generated "
                    f"by {LOCALE_GEN.relative_to(ROOT)}"
                )

    if lang != REQUIRED_LOCALE:
        problems.append(
            f"{LOCALE_CONF.relative_to(ROOT)}: LANG is {lang!r}, expected "
            f"{REQUIRED_LOCALE!r}"
        )

    if not CUSTOMIZE.is_file() or not re.search(
        r"^\s*locale-gen\s*$", CUSTOMIZE.read_text(encoding="utf-8"), re.MULTILINE
    ):
        problems.append(
            f"{CUSTOMIZE.relative_to(ROOT)}: does not run locale-gen, so the shipped "
            "locale.gen is never generated"
        )

    if not CALAMARES_SETTINGS.is_file() or not re.search(
        r"^\s*-\s*localecfg\s*$",
        CALAMARES_SETTINGS.read_text(encoding="utf-8"),
        re.MULTILINE,
    ):
        problems.append(
            f"{CALAMARES_SETTINGS.relative_to(ROOT)}: the exec sequence no longer runs "
            "localecfg, so an installed system would not generate its chosen locale"
        )

    if not CALAMARES_LOCALE.is_file() or not re.search(
        r"^localeGenPath:\s*\"/etc/locale\.gen\"\s*$",
        CALAMARES_LOCALE.read_text(encoding="utf-8"),
        re.MULTILINE,
    ):
        problems.append(
            f"{CALAMARES_LOCALE.relative_to(ROOT)}: localeGenPath is not /etc/locale.gen"
        )

    return problems


def main() -> int:
    problems = failures()
    for problem in problems:
        print(f"FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(
        "PASS: live image generates en_US.UTF-8, LANG matches it, and Calamares "
        "still generates the install-time locale"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
