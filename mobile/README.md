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

## Current scope

- create or resume planner sessions
- send text planning turns
- render the staged draft tree
- select draft nodes before sending follow-up turns
- record voice on iPhone and send audio to the desktop speech transcription endpoint
- commit or clear the staged draft

## Current boundary

This is a first mobile companion, not full parity with the desktop app yet. It does not currently expose trace inspection, inline draft-node editing, or telephony controls.
