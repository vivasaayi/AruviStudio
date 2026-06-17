# AruviStudio Release Profiles

## Development

Run the normal development app with:

```sh
npm run tauri dev
```

The development app uses bundle identifier `com.aruvi.studio`, the default app data directory, and the default MCP/mobile bridge port `8787` unless overridden by settings or environment variables.

## Local Production Copy

Build a side-by-side local production app with:

```sh
npm run release:local
```

This creates `/Users/rajanpanneerselvam/work/releases/AruviStudio Local.app` using bundle identifier `com.aruvi.studio.localrelease`. The app infers the `local-release` runtime profile from that bundle identifier, so double-clicking the app keeps it separated from development data.

Default local release paths:

```text
App:      /Users/rajanpanneerselvam/work/releases/AruviStudio Local.app
Launcher: /Users/rajanpanneerselvam/work/releases/run_aruvi_local_release.command
MCP:      http://127.0.0.1:8788/api/mcp
DB:       /Users/rajanpanneerselvam/work/releases/aruvi-studio-local-data/aruvi_studio.db
Data:     /Users/rajanpanneerselvam/work/releases/aruvi-studio-local-data
```

The generated launcher pins the same profile, database, keychain service, and MCP port explicitly. Use it when you want deterministic terminal logs. To seed the local release from an existing database only when the target DB does not exist:

```sh
ARUVI_LOCAL_RELEASE_SEED_DB=/Users/rajanpanneerselvam/work/releases/test_1.db npm run release:local
```

## Direct macOS Release

Build a direct distribution release with:

```sh
npm run release:mac
```

The direct release keeps bundle identifier `com.aruvi.studio` and writes artifacts under:

```text
/Users/rajanpanneerselvam/work/releases/direct/<timestamp>/
```

For a signed and notarized Developer ID release, provide one signing source:

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
```

or:

```sh
export APPLE_CERTIFICATE="<base64 p12>"
export APPLE_CERTIFICATE_PASSWORD="<p12 password>"
```

Then provide one notarization source:

```sh
export APPLE_API_ISSUER="<issuer uuid>"
export APPLE_API_KEY="<key id>"
export APPLE_API_KEY_PATH="/path/to/AuthKey_KEYID.p8"
```

or:

```sh
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="<app-specific password>"
export APPLE_TEAM_ID="<team id>"
```

Useful release flags:

```sh
ARUVI_SKIP_SIGNING=1 npm run release:mac
ARUVI_SKIP_STAPLING=1 npm run release:mac
ARUVI_SKIP_VERIFY=1 npm run release:mac
```

Unsigned builds are only for smoke testing. A customer-facing direct Mac release should be Developer ID signed, notarized, stapled, and verified by the script.
