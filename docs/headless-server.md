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

## Local Docker deployment (in progress)

The repository includes a Compose configuration for a local, ARM64 Docker Desktop
deployment. Create a private `.env` file at the repository root with two different,
long random tokens named `ARUVI_MOBILE_API_TOKEN` and `ARUVI_MCP_API_TOKEN`. Then run
`docker compose up --build -d` and open `http://127.0.0.1:8787/remote`. The published
port is bound to the Mac's loopback interface. The named `aruvi-data` volume holds
the database, artifacts, and fallback provider configuration across container restarts;
do not delete the volume unless that data is no longer needed. `docker compose logs -f`
shows server logs, and `docker compose down` stops the service without deleting the volume.

The separate `aruvi-repositories` volume is mounted at `/repositories`. For example,
after starting the service, `docker compose exec aruvi-studio git clone <repository-url> /repositories/my-project`
clones a repository into that persistent volume. In **Workspaces**, register its container
path and attach it to a product. Use the repository's normal authentication method for
private remotes. The workspace must be accessible to the container's `aruvi` user.

This container does not automatically include the Mac's repositories, credentials,
coding CLIs, or Docker socket. Configure explicit mounts and install or provide the
tools required by the workflows before expecting agent execution to work. Paths saved
in a desktop database must be valid inside the container. The Docker setup and full
feature parity are still being validated; see [the parity audit](web-parity-audit.md).

In the remote **Models** tab, add an OpenAI-compatible provider and a model, then select
**Use for this browser**. The provider's base URL must be reachable from the container;
for a service listening on the Mac, `host.docker.internal` can be used in place of
`localhost`. When the keychain is unavailable, provider keys are stored in the data
volume's `llm-config.json`, with owner-only file permissions on Unix. Verify the
provider URL before entering a real key.

### Connecting a native Aruvi Model Host

Model Host manages its own models and credentials. Start the Studio worktree and
use an already-running native Model Host with an installed coding model. For a
Model Host worktree, configure the Studio provider and model with:

```sh
scripts/configure-native-model-host /path/to/aruvi-model-host-worktree --model YOUR_INSTALLED_CODING_MODEL_ID
```

The helper reads the selected host's private token without printing it, verifies
model discovery from both the Mac and the Studio container, and updates the named
provider without replacing its ID. For native staging, select `--environment
staging` and the checkout used to manage that deployment; use a coding model
actually installed there. The current native staging fixture does not contain one.
Use `--name` to choose a different provider name. This setup stores the host's
bearer token in Studio's secret store; it grants the token's full Model Host
permissions. The helper neither installs models nor assigns them to agents.

Native worktree ports can change after a restart. Add `--watch` to keep the helper
running in the foreground; it polls every five seconds, retries outages, and
refreshes the provider when the endpoint or token changes. Stop it with Ctrl-C.
This watcher is opt-in and does not start automatically at login. Requests may
fail during a restart until refresh succeeds. Run the one-shot command again if
no watcher is active. The helper targets Studio's Docker worktree, using
`host.docker.internal`; a native desktop setup must use a host-reachable address.

To exercise the browser workflow without changing the running service's data, run
`scripts/test-container-workflow.sh` after `bash scripts/env worktree up`. It starts a
temporary server and repository on the worktree network and a simulated model provider, then checks task creation,
plan approval, coding, test execution, the test-review gate, and delivery to disposable
local bare Git repositories. It does not test a real model provider or authenticated
network Git remote.

To exercise a real coding model without changing the running application's data,
run an isolated end-to-end workflow against an already-running native Model Host:

```sh
ARUVI_CODING_TEST_MODEL=YOUR_INSTALLED_CODING_MODEL_ID bash scripts/test host-coding-workflow --scenario extend-module
```

The `extend-module` scenario starts a disposable Studio fixture, edits an existing
JavaScript module, verifies both its original behavior and the new function in a
network-isolated container, and checks the resulting commit in a disposable local
Git remote. The default `new-module` scenario checks creation of a new file. Both
make real inference calls and model output can vary; a passing run is evidence for
that task, not a general coding-quality guarantee. The fixture reads the validation
host's private token and does not alter the host configuration or the branch data.
The optional first positional argument selects a different native host worktree.

The **CI** tab and the `aruvi_ci` MCP tool can inspect Aruvici health, targets, runs
and stage logs, request cancellation or retry, and record feedback on passed runs.
The browser polls selected active-run logs and retains at most 1 MiB of log text.
The service is separate and is not included in Compose. An operator may
set `ARUVI_CI_SOCKET` to an absolute Unix-socket path available to the server process;
when unset the desktop's user-data socket location is used. A socket must also be
reachable from the container and accessible by its `aruvi` user. Docker Desktop on macOS
may require an explicit host-to-container transport, and host and container repository
paths must be reconciled before build dispatch can work. Connecting a socket grants the
application the authority of that CI service; connect only a trusted service. The web
CI view can assign a matching target to an attached product workspace and select a local
branch or a commit from the latest 100 reachable commits; a branch resolves to its
current commit before dispatch.
Pipeline approval and target issues are checked at queue time, and automatic delivery
can only be enabled for Preview targets. The CI service must enforce its own policy as
well, since its state can change between the check and dispatch. Run details can request
artifact readiness verification from Aruvici. The web view does not install or promote
artifacts, reveal them in Finder, or configure or launch Aruvici.
Run `scripts/test-container-ci.sh` after starting the worktree to test the
browser and MCP read paths against a disposable simulated Unix-socket service.

In **Work**, configure the workflow test command as a JSON array of executable and
arguments, such as `["npm", "test"]`. The executable and its dependencies must be
installed in the server container. Select a product and choose its own command, disable
tests for manual review, or use the existing default command. The default is a legacy
global setting and may be unset. The command runs in the attached repository as the
server user; configure it only for repositories and commands
you trust. Environment variables are cleared except `PATH`, `HOME`, and `CI`. The command
and its output are stored in application data, so do not put credentials in arguments.
The command has a 120-second timeout and its captured output is limited to 16 KiB per stream.
With no command configured, tests are reported as skipped and test review requires a
manual decision even if automatic test approval is enabled. A failing command fails the
workflow before it can advance toward Git push.

## Isolated development environments

The product adapter follows the shared `work/.org/environments.md` contract.
`bash scripts/env worktree up` builds and starts an isolated stack on a dynamic
loopback port, with separate SQLite data and repository volumes. `bash scripts/env
worktree status` shows the port, `bash scripts/test ui` runs a browser navigation
smoke test plus isolated remote UI, CI and workflow suites, and `bash scripts/env worktree destroy` removes only
that worktree's stack and volumes. Per-environment mobile and MCP tokens are generated
in the private environment state directory outside the checkout. `scripts/test unit`
and `scripts/test integration` invoke the project's existing test commands and require
their local toolchain dependencies.

On macOS, `bash scripts/test host-layout` uses locally installed Node dependencies
and Google Chrome to visit all 13 remote sections at phone and desktop viewport sizes.
It checks page errors and document-level horizontal overflow against the running
worktree app; it does not replace the interactive or isolated workflow tests. Set
`ARUVI_CHROME_PATH` if Chrome is installed elsewhere.
`bash scripts/test host-planner-scope` exercises the working-tree planner product
selector against simulated responses; it does not validate the server or provider.

For the mocked desktop Playwright suite, a locally installed Chrome can be selected
with `ARUVI_PLAYWRIGHT_EXECUTABLE_PATH`; `ARUVI_PLAYWRIGHT_PORT` chooses the Vite
port, and `ARUVI_PLAYWRIGHT_REUSE_SERVER=0` prevents attaching to an existing server.
`ARUVI_PLAYWRIGHT_LIGHTWEIGHT=1` disables traces, screenshots and video when the
Playwright browser and video bundles are not installed. Run `npm run test:e2e:raw`
after installing the Node dependencies. These tests use mocked Tauri commands.

Staging and production adapters consume an explicit image and keep separate persistent
volumes. The shared launcher requires staging builds from a clean checkout at the
configured default branch. These adapters have not yet been validated through a real
staging deployment or promotion; do not infer release readiness from their presence.
The original `compose.yaml` and its existing volumes remain a separate local deployment.

### Calling the worktree MCP server

After `bash scripts/env worktree up`, use `scripts/mcp` from this checkout. It finds
the current loopback port and reads the worktree MCP token from its private state
file; the token need not be placed on the command line. The wrapper prints JSON
and returns a nonzero exit status for HTTP, protocol, or tool errors.

```sh
scripts/mcp tools
scripts/mcp call aruvi_catalog '{"action":"list_products","arguments":{}}'
scripts/mcp call aruvi_catalog '{"action":"create_product","arguments":{"name":"Example product"}}'
scripts/mcp call aruvi_work_items '{"action":"list_work_items","arguments":{"product_id":"PRODUCT_ID"}}'
```

The result of an action is in `result`; use the returned product ID in later calls.
Pass `-` instead of the JSON argument to read the argument object from standard input;
use that form or a private input file when arguments contain sensitive information.
Tool definitions include supported actions, and the `aruvi_workflows` and
`aruvi_checkpoints` tools can start workflows and handle approval gates. These
operations can run agents, execute configured test commands, and push to a configured
repository; configure and review the target before starting a real workflow. The
worktree's data and credentials are separate from the legacy Compose deployment.
The worktree port may change when the container is recreated; the wrapper discovers
it on every call. This wrapper is a command-line HTTP client; it does not register
the server as a native tool in an already-running Codex session.

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
# Optional: override defaults for slow model hosts (applies to all providers).
export ARUVI_MODEL_CONNECT_TIMEOUT_SECS=15
export ARUVI_MODEL_REQUEST_TIMEOUT_SECS=300
export ARUVI_WEBHOOK_HOST=127.0.0.1
export ARUVI_WEBHOOK_PORT=8787
export ARUVI_MOBILE_API_TOKEN=replace-with-a-long-random-token
export ARUVI_MCP_API_TOKEN=replace-with-a-different-long-random-token

./src-tauri/target/release/aruvi-studio-server
```

`ARUVI_LLM_CONFIG_PATH` is useful on a headless Linux service because a desktop keychain may not
be available. Restrict that file to the service account because it contains provider credentials.

The model timeout settings are service-wide: they apply to chat, agent and planner
requests, discovery, streaming and remote speech. The defaults are 15 seconds to
connect and 300 seconds for the complete request, including a streamed response.
Both settings must be whole seconds from 1 to 86400, with the connection timeout
no greater than the request timeout. Invalid settings make model operations fail
with a validation error. For Compose, set the same names in the environment's
private `environment.env` or the invoking environment and recreate its service;
the running deployment does not change until recreated. Per-provider timeouts
are not yet supported.

Keep the database and its sibling `-wal` and `-shm` files in a directory writable
only by the service account and trusted administrators. The current bundled
SQLite is affected by [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
when a crafted database and WAL pair is opened. Do not point `ARUVI_DB_PATH` or
the desktop database override at files supplied by an untrusted party. Directory
permissions reduce the attack surface; they do not patch the vulnerability.

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

Workflow start requests wait until the initial execution reaches a stable stage and return the
current `workflow_run`. A failed start returns a non-success HTTP status with a JSON `error` and,
when a run was created before the failure, its latest `workflow_run` state. It does not report that
work was queued successfully. Repeating a start request for a work item with an active workflow
returns that same run rather than creating a duplicate.

Git delivery requires an initial commit, the configured branch checked out, an `origin`,
and a Git author name and email. The workflow records generated paths and their hashes;
delivery checks that the starting commit and generated files have not changed, rejects
an already-staged index, and commits only the recorded paths. Unrelated unstaged and
untracked files are left out. Synchronization refuses to overwrite an already dirty
destination and rejects symlinked or hard-linked destinations. Workflows started before
the delivery manifest was introduced cannot use this automatic delivery path and need
to be rerun or reconciled manually.

For the latest workflow, a failed Git-delivery stage offers **Retry Git delivery** in
Delivery and the `aruvi_workflows` MCP action `retry_git_delivery`. Review the failure
first: retry can create a commit and push to the configured origin. If a commit was
already recorded before a push failed, retry reuses that commit. A failure or interruption
between copying files and recording the manifest, or between creating a commit and
recording it, can require manual reconciliation. Copying multiple files is not atomic,
and concurrent external filesystem or Git operations can race these checks. Test an
authenticated network remote and its credentials before relying on this for release.

The remote UI also supports product hierarchy, workspace, provider, model, agent, team and skill
configuration. Some desktop features remain incomplete; see [the parity audit](web-parity-audit.md)
for current coverage and validation limits.

Agent tool lists control the coding tool loop. An empty list retains the default repository tools,
including write access. To restrict access, use exact names: `repo.list_tree`, `repo.read_file`,
`repo.search`, `repo.write_file`, `repo.replace_range`, or `repo.apply_patch`. Legacy aliases are
also accepted: `read`, `read_file`, `read_files`, `read_code`, `read_context`, `read_tests`,
`write`, `write_file`, `write_files`, `write_tests`, `create`, `create_file`, `create_files`,
`modify`, `modify_code`, `patch`, and `apply_patch`. Aliases expand to the related group of tools;
exact `repo.*` names grant only the named tool. Unknown entries grant no tools. Earlier builds used
substring matching and could grant write access for unrecognized lists; review existing lists when
upgrading. These controls do not sandbox the server process or configured workflow test commands.

Activity can show recorded model-call request and response snapshots on demand, and
`aruvi_models` exposes `read_model_call_snapshot`. These snapshots can contain prompts
and model output. Remote reads require the corresponding bearer token, are confined to
the artifact directory, and reject snapshots larger than 1 MiB.

Agent execution boundaries must be a JSON object containing recognized fields with valid types.
Supported fields are `instructions`, `max_tokens`, `max_context_chars`, `context_window_ratio`,
`max_files_per_run`, `max_read_file_chars`, `max_write_file_chars`, `max_file_chars`,
`max_repo_files_scanned`, `allowed_paths`, `blocked_paths`, `keep_workspace`, and
`max_tool_steps`. Invalid JSON, invalid types, and unknown field names stop execution instead of
silently dropping all limits. Review existing agent definitions after upgrading. Prompt template
references are currently stored as metadata; the execution prompt builder does not apply them.
