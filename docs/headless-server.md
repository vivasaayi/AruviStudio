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

The headless server responds to SIGTERM and SIGINT by stopping new HTTP requests
and allowing active HTTP responses to finish. The container runtime may enforce
its own stop deadline. Background workflows, imports and external CLI processes
are not drained by this HTTP shutdown; check for active work before restarting,
and review recovery state afterward.

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

For Docker Desktop on the Mac, **Use Docker Desktop host address** converts a
loopback URL in the provider form to `host.docker.internal` while preserving
its port and path. It only fills the form; save the provider to apply it. This
hostname is Docker Desktop-specific, and the model service must be reachable
from the container. HTTPS still requires a certificate valid for the chosen
hostname. Existing saved provider URLs are not rewritten automatically.
`bash scripts/test docker-host-model` checks model discovery and MCP completion
through the host alias using a disposable loopback fixture; it does not test an
installed LM Studio server. Set `ARUVI_TEST_APP_IMAGE` to test a local candidate.

In **Chat**, select **Stream response** to show text as it arrives and use
**Stop** to cancel the request. Partial output is marked incomplete when a
stream fails or is stopped, and is not added to the conversation history.
Streaming uses the configured provider's shared client and records completed,
failed and cancelled calls in the existing model-call ledger. If the browser
cannot consume the bounded event queue fast enough, the stream fails rather than
silently dropping text. Ordinary non-streaming chat remains available.

In **Workspaces**, open a text file and ask Copilot about its saved or unsaved
contents. Choose a Copilot model or use the Connection model. Streaming and
buffered responses are available, and streaming can be stopped. Studio sends
the question, relative path, selected file context (up to 12,000 characters),
and bounded conversation to the provider; snapshots are recorded in Activity.
The browser retains at most ten recent user/assistant pairs within 24,000
characters and resets the conversation when its context changes. It rejects a
file whose saved content or registered workspace path has changed since opening.
Patch proposals require saved file content, explicit review, and guarded
application; applying multiple files is not atomic. Repository content is
untrusted input and model guidance needs review.
MCP clients can call `aruvi_repositories` action `get_repository_file_sha256`,
then call `aruvi_models` action `run_file_copilot` with `repository_id`,
`relative_path`, `expected_sha256`, `provider_id`, `model_name`, and `question`.
Supply the `sha256` and `workspace_root` returned together by
`get_repository_file_sha256` as `expected_sha256` and `expected_root` to reject
a changed file or registered workspace path. The equivalent first-class tool is
`repositories.files.get_sha256`; it also accepts an optional `expectedRoot`
when an earlier workspace snapshot needs to be checked. Older Copilot callers
may omit `expected_root`, but then do not have the workspace-path guard.
Optional `working_content` lets MCP callers discuss unsaved text; patch mode
requires saved content. MCP responses are buffered.
An optional `history` array contains alternating user/assistant pairs, up to ten
pairs and 24,000 characters total. MCP clients own that history and should only
reuse it for the intended file and provider.
The operation uses the same file confinement, stale-file check, context limit,
and Activity recording as the browser; it does not modify the file.

For a general Copilot request without a file, MCP clients may call
`aruvi_models` action `run_general_copilot` with `provider_id`, `model_name`
and a `messages` array of role/content objects. Optional `temperature`,
`max_tokens`, and `patch_mode` control generation. Patch mode adds the same
server-side proposal instruction used by the browser; clients must preview and
explicitly apply proposed changes using the guarded repository actions. The
response is buffered, and callers own and bound their conversation history.
The shared general-Copilot validator accepts at most 100 messages and 256,000
Unicode characters, requires a nonempty user message, permits system/user/
assistant roles, and limits temperature to 0–2 and `max_tokens` to 1–32,768.
The browser's own conversation limits are smaller. These bounds also apply to
the browser's buffered and streaming general-Copilot endpoints; the general
model-chat action remains a separate API.

To stop using a provider key, the browser's **Detach API key** action disables
that provider's configured credential and legacy fallback keys. MCP clients can
call `aruvi_models` with action `get_provider_credential_state` and the provider
`id`, then call `detach_provider_credential` with that `id` and the returned
`credential_revision`. A concurrent credential change invalidates the revision;
fetch the state again before retrying. Detaching does not erase or revoke the
stored key, which may be shared by another provider. Replacing the key re-enables
credential use.

### Connecting a native Aruvi Model Host

Model Host manages its own models and credentials. Start the Studio worktree and
use an already-running native Model Host with an installed coding model. For a
Model Host worktree, configure the Studio provider and model with:

```sh
scripts/configure-native-model-host /path/to/aruvi-model-host-worktree --model YOUR_INSTALLED_CODING_MODEL_ID
```

For a host version that supports a separate inference credential, explicitly add
`--credential-scope inference`. The helper requires `MODEL_HOST_INFERENCE_TOKEN`
in that host environment's private configuration and refuses to fall back to
the management token. Before configuring Studio it checks that the credential is
denied `GET /api/models`; it then verifies model discovery as usual. That one
negative check is not a substitute for reviewing the host's full authorization
policy. The currently deployed host may not support this option yet. Without the
flag, the helper retains its existing management-token behavior. A failed scoped
configuration leaves any previously configured provider credential unchanged;
it does not remove or revoke an existing management token.
The independent-client probe also accepts `--credential-scope inference` after
its model argument and performs the same check before launching the container.
Its optional `--native` mode avoids a Docker build and runs the pinned consumer
as the current user on loopback. It requires the Git revision and Cargo
dependencies already cached; the default container mode remains non-root and
read-only. For example:

```sh
bash scripts/test host-shared-client /path/to/aruvi-model-host --native --model YOUR_INSTALLED_CODING_MODEL_ID
```

Select `--environment integration` when appropriate. To check synthetic speech,
provide both `--voice-model` and `--transcription-model`; all selected models
must be advertised by that host. Both modes provide the credential on standard
input and suppress provider output on failures. A native probe is not container
isolation or evidence of physical microphone and speaker behavior.

To exercise a candidate host binary with an installed local model without
changing a running host, use the disposable real-model fixture (it copies the
model into temporary storage and starts an extra CPU worker):

```sh
bash scripts/test scoped-real-host --host-binary /path/to/candidate-host \
  --worker-binary /path/to/llama-server --model-file /path/to/model.gguf \
  --studio-image YOUR_LOCAL_STUDIO_IMAGE
```

It creates synthetic credentials and isolated data, checks that management is
denied to the scoped token, and tests Docker Studio MCP and API streaming. This
is an acceptance fixture for the supplied binary and model, not a deployment.
Add `--browser` to also check streaming in local Chrome; `ARUVI_CHROME_PATH` can
select a different executable. The fixture passes its synthetic Studio token to
the browser helper on stdin.

The helper reads the selected host's private token without printing it, verifies
model discovery from both the Mac and the Studio container, and updates the named
provider without replacing its ID. Discovery rejects HTTP redirects. For the
current native Mac integration host, select `--environment integration` and the
checkout used to manage that deployment; choose a coding model actually installed
there. Its retained service identity may still contain the word `staging`.
Use `--name` to choose a different provider name. This setup stores the host's
bearer token in Studio's secret store. In the inspected Model Host source, this
same token authorizes both `/v1` inference and `/api` management operations,
including model registration, installation, loading and unloading; it is not an
inference-only credential. Anyone able to extract the token and reach the host
can use those management routes. Limit access to Studio's secret store and the
host endpoint accordingly. The helper neither installs models nor assigns them to agents.
It requires a model advertised with the `text` task and selects the standard
OpenAI-compatible completion profile for that Model Host provider. Choose the
registered model in browser chat or bind it to the desired agents in Models;
adding a provider does not change existing bindings automatically.

`bash scripts/test host-voice-browser --planner` exercises synthetic browser
recording and real Model Host transcription, then sends a corrected review
transcript to Planner. It does not test a physical microphone or guarantee
hands-free recognition accuracy.

For DeepSeek's standalone voice API, configure an OpenAI-compatible DeepSeek
provider with its API key. Select `deepseek-voice-transcription` for speech to
text, or `deepseek-voice-synthesis` for speech generation. These are Studio
selectors, not IDs sent to DeepSeek. DeepSeek detects transcription language;
the locale hint is not sent. Synthesis accepts `stella` or `tide` and does not
support speed control. The remote browser accepts an optional voice and uses
the provider's default when it is blank.
The service enforces its own audio format and duration limits. This integration
has simulated-provider coverage; live DeepSeek voice is not yet verified.

Native worktree ports can change after a restart. Add `--watch` to keep the helper
running in the foreground; it polls every five seconds, retries outages, and
refreshes the provider when the endpoint or token changes. While the host is
unchanged it also probes Studio's MCP provider list. If that probe fails, it
retries and revalidates the provider when Studio returns; it does not rotate an
unchanged credential. Port-only changes also reuse the stored credential;
changed tokens are rotated. Stop it with Ctrl-C.
This watcher is opt-in and does not start automatically at login. Requests may
fail during a restart until refresh succeeds. Run the one-shot command again if
no watcher is active. The helper targets Studio's Docker worktree, using
`host.docker.internal`; a native desktop setup must use a host-reachable address.
The probe detects reachability, not every provider or credential edit, and a
brief outage entirely between polls can be missed.

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

For an explicitly trusted Mac Docker Desktop setup, `scripts/ci-relay --socket
/absolute/path/to/ci.sock` starts an opt-in relay bound only to host IPv4
loopback and prints its selected port. Set a fresh random
`ARUVI_CI_RELAY_TOKEN` in the relay's environment, then put the same token and
`ARUVI_CI_RELAY_URL=http://host.docker.internal:PORT` in the worktree's private
environment file and restart the stack. Use a fixed `--port` if desired. Studio
accepts relay URLs only for localhost, loopback or `host.docker.internal` with
an explicit port; when configured it fails closed instead of falling back to the
Unix socket. Clear the URL to use the socket again.

The relay carries its token and CI requests in plaintext over the local Docker
host connection; it is not suitable for an untrusted network or multi-user host.
Docker administrators and processes with access to the container environment can
read its token. A holder can invoke the allowed CI operations, including queueing
approved builds and potentially triggering configured Preview behavior. Use a
dedicated token and trusted CI service, protect the private environment file, and
stop the relay when it is not needed. This transport has been tested for status
reads against a disposable Aruvici service; it is not installed automatically.
To match a host CI target to a container repository, set both
`ARUVI_CI_HOST_REPOSITORY_ROOT` and `ARUVI_CI_CONTAINER_REPOSITORY_ROOT` in the
private environment file. The first is the canonical absolute host directory;
the second is its mount path below `/repositories`. For a worktree, after the
normal worktree has been initialized and built, `scripts/ci-shared-worktree-up`
recreates it with that host root mounted read-only. A normal `scripts/env
worktree up` does not apply this optional override; use the helper again when
the CI mount is needed. The helper passes its validated mapping and relay values
to Compose even when the invoking shell defines conflicting values. The mount
exposes the selected host files to the Studio
container, so choose a dedicated trusted directory. It is read-only: editing or
workflow delivery to that mounted workspace is not supported by this helper.

Studio translates only target paths under the configured host root, resolves
the corresponding path inside the container root, rejects traversal or escapes,
and compares it with the registered repository before binding and again before
queueing. The operator must ensure the roots really represent the same shared
directory; configuration alone is not proof of identity. A disposable real CI
build through Docker Studio and the relay passed with this arrangement. Stable
environment mounts, live CI service deployment and broader product builds have
not been validated.

In **Workspaces**, select **Use code editor** to enable the locally served Monaco
editor; **Use plain text editor** switches back without discarding edits. Both use
the same conflict-checked server save and ⌘/Ctrl+S. Open-file buffers are held in
the browser and do not survive reload. Files with mixed line endings remain
read-only. Docker images include the pinned npm dependency and its notices;
they do not fetch editor code from a CDN. For a separately run native server,
set `ARUVI_MONACO_ASSETS_DIR` to the absolute `monaco-editor/min/vs` directory
from the matching `npm ci` installation. If assets are absent, the plain editor
remains available. Native application packaging of these web assets is not yet
validated.

In **Work**, a selected item can override its workspace and branch, or return to
its product/area scope default. The workspace must already be registered on the
server; it need not be attached to the item's product. Choosing a branch does not
create or check it out. The same operation is available through the `aruvi_work_items`
MCP action `assign_work_item_workspace`, with `id`, `repository_id`, and
`branch_name`; use null for both latter fields to clear the override. Browser
edits compare the previously read assignment; MCP callers may provide an
`expected` object containing `repo_override_id`, `active_repo_id`, and
`branch_name` for the same stale-write protection. Without it, MCP writes are
last-write-wins. Desktop, browser and MCP reject changes while the work item has
a running or paused workflow. These guards do not coordinate non-cooperating
database writers.

The Work view can delete a work item after confirmation. Its direct children are
detached, related workflow history and artifact records are deleted according to
database foreign keys, and workspace and artifact files are not removed. Desktop,
browser and MCP reject deletion if the item or a direct child has a running or
paused workflow. Browser deletion also compares the displayed work-item fields;
MCP `delete_work_item` may supply the complete previously read item as `expected`
to request the same stale-write check. Without that baseline, desktop and MCP
deletes do not compare earlier edits. Non-cooperating database writers and other
external filesystem changes are outside these checks.

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
workflow before it can advance toward Git push. A passing command establishes only what
that command checks: a syntax-only check can pass code with broken exports or behavior.
Configure meaningful behavioral tests and review the result before approving delivery.

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

To start this worktree's already-configured local image without building or
pulling, use `bash scripts/env worktree up --no-build`. It validates the
worktree configuration with the shared launcher and requires the image to exist
locally. It does not rebuild the image, so check that the image matches the
source revision you intend to run. The command refuses configurations with a
database service or migration/seed hooks; use the normal launcher for those.
`bash scripts/test worktree-no-build` exercises this command with a fake Docker
CLI, including missing-image and unsafe-port failures.

On macOS, `bash scripts/test host-layout` uses locally installed Node dependencies
and Google Chrome to visit all 13 remote sections at phone and desktop viewport sizes.
It checks page errors and document-level horizontal overflow against the running
worktree app; it does not replace the interactive or isolated workflow tests. Set
`ARUVI_CHROME_PATH` if Chrome is installed elsewhere.
`bash scripts/test host-planner-scope` exercises the working-tree planner product
selector against simulated responses; it does not validate the server or provider.
For `aruvi_planner` action `match_planner_actions`, callers may provide a node's
`ancestors` as ordered objects with `label` and `node_type`. The matcher uses
known ancestor names to exclude actions with conflicting scope, while retaining
ambiguous actions that omit scope. Older callers may omit this field. Names can
change or repeat, so matches are not stable identifiers or an audit record.

On macOS, `bash scripts/test native-database-override /path/to/aruvi-studio-server`
uses a disposable native server and temporary databases to check override,
restart persistence, clearing, environment precedence, authentication and an
upgrade from a reconstructed migration-52 database with existing data. It
does not change the running worktree database or validate staging migrations. Add
`--browser-executable /path/to/chrome` to keep a headless browser open across
restarts and exercise the Routing controls. Restarts reuse one dynamically
chosen loopback port, and the browser checks its saved synthetic token after
reload without entering it again. The browser test also delays settings reads
and saves to check that unsaved path edits survive their responses.

Database override settings affect the next server restart, and `ARUVI_DB_PATH`
takes precedence. Browser writes compare the last loaded override and offer an
explicit discard-and-reload action after a conflict. MCP clients can use
`aruvi_settings` `get_database_path_override`, then include an `expected`
object such as `{"path": null}` or `{"path": "/data/old.db"}` in
`set_database_path_override` or `clear_database_path_override`. A mismatch
returns an MCP error and leaves the setting unchanged. Omitting `expected`
retains legacy last-write-wins behavior. The comparison is value-based and
does not detect an intervening change that restores the same value; direct
file writers and old binaries do not participate in the Unix advisory lock.
Saving does not verify that the new database is valid.

`bash scripts/test database-override-multiprocess` starts two disposable Linux
containers sharing a uniquely named test volume and checks concurrent guarded
updates. Set `ARUVI_TEST_APP_IMAGE` to use a locally built candidate image.

For low-storage hosts, `bash scripts/test remote-host-fixture`,
`bash scripts/test workflow-host-fixture`, and `bash scripts/test ci-host-fixture` run the isolated application and PDF
fixtures in Docker while using the host's installed Node dependencies and Chrome
for Playwright. Set `ARUVI_CHROME_PATH` to the Chrome executable if needed. The
fixtures use a dynamic loopback application port. The remote and workflow modes
require loopback port 19876 for their mock provider and fail if it is occupied;
the CI mode uses a simulated CI socket and does not publish that port. Their disposable data,
containers and volumes are removed on exit. They use the worktree image, or the
image selected by `ARUVI_TEST_APP_IMAGE`; ensure it matches the source being
tested. The workflow fixture uses synthetic input, not a downloaded speech
model or live provider, and the remote fixture does not save book captures.

To add a synthetic 256 MiB-plus streaming case to the native CI artifact
browser fixture, set `ARUVI_TEST_CI_LARGE=1` when running
`scripts/test-native-ci-service` with `--audit-protocol --exercise-protocol
--artifact-browser /path/to/chrome` and the Studio and Aruvici binary paths.
The large case simulates both chunk responses and the save picker; the smaller
case uses the disposable Aruvici service but also simulates the picker.
Add `--large-real-artifact` to create a disposable 256 MiB-plus file in the
native Aruvici fixture and transfer it through the socket. This uses temporary
host disk space and has a five-minute test timeout. An initial debug-build run
timed out; a subsequent run passed with the updated Aruvici socket and base64
protocol. Progress is reported every 32 MiB.
Use `--relay-artifact` instead of `--large-real-artifact` with the protocol
fixture to verify a 512 KiB chunk through a disposable native Studio instance
and authenticated loopback relay, including invalid-token and forbidden-method
checks. It does not exercise a deployed container or a full large transfer.

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
`bash scripts/test staging` checks the locally published health endpoint, then
runs a read-only browser navigation smoke test on the stack's Compose network.
Run it on the deployment host with its local Docker daemon. Browser navigation
does not traverse the published port or external ingress, and this smoke test does
not replace authenticated workflow, persistence, or artifact identity checks.
`bash scripts/test environment-acceptance <worktree|integration|staging>`
creates a uniquely named and tagged fixture product, area and task through MCP,
checks authenticated mobile reads and PDF export, and archives the fixture in
cleanup. Pass `--expected-image-id sha256:<full-image-id>` to require both app and
PDF containers to use the specified local image ID; it is checked again after a
requested restart. Without this option, the script only reports the observed ID.
Compare the image ID with the separately recorded, source-verified artifact before
acceptance; an image ID alone does not establish its source revision or registry
digest. Add `--restart` to restart that environment's app and PDF services and
repeat the reads and export after both are healthy. This briefly interrupts that
environment and leaves an archived fixture and its related records in its data;
the script reports the product ID if cleanup needs attention. It does not test
external ingress or model inference. Run it on the
deployment host with access to that environment's private credentials.
The original `compose.yaml` and its existing volumes remain a separate local deployment.

For a same-origin browser accessing MCP through a reverse proxy, set
`ARUVI_MCP_ALLOWED_ORIGINS` in the environment's private `environment.env` to
the exact externally visible origin, for example `https://studio.example`.
Separate multiple origins with commas; do not include paths, trailing slashes,
wildcards, credentials or default ports. The proxy must preserve the external
`Host` header, provide TLS, and restrict access according to the deployment's
policy. Studio does not trust forwarded-host headers. The Origin and Host must
match and the origin must be an allowed local/LAN origin or an exact configured
entry. A configured external-origin list requires an MCP token even when the
server binds to localhost. An invalid entry never grants access, but still
disables tokenless MCP. Restart after changing this environment value. This
configuration has been tested with synthetic proxy headers, not with the VPS
ingress; verify the actual proxy and authorized browser during staging acceptance.
On macOS, `bash scripts/test tls-mcp-proxy` exercises the built worktree image
through a disposable local HTTPS proxy and headless Chrome, including missing-token
and untrusted-host rejection. It uses a temporary self-signed certificate, so it
does not validate certificate trust, the VPS proxy, or staging configuration.

### Existing VPS staging and Compose transition

Read-only inspection on 2026-11-20 found an existing native systemd staging
service on the VPS at `127.0.0.1:8788`, with a separate production service at
`127.0.0.1:8787`. Both were healthy and ran revision
`2d1130f921e9967cc8dffa77953ebc1355802e25`. Their SQLite databases exist under
`/var/lib/aruvi-staging` and `/var/lib/aruvi-prod`. The staging Nginx site points
to port 8788 and exposes the remote UI, mobile API and health, but not MCP.
The Compose staging adapter instead reserves loopback port 20280 and its own
volumes. It does not automatically adopt the native service's data, credentials,
or ingress; starting it alone will not update the existing staging website.

The inspected VPS was Linux x86_64. Docker used a rootless daemon, while root's
default Docker context could not reach a daemon; `/root/work/.org/environments.md`
was absent. Before deploying a merged candidate, identify the authorized daemon
user and install the shared environment contract in that user's environment;
build and record a Linux AMD64 artifact from the same revision accepted on Mac
integration. Keep native staging and production running while the independent
Compose staging candidate is validated. Plan any data migration or ingress
switch explicitly, including backup and rollback; never point two writers at the
same SQLite data. If exposing MCP through HTTPS, update and validate the actual
infrastructure-owned ingress and configure the exact origin and token described
above. The existing native staging installation is not evidence that this branch
or the new adapter has been deployed or accepted.

The rootless daemon was reachable as `dockerapps` using its user runtime socket.
A disposable two-container network probe on that daemon verified a service alias
and a dynamically published loopback port; its containers and network were
removed afterward and the native services remained active. This used a cached
Python-capable image, not the Studio image, Compose adapter, browser, or ingress.

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

To edit a registered workspace through MCP, first call
`aruvi_repositories.list_repositories` and pass the complete returned repository
as `expected` to `update_repository`, along with its `id`, `name`, `local_path`,
`remote_url`, and `default_branch`. A changed snapshot is rejected. Path,
remote, or branch changes are blocked while an associated workflow or CLI run is
active; inherited work-item branches are updated with the workspace. The web
Workspaces screen provides the same operation; a changed browser path must be
an accessible Git working repository. Paths refer to the Studio server, not the
browser's computer. The desktop and MCP commands retain their existing path
validation behavior.

For workspace files, `aruvi_repositories.read_repository_file` returns the
current `workspace_root` as well as the file content. Pass that value as
`expected_root` to `write_repository_file`, `apply_repository_patch`, or
`apply_reviewed_repository_patch` to reject a workspace path change before a
write. The check and write hold a SQLite write lock against cooperating database
updates. This does not prevent external filesystem changes or provide a file
content conflict check for `write_repository_file`. The root argument is optional
for existing clients; omitting it retains their previous path-selection behavior.

For a large UTF-8 artifact, call `aruvi_checkpoints` with
`{"action":"read_artifact_page","arguments":{"artifact_id":"ARTIFACT_ID"}}`.
Each response contains `content`, byte `offset`, `total_bytes`, `next_offset`
and a `version`. Repeat with that `next_offset` and `version` until
`next_offset` is null. Pages are at most 64 KiB and end on a UTF-8 character
boundary. A detected file change rejects later pages; restart from offset zero.
The version uses filesystem metadata and is not a cryptographic content
snapshot. Access is confined to the current artifact directory, and non-text
artifacts require a different recovery path.

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

In the browser's Activity view, **Configure server bind** loads and saves the
server-side `mobile.bind_host` and `mobile.bind_port` settings. Enter a local IP
address or hostname and a port from 1 through 65535. Saving does not change the
current listener: restart the server to apply the settings. Nonempty valid
`ARUVI_WEBHOOK_HOST` and `ARUVI_WEBHOOK_PORT` values take precedence. Activity's
bridge status reports the configured source, and a stale browser edit is rejected
if either saved setting changed since it was loaded. For Docker, the bind is
inside the container; configure published host ports separately in the product
environment. Choose a local address and available port that will remain
reachable after restart.

Activity's **Manage bridge tokens** can replace or clear saved mobile and MCP
tokens. The form does not reveal saved values; use printable ASCII without
spaces. Token changes apply immediately unless the matching environment variable
is present, in which case it remains authoritative. When an active saved mobile
token is replaced, the browser switches its connection credential to the new
value. Clearing it disables browser/mobile API access until an administrator
configures a token outside the browser. If a token update is interrupted, the
server may have applied it: check the new credential before retrying. Generic
desktop and MCP settings access retain their existing ability to read stored
values. The browser token revision is process-local and value-based: a restart,
different server process or an A-to-B-to-A change has the limitations described
in the parity audit.
The authenticated `/api/mobile/bridge-tokens` endpoint returns configuration
flags and opaque revisions, not token values. A POST supplies the token kind,
expected revision, action and, for replacement, the new token; stale updates
return HTTP 409. Run `bash scripts/test native-bridge-tokens <server-binary> --browser-executable <chrome-path>`
to verify rotation and restart with temporary credentials and data.
For a Docker candidate, set `ARUVI_TEST_APP_IMAGE` to the candidate image ID and
run `bash scripts/test container-bridge-tokens --browser-executable <chrome-path>`.
It uses a disposable volume, loopback port and generated credentials.

The equivalent authenticated mobile API is `GET /api/mobile/bridge-bind` and
`POST /api/mobile/bridge-bind` with `host`, `port` and the two-element `expected`
array from the GET response. A conflicting save returns HTTP 409. Operators can
exercise restart and environment precedence against an isolated native server
with `bash scripts/test native-bridge-bind <server-binary>`.

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
only by the service account and trusted administrators. The bundled
SQLite in older builds is affected by [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
when a crafted database and WAL pair is opened. The current source pins a
locally patched SQLx driver to a binding bundling SQLite 3.54.1; existing
installations must be rebuilt and verified to receive it. Do not point
`ARUVI_DB_PATH` or the desktop database override at files supplied by an
untrusted party. Directory permissions protect the database and its contents.

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
`max_tool_steps`, and `json_object_output`. Set `json_object_output` to `true`
only for a coding agent whose configured OpenAI-compatible provider and model
support JSON-object mode; it defaults to `false`. It applies to coding tool
turns and finalization, not planning or ordinary chat. The adapter checks that
responses are JSON objects, but the coding loop still validates the action
schema and permissions. Providers can reject the option; Anthropic currently
returns an explicit unsupported error. No automatic fallback is attempted.
Invalid JSON, invalid types, and unknown field names stop execution instead of
silently dropping all limits. Review existing agent definitions after upgrading.

The Agents view and `aruvi_agents` MCP actions can manage prompt templates.
To apply one, set an agent's prompt-template reference to `template:<id>` as
displayed by the editor. A template has a role (`*` or the exact agent role)
and up to 1,000 characters of additional instructions. The shared execution
service loads it when a stage starts and includes it in subsequent coding
turns. Missing templates and role mismatches fail that run; older references
without the `template:` prefix remain inactive metadata for compatibility.
Template changes use version checks; a referenced template cannot be deleted.
Instructions are sent to the configured model and do not grant tool permissions
or bypass execution boundaries. Editing a template affects future stage runs.

# External coding CLIs

The Work view and `aruvi_work_items` MCP actions can launch a configured
external CLI against a work item's resolved server workspace. Remote execution
is disabled by default. An operator who chooses to enable it must supply
`ARUVI_EXTERNAL_CLI_ENABLED=1`, set `ARUVI_EXTERNAL_CLI_HOME` to an existing
absolute directory, and provide the selected executable on the server's PATH.
The standard container image does not include these CLIs or their credentials;
an operator must provision them and a dedicated CLI home outside the image.

Remote child processes receive only PATH, the configured HOME and TMPDIR in
their environment. Provider authentication must be configured in that dedicated
home. The CLI runs as the server user with access to the server filesystem and
network, including the selected writable workspace: this is not a sandbox.
Only enable it for trusted users and workspaces. Each remote stream retains at
most 1 MiB and 256 chunks, and history returns the latest 100 runs. Browser
controls require confirmation and the normal mobile API token; MCP uses its
normal token. The actions are `run_external_cli`, `list_external_cli_runs` and
`list_external_cli_events` on `aruvi_work_items`.
Refresh the browser run list and select a run to reload its latest events.
For a disposable native-server and browser check with a fake CLI, build the
debug server and run `node scripts/test-native-external-cli.mjs` from the
repository. Setting `ARUVI_CLI_TEST_IMAGE` runs the same fixture against that
container image; the test creates and removes its own data and fake executable.

An active CLI run prevents deleting or changing the work item's workspace and
prevents another remote CLI run or new workflow for that item. This does not
coordinate different work items that share a workspace, independent filesystem
writers, or subprocesses that escape the process group. Operator configuration
and the installed provider's own security and approval behavior remain relevant.
The atomic run claim duplicates the work-item workspace resolution order; changes
to that resolver must update the claim as well.
On Unix, cooperating Studio processes sharing a SQLite database hold a shared
database-specific startup recovery lock. Startup recovery marks interrupted CLI
runs and bulk imports failed only when no other cooperating Studio process holds
that lock. If one process exits while another remains, a genuinely interrupted
job may remain marked running until all cooperating processes stop and one starts
again.
Older versions, non-Unix processes, independent database or lock-file changes,
and filesystems that do not honor advisory locks are outside this coordination. Recovery does
not guarantee that subprocesses surviving an ungraceful server termination
have stopped; inspect the workspace and processes before retrying.

# Work item decisions

The Work view offers individual approval and rejection. Approval follows the
`workflow.auto_start_after_work_item_approval` setting; the separate
“Approve & Start” button explicitly requests a workflow start. Rejection requires
a reason and returns the item to draft. Browser decisions compare the full
displayed work-item snapshot and reject stale changes or active workflow/CLI
execution. Refresh the item and review it again after a conflict. The approval
record and status update are committed together across browser, desktop and
MCP task-decision paths.

The Work view can filter by status and by an explicitly loaded hierarchy node.
The hierarchy filter shows top-level work directly attached to the selected
area or capability; it does not include descendants from nested nodes. A child
opened from a matching parent can remain selected outside the filtered list.
MCP `aruvi_work_items` / `list_work_items` accepts `source_node_id`,
`source_node_type`, and `top_level_only: true`; `include_pagination: true`
returns page metadata. Without `top_level_only`, children remain included.
The authenticated mobile list endpoint accepts the same scope and top-level
parameters and requires a product when a source node is specified. Both list
APIs use offsets, so concurrent work changes can shift later pages.

Select up to 100 loaded items from one
product for a batch decision. A batch requires every selected snapshot to be
current and free of active workflow or CLI execution; all decisions and
approval records commit together or none do. Refresh and review after a
conflict. MCP exposes the same operation through `approve_work_items` and
`reject_work_items` on `aruvi_checkpoints`. Automatic workflow starts are
requested after commit when the saved policy enables them; the decision
response does not guarantee that every workflow starts successfully.

With both status and hierarchy filters cleared, drag a work card or use its move buttons to
reorder loaded work items. Each move spans at most 1,000 loaded items. The
server checks that the displayed interval still matches the current product
order and commits the new order atomically; refresh after a conflict. Items
outside the interval, including unloaded pages, retain their relative order.
The MCP `aruvi_work_items` action `move_work_item` accepts `product_id`,
`offset`, ordered `items` entries containing `id` and `sort_order`, and the
`source_id` at either end of the interval. The older `reorder_work_items`
action is transactional but does not provide this stale or product-scope check.
