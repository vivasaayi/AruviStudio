# Releasing AruviStudio

Desktop releases are published from GitHub Actions by pushing a version tag.

## Trigger a release

Use the version rotation script to update the app version, back up the current
database, commit the version bump, and create the release tag:

```bash
npm run version:rotate -- patch --commit --tag
```

You can pass `major`, `minor`, `patch`, or an explicit `X.Y.Z` version:

```bash
npm run version:rotate -- 0.2.0 --commit --tag
```

The script updates:

- `/package.json`
- `/package-lock.json`
- `/src-tauri/tauri.conf.json`
- `/src-tauri/Cargo.toml`
- `/src-tauri/Cargo.lock`

By default, it runs `/backup.sh` before changing the version, and labels that
backup with the version you are leaving. The backup source defaults to
`livedb_path` when present, then `ARUVI_DB_PATH`, then the standard AruviStudio
app data database path. Override paths when needed:

```bash
ARUVI_BACKUP_SOURCE_DB=/path/to/aruvi_studio.db \
ARUVI_BACKUP_DIR=/path/to/backups \
npm run version:rotate -- patch --commit --tag
```

Push the created tag to publish:

```bash
git push origin vX.Y.Z
```

The workflow in `/.github/workflows/publish.yml` builds desktop bundles for:

- macOS Apple Silicon
- macOS Intel
- Windows x64
- Linux x64

## GitHub secrets for macOS signing

If these secrets are configured, the macOS release is signed with Developer ID and notarized with Apple:

- `APPLE_SIGNING_IDENTITY`
- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `KEYCHAIN_PASSWORD`
- `APPLE_API_ISSUER`
- `APPLE_API_KEY`
- `APPLE_API_KEY_P8`

Notes:

- `APPLE_CERTIFICATE` is the base64-encoded `.p12` export of the Developer ID Application certificate.
- `APPLE_API_KEY_P8` is the full contents of the App Store Connect `.p8` key file.
- `APPLE_SIGNING_IDENTITY` should match the output of `security find-identity -v -p codesigning` on the Mac where the certificate was created.

## macOS fallback behavior

If the Apple signing secrets are missing, the workflow sets `APPLE_SIGNING_IDENTITY=-` and produces an ad-hoc signed macOS bundle instead of an unsigned one.

This is enough for local testing and is usually better than a completely unsigned app, especially on Apple Silicon. It is not equivalent to a notarized Developer ID build. Users may still need to:

- right-click the app and choose `Open`, or
- allow the app from `System Settings > Privacy & Security`

If you want the GitHub release download to install cleanly without that override, you need the full Developer ID + notarization secret set above.
