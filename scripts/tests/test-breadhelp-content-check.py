#!/usr/bin/env python3
"""Fixture tests for scripts/check-breadhelp-content.py (BOS-112).

Builds good and bad baked-image layouts in temporary directories and asserts
that the resolver accepts a real system layout, merges a user overlay, and
rejects the layout that shipped a hollow Learn tab (content only under
/usr/local/share with no /usr/share/breadhelp/content path).
"""

from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile

SCRIPT = (
    pathlib.Path(__file__).resolve().parents[2] / "scripts/check-breadhelp-content.py"
)
SYSTEM_CONTENT = pathlib.Path("usr/share/breadhelp/content")
IMAGE_CONTENT = pathlib.Path("usr/local/share/breadhelp/content")


def write_guide(
    content_root: pathlib.Path, category: str, guide: str, body: str = "Body.\n"
) -> None:
    guide_dir = content_root / category / guide
    guide_dir.mkdir(parents=True)
    (guide_dir / "meta.toml").write_text(f'title = "{guide}"\n', encoding="utf-8")
    (guide_dir / "content.md").write_text(body, encoding="utf-8")


def link_system_content(root: pathlib.Path) -> None:
    link = root / SYSTEM_CONTENT
    link.parent.mkdir(parents=True)
    link.symlink_to("/usr/local/share/breadhelp/content")


def run(root: pathlib.Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(root), *args],
        capture_output=True,
        text=True,
    )


with tempfile.TemporaryDirectory(prefix="bos-112-test.") as tmp:
    tmp_path = pathlib.Path(tmp)

    # Good: the baked layout plus the compatibility path the runtime reads.
    good = tmp_path / "good"
    write_guide(good / IMAGE_CONTENT, "getting-started", "01-what-is-bos")
    link_system_content(good)
    result = run(good)
    assert result.returncode == 0, (result.stdout, result.stderr)
    assert "resolved 1 breadhelp guides" in result.stdout, result.stdout
    assert "readable body getting-started/01-what-is-bos/content.md" in result.stdout, (
        result.stdout
    )

    # Good: a user overlay is merged on top of the system content.
    write_guide(
        good / "home/testuser/.local/share/breadhelp/content",
        "customization",
        "01-local-guide",
    )
    result = run(good, "--home", "/home/testuser")
    assert result.returncode == 0, (result.stdout, result.stderr)
    assert "resolved 2 breadhelp guides (1 system, 1 user)" in result.stdout, (
        result.stdout
    )

    # Bad: content baked at /usr/local/share only, exactly the HELP-20 layout
    # that the old directory-existence check accepted.
    bad_path = tmp_path / "bad-path"
    write_guide(bad_path / IMAGE_CONTENT, "getting-started", "01-what-is-bos")
    result = run(bad_path)
    assert result.returncode == 1, result.stdout
    assert "system content directory missing" in result.stderr, result.stderr

    # Bad: the path exists but every guide body is empty.
    bad_body = tmp_path / "bad-body"
    write_guide(
        bad_body / IMAGE_CONTENT, "getting-started", "01-what-is-bos", body="\n"
    )
    link_system_content(bad_body)
    result = run(bad_body)
    assert result.returncode == 1, result.stdout
    assert "no merged guide has a readable" in result.stderr, result.stderr

    # Bad: unparseable metadata means the runtime would list no guides.
    bad_meta = tmp_path / "bad-meta"
    guide_dir = bad_meta / IMAGE_CONTENT / "getting-started" / "01-broken"
    guide_dir.mkdir(parents=True)
    (guide_dir / "meta.toml").write_text("title = [unterminated\n", encoding="utf-8")
    (guide_dir / "content.md").write_text("Body.\n", encoding="utf-8")
    link_system_content(bad_meta)
    result = run(bad_meta)
    assert result.returncode == 1, result.stdout
    assert "unreadable guide metadata" in result.stderr, result.stderr

print(
    "breadhelp content resolution fixture tests passed (good, user overlay, missing path, empty body, bad meta)"
)
