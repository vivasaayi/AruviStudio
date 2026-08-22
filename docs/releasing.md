# Releasing AruviStudio

Desktop releases use a build-once, promote-without-rebuilding factory in GitHub Actions.

## 1. Prepare the version and tag

Rotate the version, back up the current database, commit the version files, and create the release tag:

```bash
npm run version:rotate -- patch --commit --tag
```

You can pass `major`, `minor`, `patch`, or an explicit `X.Y.Z` version. The script updates:

- `/package.json`
- `/package-lock.json`
- `/src-tauri/tauri.conf.json`
- `/src-tauri/Cargo.toml`
- `/src-tauri/Cargo.lock`

By default, version rotation runs `/backup.sh` before changing the version. Override the database paths when needed, or explicitly pass `--no-backup` when a release is not tied to a local application database.

Push the commit and tag:

```bash
git push origin HEAD
git push origin vX.Y.Z
```

Pushing a tag does not publish a release.

## 2. Build a release candidate

In GitHub Actions, manually run **Build Desktop Release Candidate** and enter the existing `vX.Y.Z` tag.

The workflow:

1. Checks out the tagged source.
2. Verifies that the tag matches every application version file.
3. Runs terminology/performance guards, Rust tests, frontend tests, Playwright tests, and a production frontend build.
4. Builds desktop bundles for macOS Apple Silicon, macOS Intel, Windows x64, and Linux x64.
5. Signs and notarizes macOS artifacts when the Apple secrets are configured.
6. Uploads all artifacts, `RELEASE_MANIFEST.json`, and `SHA256SUMS.txt` to a draft GitHub Release.

The draft contains the tested Git commit. It is the release candidate to install on the test machine.

## 3. Test the candidate

Download artifacts from the draft GitHub Release and test those exact files. At minimum, verify:

- Clean installation and first launch
- Upgrade from the current production version
- Database migrations and existing user data
- Core planner, product, work-item, repository, and MCP flows
- Restart behavior and application permissions

If testing fails, fix the source and create a new version/candidate. Do not modify candidate artifacts manually.

## 4. Promote the tested artifacts

After acceptance testing, manually run **Promote Desktop Release**.

Enter the draft release tag twice: once as the target and once as confirmation. The promotion workflow:

1. Confirms the release is still a draft and has artifacts.
2. Downloads the candidate artifacts.
3. Verifies every artifact against the candidate's existing `SHA256SUMS.txt`.
4. Publishes the existing draft release without rebuilding anything.

Configure required reviewers for the `production-release` GitHub Environment to enforce approval before promotion.

## macOS signing secrets

For Developer ID signing and notarization, configure:

- `APPLE_SIGNING_IDENTITY`
- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `KEYCHAIN_PASSWORD`
- `APPLE_API_ISSUER`
- `APPLE_API_KEY`
- `APPLE_API_KEY_P8`

If certificate secrets are missing, candidate builds fall back to ad-hoc macOS signing. Such artifacts are suitable for smoke testing but are not customer-ready Developer ID releases.

## Local scripts

`npm run release:local` and `npm run release:mac` remain available for local diagnostics and emergency use. They are not the canonical production factory and their outputs should not be promoted through the GitHub release workflow.
