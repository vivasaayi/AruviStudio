# Headless AruviStudio deployment

The headless server turns a Linux VM into the always-on AruviStudio runtime. The existing Tauri
desktop application remains supported; both entry points use the same Rust services and database
model.

## What runs on the VM

- AruviStudio SQLite database and artifacts
- planner and model-provider integrations
- repository workspaces and coding agents
- workflow orchestration and approvals
- authenticated remote web interface at `/remote`
- authenticated mobile API and HTTP MCP endpoint

The repositories and any coding CLIs used by agents must also be installed or cloned on the VM.

## Build

On Ubuntu 22.04, install the current native build dependencies:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential curl git pkg-config libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

Then build the server:

```bash
cargo build --release \
  --manifest-path src-tauri/Cargo.toml \
  --bin aruvi-studio-server
```

The binary is created at `src-tauri/target/release/aruvi-studio-server`.
Tagged release-candidate builds also attach `aruvi-studio-server-linux-x86_64.tar.gz` to the draft
GitHub release, alongside the desktop installers and verification checksums.

## Runtime configuration

Use persistent, explicitly configured paths. Generate separate long random values for the mobile
and MCP tokens.

```bash
export ARUVI_APP_DATA_DIR=/var/lib/aruvi-studio
export ARUVI_DB_PATH=/var/lib/aruvi-studio/aruvi-studio.db
export ARUVI_LLM_CONFIG_PATH=/etc/aruvi-studio/llm-config.json
export ARUVI_WEBHOOK_HOST=127.0.0.1
export ARUVI_WEBHOOK_PORT=8787
export ARUVI_MOBILE_API_TOKEN=replace-with-a-long-random-token
export ARUVI_MCP_API_TOKEN=replace-with-a-different-long-random-token

./src-tauri/target/release/aruvi-studio-server
```

`ARUVI_LLM_CONFIG_PATH` is useful on a headless Linux service because a desktop keychain may not
be available. Restrict that file to the service account because it contains provider credentials.

## systemd service

Install the binary as `/usr/local/bin/aruvi-studio-server`, create a dedicated `aruvi` account,
and place the environment values in `/etc/aruvi-studio/aruvi.env` with mode `0600`.

```ini
[Unit]
Description=AruviStudio headless runtime
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=aruvi
Group=aruvi
EnvironmentFile=/etc/aruvi-studio/aruvi.env
ExecStart=/usr/local/bin/aruvi-studio-server
Restart=on-failure
RestartSec=5
WorkingDirectory=/var/lib/aruvi-studio

[Install]
WantedBy=multi-user.target
```

## Remote access

Do not expose port `8787` directly to the public internet. Keep AruviStudio bound to
`127.0.0.1` and publish it through an HTTPS reverse proxy or a private network such as Tailscale.
The browser URL is:

```text
https://your-private-studio-host/remote
```

Enter `ARUVI_MOBILE_API_TOKEN` in the Connection panel. Safari stores it locally on that device
and sends it as a bearer token for mobile API requests.

## Daily flow

1. Open `/remote` from Safari and select **Work**.
2. Select a product and create a task with its expected outcome and acceptance criteria.
3. Review the task and select **Approve & Start**.
4. Refresh Delivery to follow the workflow stage and generated artifacts.
5. Use **Approve Gate** or **Reject Gate** when the workflow pauses for review.

The current remote surface is intentionally operational rather than a copy of every desktop
administration screen. Product, repository, provider, and agent configuration can remain a desktop
setup activity while the remote workflow matures.
