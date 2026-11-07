#!/usr/bin/env python3
"""Regression checks for the live-only coredump policy (BOS-107).

BOS-99 disables coredumps on the live medium so a crash loop cannot fill the
RAM-backed overlay. unpackfs copies the whole airootfs to the target, so the
drop-in reached installed systems and silently stopped them writing cores,
which would hide exactly the crashes breadbug is meant to capture.

Pins:
1. The live drop-in still disables coredumps on the live medium.
2. Calamares post-install removes that drop-in from the installed target.
3. Executing that removal against a fabricated target really deletes it.

Run from the repository root:
    python3 scripts/test-live-coredump-scope.py
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DROP_IN = ROOT / "iso/airootfs/etc/systemd/coredump.conf.d/90-bos-live.conf"
POST_INSTALL = ROOT / "iso/airootfs/etc/calamares/post-install.sh"
DROP_IN_PATH = "/etc/systemd/coredump.conf.d/90-bos-live.conf"


def failures() -> list[str]:
    problems: list[str] = []

    if not DROP_IN.is_file():
        problems.append(f"{DROP_IN.relative_to(ROOT)}: missing (live cap dropped)")
    else:
        text = DROP_IN.read_text(encoding="utf-8")
        for setting in ("Storage=none", "ProcessSizeMax=0", "ExternalSizeMax=0"):
            if not re.search(rf"^{re.escape(setting)}$", text, re.MULTILINE):
                problems.append(
                    f"{DROP_IN.relative_to(ROOT)}: {setting} is no longer set on the "
                    "live medium"
                )

    post_install = POST_INSTALL.read_text(encoding="utf-8")
    removal = re.search(
        rf"^[ \t]*rm[ \t]+-f[ \t]+{re.escape(DROP_IN_PATH)}[ \t]*$",
        post_install,
        re.MULTILINE,
    )
    if not removal:
        problems.append(
            f"{POST_INSTALL.relative_to(ROOT)}: does not remove {DROP_IN_PATH}, so an "
            "installed system inherits the live coredump disable"
        )
    else:
        with tempfile.TemporaryDirectory(prefix="bos-coredump-scope.") as tmp:
            target = Path(tmp)
            installed_drop_in = target / DROP_IN_PATH.lstrip("/")
            installed_drop_in.parent.mkdir(parents=True)
            shutil.copy2(DROP_IN, installed_drop_in)
            command = removal.group(0).replace(DROP_IN_PATH, str(installed_drop_in))
            result = subprocess.run(
                ["bash", "-c", command], capture_output=True, text=True
            )
            if result.returncode != 0 or installed_drop_in.exists():
                problems.append(
                    f"{POST_INSTALL.relative_to(ROOT)}: the removal command did not "
                    f"delete a fabricated installed copy (rc={result.returncode})"
                )

    return problems


def main() -> int:
    problems = failures()
    for problem in problems:
        print(f"FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(
        "PASS: the live medium caps coredumps and post-install removes the "
        "drop-in from the installed target"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
