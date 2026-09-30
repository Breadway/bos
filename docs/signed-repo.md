# Signing isolation for BOS

The existing published pacman repository and historical ISOs use the same
legacy key. The new workflows prepare **unsigned candidates**. They do not
import or reference any signing private key, and ISO candidates remain draft
releases until an offline signer verifies and signs them.

## Identity and trust inventory

| Purpose | Public fingerprint | Public key | Intended private-key location after activation |
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
It refuses every mutation if a release or candidate already exists for that
tag. Candidates are immutable; retries cannot delete or replace assets. Each
upload rechecks that the new candidate is still a draft. It does not publish a
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

Exclude signer/publication activity while the candidate build is active. The
release operator must check that its Actions run completed successfully and is
no longer running before downloading, signing, uploading a signature or
publishing. A release missing either uploaded file or associated success
handoff must not be signed. Immediate draft-state checks do not provide a
transaction against an operator publishing concurrently, so this exclusion is
an operational activation requirement. Failed candidates remain unsigned
drafts for investigation, and a rerun is refused until the operator has
explicitly disposed of that failed candidate through the release procedure.

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

The protected `packaging/provenance/approved-packages.json` policy specifies
one name, complete version (including epoch/pkgrel), basename, architecture and
SHA-256 per package. Collection requests those filenames directly. Registry
metadata cannot choose what gets signed. Local overrides and offline sign-only
mode enforce the same exact-set and digest policy before any signature.

The initial policy is bootstrapped from the previously published database:
its detached signature was verified against the pinned repository fingerprint
`56203B86A110695AE7F310934AF3323D678EB5E2`, dated 2026-09-19. Each baseline
record includes database/signature hashes and source URL. The signed bootstrap
database and signature are committed alongside the policy, and the provenance
test independently verifies them against the pinned public key. This continuity
baseline does not claim new independent historical build provenance.

For each new build, the BOS packaging workflow emits an **unapproved**
`*.provenance.json` artifact with the producing repository, full source commit,
workflow, run ID, package version and archive digest. Registry writes and these
candidate artifacts cannot advance the policy automatically. Review the source
and build evidence, independently verify the package bytes against the build
candidate, and approve its entry through a protected-main PR before publication.
Use a reviewed checkout and policy from `main` on the offline signer. Never
accept a policy included in a registry response or candidate artifact.

`breadlock` builds in a separate repository. Its release operator must run the
same recorder against the actual built archive, supplying that repository's
source commit, workflow and run ID, then submit the reviewed entry to this
policy. No signing occurs for an unapproved version from any producer.

```sh
python3 scripts/package-provenance.py record /build/package.pkg.tar.zst \
  --source-repo Breadway/breadlock --source-commit FULL_BUILD_COMMIT \
  --workflow package.yml --run-id BUILD_RUN_ID > package.provenance.json
```

This approval requirement deliberately trades unattended package updates for
protection against an attacker with registry write access. New Zen packages
may be built every six hours, but release-to-repository lag also includes policy
review and offline approval. An unapproved version stays out of the signed repo.

On the dedicated offline repository signer, verify the protected policy and
build provenance before accepting a candidate. Use the reviewed checkout,
not scripts or policies in the candidate.

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
candidate workflow runs after that build completes. Publishing then requires
reviewed digest-policy advancement and offline signing approval. The six-hour
poll interval is not an upper bound on publication lag. A failed poll or publish is visible as a
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
python3 scripts/test-package-provenance.py
```

The test creates temporary, unrelated expiring keys. It verifies package and
database signatures, a distinct ISO checksum signature, rejection of altered
ISO bytes and generic checksum lists, rejection of reused/non-expiring keys,
and refusal to sign on Actions. It uses no personal keyring or real signing
material and leaves the live repository unchanged.

The provenance test serves a throwaway Arch registry over loopback HTTP. It
accepts the approved package set, then alters one archive at the same filename
and version. Collection and offline signing both reject it before producing a
signature, and the prior publication remains unchanged. It also checks version,
extra-file, duplicate-policy and unsafe-filename rejection.
