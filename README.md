# Aruvi Studio

Aruvi Studio is a local-first Tauri app for product planning, repository execution, workflow orchestration, and checkpointing.

## MCP

Aruvi Studio now exposes MCP in two ways:

- embedded HTTP MCP from the running desktop app
- standalone stdio MCP for clients that still prefer a child process

Build or run it from the workspace root:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-mcp
```

Or build a binary:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-mcp
```

The MCP server uses the same bootstrap path as the desktop app, so it resolves the same app data directory, database path override file, and `ARUVI_DB_PATH` environment variable.

### Embedded HTTP MCP

When the desktop app is running, the existing embedded Axum bridge now also serves MCP at:

```text
http://127.0.0.1:8787/api/mcp
```

The host and port come from the same bridge settings already used for the mobile companion:

- `mobile.bind_host`
- `mobile.bind_port`
- `ARUVI_WEBHOOK_HOST`
- `ARUVI_WEBHOOK_PORT`

Authentication for HTTP MCP:

- if `mcp.api_token` or `ARUVI_MCP_API_TOKEN` is set, clients must send `Authorization: Bearer <token>`
- if the bridge remains localhost-only, the token is optional for local clients
- if the bridge is exposed beyond localhost and no token is configured, MCP HTTP requests are rejected

The HTTP endpoint follows MCP streamable HTTP request semantics for `POST /api/mcp`. `GET /api/mcp` intentionally returns `405 Method Not Allowed` because Aruvi does not currently expose an SSE stream.

### Tool Groups

The MCP surface is grouped by domain instead of exposing one tool per Tauri command:

- `aruvi_catalog`: products, modules, capabilities, product trees
- `aruvi_work_items`: work item CRUD, hierarchy, summaries
- `aruvi_repositories`: repo registration, scope attachment, workspace creation, tree/file operations, patching
- `aruvi_planner`: planner sessions, draft editing, repository analysis, plan confirmation
- `aruvi_workflows`: workflow start/advance/restart plus run and history inspection
- `aruvi_checkpoints`: approvals, artifacts, findings, logs
- `aruvi_agents`: agent registry, teams, skills, bindings, routing policies
- `aruvi_models`: providers, model definitions, local model registration, chat completion
- `aruvi_settings`: settings, mobile and MCP bridge status, database health/path controls
- `aruvi_channels`: WhatsApp, voice call, planner contact routing
- `aruvi_speech`: transcription and native speech output

Each tool accepts:

```json
{
  "action": "domain_specific_action",
  "arguments": {
    "field_name": "value"
  }
}
```

`arguments` accepts snake_case and common camelCase aliases for the main fields.

### Client Setup

If Aruvi Studio is already running, prefer the embedded HTTP server. A VS Code workspace config looks like this:

```json
{
  "inputs": [
    {
      "type": "promptString",
      "id": "aruvi-mcp-token",
      "description": "Aruvi MCP bearer token",
      "password": true
    }
  ],
  "servers": {
    "aruviStudio": {
      "type": "http",
      "url": "http://127.0.0.1:8787/api/mcp",
      "headers": {
        "Authorization": "Bearer ${input:aruvi-mcp-token}"
      }
    }
  }
}
```

For clients that want stdio, point any stdio-capable MCP client at the standalone server command.

A generic stdio config looks like this:

```json
{
  "mcpServers": {
    "aruvi-studio": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--bin",
        "aruvi-studio-mcp"
      ]
    }
  }
}
```

If your agent should target a specific database, set `ARUVI_DB_PATH` in the server environment.

### Intentional Omissions

The MCP server is headless-first. Desktop-only affordances that require native dialogs or Tauri event streaming are not exposed as MCP tools:

- repository folder picker
- Finder reveal
- local model file picker
- Tauri chat stream event channel

For agent use, the non-streaming equivalents and direct path-based operations are exposed instead.
