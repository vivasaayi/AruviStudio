# Aruvi Studio

Aruvi Studio is a local-first Tauri app for product planning, repository execution, workflow orchestration, and checkpointing.

## MCP

The repo now includes a standalone stdio MCP server so external agents can drive Aruvi Studio without going through the UI.

Build or run it from the workspace root:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-mcp
```

Or build a binary:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-mcp
```

The MCP server uses the same bootstrap path as the desktop app, so it resolves the same app data directory, database path override file, and `ARUVI_DB_PATH` environment variable.

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
- `aruvi_settings`: settings, mobile bridge status, database health/path controls
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

Point any stdio-capable MCP client at the server command. A generic config looks like this:

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
