#!/usr/bin/env python3
"""Reject attacker-controlled registry bytes before signing or publication."""
from functools import partial
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parent.parent
NAMES = list(json.loads((ROOT / "packaging/provenance/approved-packages.json").read_text())["packages"])


def run(args, env, success=True, cwd=None, message=None):
    result = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
    if (result.returncode == 0) != success or (message and message not in result.stderr):
        raise AssertionError(f"unexpected result {result.returncode}: {args}: {result.stderr}")
    return result


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass


# The committed bootstrap has independently checkable signed-repository evidence.
with tempfile.TemporaryDirectory(prefix="bos-bootstrap-check-") as temporary:
    baseline = ROOT / "packaging/provenance/bootstrap-breadway.db"
    signature = baseline.with_name(baseline.name + ".sig")
    keyring = Path(temporary) / "repo.gpg"
    env = os.environ.copy()
    run(["gpg", "--homedir", temporary, "--batch", "--dearmor", "--output", str(keyring), str(ROOT / "iso/airootfs/etc/pacman.d/breadway-repo.asc")], env)
    verified = run(["gpgv", "--homedir", temporary, "--keyring", str(keyring), "--status-fd", "1", str(signature), str(baseline)], env).stdout
    assert "VALIDSIG 56203B86A110695AE7F310934AF3323D678EB5E2 " in verified
    import tarfile
    descriptions = {}
    with tarfile.open(baseline) as archive:
        for member in archive.getmembers():
            if member.name.endswith("/desc"):
                fields = {}
                for chunk in archive.extractfile(member).read().decode().strip().split("\n\n"):
                    lines = chunk.splitlines()
                    fields[lines[0].strip("%")] = "\n".join(lines[1:])
                descriptions[fields["NAME"]] = fields
    policy = json.loads((ROOT / "packaging/provenance/approved-packages.json").read_text())
    for name, entry in policy["packages"].items():
        source = entry["source"]
        if source["kind"] == "signed-repository-bootstrap":
            assert source["database_sha256"] == hashlib.sha256(baseline.read_bytes()).hexdigest()
            assert source["signature_sha256"] == hashlib.sha256(signature.read_bytes()).hexdigest()
            assert entry["filename"] == descriptions[name]["FILENAME"]
            assert entry["pkgver"] == descriptions[name]["VERSION"]
            assert entry["sha256"] == descriptions[name]["SHA256SUM"]

with tempfile.TemporaryDirectory(prefix="bos-provenance-test-") as temporary:
    base = Path(temporary)
    registry = base / "registry" / "x86_64"
    registry.mkdir(parents=True)
    package_source = base / "package-source"
    package_source.mkdir()
    approved = {}
    env = os.environ.copy()
    for name in NAMES:
        (package_source / ".PKGINFO").write_text(f"pkgname = {name}\npkgver = 1.0-1\narch = x86_64\n")
        archive = registry / f"{name}-1.0-1-x86_64.pkg.tar.zst"
        run(["tar", "--zstd", "-cf", str(archive), ".PKGINFO"], env, cwd=package_source)
        approved[name] = {"filename": archive.name, "pkgver": "1.0-1", "arch": "x86_64", "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(), "source": {"kind": "throwaway-test"}}
    policy = base / "policy.json"
    policy.write_text(json.dumps({"schema": 1, "packages": approved}))
    destination = base / "published"
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(base / "registry")))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        env |= {"BREADWAY_COLLECT_ONLY": "1", "BREADWAY_DEST": str(destination), "BREADWAY_REGISTRY": f"http://127.0.0.1:{server.server_port}", "BREADWAY_PROVENANCE_FILE": str(policy)}
        command = ["bash", str(ROOT / "scripts/ci-publish-signed-repo.sh")]
        run(command, env)
        before = {path.name: path.read_bytes() for path in destination.iterdir()}
        assert set(before) == {entry["filename"] for entry in approved.values()}
        # Same filename, version and .PKGINFO; the registry uploader changes bytes.
        first = NAMES[0]
        archive = registry / approved[first]["filename"]
        (package_source / ".PKGINFO").write_text(f"pkgname = {first}\npkgver = 1.0-1\narch = x86_64\n")
        (package_source / "unapproved-payload").write_text("attacker-controlled replacement")
        run(["tar", "--zstd", "-cf", str(archive), ".PKGINFO", "unapproved-payload"], env, cwd=package_source)
        run(command, env, success=False, message="approved SHA256 mismatch")
        assert before == {path.name: path.read_bytes() for path in destination.iterdir()}
        assert not list(base.rglob("*.sig"))
        # Offline sign-only and local overrides cannot bypass provenance.
        run(command, env | {"BREADWAY_SIGN_ONLY": "1", "BREADWAY_REPO_DIR": str(registry)}, success=False, message="approved SHA256 mismatch")
        run(command, env | {"BREADWAY_PKG_DIR": str(registry)}, success=False, message="approved SHA256 mismatch")
        verifier = ["python3", str(ROOT / "scripts/package-provenance.py"), "verify", str(policy), str(destination), *NAMES]
        bad = json.loads(policy.read_text())
        bad["packages"][first]["pkgver"] = "2.0-1"
        policy.write_text(json.dumps(bad))
        run(verifier, env, success=False, message="name/version/architecture mismatch")
        bad["packages"][first]["filename"] = "../escape.pkg.tar.zst"
        policy.write_text(json.dumps(bad))
        run(command, env, success=False, message="unsafe or duplicate filename")
        policy.write_text('{"schema": 1, "schema": 1, "packages": {}}')
        run(command, env, success=False, message="duplicate policy field")
        policy.write_text(json.dumps({"schema": 1, "packages": approved}))
        (destination / "unapproved-1.0-1-x86_64.pkg.tar.zst").write_bytes(b"extra archive")
        run(verifier, env, success=False, message="exactly match the approved set")
        recorded = json.loads(run(["python3", str(ROOT / "scripts/package-provenance.py"), "record", str(archive), "--source-repo", "Breadway/bos", "--source-commit", "a" * 40, "--workflow", "fixture.yml", "--run-id", "1"], env).stdout)
        assert recorded["packages"][first]["sha256"] == hashlib.sha256(archive.read_bytes()).hexdigest()
        assert recorded["packages"][first]["source"]["commit"] == "a" * 40
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
print("PASS: local registry substitution, local override and sign-only rejection; prior publish preserved; strict identity/set/policy checks and candidate provenance recording")
