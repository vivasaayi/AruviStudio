# Aruvi Planner Mobile

This is a lightweight iPhone companion scaffold for the desktop planner.

It talks to the same Rust planner and speech APIs exposed by the desktop app:

- `POST /api/mobile/planner/sessions`
- `POST /api/mobile/planner/sessions/:session_id`
- `POST /api/mobile/planner/sessions/:session_id/turn`
- `POST /api/mobile/planner/sessions/:session_id/confirm`
- `POST /api/mobile/planner/sessions/:session_id/clear`
- `POST /api/mobile/speech/transcribe`

## Desktop setup

1. In desktop Settings, configure:
   - `speech.transcription_provider_id`
   - `speech.transcription_model_name`
   - `speech.locale`
   - `mobile.api_token`
2. Start the desktop app with a reachable bind address if the phone is on the same LAN:
   - `ARUVI_WEBHOOK_HOST=0.0.0.0`
   - optional `ARUVI_WEBHOOK_PORT=8787`
3. Use your desktop machine IP as the mobile base URL, for example:
   - `http://192.168.1.15:8787`

## Mobile setup

```bash
cd mobile
npm install
npm run ios
```

The app stores the base URL, bearer token, provider id, model name, and locale in secure storage on the device.

## App Store release

The repo now includes a GitHub Actions workflow at `.github/workflows/mobile-ios-release.yml` plus `mobile/eas.json` for EAS Build.

What still has to be done once:

1. Create the app in App Store Connect with the same bundle identifier used here: `com.aruvi.plannermobile`.
2. Sign in to Expo locally and run a successful one-time iOS production build from `mobile/`:

```bash
npx eas-cli build --platform ios --profile production
```

That initial run lets Expo create the EAS project, write `expo.extra.eas.projectId`, and walk through any Apple credential prompts that GitHub Actions cannot answer interactively.

3. Replace `REPLACE_WITH_APP_STORE_CONNECT_APP_ID` in `mobile/eas.json` with the Apple ID / `ascAppId` from App Store Connect.
4. Add these GitHub repository secrets before running the workflow:
   - `EXPO_TOKEN`
   - `APP_STORE_CONNECT_API_KEY_P8`
   - `APP_STORE_CONNECT_KEY_ID`
   - `APP_STORE_CONNECT_ISSUER_ID`
   - `APP_STORE_CONNECT_TEAM_ID`
   - `APP_STORE_CONNECT_TEAM_TYPE`

How to use the workflow:

1. Open GitHub Actions.
2. Run `Mobile iOS Release`.
3. Choose `preview` to create an internal iOS build, or `production` for the App Store build.
4. Set `submit` to `true` only when the App Store Connect record, metadata, and `ascAppId` are ready. That uploads the finished build to App Store Connect / TestFlight.

Current blockers before a real App Store submission:

- No iOS app icon or store artwork is checked into `mobile/` yet.
- App Store screenshots, privacy answers, and listing metadata still need to be filled in inside App Store Connect.

## Current scope

- create or resume planner sessions
- send text planning turns
- render the staged draft tree
- select draft nodes before sending follow-up turns
- record voice on iPhone and send audio to the desktop speech transcription endpoint
- commit or clear the staged draft

## Current boundary

This is a first mobile companion, not full parity with the desktop app yet. It does not currently expose trace inspection, inline draft-node editing, or telephony controls.
