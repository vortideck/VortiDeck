# Marketplace, R2, and package lifecycle

Status: implemented foundation for VortiDeck 0.7.0

## Ownership and data flow

The Django backend owns catalog metadata, creator ownership, immutable releases,
entitlements, reviews, download receipts, quarantine state, compatibility, and
the marketplace signing key. Cloudflare R2 stores only private package bytes.
The desktop owns local consent, activation, last-known-good recovery, and
uninstall compensation in its Diesel database.

```text
creator -> Django upload session -> private R2 quarantine object
        -> backend size/hash/archive/policy validation
        -> Ed25519 marketplace attestation
        -> immutable private R2 release object

desktop -> public compatible catalog -> authenticated entitlement check
        -> five-minute signed GET -> hash + attestation verification
        -> safe extraction outside watched folders
        -> lifecycle-owned atomic install/update -> consent -> activation
```

R2 is deliberately not the database and is never the authority for ownership or
release status. A package becomes visible only after its database release is
`published`. R2 object keys are content-addressed and immutable.

## Backend API

- `GET /api/v1/marketplace/packages/?host_version=0.7.0` lists published,
  compatible packages.
- `GET /api/v1/marketplace/packages/<package_id>/` returns one package.
- `POST /api/v1/marketplace/packages/resolve/` resolves at most 200 ids.
- `GET /api/v1/marketplace/library/` returns the signed-in user's entitlements.
- `POST /api/v1/marketplace/releases/<release_id>/download/` authorizes a
  download, records it, and returns a short-lived artifact URL plus attestation.
- `GET|POST /api/v1/marketplace/creator/packages/` and
  `GET|PATCH /api/v1/marketplace/creator/packages/<package_id>/` manage creator
  metadata. Package ids are immutable, lowercase identifiers.
- `POST /api/v1/marketplace/uploads/` creates a release and quarantine session.
- `PUT <upload_url>` uploads the exact declared bytes with
  `Content-Type: application/vnd.vortideck.package+zip`.
- `POST /api/v1/marketplace/uploads/<upload_id>/finalize/` validates and
  publishes; `GET` on the upload resource reports durable status/errors.

The browser/CLI must use presigned `PUT`; R2 does not provide a presigned HTML
POST form flow. Production publication is asynchronous through Celery. Local
development defaults to synchronous publication so a missing worker cannot make
the UI look stuck.

## Required production configuration

Backend secrets/configuration:

```text
USE_R2_STORAGE=true
R2_ENDPOINT_URL=https://<account-id>.r2.cloudflarestorage.com
R2_ACCESS_KEY_ID=<R2 API token access key>
R2_SECRET_ACCESS_KEY=<R2 API token secret>
R2_MARKETPLACE_BUCKET_NAME=vortideck-marketplace
MARKETPLACE_SIGNING_PRIVATE_KEY_B64=<raw 32-byte Ed25519 private key, base64>
MARKETPLACE_SIGNING_KEY_ID=marketplace-root-v1
MARKETPLACE_PUBLISH_SYNCHRONOUS=false
```

Generate a new key pair with:

```powershell
uv run python manage.py marketplace_generate_signing_key
```

Store only the private key on the backend. Put the printed public key and key id
into repository/environment variables used by every official desktop build:

```text
VORTIDECK_MARKETPLACE_ROOT_PUBLIC_KEY_B64
VORTIDECK_MARKETPLACE_ROOT_KEY_ID
```

The values are compiled into the desktop binary. Debug builds may read them at
runtime. A production binary without a pinned key fails closed: it may display
the catalog but cannot install remote artifacts.

### Backup key

Official builds also pin a second, offline backup key, so a lost or exposed root
key does not require an app update before marketplace installs work again.
Generate it separately and give it its own key id:

```powershell
uv run python manage.py marketplace_generate_signing_key --key-id marketplace-backup-v1
```

```text
VORTIDECK_MARKETPLACE_BACKUP_PUBLIC_KEY_B64
VORTIDECK_MARKETPLACE_BACKUP_KEY_ID
```

Do not put the backup private key on any server. Keep it with the other
irreplaceable keys in offline storage (password manager plus an encrypted
offline copy). The release workflows refuse to build without both keys.

The desktop accepts a release signed by either pinned key and matches the key by
the attestation's key id. Signatures are made once at publication and stored, so:

- **Root key lost:** set `MARKETPLACE_SIGNING_PRIVATE_KEY_B64` and
  `MARKETPLACE_SIGNING_KEY_ID` on the backend to the backup pair. Existing
  releases keep verifying under the root key; new ones are signed with the backup.
- **Root key exposed:** switch to the backup pair as above, then re-sign every
  published release with `uv run python manage.py marketplace_resign_releases`
  (preview with `--dry-run`). Ship the next desktop release with the backup as
  the root key and a newly generated backup, so the exposed key is no longer
  trusted.
- **After either event:** generate a new backup pair and pin it in the next
  release, so there is always an unused offline key.

The R2 bucket stays private. Configure bucket CORS only for the actual creator
web origins and the `PUT`, `GET`, and `HEAD` methods/headers they use. Never log
presigned URLs: they are temporary bearer capabilities. Presigning uses the R2
S3 API endpoint, not a custom domain.

## Validation and trust boundaries

Publication rejects missing or mismatched manifests, duplicate/non-canonical or
traversing paths, backslashes, NULs, symbolic links, excessive file counts,
oversized expansion, suspicious compression ratios, identity/version/hash/size
mismatches, native service runtimes, and broad command/filesystem permissions in
marketplace v1. The backend signs a canonical release attestation after promotion.

The desktop independently verifies the artifact size and SHA-256, every signed
attestation identity field, host-version bounds, key id, and Ed25519 signature.
It then independently applies archive path and resource limits. Legacy embedded
manifest signatures remain local self-integrity only and cannot create
marketplace trust.

## Install, update, recovery, and uninstall

- First install publishes a verified extraction atomically, writes an
  `awaiting_review` provenance receipt, then opens the one package-level consent
  dialog. No code runs before consent.
- Update extraction remains outside the watched plugins folder. The lifecycle
  coordinator snapshots the active package, stops it, atomically swaps files,
  stores exact old/new release provenance, and either activates unchanged access
  or requests review.
- Failed activation restores the prior files, runtime, and exact release/hash/key
  receipt. A new package is never loaded directly from a recovery folder.
- Startup repairs receipts interrupted during `installing` or `updating` before
  discovery. A matching published folder returns to review/reconciliation; a
  missing first-install folder removes its incomplete receipt; mismatched update
  content fails closed.
- Uninstall stages the folder out of the watched directory, unloads runtime,
  revokes consent, removes provenance, and restores all of them if any step fails.
- Local developer folders receive a `local` receipt when consent is granted;
  they never inherit remote marketplace trust.

The marketplace UI derives one card projection from three independent inputs:
immutable remote catalog, local discovery/runtime state, and Diesel installation
provenance. Lifecycle events update that projection without blanking the list,
and stale overlapping refresh responses are discarded.
Therefore uninstall removes local files without deleting the catalog card,
offline catalog failure does not hide installed packages, and staged access is
not displayed as active.

`VortiDeckCore` owns the only production `PluginManager`. Tauri commands,
watchers, startup and the marketplace lifecycle are adapters over that same
engine; none creates a shadow registry. Package installation state belongs to
the lifecycle coordinator and Diesel, while the manager owns discovery,
permissions and loaded runtimes.

Publication workers use a durable scan lease. A periodic recovery task requeues
stale scans, while validation failures remain terminal and infrastructure
failures retry with backoff. Creator self-signing records were removed because
they were not verified or used; marketplace trust is the pinned marketplace
attestation only.

## Release and operational gates

Before enabling marketplace installation in an official build:

1. Apply Django migrations on PostgreSQL.
2. Create the private R2 bucket and least-privilege R2 API token.
3. Generate and securely store the Ed25519 root key.
4. Publish the public key/key id into every desktop platform build environment.
5. Run a real upload/finalize/download against R2 and verify the desktop install.
6. Test first install, permission-expanding update, activation-failure rollback,
   offline catalog, expired URLs, revoked entitlement, uninstall, and restart.
7. Monitor rejected uploads, publication duration, download authorization errors,
   and rollback lifecycle events.

Payment checkout, creator payouts/tax handling, moderation queues, malware
scanning, key rotation/revocation, delta updates, and automatic update scheduling
are intentionally separate post-foundation systems. Paid packages remain
unavailable in the desktop until the payment/entitlement workflow exists.
