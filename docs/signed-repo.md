# Signing isolation for BOS

The existing published pacman repository and historical ISOs use the same
legacy key. The new workflows prepare **unsigned candidates**. They do not
import or reference any signing private key, and ISO candidates remain draft
releases until an offline signer verifies and signs them.

## Identity and trust inventory

| Purpose | Public fingerprint | Public key | Private-key location |
| --- | --- | --- | --- |
| Installed `[breadway]` repository | `56203B86A110695AE7F310934AF3323D678EB5E2` | `iso/airootfs/etc/pacman.d/breadway-repo.asc` | Dedicated offline repository signer, never an Actions runner |
| Future ISO `SHA256SUMS` | Not provisioned yet, activation blocked | Replace `KEYS.asc` with a distinct expiring ISO-only identity in a reviewed change | Separate offline ISO signer, never an Actions runner |
| Historical ISO releases | `56203B86A110695AE7F310934AF3323D678EB5E2` | Existing `KEYS.asc`, retain historical public copy when replaced | Verification only; stop using the repository key for new ISOs |

The existing repository primary key expires on 2028-07-03 UTC. The installed
image trusts only `breadway-repo.asc` for `[breadway]`; it does not import
`KEYS.asc`. This change does not rotate keys or alter installed trust.
The offline ISO signer refuses the legacy shared identity, so an unprovisioned
installation cannot silently continue sharing a key.

## Activation checklist, required before the next release or repo update

1. Provision a separate ISO-only signing identity with a finite expiry, for
   example one year, on an offline machine without the repository private key.
   Commit its public half as `KEYS.asc` and record its full fingerprint above in
   a reviewed PR. Preserve the legacy public key for historical downloads.
2. Place the repository private key only on a separate dedicated signing host,
   with a private `GNUPGHOME`. It must not run Forgejo Actions or mount runner
   storage. Remove access from the ISO build host and every other runner.
3. Remove the old `GPG_PRIVATE_KEY` secret from repository and organization
   Actions stores. Renaming it or putting two secrets in the same Forgejo
   repository does not isolate workflows. Review all workflows and scheduled
   jobs before declaring the former runner unable to access it. Because the
   shared key was exposed to a runner, separately assess whether rotation is
   required; do not silently replace installed trust.
4. Protect `main`, release tags and signing policy files so only reviewed
   changes reach the offline signer. The signer uses a reviewed checkout from
   that protected branch, never a script supplied inside an untrusted artifact.
5. Verify the signing tests below and perform a candidate publication with
   the provisioned identities. Check both fingerprints, confirm the image only
   trusts the repository key, and prove Actions has no signing private key.

These are deployment steps, not evidence of a completed production migration.
Forgejo v15 documents secrets at repository and organization scope, not a
workflow-only trust boundary. Keeping all private signing material outside
Actions avoids depending on an unsupported environment restriction. See
[Forgejo secrets](https://forgejo.org/docs/v15.0/user/actions/basic-concepts/).

## ISO candidates and offline approval

`release-iso.yml` still builds in a privileged Arch container. It writes
`SHA256SUMS` and uploads the ISO and checksum file to an **unsigned draft**.
It refuses to replace assets of an already published release. Rebuilding a
candidate removes an obsolete checksum signature. It does not publish a
GitHub announcement or advance `stable` before approval.

On the offline ISO signer, use the reviewed checkout and a downloaded candidate:

```sh
export GNUPGHOME=/secure/bos-iso-signing
bash scripts/sign-iso-checksums.sh /staging/bos-candidate
```

The script checks the pinned public identity is expiring, is distinct from the
repository identity, and exists in the dedicated secret keyring. It accepts
exactly one BOS ISO basename in `SHA256SUMS`, verifies the bytes, and signs only
that file. It refuses an Actions environment. This verifies integrity and key
purpose; the release operator must independently approve the build source and
acceptance evidence before signing.

Upload `SHA256SUMS.asc` to the same draft, verify its signature with the new
ISO public key and verify the downloaded ISO checksum, then publish the draft
through Forgejo. Announce the signed release on GitHub and fast-forward the
`stable` marker only after approval. Do not publish a candidate missing its
signature. This manual approval replaces automatic signing and promotion.

## Repository candidates and offline publication

`signed-repo.yml` runs only for `main`, downloads the current package set and
uploads `unsigned-breadway-candidate` with Forgejo's supported v3 artifact
protocol. It has no signing secret and cannot write the published repository.
See [Forgejo artifacts](https://forgejo.org/docs/v15.0/user/actions/advanced-features/#artifacts).

On the dedicated offline repository signer, inspect package provenance before
accepting any candidate. Registry metadata alone is not trustworthy. A separate
package allowlist/provenance policy is required to prevent signing substituted
registry bytes. Use the reviewed checkout, not scripts in the candidate.

```sh
export GNUPGHOME=/secure/bos-repo-signing
BREADWAY_SIGN_ONLY=1 BREADWAY_REPO_DIR=/staging/verified-packages \
  bash scripts/ci-publish-signed-repo.sh
```

The script pins the key to `breadway-repo.asc`, requires an unexpired identity
with finite expiry, signs package sidecars and builds signed pacman databases
with `repo-add -s`. It refuses Actions environments and does not install host
packages or import secret material from environment variables. Install `gpg`,
`repo-add`, `python3`, `tar` and `zstd` on the dedicated signer beforehand.

Transfer the verified signed tree to `/srv/breadway-dl/arch/x86_64/` only after
independent approval. Verify `breadway.db.sig`, package signatures and the
installed system's `pacman -Sy` before announcing an update. Existing releases
remain served while new candidates await signing.

### Zen Browser release lag

The in-house `zen-browser-bin` PKGBUILD pins the official upstream release
archive with a SHA-256 checksum. A scheduled Forgejo Actions workflow polls
Zen's stable GitHub release feed every six hours, reads the Linux archive's
`digest` from the release asset metadata, and compares the downloaded bytes
against that independently supplied value before building. GitHub documents
this field in its [release assets API](https://docs.github.com/en/rest/releases/assets).
The watcher compares the version with the signed `[breadway]` database and
builds and uploads the release only when `[breadway]` is behind. Missing or
malformed upstream digests and mismatched archives fail closed. The
signed-repository workflow runs after that build completes, so the intended
maximum release-to-repository lag is six hours while the runner and upstream
release service are available. A failed poll or publish is visible as a
failed Actions run; use workflow dispatch to retry it sooner than the next
scheduled poll.

Run `scripts/test-ci-verify-zen-release-asset.sh` to verify the pinned Zen
1.22.3b release archive and confirm that a modified copy is rejected.

## Expiry and rotation

Review expiry every quarter and start renewal at least 30 days before expiry.
`signing-key-policy.py` fails closed on expired, revoked, non-signing or
non-expiring identities and enforces the full primary fingerprint.

For ISO rotation, provision a new ISO-only expiring key offline, publish its
fingerprint and public half in a reviewed change, and retain old public keys
for historical verification. Sign subsequent checksums with the new identity.

For repository rotation, distribute and verify the replacement public key to
existing installations before changing the signing identity. Ship it in the
ISO keyring and update every pinned repository fingerprint together. Verify a
transition with a disposable installed-system test. Never rotate the repository
key solely by replacing a CI secret or deleting the old public key.

## Isolated verification

```sh
python3 scripts/test-offline-signing.py
```

The test creates temporary, unrelated expiring keys. It verifies package and
database signatures, a distinct ISO checksum signature, rejection of altered
ISO bytes and generic checksum lists, rejection of reused/non-expiring keys,
and refusal to sign on Actions. It uses no personal keyring or real signing
material and leaves the live repository unchanged.
