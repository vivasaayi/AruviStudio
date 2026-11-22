# Web and MCP parity audit

**Current evidence:** The later checkpoints in [model-client-goal.md](model-client-goal.md)
supersede the historical results in this document. They include real Model Host
browser coding and speech generation, a browser repository-analysis flow with MCP
confirmation, an MCP speech generation/transcription round trip, and a two-server
planner concurrency fixture. These are specific scenarios, not proof of full parity.
Physical desktop interaction and audio devices, live third-party providers, and a
real Aruvici connection remain unverified.

The previously generated macOS bundle lacked the microphone purpose string
required for microphone access. The new `src-tauri/Info.plist` supplies it, and
an isolated unsigned debug bundle was checked for the merged value. The hardened
runtime's audio-input entitlement is also configured and was verified in a
separate ad-hoc-signed debug bundle. A physical permission prompt and microphone
capture still need native-window validation.

An isolated startup fixture reconstructed a populated migration-42 database and
verified product hierarchy, story, model and agent binding through the current
native and Linux servers across restart. The browser product API rejected
unauthenticated access and returned the preserved product with a token. This is
synthetic historical data and does not establish all desktop database upgrades.

A disposable real-model coding workflow now also links an enabled and a disabled
skill to the coding agent through MCP. Its recorded coding requests included the
enabled instruction but not the disabled one, and the generated file followed
the enabled instruction before browser review and delivery. This is one model
sample and does not establish team-skill or general instruction-following behavior.

Manual browser recording now discards unfinished audio on navigation or page
hide and closes a late microphone permission result; a disposable native-server
browser fixture verified the navigation case. Hardware capture remains unverified.
Manual recording also resumes a suspended audio context and closes it and the
microphone when navigation interrupts that resume. Simulated browser coverage
passed on disposable native and Linux servers, and the deployed branch recorded
synthetic audio and transcribed it through the real local Model Host. Physical
microphone behavior remains unverified.
Desktop Planner, Direct Chat and Voice Chat now cancel pending microphone
startup when stopped or unmounted; delayed permission and audio-resume cases
were checked in mocked-Tauri browser tests. Continuous browser voice also
reports a context that remains suspended and releases an interrupted capture;
its simulated-device checks passed against native and Linux servers. The updated
branch also passed manual and continuous browser voice with synthetic audio and
the real local Model Host. These results do not establish physical-device or
native-window behavior.
Desktop Planner now allows its own completed transcription to proceed while
blocking competing planner actions. A mocked-Tauri test verified normal
automatic submission and that a late transcription after navigation does not
create a planner turn; the prior lifecycle sent the late turn when automatic
submission was enabled. Actual hardware remains unverified.
Continuous voice now accepts a short detected buffer, including a simulated
200 ms answer, and discards simulated silence. Its RMS threshold can also detect
noise; physical capture and broader noise handling remain unverified.

Stage review now includes an optional connected diagram and focused-stage view;
a disposable native-server browser fixture checked its connections, selection,
focus and mobile containment. Its explanatory topology is shared with desktop
through one repository resource; full native-window comparison remains open.

Desktop IDE buffers now capture their workspace root and file content through
Rust. Guarded saves reject a changed root or content; reviewed patches carry the
captured root through preview, open-file alias checks and application. SQLite
locking coordinates workspace changes between cooperating Studio processes.
Older IPC commands remain compatible and do not require the new guards. External
filesystem writers can still race the content check, and physical native-window
acceptance remains open.

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
Explicit `ARUVI_MCP_ALLOWED_ORIGINS` entries now support exact public origins
when a proxy preserves Host, and require an MCP token even on localhost. A
disposable fixture tested matching and rejected proxy headers; the actual VPS
proxy remains unvalidated. See [the deployment guide](headless-server.md).

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

The desktop IDE can use Copilot without an open file; the browser now supports a
separate general conversation as well. A disposable native-server/browser fixture
verified buffered and streamed simulated-provider replies and switching between
general and file conversations. With a workspace selected, both can request a
patch without an open file. The browser fixture also checks streamed and buffered
proposals, stale review rejection, and application with an open file; the
provider is simulated and multi-file application can partially succeed.

The browser workspace tree can download a saved server file, including binary
files up to 8 MiB; the separate open-copy action includes unsaved editor edits.
MCP exposes `aruvi_repositories` / `read_repository_binary` and
`repositories.files.read_binary`, returning base64 bytes. The browser supplies
the displayed workspace root, and MCP callers may do the same; a SQLite write
lock coordinates that check with workspace updates by cooperating processes.
Unix reads use descriptor-relative no-follow opens and reject hard links and
non-regular files. Non-Unix reads use a canonical-path check with weaker race
protection. External writers may modify file contents while a download is read;
it is not an immutable snapshot. Native disposable server/browser and MCP tests
covered a binary download, both MCP names, authorization, stale root and the
8 MiB boundary; a Rust test covered traversal, links and oversized files. The
Linux candidate browser suite verified the same full-size download, filtering,
authorization and over-limit rejection. Physical desktop acceptance remains open.

The desktop IDE supports ⌘/Ctrl+S. The browser file editor now supports the
same shortcut while the editor is focused, using its existing conflict-checked
save operation. A disposable browser fixture saved a file with the shortcut
and read it back through MCP. The container also offers an opt-in Monaco code
editor, with the plain text editor as a fallback. Browser checks cover JavaScript
language detection, per-file edits and undo, guarded saves and read-only files;
native dialogs and full IDE behavior remain open. The image serves the editor
assets from its own filesystem; separately run native servers require explicit
asset configuration and packaged-native delivery is unverified.
An isolated browser test also sends an unsaved Monaco buffer to a simulated
Copilot provider and confirms through MCP that the saved file is unchanged.
A disposable Studio fixture additionally used the real local
`qwen25-coder-7b-q4` model to discuss an unsaved Monaco buffer, confirmed the
saved file was unchanged, then reviewed and applied a model-proposed patch.
That is one successful scenario, not a general model reliability guarantee.
Browser Copilot can now select an enabled registered model independently of the
Connection model, or explicitly use the Connection model. The choice lasts for
the page session; refresh retrieves current models and retains an unavailable
selection without silently falling back. A disposable browser test exercised
independent routing, disabling and re-enabling a selected model with a simulated
provider. Live-provider behavior and full IDE parity remain open.

### Audit findings before merge

Web work-item review now displays the same scoped findings available to desktop
and MCP, including severity, category, status and blocking state. A disposable
native server checked nonempty task isolation, authorization and MCP agreement;
native and clean-image browser fixtures checked safe rendering of untrusted
text. The worktree database contained no findings at acceptance. The list is
unbounded and delivery sections are not read as a single snapshot; the goal
checkpoint records the remaining validation limits.

Browser work creation now lists child work items as possible parents, matching
the backend and MCP's support for deeper nesting. A disposable native fixture
and ARM64/AMD64 clean-image browser suites created a nested child and verified
its inherited scope and MCP export. Parent labels mark child items but do not
show full ancestry; paged results are not a stable snapshot. The goal checkpoint
records the branch validation and remaining acceptance gaps.

Work now offers the desktop's stuck-run recovery action. Browser, desktop and
MCP record an operator failure and history atomically; the browser and MCP can
reject a changed stage or status. Legacy partially recorded failures can be
reconciled. The browser warns that the override does not stop an executor or
undo external effects. Disposable browser and MCP fixtures cover the action,
and hosted and branch validation are recorded in the goal checkpoint. A stage
and status are not a unique revision, and physical desktop acceptance remains
open.

Shared agent workspace writes now reject symlinked path components and existing
hard-linked destinations on Unix before modifying a file; tool-loop and legacy
agent writes use the same guard. Native Rust regression coverage and hosted
Linux workflow checks passed. The checked-path fallback on non-Unix and races
with external filesystem writers have weaker guarantees; this is not an agent
sandbox. The goal checkpoint records the worktree deployment and test limits.

MCP reviewed-patch reads, previews and application can now bind to the
captured workspace root. First-class preview and reviewed application require
that root; legacy actions remain compatible without it. Hosted clean-image and
disposable branch fixtures verified stale-root and stale-content rejection and
successful application. The lock coordinates Studio processes, while external
filesystem writers can still race. See the goal checkpoint for deployment and
test evidence.

The branch now exposes desktop work-item scope summaries and the complete
repository attachment inventory through MCP. Hosted and disposable local
fixtures verified discovery, filtering and nonempty attachments; the live
branch also checked a scoped summary and compared the two MCP naming styles.
See the goal checkpoint for limits and deployment evidence.

The current source adds MCP direct capability lookup, product-wide capability
listing and product-tree summary actions corresponding to the desktop Rust
queries. Definitions, dispatch and HTTP discovery passed hosted checks, and the
new actions passed a disposable hierarchy check on the branch deployment; see
the current goal checkpoint for details.

#### CI artifact delivery and service dependency

The desktop CI run panel can reveal a verified artifact in Finder. The browser
now downloads artifacts using authenticated bounded chunks and checks the
complete SHA-256 before offering the file. Its standard download requires a
secure browser context (HTTPS or localhost) and buffers at most 256 MiB. A
separate streaming save uses the File System Access API where supported and
commits the selected file only after incremental SHA-256 verification. It
vendors MIT-licensed js-sha256 1.1.0 with its license in `services/vendor`.
MCP exposes `artifact_metadata`
and `artifact_chunk` as well as `artifact_readiness`; the metadata may include
the service's server-side path. The Aruvici checkout's private socket protocol provides
`artifact` metadata and bounded `artifact_chunk` reads by numeric ID; the newer
protocol adds a larger base64 chunk operation. Studio's CI relay permits these
methods. The
receiver must check offsets, size and SHA-256 across the completed download;
readiness alone does not protect against a file changing during transfer.
An isolated native Studio and Aruvici protocol-branch fixture reconstructed a
76,800-byte binary through MCP in multiple chunks, checked its digest and size,
and rejected a negative offset. Chrome downloaded the same artifact from the
native Studio server and rejected a modified file; unauthenticated chunk access
was rejected. The streaming path passed the same multichunk check using a
simulated save picker and aborted the write after a digest mismatch. A synthetic
268,435,457-byte browser test also completed in 4,097 writes, with both chunk
responses and the save picker simulated. An actual picker and a large artifact
from a deployed CI service remain untested. A real 268,435,457-byte file in a
disposable native debug-service fixture did not finish within five minutes;
after buffering and socket-mode fixes and an optional 512 KiB base64 protocol,
the same-sized file streamed through the native debug service in 513 writes and
passed its digest check. The picker was simulated and staging remains untested.

The `promotion_readiness` operation used by Studio exists in the separate
`codex/studio-ci-protocol` Aruvici branch but was not found in the
local Aruvici `main` checkout when checked. It is proposed in draft
[#1](https://github.com/vivasaayi/aruvici/pull/1), and its eleven CI API tests
passed locally, including cross-run artifact rejection. A simulated CI service or the presence of a socket does not
establish compatibility with the actually deployed Aruvici version. Reconcile
and validate the service protocol before claiming staging CI parity.

#### Product references

The browser can load references on demand for a product, area, capability or
feature. The authenticated endpoint validates the scope against the selected
product and the node kind, and pages 50 items at a time. The browser renders
text safely and links only credential-free HTTP(S) URLs. A disposable browser
and MCP fixture covered product and capability scopes, a wrong product and
wrong kind, unauthorized access, unsafe URLs and 52-item pagination. Pagination
uses an offset and does not provide a stable snapshot if references change while
paging. The browser can also create and delete references at those scopes;
creation verifies product ownership in the insert, and deletion compares the
full displayed reference and scope. An isolated browser/MCP fixture verified
create, delete, unauthorized and cross-product requests and stale deletion.

#### Product details

Browser Products now edits the desktop product fields: name, description,
vision, goals, tags, lifecycle, health, owner, investment, roadmap and evidence.
The browser sends its loaded product snapshot; the server compares and writes
under an immediate SQLite transaction, rejecting stale edits and preserving the
browser draft. A disposable browser/MCP test covers a concurrent edit, refresh
and retry, all fields, authentication, identity and validation. Existing desktop
and generic MCP product updates do not use the browser snapshot guard and remain
last-write-wins for the same field. Their partial updates now change only supplied
fields in a single SQL statement, so concurrent changes to different fields are
not overwritten by an earlier read. The
browser uses comma-separated goals and tags, matching the desktop form. Other
product actions still need comparison.

#### Product hierarchy details

The browser now also supports the desktop product-area purpose field during
creation and editing, with expected-value conflict checks. The desktop
capability/feature editor exposes acceptance criteria and technical
notes in addition to name and description. The browser tree editor now exposes
those two fields, with product scoping and expected-value checks; older clients
can still update only name and description without clearing the new fields. A
disposable browser/MCP test saved both fields, rejected a stale MCP edit and a
cross-product update, and rejected an update missing its expected value. Current canonical hierarchy rules allow only a capability under a product area
and only a feature under a capability, so the existing conversion action cannot
change a valid node between those kinds without violating its parent rule. The
shared conversion operation now runs under an immediate transaction, but its
child-reparent path is unreachable for a valid canonical tree and remains
unverified; no browser conversion control is offered. Other product-management
actions still need comparison.

#### Portfolio strategy and dependencies

The desktop Portfolio includes a strategy hierarchy, product-to-strategy links
and product dependencies. Browser Products now provides strategy node creation,
editing and deletion, product linking and unlinking, and dependency creation and
deletion. Optional capability references are selected by product and hierarchy name;
the dependency list can still display IDs for those references. The `aruvi_catalog` MCP namespace now exposes list, create, update and
delete actions for strategy nodes; list/link/unlink for product-strategy links;
and list/create/delete for product dependencies. These use the existing desktop
repository and its hierarchy and dependency validation. Primary-link replacement
is now transactional so a rejected replacement cannot remove the old primary.
A disposable database test exercised the MCP dispatch lifecycle and rejected
invalid hierarchy, self-dependency and an invalid primary replacement. Dependency
writes reject capabilities that belong to the wrong side's product. Strategy edits
reject cycles and incompatible existing children. Browser strategy edits require the displayed node snapshot and reject stale
changes. Strategy creation and updates validate under SQLite write transactions;
legacy desktop/MCP updates that omit a snapshot remain last-write-wins, and
browser strategy deletion compares the confirmed subtree and its links under a write lock. Browser link and unlink operations compare the product's complete link set under the same lock, including changes to the primary link. Legacy desktop/MCP link actions can still omit the optional snapshot.
Native-window acceptance and broader comparison remain open. This is not complete portfolio parity.

#### Settings and outbound contact comparison — 2026-11-14

This is a source comparison of the desktop Settings page, browser Routing and
Voice sections, and the MCP handlers; it is not native-window or live Twilio
acceptance. The generic MCP setting actions can read and write arbitrary keys,
but that alone does not provide the desktop form's validation, conflict handling,
or evidence that a setting affects runtime behavior.

| Desktop action | Browser | MCP | Follow-up |
| --- | --- | --- | --- |
| Hide examples; seed/repair catalog | Routing controls and authenticated catalog APIs | `aruvi_settings` and catalog actions | Existing isolated browser tests cover catalog settings; confirm native comparison. |
| Set planner provider and model | Routing defaults form; environment values can override | `aruvi_settings` | Existing browser form uses a revision check; desktop/MCP setting writes are last-write-wins. |
| Set outbound channel preference, ambiguity escalation and call quiet hours | Routing contact policy form updates the four server settings atomically | `aruvi_settings` can update the underlying keys | The isolated browser test covers invalid times, authentication and rollback after a concurrent MCP edit; live outbound routing and native comparison remain open. Quiet hours use the server's local timezone. |
| Send WhatsApp, start a voice call or route planner contact | Routing offers planner contact and direct WhatsApp/voice actions; direct actions explicitly bypass planner policy | `aruvi_channels` exposes all three operations | Browser actions use simulated responses in tests; live outbound delivery remains unverified. |
| Configure Twilio and default contact details | Routing edits contact defaults and Twilio settings; the auth token is write-only in the browser form, with conflict checks and environment override indicators | Generic `aruvi_settings` operations | The existing settings database stores the token and generic desktop/MCP settings can read it. Live Twilio remains unverified. |
| Set transcription provider/model/locale | Voice server-default controls; browser can also choose per-session values | `aruvi_settings` and `aruvi_speech` | An explicit nonblank request wins. Browser API, desktop and MCP then use `ARUVI_SPEECH_PROVIDER_ID`, `ARUVI_SPEECH_MODEL_NAME` and `ARUVI_SPEECH_LOCALE` when present, followed by saved settings. A present but blank environment value suppresses the corresponding saved default. If the environment provider differs from the saved provider and no environment model is provided, the saved model is not carried across. An explicit different provider likewise does not inherit the configured model. Desktop and MCP now honor environment overrides that previously applied only to the browser. |
| Set native voice, microphone, automatic speech and transcript review | Browser Voice has device-specific recording, review and speech controls | `aruvi_speech` supports speech operations | Native OS voice and microphone preferences are not the same as browser device settings; physical capture and playback remain unverified. |
| Configure mobile/MCP bind address, port and tokens; view bridge status | Activity shows the browser-visible MCP endpoint and server configuration summary, edits saved server bind host/port, and rotates or clears saved bridge tokens | `aruvi_settings` exposes status and generic setting operations | Authenticated browser forms reject stale edits. Token values are write-only in these forms; generic settings operations still expose stored values. Changes to saved tokens apply immediately unless overridden by environment variables. Configured server bind may differ from the published browser address, and bind changes require restart. |
| Inspect database health and override its path | Routing database panel | `aruvi_settings` database actions | Both server paths and restart/environment precedence need to remain explicit. |
| Set Docker host and maximum workflow retries | No corresponding controls found | Generic `aruvi_settings` can store values | Repository search found these keys only in the desktop settings form and loader, with no backend readers. Do not present them as working execution controls until behavior is implemented and verified. |

The browser Twilio form does not return stored or environment-provided tokens.
It uses a process-local keyed token revision to reject stale saves; a restart or
another process requires refresh. It refuses to display legacy malformed or
credential-bearing webhook URLs. Existing plaintext database storage and generic
settings access remain unchanged, and clearing a saved token does not revoke it
or change an environment override. Browser fixtures cover token replacement,
clearing, stale edits and unsafe legacy URLs, not live Twilio delivery.

Browser contact defaults use the same desktop settings and reject stale writes
after an MCP edit; applying them to a contact is explicit. Refreshing defaults
replaces unsaved edits in the defaults form, but does not replace an unsent
contact draft. The isolated workflow test covers authentication and rollback.

The browser planner-contact form calls an authenticated server endpoint, requires
explicit confirmation and displays the selected channel, status and reason. It
can allow calls during quiet hours explicitly; routing may otherwise fall back
between configured channels. Provider failures report uncertain delivery without
returning provider details. Isolated browser coverage intercepts the outbound
request, verifies cancellation and blocked-result display, and checks authentication,
invalid inputs and unconfigured service handling. A separate browser-to-server test confirmed the real policy blocks a voice
fallback during quiet hours with outbound HTTPS disabled. It does not test an
actual Twilio send or the provider-error path. Direct browser sends and calls
have separate simulated coverage and explicitly bypass planner policy.

The four outbound policy keys are consumed by `channel_contact_policy.rs`. The
browser form writes those keys and validates complete time ranges; it does not
exercise an actual outbound delivery. This is distinct from the two execution
fields whose stored values currently have no identified consumer. Existing MCP
access is not evidence of complete browser parity or live channel
delivery. The rest of the desktop settings and the full IDE/CI comparison remain
open.

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
| Portfolio, product overview, products | Create products, product areas, capabilities and features; edit and delete hierarchy nodes; list, summary and on-demand hierarchy tree; download Product Book Docs HTML, Book HTML, EPUB and PDF | `aruvi_catalog`, `aruvi_work_items` | Partial: browser, MCP and desktop PDF export share the guarded renderer; native packaging, image, reader and deployment limitations are tracked in [Product Book migration](product-book-migration.md). |
| Work items and delivery | Create work with problem, outcome, acceptance criteria, constraints, type, priority, complexity and hierarchy placement; edit the desktop form's task details and status, approve/start, configure a test command, inspect test results, delivery and gates | `aruvi_work_items`, `aruvi_workflows`, `aruvi_checkpoints` | Partial: navigation and workflow actions need comparison; changing status does not run a workflow, and test commands execute inside the server container. |
| Backlog decisions and ordering | Browser supports individual and multi-select approve and reject actions, status filtering, and stale and active-execution checks. It supports drag and move buttons for the unfiltered product backlog. | `aruvi_checkpoints` supports individual and batch decisions; `aruvi_work_items` exposes a guarded move and the legacy reorder action. | The browser move validates the displayed contiguous interval and commits atomically, including when further pages exist. Filtered reordering is disabled, and a move spans at most 1,000 loaded items. The legacy reorder action is atomic but does not detect stale or cross-product inputs. Approving follows the configured automatic-start policy. |
| Work item external CLI | Browser can launch an explicitly enabled server CLI and inspect recent runs and events | `aruvi_work_items` launch, list runs and list events | Partial: a disposable native server and headless browser exercised a fake CLI, MCP history, environment isolation, output truncation and active-run guards. The container image does not install the CLIs; actual provider installation, authentication and live execution remain unverified. The CLI runs as the server user and is not sandboxed. |
| Planner | Sessions, turns, draft editing, repository analysis, confirmation and clear | `aruvi_planner` | Partial: native interaction, physical voice and other desktop behaviors need comparison. |
| Repositories and IDE | Create, register and attach workspaces for products or areas; browse, filter and edit text files in separate in-memory tabs, inspect Git status and diff, ask Copilot about saved or unsaved content, configure generation and file-context settings, review multi-file patch proposals, and change defaults or detach for products and product areas | `aruvi_repositories`, `aruvi_models` | Partial: desktop, browser and MCP workspace creation reject existing paths and preserve prior attachments; browser file Copilot supports streaming, while MCP remains buffered and patch proposals require saved content. Isolated browser checks cover separate unsaved tabs and protection of patch targets; tabs do not survive reload. A streamed proposal passed in the browser with a real local model and a two-file proposal passed through MCP. Native dialogs and other IDE behavior require comparison and implementation. |
| Agents and teams | Manage agent definitions, team profiles, rosters, product/area/capability scope assignments, skill definitions and links, prompt templates, and workflow-stage routing; inspect and replace individual agent model bindings from Models | `aruvi_agents`, `aruvi_agent_work` | Template references using the explicit `template:` prefix are applied by shared execution; legacy references remain inactive. The desktop editor accepts references but template management is in the browser and MCP. A desktop bulk-binding operation was not found in the current agent commands. |
| Models and providers | Add, edit (including model context, tags and notes; OpenAI-compatible, reasoning profile and Anthropic protocol), enable/disable and remove providers and models; set the transcription language field format; install checksum-pinned Whisper presets or register an existing server-side file, test providers, detach API credentials and select a browser model | `aruvi_models` | Partial: The desktop model and provider form fields have browser equivalents, and browser model presets fill the same model details while preserving the explicitly selected provider. Browser quick starts also fill Aruvi Model Host, DeepSeek and LM Studio provider drafts, with confirmation before replacing a draft and clearing its entered key; the host endpoint remains operator-supplied. Their source-HTML browser test passed, while packaged-image validation is pending. Other provider-management behavior still needs comparison; custom files may be unverified. The desktop transcription-format control was checked with a mocked UI, not a native window. Detaching disables fallback keys but does not revoke provider credentials. MCP and browser local paths refer to the server filesystem. |

| Chat and voice | Chat with optional streaming and cancellation; voice with browser-specific transcription provider, model and locale, transcript review and device speech controls. A continuous voice-chat session listens for pauses, submits recognized turns without review and waits for device speech before listening again. | `aruvi_models`, `aruvi_speech` | Partial: a disposable Studio server and simulated provider exercised a full continuous turn, and synthetic Piper audio reached real Model Host Whisper and a local coding model through the deployed browser session. Noise handling, physical microphone and device speech playback, desktop comparison and broader provider/model validation remain open. |
| Model calls | Calls grouped by session, filters over loaded calls, older-page loading, selected details and on-demand snapshots in Activity | `aruvi_models` list, detail and snapshot read | Browser, desktop and MCP support cursor pagination. Browser and desktop pause auto-refresh while viewing older calls and filter only loaded calls. Authenticated browser API and MCP pages agreed on existing 7B-model records; a live headless browser loaded older records and refreshed. Desktop browser and Tauri mock-runtime checks passed; native-window acceptance remains outstanding. |
| CI | Inspect service, targets, revisions, runs and live logs; manage target assignments; queue, cancel or retry builds; request artifact readiness and record feedback | `aruvi_ci` | Partial: artifact access and host service integration require work and validation. |
| Settings | Connection token and browser model fields; server transcription defaults on Voice; external planner model and product defaults and workflow auto-start and approval settings on Routing | `aruvi_settings` | Partial: compare remaining settings, database and bridge controls. Environment variables can override saved transcription and external-channel defaults; live external-channel use remains unverified. |

The browser Work list loads 100 items at a time and offers a load-more action
when more are available. A failed later-page request leaves the current items
in place for retry; refreshing starts over, and a late response from a previous
product is ignored. An isolated browser test used 101 actual work items,
including a failed page and a delayed response after a product switch. Paging
uses offsets rather than a snapshot: concurrent inserts, deletions or reorders
can move items between pages, so refresh when the list changes.

Work item workspace and branch overrides are now available in the browser and
`aruvi_work_items` MCP action as well as desktop, through a shared backend guard.
An isolated workflow wrote to a selected override instead of the product's
default workspace; the model was simulated. The operation selects a registered
server workspace without checking out a branch, and rejects changes during a
running or paused workflow. Browser edits and optional MCP baselines reject
stale assignments; desktop and MCP calls without a baseline remain last-write-wins.
A separate real 7B Model Host workflow also wrote, independently checked and
delivered a generated file from an override workspace while leaving the product
default untouched, after earlier truncated-response failures. See the model
client goal checkpoint for the model budget and scope of that evidence.

The browser can now delete a work item with confirmation, like desktop and MCP.
The shared backend rejects deletion during an active workflow for that item or a
direct child; the browser also rejects stale details. Related records follow the
database's deletion rules, direct children are detached, and files are retained.

The external CLI action was identified in a later source comparison of the
desktop Work tab. Remote launches now require an operator opt-in and dedicated
CLI home and receive a cleared, minimal environment. A disposable native
server test used a fake CLI through the browser and MCP, confirmed server
secrets were not inherited, checked bounded output, workspace override and
active-run guards, and verified missing-executable failure and history after a
restart. Desktop continues to inherit its own process environment. A read-only
check of the previous running worktree app found `codex`, `claude`,
`cursor-agent` and `copilot` unavailable. Real CLI provisioning, provider
authentication, other failure and restart scenarios, and container deployment remain
open; see the [server guide](headless-server.md) for the execution limits.

The browser can now edit and disable an existing local Whisper provider. Its
file path is treated as a server path instead of an HTTP URL; a newly selected
path must resolve to an existing regular file, while an unchanged missing path
can still be disabled. The update retains the existing stale-write check and
checks that the provider type has not changed. The isolated workflow fixture
exercises disable/re-enable, a valid replacement file, a missing file and a
stale edit. This does not validate model contents or physical audio.

The Work view now opens workflow text artifacts as well as test results. Its
authenticated endpoint checks that the artifact belongs to the selected work
item, resolves it under the server artifact directory, and limits the read to
2 MiB of UTF-8 text. An isolated workflow compared a generated coding artifact
through browser and MCP; a Rust test covered authentication, incorrect work
items, outside paths, a symlink escape, invalid text and oversized files. The
filesystem checks do not eliminate races with independent filesystem writers.
Desktop, browser and MCP text reads now share artifact-directory confinement.
The full-text operation has a 2 MiB limit; a separate 64 KiB paged reader lets
all three clients inspect larger UTF-8 artifacts. Subsequent pages require the
version returned with the first page and reject detected changes. The browser's
test-result endpoint retains its 256 KiB limit and work-item/type checks. On Unix, descriptor-relative opens
reject a symlink inserted into a path after canonicalization; other platforms
still use a canonicalized path followed by an ordinary open. Historical records
whose files live outside the current artifact directory, and non-UTF-8
artifacts, can no longer be read through desktop or MCP; move or recover those
files separately. The page version is based on file metadata, not a content
hash; it detects ordinary changes but is not an immutable snapshot against a
malicious or non-cooperating writer. Hard links, changes to file contents while reading, and
concurrent changes to the artifact directory itself are not eliminated by
these checks. This is not a claim of complete artifact-format parity.

### Workspace update

Browser Workspaces and `aruvi_repositories.update_repository` can update the
name, server path, remote URL and default branch using an expected repository
snapshot. The existing desktop command uses the same backend without a snapshot.
A changed browser path must identify an accessible Git working repository, as
required by browser registration; desktop and MCP retain their prior path rules.
The update checks duplicate paths and commits the workspace and inherited
work-item branch updates in one SQLite transaction. Changes to path, branch or
remote are rejected when an associated workflow is running or paused, or a CLI
run is active; the attachment check is conservative. Registration and local
workspace creation use the same database write lock for duplicate checks.
Cooperating processes using the same SQLite database participate; direct writes,
older binaries and external filesystem changes do not. Open browser buffers
and reviewed patches carry their opened root and reject a changed path; MCP
writes can opt into the same guard. Older callers without it and desktop buffers
are not protected against path redirection. The browser preserves an editing
draft on a conflict and asks before changing a workspace with open files.

Native browser/MCP fixtures covered path conflicts, a registration race,
active and paused workflows, an active CLI run, a forced branch-update rollback,
inherited versus custom branches, stale browser drafts and stale open-file
saves. A derived Linux candidate passed the 45-test browser suite (one skipped)
and 53 workflow tests; its deployed branch browser edit was verified through MCP
and rejected a stale snapshot. Physical desktop acceptance remains open.

### Catalog bulk import

Browser Products and MCP can submit up to 1 MiB of inline UTF-8 JSON or CSV to
the existing asynchronous importer, while desktop and MCP retain their
server-file path option. The browser shows recent job status and errors. Import
updates records with matching IDs and commits in batches; an error can leave
prior batches applied. Native and Linux browser/MCP fixtures passed, and a browser-created branch
fixture was verified and archived through MCP. Physical desktop comparison
remains open.

### Settings action inventory

| Desktop action | Browser and MCP assessment |
| --- | --- |
| Hide seeded products and repair example catalog | Browser Routing exposes the preference and seed/repair action; MCP `aruvi_settings` reads/sets the same preference and `aruvi_catalog` seeds examples. Browser updates reject stale values. |
| Docker daemon and workflow retry settings | Desktop saves `docker_host` and `max_workflow_retries`, and MCP generic setting operations can read and update them. A source search found no backend reads of either key. New workflow runs instead receive a hard-coded maximum of three, and the test stage invokes the configured command in the server environment; `docker_service.rs` is a stub. The desktop descriptions now explicitly say these settings are not used or enforced and identify the server user as the command executor. Browser equivalents should not imply otherwise. Retry behavior and a deliberate execution-isolation design remain open. |
| Product workflow test command | Browser Work edits the product policy and indicates that commands execute as the server user. It submits the loaded effective policy as an expected snapshot; the backend compares and writes under a SQLite write transaction, including changes to an inherited default. A conflict preserves the browser draft and offers an explicit reload. Older API clients without a snapshot remain last-write-wins; generic MCP settings can still update the stored values. This check does not sandbox command execution. |
| Planner defaults, speech defaults and workflow approval switches | Browser Routing and Voice expose relevant server settings; MCP generic settings operations also exist. Device-local speech controls are browser-specific and are not proof of physical audio parity. |
| MCP/mobile bind, token and bridge status | Browser Activity can edit the saved bind host/port and replace or clear saved mobile/MCP tokens, with stale-write protection. Disposable native and Docker browser fixtures covered conflicts, rotation, restart, clearing and environment recovery. Token values are not returned by these browser-specific endpoints; generic desktop/MCP setting operations retain their existing access. Environment variables may override stored values, bind changes require restart, and clearing an active mobile token removes browser access. |
| Twilio and planner contact | Browser Routing provides configuration, contact defaults, planner routing and direct outbound actions; MCP has channel operations. Simulated and policy-blocked cases are covered; live Twilio delivery and provider-error handling remain open. |
| Database path, override and migration health | Browser Routing and MCP expose path, override and migration health through the same Rust service as desktop. The mobile token authorizes these operations. Paths refer to the Studio host/container filesystem, changes take effect on restart, and `ARUVI_DB_PATH` takes precedence. Browser writes compare the loaded override and preserve drafts on conflict; MCP can opt into the same check. Existing desktop and older API/MCP clients remain last-write-wins. Cooperating Unix processes serialize writes and replace the override file atomically; older binaries and direct file writes do not participate. Saving does not validate the new database. A disposable native server and persistent headless browser verified cross-client conflict, reload, override and clear across restarts, isolated persisted products, environment precedence and unauthorized rejection. A reconstructed migration-52 database upgraded through server startup while retaining a product and legacy channel binding. Delayed browser refresh and save responses preserve an unsaved path draft. This does not prove upgrades from all historical databases. |

Workflow test commands run in a separate process group on Unix. After the main
command exits, times out, or its execution future is cancelled, Studio signals
the group to stop remaining members. On macOS and Linux it waits to reap the
leader until after the signal, avoiding group-ID reuse in the normal exit path.
A command can start a new session or process group, so this cleanup is not a
sandbox and cannot guarantee that every descendant is stopped.

Workflow starts now reserve the selected team slot and work item in the same
SQLite insert that creates and assigns the run. A full or disabled team, an
active external CLI run, or an already active workflow prevents the insert;
concurrent starts cannot both reserve the final slot. A repository fixture
checks concurrent contention, absence of a rejected run, slot reuse and
duplicate work-item rejection. Assignment of an existing active run also
checks capacity atomically, including concurrent assignments; retaining the
same team and releasing a slot remain possible. The fixture verifies both
contention and slot reuse. This does not implement automatic workflow-stage
retries or command isolation.

Main workflow transitions and failure recording commit lifecycle, stage and
history together and reject stale sources. Manual advancement records the Git
stage before delivery and completes only after a successful push. Cooperating
processes on Unix with a shared file-backed database serialize Git delivery per
run; in-memory and non-Unix databases have only in-process coordination. A stale
failure cannot mark a later stage failed. The native tests exercised concurrent
advances against a local Git remote and the cross-process delivery lock; the
deployed candidate also passed five two-container decision races and a forced
two-container MCP Git-delivery race against a local bare remote. The prior image
failed the new delivery test by bypassing its held lock. Database
atomicity does not include Git or other external effects: a crash after a push
can lead to retrying the same recorded commit. Agent writes and coordinator
pending-stage updates remain outside that transaction. See the current checkpoint
in [model-client-goal.md](model-client-goal.md) for validation and limitations.

### Planner action inventory

This comparison is based on the desktop planner, the remote HTML/API and MCP
handlers. An available handler is not evidence of a successful real-model workflow.

| Desktop action | Browser and MCP status |
| --- | --- |
| Create or update a planner session and choose provider/model | Browser connection settings create/update a session; `aruvi_planner` provides both operations. |
| Submit a planner turn with selected draft node and product | Browser presents a product selector and sends its selected node and product; MCP accepts both. Simulated browser tests cover scope and node selection. A disposable browser fixture sent a product-scoped turn to the real 7B Model Host, inspected the staged draft and review packet, verified the catalog remained unchanged, and confirmed through MCP. The model required a format retry; A subsequent disposable real-model browser fixture selected the drafted area, added a capability beneath it, and verified the confirmed hierarchy through MCP; an earlier attempt did not produce the expected visible capability. Other models and general model reliability remain unverified. |
| Select and inspect the staged draft tree | Browser renders/selects nodes, shows their path, summary, source, confidence and evidence, and requests suggested prompts, name-matched recent plan actions and structural validation from shared Rust services. MCP exposes these services; desktop uses the shared suggestions and action-matching commands, and shared validation with a local validation fallback. The clients now send ancestor names so the matcher can exclude actions with explicitly conflicting product or area scope. Missing scope remains ambiguous, names or ancestors changed since the plan was created can suppress matches, and same-named nodes in the same scope can still be conflated. Recent actions are limited to the current page session. |
| Rename a draft node, add a child or delete a draft node | Browser and `aruvi_planner` expose these operations. An isolated Docker browser test exercised the authenticated API and verified the actual product remained unchanged. A two-server Linux fixture covers cooperating processes sharing a database; see the planner coordination checkpoint for platform and filesystem limitations. |
| Analyze a registered repository into a design draft | Browser and `aruvi_planner` can analyze a registered workspace into the selected product draft. Simulated tests cover authentication, staging and scan-failure recovery. A disposable browser test also analyzed a synthetic workspace with the real 7B Model Host, then confirmed the staged design through MCP; broader repositories and models remain unverified. |
| Confirm or clear a staged design | Browser and MCP expose both. A disposable two-server fixture covered concurrent draft edits, confirmation without duplicate application, and a stale server confirming after another server cleared the draft. These checks require cooperating Unix processes sharing a database and do not cover every failure or client race. |
| Export and reveal a design-review packet | Browser can download and MCP can retrieve a Rust-generated HTML packet from a staged session. It contains the tree, validation, current context, pending actions and review sections. Desktop uses the same backend renderer for active sessions and retains native file/reveal; its prior renderer remains as a no-session fallback. The offline SVG and pending-action summaries differ from the older Mermaid packet, and native UI verification remains open. |
| Voice transcription, voice planner commands and native speech | Browser has transcription, device speech controls and a Planner destination; MCP exposes speech and voice turns. Simulated browser tests cover view switching, draft selection, and global and targeted expand/collapse while preserving the selected node. Real Model Host speech passed through MCP and browser recording with a synthetic microphone. Native service tests cover mixed-case node types and a single speech-added final period for named commands; physical microphone, audible playback and native-window behavior remain unverified. |
| WhatsApp or phone contact routing | Browser Routing supports planner contact with channel preference and an explicit quiet-hours override; MCP has channel operations. Simulated and policy-blocked cases are covered; live delivery and provider-error handling remain unverified. |

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
user-data socket was absent. A later disposable native Studio and Aruvici check
passed MCP health, empty target/run listings and browser API health; its Docker
Desktop socket-mount probe returned `ENOTSUP`. This does not validate a real
target, build, or a working Docker-to-host CI transport.

An additional disposable native protocol audit used Studio `f90987b` and Aruvici
`5851f2594bbbd83193e259a566004088ac28c52e` with one planning-only fixture target.
The real target response omitted the repository path that Studio requires for
safe product/repository binding, so binding failed closed. The service also
returned unsupported-method errors for Studio's retry and artifact-readiness
requests. `bash scripts/test native-ci-service STUDIO_BINARY ARUVICI_BINARY
--audit-protocol` reproduces these read-only protocol checks. A separate
loopback TCP reachability probe from the Docker image succeeded, but network
reachability alone does not fix the protocol or shared-repository identity
requirements. A complete bridge must authenticate requests, reconcile trusted
shared paths, and resolve the API mismatch before enabling build dispatch.
An isolated candidate branch in Aruvici now implements the missing metadata,
retry and integrity operations and has passed the native protocol fixture; it
is not deployed. A disposable minimal Rust/OCI build has also passed through
native Studio MCP, with result reads via the browser API. Real product builds,
native browser interaction, and Docker-to-host transport remain open.
An opt-in, token-authenticated host-loopback relay now lets a Docker Studio
fixture read status from a real disposable Aruvici service on Mac Docker Desktop.
The relay is plaintext on the trusted local host connection and is not deployed;
an explicit trusted root mapping and read-only worktree mount now support a
container-initiated disposable CI build. The operator must ensure the two roots
represent the same files; live-service deployment and broader product builds
remain open.

A headless Chrome browser also inspected the completed disposable real-service
build through the Docker Studio relay, including its stages, Rust-test log and
OCI artifact verification. A follow-up fixture selected and queued a different
commit in the browser and verified the real build passed. This exposed and fixed
a selector that hid valid mapped host/container targets; the backend validates
target identity when assigning or queueing. A later fixture used both MCP and
a real browser to cancel an active Rust test and retry the same revision through
the authenticated relay. The cancelled run published no package, and the retry
passed. This uses a disposable service and synthetic repository rather than a
deployed product CI.

The current branch's Linux container and a newly built native Studio server
repeated the real-service relay fixture with Aruvici protocol candidate
`33e5a08`. Container-queued and browser-queued disposable Rust/OCI builds,
browser inspection and artifact verification passed. The fixture now checks
the required scanner, formatter, linter and Buildx before starting. This is not
the deployed Aruvici service, a production product build or staging acceptance.

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

- The workspace browser lists a bounded tree, reads UTF-8 text files up to 2 MiB and displays Git status and bounded staged/working-tree diffs. A browser/API test verified contents, authentication, traversal and symlink escape rejection, oversized-file rejection, and exclusion of `node_modules`. The tree excludes hidden paths and common generated directories and uses a depth limit of eight and a cap of 10,000 entries. The editor supports existing UTF-8 files up to 2 MiB, preserves consistent CRLF line endings, and opens mixed-line-ending files read-only. It compares the file contents before saving and serializes browser saves, but cannot prevent a non-cooperating external process from writing between the comparison and filesystem write. Browser tests covered saves, stale-content conflicts, unsaved-change protection and line endings. The open UTF-8 buffer can be downloaded with unsaved edits, retaining CRLF and the original mixed endings for read-only files. This is a browser copy, which can differ from later disk changes; it does not reveal the server folder. Saved binary-file downloads were added later as described above.

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

- The Agents screen creates, edits, disables and deletes definitions, including tool lists, skill tags, execution boundaries and employment status. Edits and deletions compare structured snapshots under a database write lock; recorded runs prevent deletion. An isolated workflow used a browser-created agent for coding, and UI/API tests verified conflicts, validation, link cleanup and history protection. Exact repository tool names now grant only that tool; known legacy aliases expand to groups, unknown nonempty entries grant no tools, and an empty list retains the existing defaults including writing. Invalid types or unknown execution boundary fields now stop execution rather than silently replacing limits with defaults; the existing Docker database had nine agents and no incompatible boundaries when checked. The editor and [headless documentation](headless-server.md) explain these behaviors. At that checkpoint prompt template references were stored metadata and were not used by the prompt builder; the later template checkpoint above supersedes that behavior. The final Rust library suite passed 133 tests with one live-provider test ignored; 11 isolated tests passed, including rejection of an invalid boundary imported through MCP.

- The Routing screen edits and resets per-stage role preferences, fallbacks and coordinator review with stale-write protection. It makes clear that review runs only when an active coordinator is available and that push stages are system operations whose role lists do not select an agent. Empty role lists use stage defaults. A malformed stored role list now returns an error instead of silently using defaults; an invalid policy can be removed with the MCP `aruvi_agents` `delete_workflow_stage_policy` action. In an isolated workflow, a custom coding role selected the intended team agent and completed coding against a simulated provider. Twelve isolated tests passed, including MCP/browser conflicts and reset; the Rust library suite passed 134 tests with one live-provider test ignored. Real-provider execution and an actual Git push remain unverified.

- Coordinator selection now revalidates the previously assigned coordinator against active status, role and current team membership, falling back to another eligible team coordinator when available. Recovery no longer closes a coordinator-review run merely because its original reviewer is unavailable if the assigned team has a replacement; it still closes reviews with no eligible reviewer. A database test covered disabled, removed and cross-team reviewers and recovery; an isolated browser workflow used a simulated provider to verify that subsequent reviews moved to a replacement after the original coordinator was disabled. The final Rust library suite passed 135 tests with one live-provider test ignored, and 13 isolated browser/workflow tests passed. The broader 12-test remote suite passed before the recovery-only adjustment. Coordinator review still bypasses when no active coordinator is available, and real-provider review remains unverified.

- Git delivery now records generated paths and content hashes, checks the original commit and configured branch, and commits only recorded files. An isolated browser workflow pushed a generated file to a local bare remote while preserving unrelated tracked and untracked user changes. Separate tests verified that staged user changes and edits to generated files stop delivery, and that a corrected file or an unavailable remote can be retried from the browser. Destination checks reject dirty files, symlinks and hard links. Seventeen isolated browser/workflow tests and 12 deployed-container browser tests passed; the Rust suite passed 139 tests with one live-provider test ignored. Authenticated network pushes and real-provider execution remain unverified; multi-file synchronization is not atomic and external processes can race filesystem or Git checks. Interrupted manifest or commit recording can require manual reconciliation. Older in-flight workflows without a manifest fail delivery rather than claiming success.
