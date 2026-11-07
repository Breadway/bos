#!/usr/bin/env python3
"""Resolve breadhelp guides the way the packaged runtime does (BOS-112).

breadhelp 0.2.7 scans /usr/share/breadhelp/content plus the user's
~/.local/share/breadhelp/content (or $XDG_DATA_HOME/breadhelp/content). The
BOS bake installs the package data under /usr/local/share, which the runtime
never read: verify-bake only checked that the baked directory existed, so a
hollow Learn tab shipped.

This check runs against a staged or mounted image root, maps absolute
symlinks inside that root (the chroot view, not the host view), applies the
same two-root merge and meta.toml requirements, and requires a non-empty
guide list with at least one readable body.

Usage:
    python3 scripts/check-breadhelp-content.py /tmp/bos-iso-stage/airootfs
    python3 scripts/check-breadhelp-content.py ROOT --home /home/liveuser
"""

from __future__ import annotations

import argparse
import os
import sys
import tomllib
from pathlib import Path

SYSTEM_CONTENT = "/usr/share/breadhelp/content"
SKIP_IDS = {"_symptoms"}


def resolve_under(root: Path, absolute: str) -> Path:
    """Resolve a path inside an image root, including absolute symlinks.

    A symlink whose target is /usr/local/share/... must resolve inside the
    staged image, not on the host running the check.
    """
    path = root / absolute.lstrip("/")
    for _ in range(16):
        if not path.is_symlink():
            return path
        target = os.readlink(path)
        path = (
            root / target.lstrip("/") if os.path.isabs(target) else path.parent / target
        )
        path = Path(os.path.normpath(path))
    return path


def user_content_root(root: Path, home: str | None, xdg_data_home: str | None) -> Path:
    if xdg_data_home:
        base = resolve_under(root, xdg_data_home)
    else:
        base = resolve_under(root, home or "/home/liveuser") / ".local/share"
    return base / "breadhelp/content"


def scan(
    content_root: Path, source: str, problems: list[str]
) -> dict[tuple[str, str], Path]:
    guides: dict[tuple[str, str], Path] = {}
    if not content_root.is_dir():
        problems.append(f"{source} content directory missing: {content_root}")
        return guides

    for category_dir in sorted(content_root.iterdir()):
        if not category_dir.is_dir():
            continue
        for guide_dir in sorted(category_dir.iterdir()):
            if not guide_dir.is_dir() or guide_dir.name in SKIP_IDS:
                continue
            meta_path = guide_dir / "meta.toml"
            try:
                meta = tomllib.loads(meta_path.read_text(encoding="utf-8"))
            except (OSError, tomllib.TOMLDecodeError) as error:
                problems.append(f"{meta_path}: unreadable guide metadata ({error})")
                continue
            category = str(meta.get("category") or category_dir.name)
            guides[(category, guide_dir.name)] = guide_dir
    return guides


def readable_body(guide_dir: Path) -> str | None:
    for name in ("content.beginner.md", "content.md"):
        body = guide_dir / name
        if body.is_file():
            try:
                text = body.read_text(encoding="utf-8")
            except OSError:
                continue
            if text.strip():
                return name
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", default="/", help="image root (default /)")
    parser.add_argument(
        "--home",
        default=None,
        help="HOME path inside the image (default /home/liveuser)",
    )
    parser.add_argument(
        "--xdg-data-home",
        default=None,
        help="XDG_DATA_HOME path inside the image, if set",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    problems: list[str] = []
    system_root = resolve_under(root, SYSTEM_CONTENT)
    user_root = user_content_root(root, args.home, args.xdg_data_home)

    system_guides = scan(system_root, "system", problems)
    user_guides = scan(user_root, "user", problems) if user_root.is_dir() else {}

    merged = dict(system_guides)
    merged.update(user_guides)

    if not merged:
        problems.append(f"resolved 0 guides from {system_root} and {user_root}")

    body_guide = None
    for key in sorted(merged):
        body_name = readable_body(merged[key])
        if body_name:
            body_guide = (key, body_name)
            break

    if merged and not body_guide:
        problems.append(
            "no merged guide has a readable content.md or content.beginner.md"
        )

    for problem in problems:
        print(f"FAIL: {problem}", file=sys.stderr)
    if problems:
        return 1

    assert body_guide is not None
    (category, guide_id), body_name = body_guide
    print(
        f"PASS: resolved {len(merged)} breadhelp guides "
        f"({len(system_guides)} system, {len(user_guides)} user); "
        f"readable body {category}/{guide_id}/{body_name}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
