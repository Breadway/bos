#!/usr/bin/env python3
"""Exercise offline signing with throwaway keys, no user keyring or live repo."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def run(args, env, success=True, cwd=None):
    result = subprocess.run(args, env=env, cwd=cwd, capture_output=True, text=True)
    if (result.returncode == 0) != success:
        raise AssertionError(f"unexpected exit {result.returncode}: {args}: {result.stderr}")
    return result


with tempfile.TemporaryDirectory(prefix="bos-signing-test-") as temporary:
    base = Path(temporary)
    env = os.environ.copy()
    for variable in ("CI", "GITHUB_ACTIONS", "FORGEJO_ACTIONS"):
        env.pop(variable, None)
    for role in ("iso", "repo", "no-expiry"):
        home = base / role
        home.mkdir(mode=0o700)
        env["GNUPGHOME"] = str(home)
        expiry = "0" if role == "no-expiry" else "1d"
        run(["gpg", "--batch", "--pinentry-mode", "loopback", "--passphrase", "", "--quick-generate-key", f"BOS test {role} <{role}@example.invalid>", "ed25519", "sign", expiry], env)
        public = run(["gpg", "--batch", "--armor", "--export"], env).stdout
        (base / f"{role}.asc").write_text(public)
    env["GNUPGHOME"] = str(base / "iso")
    env["BOS_ISO_PUBLIC_KEY_FILE"] = str(base / "iso.asc")
    env["BREADWAY_REPO_PUBLIC_KEY_FILE"] = str(base / "repo.asc")
    candidate = base / "candidate"
    candidate.mkdir()
    iso = candidate / "bos-0.0.1-x86_64.iso"
    iso.write_bytes(b"throwaway ISO fixture")
    checksum = f"{hashlib.sha256(iso.read_bytes()).hexdigest()}  {iso.name}\n"
    (candidate / "SHA256SUMS").write_text(checksum)
    signer = ["bash", str(ROOT / "scripts/sign-iso-checksums.sh"), str(candidate)]
    run(signer, env)
    assert (candidate / "SHA256SUMS.asc").is_file()
    iso.write_bytes(b"tampered fixture")
    run(signer, env, success=False)
    iso.write_bytes(b"throwaway ISO fixture")
    (candidate / "SHA256SUMS").write_text(checksum + checksum)
    run(signer, env, success=False)
    (candidate / "SHA256SUMS").write_text(checksum)
    same = env | {"BOS_ISO_PUBLIC_KEY_FILE": str(base / "repo.asc")}
    run(signer, same, success=False)
    run(signer, env | {"CI": "true"}, success=False)
    run(["python3", str(ROOT / "scripts/signing-key-policy.py"), str(base / "no-expiry.asc")], env, success=False)
    env["GNUPGHOME"] = str(base / "repo")
    package_dir = base / "packages"
    package_dir.mkdir()
    approved = {}
    names = json.loads((ROOT / "packaging/provenance/approved-packages.json").read_text())["packages"]
    for name in names:
        (package_dir / ".PKGINFO").write_text(f"pkgname = {name}\npkgbase = {name}\npkgver = 1.0-1\npkgdesc = signing fixture\nsize = 0\narch = x86_64\n")
        archive = package_dir / f"{name}-1.0-1-x86_64.pkg.tar.zst"
        run(["tar", "--zstd", "-cf", str(archive), ".PKGINFO"], env, cwd=package_dir)
        approved[name] = {"filename": archive.name, "pkgver": "1.0-1", "arch": "x86_64", "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(), "source": {"kind": "throwaway-test"}}
    policy = base / "policy.json"
    policy.write_text(json.dumps({"schema": 1, "packages": approved}))
    env["BREADWAY_PROVENANCE_FILE"] = str(policy)
    publisher = ["bash", str(ROOT / "scripts/ci-publish-signed-repo.sh")]
    env |= {"BREADWAY_SIGN_ONLY": "1", "BREADWAY_REPO_DIR": str(package_dir)}
    run(publisher, env)
    run(["gpg", "--batch", "--verify", str(archive) + ".sig", str(archive)], env)
    run(["gpg", "--batch", "--verify", str(package_dir / "breadway.db.tar.gz.sig"), str(package_dir / "breadway.db.tar.gz")], env)
    run(publisher, env | {"GITHUB_ACTIONS": "true"}, success=False)
    for home in (base / "iso", base / "repo", base / "no-expiry"):
        run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"], env)
print("PASS: distinct expiring offline keys, ISO checksum restriction, tamper rejection, signed pacman package/db and Actions refusal")
