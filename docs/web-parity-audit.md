# Web and MCP parity audit

**Current evidence:** The later checkpoints in [model-client-goal.md](model-client-goal.md)
supersede the historical results in this document. They include real Model Host
browser coding and speech generation, a browser repository-analysis flow with MCP
confirmation, an MCP speech generation/transcription round trip, and a two-server
planner concurrency fixture. These are specific scenarios, not proof of full parity.
Physical desktop interaction and audio devices, live third-party providers, and a
real Aruvici connection remain unverified.

**Earlier revalidation:** Docker capacity was increased and the worktree database
passed a read-only SQLite `quick_check`. The current worktree image built and started
successfully. The Rust library suite passed 145 tests with one live-provider test
ignored; the frontend suite passed 90 tests. The deployed-browser smoke test, 16
remote UI tests, simulated CI test and 17 isolated workflow tests passed, including
the previously failing hierarchy deletion cases. Real providers, real CI integration
and complete desktop feature parity remain unverified. Historical notes below
describe results and limitations at the time they were recorded.

Browser-origin MCP requests previously failed because the server compared the
browser's published port to the container's internal port. It now compares Origin
with the request Host and still restricts allowed hostnames and requires the MCP
token. A host-Chrome test sent an authenticated MCP ping through the dynamically
published port and verified that missing credentials, another port, and a null
origin are rejected. The same test verifies that an untrusted host and duplicate
Origin headers are rejected. Reverse-proxy deployments and public hostnames require separate
configuration and validation; forwarded host headers are not trusted by this check.

On the dedicated `codex/aruvi-web-docker` branch, the worktree image rebuilt and the
deployed smoke, 17 remote UI, simulated CI and 17 isolated workflow tests passed.
The remote UI tests include a delayed simulated planner response to check that scope
and draft selection remain locked during a turn. Host Chrome also checked all 13
sections at phone and desktop widths, and 14 mocked Tauri browser tests passed.

The later multi-process planner checkpoint in `model-client-goal.md` supersedes
older concurrency notes below: cooperating Unix Studio processes sharing a
SQLite database now use a per-session advisory lock and reload persisted state.
The fixture covers two Linux containers; Windows and mixed old/new versions are
not covered.

This is a working inventory, not a claim of tested parity. The desktop routes in
`src/App.tsx` are the baseline. The web UI is `src-tauri/src/services/remote_app.html`;
MCP handlers are under `src-tauri/src/mcp/tools`. An MCP action being present does
not establish that all desktop behavior is available or works in a container.

### Audit findings before merge

An isolated MCP-only lifecycle test now creates and updates a product, creates an
area, capability and work item, attaches a Git repository, configures model bindings,
approves both workflow gates, and reaches `done`. It verifies the generated file was
pushed to a disposable local Git remote and that the browser displays the product.
The model service is simulated; this is not proof of real-provider behavior. On the
running worktree MCP discovery and product listing succeeded, but no repository was
registered and the default LM Studio endpoint was unavailable.

- Planner draft editing and repository analysis have browser controls backed by
  Rust services also used by desktop and MCP. Cooperating Unix processes coordinate
  planner session mutations; the limitations are recorded in the planner checkpoint.
- Real Model Host coding and a speech fixture have been exercised. Physical
  microphone and audible playback, live third-party providers and a real Aruvici
  connection remain unverified; automated CI tests use a simulated service.
- The desktop IDE, native file reveal, local model runtime management, channel
  routing and remaining settings need action-by-action comparison or a documented
  browser alternative. Passing section navigation does not establish full parity.
- Staging, production promotion and release acceptance have not been validated.
  The worktree build uses its own image, loopback port and persistent worktree volumes.

| Desktop area | Current remote UI | Relevant MCP | Initial assessment |
| --- | --- | --- | --- |
| Portfolio, product overview, products | Create products, product areas, capabilities and features; edit and delete hierarchy nodes; list, summary and on-demand hierarchy tree; download Product Book Docs HTML, Book HTML, EPUB and PDF | `aruvi_catalog`, `aruvi_work_items` | Partial: browser and MCP exports use the shared renderer, but native PDF generation differs and remaining image, reader and deployment limitations are tracked in [Product Book migration](product-book-migration.md). |
| Work items and delivery | Create work with problem, outcome, acceptance criteria, constraints, type, priority, complexity and hierarchy placement; edit the desktop form's task details and status, approve/start, configure a test command, inspect test results, delivery and gates | `aruvi_work_items`, `aruvi_workflows`, `aruvi_checkpoints` | Partial: navigation and workflow actions need comparison; changing status does not run a workflow, and test commands execute inside the server container. |
| Planner | Sessions, turns, draft editing, repository analysis, confirmation and clear | `aruvi_planner` | Partial: native interaction, physical voice and other desktop behaviors need comparison. |
| Repositories and IDE | Register and attach workspaces; browse and edit text files, inspect Git status and diff, ask Copilot follow-up questions and review multi-file patch proposals through browser or MCP, change defaults and detach for products and product areas | `aruvi_repositories`, `aruvi_models` | Partial: browser file Copilot supports streaming; MCP remains buffered. A streamed proposal passed in the browser with a real local model and a two-file proposal passed through MCP. Native dialogs and other IDE behavior require comparison and implementation. |
| Agents and teams | Manage agent definitions, team profiles, rosters, product/area/capability scope assignments, skill definitions and links, and workflow-stage routing; inspect and replace individual agent model bindings from Models | `aruvi_agents`, `aruvi_agent_work` | Partial: prompt-template application remains unimplemented. A desktop bulk-binding operation was not found in the current agent commands. |
| Models and providers | Add, edit (including OpenAI-compatible, reasoning profile and Anthropic protocol), enable/disable and remove providers and models; register an existing server-side Whisper file, test providers, detach API credentials and select a browser model | `aruvi_models` | Partial: browser preset model downloads and remaining provider/model fields still need comparison. Detaching disables fallback keys but does not revoke provider credentials. MCP and browser local paths refer to the server filesystem. |
| Chat and voice | Chat with optional streaming and cancellation; voice with browser-specific transcription provider, model and locale, transcript review and device speech controls | `aruvi_models`, `aruvi_speech` | Partial: physical microphone, device speech playback and broader provider/model validation remain open. |
| Model calls | Recent calls grouped by session, source filters, selected details and on-demand request/response snapshots in Activity | `aruvi_models` snapshot read | Partial: the view is limited to the latest 300 calls; real-provider validation remains outstanding. |
| CI | Inspect service, targets, revisions, runs and live logs; manage target assignments; queue, cancel or retry builds; request artifact readiness and record feedback | `aruvi_ci` | Partial: artifact access and host service integration require work and validation. |
| Settings | Connection token and model fields; workflow auto-start and approval settings on Routing | `aruvi_settings` | Partial: compare remaining settings, database and bridge controls. |

### Planner action inventory

This comparison is based on the desktop planner, the remote HTML/API and MCP
handlers. An available handler is not evidence of a successful real-model workflow.

| Desktop action | Browser and MCP status |
| --- | --- |
| Create or update a planner session and choose provider/model | Browser connection settings create/update a session; `aruvi_planner` provides both operations. |
| Submit a planner turn with selected draft node and product | Browser presents a product selector and sends its selected node and product; MCP accepts both. Simulated browser tests cover scope. An isolated MCP test exercised a scoped proposal and confirmation with the real 7B Model Host; other models and real-provider browser turns remain unverified. |
| Select and inspect the staged draft tree | Browser renders/selects nodes, shows their path, summary, source, confidence and evidence, and requests suggested prompts, name-matched recent plan actions and structural validation from shared Rust services. MCP exposes these services; desktop uses the shared suggestions and action-matching commands, and shared validation with a local validation fallback. Matching is by name and can conflate similarly named nodes; recent actions are limited to the current page session. |
| Rename a draft node, add a child or delete a draft node | Browser and `aruvi_planner` expose these operations. An isolated Docker browser test exercised the authenticated API and verified the actual product remained unchanged. A two-server Linux fixture covers cooperating processes sharing a database; see the planner coordination checkpoint for platform and filesystem limitations. |
| Analyze a registered repository into a design draft | Browser and `aruvi_planner` can analyze a registered workspace into the selected product draft. Simulated tests cover authentication, staging and scan-failure recovery. A disposable browser test also analyzed a synthetic workspace with the real 7B Model Host, then confirmed the staged design through MCP; broader repositories and models remain unverified. |
| Confirm or clear a staged design | Browser and MCP expose both; failure, conflict and multi-client behavior require further validation. |
| Export and reveal a design-review packet | Browser can download and MCP can retrieve a Rust-generated HTML packet from a staged session. It contains the tree, validation, current context, pending actions and review sections. Desktop uses the same backend renderer for active sessions and retains native file/reveal; its prior renderer remains as a no-session fallback. The offline SVG and pending-action summaries differ from the older Mermaid packet, and native UI verification remains open. |
| Voice transcription, voice planner commands and native speech | Browser has transcription and device speech controls; MCP exposes speech and voice-turn operations. Real Model Host speech passed through MCP and through browser recording with a synthetic microphone. Desktop view-switch commands, physical microphone and audible playback remain unverified. |
| WhatsApp or phone contact routing | Desktop uses Twilio commands; MCP has channel operations. Equivalent browser planner controls and real-service tests were not found. |

For each desktop action, parity requires a real backend operation, an appropriate
browser or MCP entry point, and tests for success, failure, authorization and
persistence. Native-only behavior such as revealing a file in Finder needs an
explicit browser alternative or a documented platform limitation. Planner session
locking covers cooperating Unix processes under the conditions documented in the
planner coordination checkpoint; direct database writers, mixed versions and Windows
are outside that guarantee.

The 14 host desktop Playwright tests use mocked Tauri calls and cover a deterministic
planner flow, repository analysis and voice commands. They do not prove these actions
work through the running web server or with a real model provider.

A host Chrome test served the working-tree remote HTML with simulated HTTP responses
and verified selected-product and explicitly unscoped planner turns, including clearing
a stale draft-node selection after changing product. If product loading fails, the
user must retry or explicitly choose an unscoped turn. This does not establish
end-to-end behavior with a real model provider.
The same simulated-response test holds a planner turn open and verifies that product
and draft-node controls cannot be changed until it finishes, and that the submitted
node matches the selection made before the request.

### CI action inventory

The desktop CI page and product build panel call the Aruvici bridge in
`src-tauri/src/commands/ci_commands.rs`. The Docker image does not include Aruvici or expose
its socket. On the inspected Mac, `aruvici` was not on `PATH` and the expected
user-data socket was absent; no live CI service was available to validate.

| Desktop action | Dependency or constraint for web parity |
| --- | --- |
| Show service health, configured targets, pipeline readiness and recent runs | Authenticated browser and MCP reads now use the existing bounded JSON bridge. An explicitly configured trusted service still needs end-to-end validation. |
| Expand a run, inspect stages, read logs, cancel or retry | Browser and MCP operations send the desktop protocol to Aruvici. The browser polls active-run logs, loads terminal-run chunks on request and asks for confirmation for cancel/retry. |
| Assign or remove a target for a product repository; enable automatic Preview | Browser and MCP reuse desktop attachment and target-repository checks; attachment existence is rechecked atomically during assignment. Target policy approval remains an independent Aruvici operation; assignment does not imply approval. |
| Choose a registered repository branch or commit and queue a target | Browser and MCP list local branches and the latest 100 reachable commits. They resolve the supplied revision to an immutable Git SHA and validate the binding, matching repository and pipeline readiness before dispatch. Repository paths must refer to the same files from both services. |
| Queue Preview or artifact-only release builds | Preserve the desktop restrictions on target and revision selection; queueing a build must not install, promote or deploy it. |
| Inspect artifact metadata and promotion readiness; record feedback | Run details show artifact metadata and authenticated browser/MCP operations request readiness from Aruvici, which owns the path, size and hash checks. Browser/MCP feedback is stored in Studio after checking the service reports a passed run and a valid commit. |
| Open the installed Preview app or reveal an artifact in Finder | These invoke macOS applications on the desktop host. A Linux container cannot perform the same action locally; an explicit host integration or a suitable browser-oriented alternative is required. |

Workflow delivery already attempts automatic Preview dispatch for matching bindings,
but in this Compose deployment the service is unreachable. Failures are retained in
`ci_dispatches` and do not prove a build ran. The Docker Desktop networking, socket
transport, shared repository paths, access controls, and a real service need an
end-to-end design and validation; mounting an arbitrary host control socket would
grant substantial authority and is not configured by default.

The socket path may be configured with `ARUVI_CI_SOCKET`. A disposable container
test used a simulated service on a shared Unix socket and verified authenticated
browser and MCP reads; a separate deployed-container browser test verified the
offline state, authorization and safe rendering. This does not validate the real
Aruvici service, macOS socket transport, or CI build execution. The Rust library
suite passed 140 tests with one live-provider test ignored, and the 17-test isolated
workflow suite passed after this change. A subsequent simulated-service test exercised
run details, chunked logs, browser-confirmed cancel and retry, MCP log access and
unauthorized action rejection. These service responses were simulated, including the
outcome of cancellation and retry.

A simulated-service browser test also assigned a target and queued a branch, observing
the resolved full commit SHA. It verified that an unattached workspace, mismatched
target, missing assignment and unapproved pipeline cannot be used. This does not prove
real-service policy enforcement, network transport or artifact production.

The simulated-service browser test subsequently selected both a branch and a historical
commit and requested artifact readiness; MCP readiness and unauthorized requests were
also checked. Artifact verification was simulated and no real artifact was inspected.

A simulated service test exercised live log polling and browser feedback, checked the
recorded commit through MCP, and rejected feedback for a failed run. The Rust suite
passed 140 tests with one live-provider test ignored. Real CI execution, artifacts and
the macOS service connection remain unverified.

The shared environment adapter now provides worktree, staging and production Compose
definitions and product scripts. Two disposable worktree stacks were started together;
their project names, dynamically assigned ports, tokens, authorization and product
data were distinct. One stack was destroyed while the other remained healthy, and its
browser navigation smoke test passed. The existing local Compose deployment was kept.
The staging and production adapters have not yet been deployed or validated with a
clean default-branch image; the current checkout contains uncommitted changes.
The remote UI, CI and workflow fixture scripts now use the worktree's image and Compose
network, with disposable in-memory data, per-run socket volumes and no host test ports.
The 13-test remote suite, CI suite and all 17 workflow tests passed under that lifecycle.

Model-call request and response snapshots are available on demand in Activity and via
MCP. A simulated agent workflow exercised authenticated API, MCP and browser reads;
unauthorized and invalid-kind requests were rejected. The Rust suite passed 141 tests
with one live-provider test ignored, including confinement, symlink and size-limit
checks. These checks do not prove behavior with a real model provider.

Activity now groups loaded calls using the desktop session/workflow/agent/source keys,
shows token totals, and filters by desktop, mobile, workflow, planner or failure. The
browser test covered grouping, totals, filters, detail selection and safe rendering;
the full worktree UI command passed 32 tests after the change. Activity now refreshes
every five seconds while selected; a browser test verified that an open snapshot and
active filter survive the refresh as new calls arrive.

The browser work-creation form now exposes the existing API's type, priority,
complexity, problem statement and constraints fields. An isolated browser test
created a high-priority, very-high-complexity bug and verified every submitted field
through the API. The full worktree UI command passed 33 tests afterward; hierarchy
placement and the remaining edit and navigation differences are still open.

Hierarchy and parent placement are implemented in the working tree, including checks
that parents belong to the product and children inherit their parent's scope. A browser
test passed the new placement flow, but the same run exposed two hierarchy-deletion
regressions. The working tree now restores SQLite foreign-key enforcement when pooled
connections are released, makes legacy repair transactional on one connection, and
explicitly clears deleted hierarchy links. The Rust suite passed 144 tests, including
new connection, rollback and parent-scope tests. The fixed image and complete browser
regression suite have **not** been rebuilt or rerun: shared Docker storage is full.
The running worktree app therefore does not contain all of these latest fixes.

With Docker storage exhausted, the worktree container still reports healthy and
`/health` and `/remote` return 200, but an MCP ping returned HTTP 503 with a SQLite
disk I/O error, before authentication could be evaluated. The health endpoint is
therefore not evidence that the database or application workflows are usable. Restore
Docker capacity, inspect database integrity, and revalidate the deployment before
performing further writes or claiming it is operational.
The working-tree `/health` now attempts a settings-table read and returns 503 if it
fails, with a Rust test for a closed pool. This change and test have not been built
or run. A successful read still does not prove that storage has room for writes or
that end-to-end workflows work.

The current MCP origin check compares a browser's origin port to the server's bind
port. A Docker worktree publishes a different host port, so browser-to-MCP requests
may be rejected even when they originate from the app itself. This is a source-code
finding, not a verified runtime result: the disk I/O error prevented the origin check
from completing. Any browser feature using MCP needs a secure external-origin design
and an integration test against the published Docker port.

While Docker storage was full, the host frontend build and terminology check passed,
as did 90 frontend tests across 24 files (Node 26 on this Mac). These checks do not
exercise the Docker server or replace the pending browser regression run.

The 14 mocked desktop Playwright tests also passed on host Chrome with a separately
selected local Vite port. The repository test configuration now supports an explicit
browser executable, port, server-reuse setting and lightweight mode. These tests do
not exercise the Tauri backend or the Docker database fixes.

With Docker storage still full, a read-only host Chrome check visited all 13 sections
of the currently running worktree app at 390×844 and 1280×900. It found no page errors
or document-level horizontal overflow. The check does not exercise every control or
validate the newer database fixes that are absent from that running image.

## Validation gates

- Inventory individual desktop actions and their browser and MCP counterparts, including failure behavior and authorization.
- Verify the deployed container, authentication, persistent data, restart and actual browser navigation.
- Exercise representative create, edit, workflow and agent paths against real services and repositories.
- Run the project checks and record results; existing mocked desktop E2E tests alone do not prove web parity.
- Track every missing or intentionally platform-specific behavior and resolve it before declaring parity.

## Initial evidence

- The desktop frontend build and frontend unit suite passed in a Linux ARM64 Node 22 container. These tests do not exercise the remote UI or real backend.
- The ARM64 Docker image built and ran on Docker Desktop. `/health` and `/remote` returned 200, unauthenticated mobile and MCP requests returned 401, authenticated MCP listed tools and read the catalog, and a newly created product survived a container restart before being archived.
- `e2e-remote/remote.spec.js` passed against the live container in Chromium: it connects, visits the six current tabs and checks unauthenticated API rejection. It does not validate every action, every UI state, or agent execution.
- A live Chromium test created a product hierarchy through MCP, verified that the browser does not fetch the tree until requested, exercised retry after a simulated server error, expanded nested nodes, and archived the test product. The terminology check also passed.
- The remote Chromium suite also verifies the authenticated model-call list endpoint and exercises the Activity list and detail rendering with simulated call data; a real provider call and snapshot inspection remain unverified.
- The browser can create a product through the authenticated mobile API; a live Chromium test creates one and confirms it appears in the product list and overview. The test archives its product afterward.
- A live browser test configured a disposable provider and model, checked connectivity to a simulated OpenAI-compatible server, selected the model, and received a simulated chat reply through the container. Targeted Rust tests verified provider API authentication and credential redaction, and private fallback-file permissions. An isolated container with a temporary filesystem also stored a synthetic key with mode `0600` and redacted it from the API response. A real provider and agent workflow remain unverified.
- A live browser test registered and attached two disposable Git repositories, switched the default through both browser and MCP, verified missing-target rollback and authorization, detached the default and verified fallback, then detached the remaining repository through MCP. The browser also verified product-area overrides, default switching and fallback within an area and back to the product, as well as rejection of an area belonging to another product. Targeted Rust tests covered authorization, invalid paths, duplicate registration across mobile/MCP, and desktop duplicate handling. A separate marker test verified the repository volume survives a container restart. No agent execution was exercised by these tests.
- In a separate disposable container, browser-created tasks reached plan approval, executed the coding agent against a simulated model provider, wrote a file to a temporary Git repository, and exercised passing, failing, and skipped test commands. Failures stopped the workflow; skipped tests required manual review even with automatic approval enabled. Tests verified output truncation and that the command did not inherit the mobile token. Test output is available in an authenticated, path-confined artifact. The command runs as the server user, not in a separate sandbox, and each product can configure or disable its command, with an optional legacy global fallback. The isolated browser/API suite verified product overrides and explicit disablement against a conflicting global command, along with inheritance and invalid settings. Model responses were simulated; real-provider behavior and Git push remain unverified. Model-binding replacement is transactional and a targeted API test verified that a failed replacement preserves the previous binding.
- The mocked desktop browser suite passed 14 tests after updating workspace assertions for the current product-based attachment screen. The latest Rust `--lib` and `--tests` runs each passed 119 tests with one live-provider test ignored; `--tests` also ran three binary targets with zero tests. The Rust runs used reduced debug information. These checks do not establish remote feature parity.
- Dependency versions were updated within existing major ranges; `npm audit --audit-level=low` reported zero vulnerabilities on 2026-11-12. The frontend build, unit suite, and mocked desktop browser suite passed with the updated lockfile. Recheck at release time.
- The remote work HTTP test in `src-tauri/src/services/webhook_service/tests.rs` covers authorization, task creation, a seeded workflow and delivery, but does not validate an actual agent execution.
- Desktop CI commands connect to a private Aruvici Unix socket and include a macOS Preview app launcher. Neither a socket nor that app is supplied by the Docker configuration; container parity will require an explicit integration and appropriate platform behavior.

- A live browser test edited task details and checked persistence and validation. It then changed the task through MCP while the browser editor was open; the stale save returned a conflict, preserved the server change and kept the unsaved browser text available.

- The workspace browser lists a bounded tree, reads UTF-8 text files up to 2 MiB and displays Git status and bounded staged/working-tree diffs. A browser/API test verified contents, authentication, traversal and symlink escape rejection, oversized-file rejection, and exclusion of `node_modules`. The tree excludes hidden paths and common generated directories and uses a depth limit of eight and a cap of 10,000 entries. The editor supports existing UTF-8 files up to 2 MiB, preserves consistent CRLF line endings, and opens mixed-line-ending files read-only. It compares the file contents before saving and serializes browser saves, but cannot prevent a non-cooperating external process from writing between the comparison and filesystem write. Browser tests covered saves, stale-content conflicts, unsaved-change protection and line endings.

- A disposable Git fixture verified the browser diff contains staged and working-tree changes; MCP checks exercised truncation across multibyte boundaries without a panic.

- A live browser test created a product area, capability and feature and verified their nested structure through the authenticated API. It also checked authentication and rejected an area belonging to a different product.

- A live browser test edited a feature and verified its saved description. It also changed a product area through MCP while the browser editor was open: the stale update was rejected and the browser retained the unsaved input.
- Browser, desktop and MCP hierarchy deletion now use a shared transaction that removes descendants, detaches retained delivery work, moves scoped references to the product, and removes scoped team assignments and product-area repository attachments. Browser/MCP tests checked unrelated areas remain intact, confirmation and stale-name rejection, and the resulting delivery and reference records. An isolated database test forced a mid-delete failure and verified rollback. The product-plan reset now uses the same hierarchy reconciliation inside its transaction; preserving delivery also retains the historical agent-work ledger, whose feature identifiers are external to the hierarchy.

- The latest Rust library suite passed 120 tests, with one live-provider test ignored. The isolated simulated-provider workflow suite passed four tests after the deletion changes.

- Product-plan reset is available in the browser with product-name and hierarchy confirmation, and via MCP. Browser tests covered retaining versus deleting delivery work; a database test verified scoped-record reconciliation, preservation of unrelated products and ledger data, stale-name rejection, and rollback after an injected failure. The latest Rust library suite passed 121 tests with one live-provider test ignored.

- A simulated-provider browser test edited and disabled/re-enabled a provider, edited and deleted a model, rejected stale concurrent edits, verified agent-binding removal, and required models to be removed before deleting a provider. Browser provider settings edits leave API keys unchanged; a separate key-replacement control stores a new managed reference. Deleting a provider or rotating a key may leave older credentials in the keychain or fallback configuration and this must be addressed before claiming credential-lifecycle parity.

- Synthetic credential tests verified both the existing managed-key path and browser rotation against authenticated simulated endpoints without exposing keys in API responses. A disposable persistent volume retained a rotated key across a container restart, with fallback configuration mode 0600. Credential file writes use atomic replacement, reject malformed existing JSON, and serialize cooperating processes on Unix with a private lock file; non-Unix builds retain process-local locking. New explicit `ref:` values preserve their marker, and missing explicit or managed references do not fall back to unrelated provider/global keys. Comprehensive keychain and legacy-key cleanup, non-cooperating writers, and older non-managed references already stored without their marker remain open.

- A two-process Rust test verified both writers retain their configuration changes, and an isolated browser/API test resolved an explicit hyphenated reference without sending its name as a key. The latest Rust library suite passed 127 tests with one live-provider test ignored; the isolated suite passed five tests.

- Provider settings include an explicit cleanup of unreferenced fallback-file entries matching the provider’s canonical rotation-reference format. It checks references held by all current providers, including explicit shared `ref:` values. A manually named entry can match that format, so review the confirmation before cleanup. This does not revoke provider keys, clean the system keychain, delete legacy provider-level entries, or automatically clean up after rotation or provider deletion. Non-cooperating writers and older references stored without their `ref:` marker remain limitations.

- After the fallback cleanup change, the Rust library suite passed 128 tests with one live-provider test ignored. The deployed-container remote UI suite passed 12 tests, and five isolated workflow and credential tests passed, including preservation of an actively shared rotation reference.

- The browser Voice screen can select a transcription provider and model and set a locale without changing server-wide speech settings. Empty values use server defaults. Transcript review is enabled by default; recording is bounded to 30 seconds and 600,000 samples, whichever is shorter. An isolated Chromium test used synthetic microphone samples and a simulated OpenAI-compatible transcription service to verify WAV upload, the selected settings, authentication, review and automatic send, capped recording, and track cleanup after audio setup failure. Actual microphone permissions and hardware, device speech playback, local Whisper, and real-provider transcription remain unverified.

- The browser Teams screen creates, edits, enables/disables and deletes teams, and adds or removes roster members with titles and lead status. Updates and deletions compare current team fields to the displayed values. An isolated browser test verified authorization, invalid member rejection, MCP/browser edit conflicts, roster changes, and membership cleanup after deletion. It did not validate team scope assignments, skill links, routing, or actual team-directed workflow execution. The deployed remote browser suite passed 12 tests, and the isolated workflow/voice/team suite passed seven tests.

- Team capacity validation was aligned with the desktop’s positive integer behavior; the final isolated browser test created and edited a team with capacity 103 and rejected zero. The preceding Rust library run passed 128 tests with one live-provider test ignored; it ran before that capacity-only correction.

- The Teams screen can assign and remove product, area, capability and nested feature scopes. Browser assignment verifies the selected scope belongs to its product in the same SQL statement as insertion; stale removals are rejected. Routing favors the most specific enabled team and uses insertion order to break equal-timestamp ties. The Rust suite passed 129 tests with one live-provider test ignored, including specificity, disabled-team fallback and tie-breaking. Eight isolated browser/API tests passed, including a simulated workflow assigned to its product team; the deployed remote suite passed 12 tests. MCP and desktop assignments also reject nonexistent teams or scopes using an atomic existence check. Real-provider team execution remains unverified.

- The browser Skills screen creates, edits, enables/disables and deletes skill definitions and manages links to agents (including proficiency) and teams. Optimistic checks reject stale edits and deletions; duplicate links are rejected and deleting a skill removes its links. Agent prompts now exclude disabled skills and include enabled linked instructions once, with a combined limit of 4,000 characters or the smaller context budget. A simulated provider captured a coding prompt containing the active team skill instruction and excluding a disabled skill. Two Rust tests cover deduplication, Unicode-safe bounds and coding prompt inclusion. The final isolated browser suite passed nine tests; the broader remote suite passed 12 tests before the final explanatory text was added, and the Rust library suite passed 131 tests with one live-provider test ignored. Real-provider behavior remains unverified.

- The Agents screen creates, edits, disables and deletes definitions, including tool lists, skill tags, execution boundaries and employment status. Edits and deletions compare structured snapshots under a database write lock; recorded runs prevent deletion. An isolated workflow used a browser-created agent for coding, and UI/API tests verified conflicts, validation, link cleanup and history protection. Exact repository tool names now grant only that tool; known legacy aliases expand to groups, unknown nonempty entries grant no tools, and an empty list retains the existing defaults including writing. Invalid types or unknown execution boundary fields now stop execution rather than silently replacing limits with defaults; the existing Docker database had nine agents and no incompatible boundaries when checked. The editor and [headless documentation](headless-server.md) explain these behaviors. Prompt template references remain stored metadata and are not used by the prompt builder. The final Rust library suite passed 133 tests with one live-provider test ignored; 11 isolated tests passed, including rejection of an invalid boundary imported through MCP.

- The Routing screen edits and resets per-stage role preferences, fallbacks and coordinator review with stale-write protection. It makes clear that review runs only when an active coordinator is available and that push stages are system operations whose role lists do not select an agent. Empty role lists use stage defaults. A malformed stored role list now returns an error instead of silently using defaults; an invalid policy can be removed with the MCP `aruvi_agents` `delete_workflow_stage_policy` action. In an isolated workflow, a custom coding role selected the intended team agent and completed coding against a simulated provider. Twelve isolated tests passed, including MCP/browser conflicts and reset; the Rust library suite passed 134 tests with one live-provider test ignored. Real-provider execution and an actual Git push remain unverified.

- Coordinator selection now revalidates the previously assigned coordinator against active status, role and current team membership, falling back to another eligible team coordinator when available. Recovery no longer closes a coordinator-review run merely because its original reviewer is unavailable if the assigned team has a replacement; it still closes reviews with no eligible reviewer. A database test covered disabled, removed and cross-team reviewers and recovery; an isolated browser workflow used a simulated provider to verify that subsequent reviews moved to a replacement after the original coordinator was disabled. The final Rust library suite passed 135 tests with one live-provider test ignored, and 13 isolated browser/workflow tests passed. The broader 12-test remote suite passed before the recovery-only adjustment. Coordinator review still bypasses when no active coordinator is available, and real-provider review remains unverified.

- Git delivery now records generated paths and content hashes, checks the original commit and configured branch, and commits only recorded files. An isolated browser workflow pushed a generated file to a local bare remote while preserving unrelated tracked and untracked user changes. Separate tests verified that staged user changes and edits to generated files stop delivery, and that a corrected file or an unavailable remote can be retried from the browser. Destination checks reject dirty files, symlinks and hard links. Seventeen isolated browser/workflow tests and 12 deployed-container browser tests passed; the Rust suite passed 139 tests with one live-provider test ignored. Authenticated network pushes and real-provider execution remain unverified; multi-file synchronization is not atomic and external processes can race filesystem or Git checks. Interrupted manifest or commit recording can require manual reconciliation. Older in-flight workflows without a manifest fail delivery rather than claiming success.
