#!/usr/bin/env python3
"""Stage bakery artifacts from the verified stable index into $LAPTOP_HOME.

Used by .forgejo/workflows/release-iso.yml so the ISO bake does not invent
binaries, fake installed.json, or cargo-build bread-theme. Never downloads
breadcast or breadarr.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urljoin, urlparse

INDEX_URL = "https://dl.breadway.dev/index.json"
DL_ORIGIN = "https://dl.breadway.dev"
# Same key as bread-ecosystem/scripts/get.sh and bakery/src/manifest.rs.
MINISIGN_PUBKEY = "RWTBR8w/IJ+jaylOv80b52DzekKbSR2CvOVGvzB0ipGBaMhJPAOiEWq8"
BLOCKED = frozenset({"breadcast", "breadarr"})
ARCH_SUFFIXES = ("-x86_64", "-aarch64", "-arm64", "-armv7")


def die(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    raise SystemExit(1)


def dest_name(name: str) -> str:
    for suf in ARCH_SUFFIXES:
        if name.endswith(suf):
            return name[: -len(suf)]
    return name


def valid_name(name: str) -> bool:
    return bool(name) and "/" not in name and name not in (".", "..")


def load_versions(data: dict, path: Path) -> dict[str, str]:
    """Optional [versions] map and/or [[pin]] tables → package → version."""
    versions: dict[str, str] = {}

    raw_map = data.get("versions")
    if raw_map is not None:
        if not isinstance(raw_map, dict):
            die(f"{path}: [versions] must be a table of package = \"version\"")
        for pkg, ver in raw_map.items():
            if not isinstance(pkg, str) or not valid_name(pkg):
                die(f"{path}: invalid [versions] package {pkg!r}")
            if not isinstance(ver, str) or not valid_name(ver):
                die(f"{path}: invalid [versions] version for {pkg}: {ver!r}")
            versions[pkg] = ver

    pins = data.get("pin")
    if pins is not None:
        if not isinstance(pins, list):
            die(f"{path}: [[pin]] must be an array of tables")
        for i, entry in enumerate(pins):
            if not isinstance(entry, dict):
                die(f"{path}: [[pin]] #{i} must be a table")
            pkg = entry.get("package", entry.get("pkg"))
            ver = entry.get("version")
            if not isinstance(pkg, str) or not valid_name(pkg):
                die(f"{path}: [[pin]] #{i}: missing valid package")
            if not isinstance(ver, str) or not valid_name(ver):
                die(f"{path}: [[pin]] #{i}: missing valid version")
            if pkg in versions and versions[pkg] != ver:
                die(f"{path}: conflicting pin for {pkg}: {versions[pkg]} vs {ver}")
            versions[pkg] = ver
    return versions


def load_lockfile(path: Path) -> tuple[list[str], list[str], dict[str, str]]:
    with path.open("rb") as f:
        data = tomllib.load(f)
    required = data.get("required_bins")
    optional = data.get("optional_bins") or []
    if required is None:
        required = data.get("bins") or data.get("binaries")
    if not isinstance(required, list) or not required:
        die(f"{path}: missing non-empty required_bins (or bins) list")
    if not isinstance(optional, list):
        die(f"{path}: optional_bins must be a list")
    for label, names in (("required_bins", required), ("optional_bins", optional)):
        for b in names:
            if not isinstance(b, str) or not valid_name(b):
                die(f"{path}: invalid {label} name {b!r}")
            if b in BLOCKED:
                die(f"{path}: {b} is not shipped on the ISO")
    overlap = set(required) & set(optional)
    if overlap:
        die(f"{path}: bins in both required and optional: {sorted(overlap)}")
    return list(required), list(optional), load_versions(data, path)


def pinned_artifact_url(pkg: str, version: str, filename: str) -> str:
    if not valid_name(pkg) or not valid_name(version) or not valid_name(filename):
        die(f"refusing pinned URL with unsafe path {pkg}/{version}/{filename}")
    return f"{DL_ORIGIN}/{pkg}/{version}/{filename}"


def package_base_url(pkg_name: str, versions: dict[str, str], first_url: str) -> str:
    pin = versions.get(pkg_name)
    if pin:
        if not valid_name(pkg_name) or not valid_name(pin):
            die(f"refusing pinned version dir {pkg_name}/{pin}")
        return f"{DL_ORIGIN}/{pkg_name}/{pin}/"
    return version_dir(first_url)


def fetch(url: str, dest: Path, *, required: bool = True) -> bool:
    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        urllib.request.urlretrieve(url, dest)
        return True
    except (urllib.error.URLError, OSError) as e:
        if required:
            die(f"download failed: {url}: {e}")
        print(f"WARN: download failed: {url}: {e}", file=sys.stderr)
        if dest.exists():
            dest.unlink()
        return False


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def require_sha256(value: object, what: str) -> str:
    if not isinstance(value, str) or not value.strip():
        die(f"{what}: index sha256 is required and must be non-empty")
    return value.strip().lower()


def verify_sha256(path: Path, expected: str, what: str) -> None:
    actual = sha256_file(path)
    if actual != expected:
        die(f"{what}: sha256 mismatch (expected {expected}, got {actual})")


def version_dir(first_dl_url: str) -> str:
    parsed = urlparse(first_dl_url)
    parent = parsed.path.rsplit("/", 1)[0]
    return f"{parsed.scheme}://{parsed.netloc}{parent}/"


def verify_index(index_path: Path, sig_path: Path) -> None:
    if shutil.which("minisign") is None:
        die("minisign is not installed: refuse to trust an unsigned index")
    cmd = [
        "minisign",
        "-V",
        "-q",
        "-m",
        str(index_path),
        "-x",
        str(sig_path),
        "-P",
        MINISIGN_PUBKEY,
    ]
    result = subprocess.run(cmd, check=False)
    if result.returncode != 0:
        die("index.json minisign verification FAILED: refusing to proceed")
    print("index.json minisign OK")


def bin_index(packages: dict) -> dict[str, tuple[str, dict, dict]]:
    out: dict[str, tuple[str, dict, dict]] = {}
    for pkg_name, pkg in packages.items():
        if pkg_name in BLOCKED:
            continue
        for b in pkg.get("binaries") or []:
            if not isinstance(b, dict):
                continue
            raw = b.get("name")
            if not isinstance(raw, str):
                continue
            dest = dest_name(raw)
            if dest in BLOCKED or pkg_name in BLOCKED:
                continue
            if dest in out and out[dest][0] != pkg_name:
                die(f"index publishes {dest} from both {out[dest][0]} and {pkg_name}")
            out[dest] = (pkg_name, pkg, b)
    return out


def patch_exec_start(text: str, bin_dir: Path) -> str:
    lines = []
    for line in text.splitlines():
        if line.lstrip().startswith("ExecStart="):
            rest = line.split("=", 1)[1]
            argv = rest.split()
            if argv:
                name = os.path.basename(argv[0])
                new_path = bin_dir / name
                if len(argv) == 1:
                    line = f"ExecStart={new_path}"
                else:
                    line = f"ExecStart={new_path} {' '.join(argv[1:])}"
        lines.append(line)
    out = "\n".join(lines)
    if text.endswith("\n"):
        out += "\n"
    return out


def wanted_by(text: str) -> list[str]:
    targets: list[str] = []
    for line in text.splitlines():
        if line.startswith("WantedBy="):
            targets.extend(line.split("=", 1)[1].split())
    return targets or ["default.target"]


def assert_safe_archive(path: Path) -> None:
    with tarfile.open(path, "r:gz") as tf:
        for info in tf.getmembers():
            name = info.name
            if info.issym() or info.islnk():
                die(f"refusing archive with symlink entry {name!r}")
            if name.startswith("/") or any(p in ("..", "") for p in Path(name).parts if p == ".."):
                die(f"refusing archive with unsafe path {name!r}")
            if Path(name).is_absolute() or ".." in Path(name).parts:
                die(f"refusing archive with unsafe path {name!r}")


def stage_file(
    url: str,
    dest: Path,
    sha256: str | None,
    what: str,
    mode: int | None = None,
    *,
    required: bool = True,
) -> bool:
    if not fetch(url, dest, required=required):
        return False
    if sha256 is not None:
        verify_sha256(dest, sha256, what)
    if mode is not None:
        dest.chmod(mode)
    return True


def main() -> int:
    # CI logs mix stdout/stderr; keep them in source order.
    try:
        sys.stdout.reconfigure(line_buffering=True)
        sys.stderr.reconfigure(line_buffering=True)
    except (AttributeError, OSError):
        pass
    repo = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--home",
        default=os.environ.get("LAPTOP_HOME", "/build-home"),
        help="builder home to populate (default: $LAPTOP_HOME or /build-home)",
    )
    parser.add_argument(
        "--lockfile",
        default=str(repo / "iso" / "bread-lockfile.toml"),
    )
    parser.add_argument("--index-url", default=INDEX_URL)
    args = parser.parse_args()

    home = Path(args.home)
    lockfile = Path(args.lockfile)
    if not lockfile.is_file():
        die(f"lockfile missing: {lockfile}")

    required, optional, versions = load_lockfile(lockfile)
    print(
        f"lockfile {lockfile}: {len(required)} required, {len(optional)} optional"
        + (f", {len(versions)} pinned" if versions else "")
    )

    bin_dir = home / ".local" / "bin"
    state_dir = home / ".local" / "state" / "bakery"
    cache_dir = home / ".cache" / "bakery"
    share_dir = home / ".local" / "share"
    unit_dir = home / ".config" / "systemd" / "user"
    for d in (bin_dir, state_dir, cache_dir, share_dir, unit_dir):
        d.mkdir(parents=True, exist_ok=True)

    index_path = cache_dir / "index.json"
    sig_path = cache_dir / "index.json.minisig"
    print(f"fetch {args.index_url}")
    fetch(args.index_url, index_path)
    print(f"fetch {args.index_url}.minisig")
    fetch(args.index_url + ".minisig", sig_path)
    verify_index(index_path, sig_path)

    with index_path.open() as f:
        idx = json.load(f)
    packages = idx.get("packages")
    if not isinstance(packages, dict):
        die("index.json: missing packages object")

    published = bin_index(packages)
    selected: dict[str, dict] = {}
    installed_bins: dict[str, list[str]] = {}
    installed_sha: dict[str, dict[str, str]] = {}
    fetched_url: dict[str, str] = {}

    pin_warned: set[str] = set()

    def pin_digest(pkg_name: str, pkg: dict, value: object, what: str) -> str | None:
        """Index sha256 is only valid when it describes the pinned version."""
        digest = require_sha256(value, what)
        pin = versions.get(pkg_name)
        if pin and str(pkg.get("version")) != pin:
            if pkg_name not in pin_warned:
                print(
                    f"WARN: {pkg_name} pin {pin} != index {pkg.get('version')}; "
                    f"fetching pinned URL without index sha256",
                    file=sys.stderr,
                )
                pin_warned.add(pkg_name)
            return None
        return digest

    def take_bin(name: str, *, required_bin: bool) -> bool:
        hit = published.get(name)
        if hit is None:
            if required_bin:
                die(f"required bin {name!r} is not in the verified stable index")
            print(f"WARN: optional bin {name} not in index: skipping", file=sys.stderr)
            return False
        pkg_name, pkg, binary = hit
        if pkg_name in BLOCKED or name in BLOCKED:
            die(f"refusing blocked package/bin {pkg_name}/{name}")
        raw = binary.get("name")
        index_url = binary.get("dl_url")
        pin = versions.get(pkg_name)
        if pin:
            if not isinstance(raw, str) or not valid_name(raw):
                die(f"{name}: missing binary filename for pinned URL")
            url = pinned_artifact_url(pkg_name, pin, raw)
        else:
            url = index_url
        if not isinstance(url, str) or not url:
            die(f"{name}: missing dl_url")
        digest = pin_digest(pkg_name, pkg, binary.get("sha256"), f"binary {name}")
        dest = bin_dir / name
        note = f"  (pin {pkg_name}={pin})" if pin else ""
        print(f"  {name}  <-  {url}{note}")
        if not stage_file(
            url, dest, digest, f"binary {name}", mode=0o755, required=required_bin
        ):
            return False
        selected[pkg_name] = pkg
        installed_bins.setdefault(pkg_name, []).append(name)
        if digest is not None:
            installed_sha.setdefault(pkg_name, {})[name] = digest
        fetched_url.setdefault(pkg_name, url)
        return True

    for name in required:
        take_bin(name, required_bin=True)
    for name in optional:
        take_bin(name, required_bin=False)

    if not selected:
        die("no packages selected from lockfile ∩ index")

    now = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    installed: dict[str, dict] = {}

    for pkg_name, pkg in sorted(selected.items()):
        bins = pkg.get("binaries") or []
        first_url = fetched_url.get(pkg_name)
        if not first_url:
            for b in bins:
                if isinstance(b, dict) and b.get("dl_url"):
                    first_url = b["dl_url"]
                    break
        if not first_url:
            die(f"{pkg_name}: no binary dl_url to derive version dir")
        base = package_base_url(pkg_name, versions, first_url)
        service_names: list[str] = []

        for svc in pkg.get("services") or []:
            if not isinstance(svc, dict):
                die(f"{pkg_name}: service entry must be an object with unit + sha256")
            unit = svc.get("unit")
            if not isinstance(unit, str) or not valid_name(unit):
                die(f"{pkg_name}: invalid service unit {unit!r}")
            digest = pin_digest(pkg_name, pkg, svc.get("sha256"), f"{pkg_name} {unit}")
            dest = unit_dir / unit
            url = urljoin(base, unit)
            print(f"  {unit}  <-  {url}")
            fetch(url, dest)
            if digest is not None:
                verify_sha256(dest, digest, f"unit {unit}")
            dest.write_text(patch_exec_start(dest.read_text(), bin_dir))
            dest.chmod(0o644)
            if svc.get("enable"):
                for target in wanted_by(dest.read_text()):
                    if not valid_name(target):
                        die(f"{unit}: invalid WantedBy {target!r}")
                    wants = unit_dir / f"{target}.wants"
                    wants.mkdir(parents=True, exist_ok=True)
                    link = wants / unit
                    if link.exists() or link.is_symlink():
                        link.unlink()
                    link.symlink_to(Path("..") / unit)
                    print(f"    enabled {target}.wants/{unit}")
            service_names.append(unit)

        archive = pkg.get("data_archive")
        if archive:
            if not isinstance(archive, str) or not valid_name(archive):
                die(f"{pkg_name}: invalid data_archive {archive!r}")
            digest = pin_digest(
                pkg_name, pkg, pkg.get("data_archive_sha256"), f"{pkg_name} {archive}"
            )
            url = urljoin(base, archive)
            data_dir = share_dir / pkg_name
            data_dir.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(prefix=f"bos-{pkg_name}-") as tmp:
                tmp_path = Path(tmp) / archive
                print(f"  {archive}  <-  {url}")
                stage_file(url, tmp_path, digest, f"{pkg_name} {archive}")
                assert_safe_archive(tmp_path)
                subprocess.run(
                    [
                        "tar",
                        "xzf",
                        str(tmp_path),
                        "--no-same-owner",
                        "--no-same-permissions",
                        "-C",
                        str(data_dir),
                    ],
                    check=True,
                )
            print(f"    extracted to {data_dir}")

        desktop = pkg.get("desktop_file")
        if desktop:
            if not isinstance(desktop, str) or not valid_name(desktop):
                die(f"{pkg_name}: invalid desktop_file {desktop!r}")
            digest = pin_digest(
                pkg_name, pkg, pkg.get("desktop_file_sha256"), f"{pkg_name} {desktop}"
            )
            dest = share_dir / "applications" / f"{pkg_name}.desktop"
            stage_file(urljoin(base, desktop), dest, digest, f"{pkg_name} {desktop}")

        license_file = pkg.get("license_file")
        if license_file:
            if not isinstance(license_file, str) or not valid_name(license_file):
                die(f"{pkg_name}: invalid license_file {license_file!r}")
            digest = pin_digest(
                pkg_name, pkg, pkg.get("license_file_sha256"), f"{pkg_name} {license_file}"
            )
            dest = share_dir / "licenses" / pkg_name / "LICENSE"
            stage_file(urljoin(base, license_file), dest, digest, f"{pkg_name} {license_file}")

        installed[pkg_name] = {
            "name": pkg_name,
            "version": versions.get(pkg_name, pkg.get("version")),
            "binaries": installed_bins.get(pkg_name, []),
            "services": service_names,
            "installed_at": now,
            "track": "stable",
            "binary_sha256": installed_sha.get(pkg_name, {}),
        }

    if "breadhelp" in installed:
        content = share_dir / "breadhelp" / "content"
        if not content.is_dir():
            die(f"breadhelp data_archive did not produce {content}")

    state_path = state_dir / "installed.json"
    state_path.write_text(json.dumps({"track": "stable", "packages": installed}, indent=2) + "\n")
    print(f"installed.json written ({len(installed)} packages): {', '.join(sorted(installed))}")
    print(f"staged bins: {', '.join(sorted(p.name for p in bin_dir.iterdir() if p.is_file()))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
