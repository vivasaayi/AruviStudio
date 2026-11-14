# Shared model client and Aruvi Model Host integration

This is the active implementation checkpoint. Completed items require the
evidence named below; simulated providers do not prove real-provider behavior.

## Objective

Create a reusable, independently consumable Rust library for model access, and
use it in AruviStudio while preserving existing desktop, web, MCP and workflow
behavior. Route local model inference to Aruvi Model Host. Do not move third-party
provider hosting into the model-host service as part of this work.

## Checkpoints

### Optional OpenAI-compatible stream usage

The shared client now offers `with_stream_usage(true)` to request a terminal
usage event on compatible streaming endpoints. It defaults off so existing
Model Host, LM Studio and other compatible configurations retain their request
shape; providers may reject the option or omit usage. Studio exposes the option
per OpenAI-compatible provider in desktop and browser model settings and MCP.
The browser uses a revision check to reject stale updates; desktop and MCP writes
are last-write-wins. Deleting a provider removes its setting. A focused Rust
gateway test, 12 simulated desktop provider tests and 39 isolated browser/MCP
workflow tests passed; these tests use fixtures, not live third-party endpoints.
The refreshed worktree deployment became healthy and its MCP coding fixture and
token-limit error check passed against the real `qwen25-coder-7b-q4` Model Host
model with stream usage left at its default. Model routing checks also passed.
All 56 shared-client tests passed in Linux,
including an HTTP fixture that checks both request shapes and returned usage.
The Studio Docker build and all 38 isolated workflows passed with the option
disabled. The independent consumer pinned `dd614f4a38a10e49017b09dc71f7140dce3a5452` and passed 23 offline
tests after vendoring, including a usage-event fixture. The same pin passed
real Model Host discovery, completion, streaming and token-limit checks with the
option disabled. LM Studio's configured endpoint was unreachable during this check; no live
OpenAI or DeepSeek credential was used.

### GTK and rand advisory investigation

At branch revision `53eeffa` on 2026-11-14, `cargo tree --locked --offline
--target all` traced `glib` 0.18.5 through Tauri's Linux GTK/WebKit graph and
`rand` 0.7.3 through `phf_generator` and `selectors` build dependencies. The
feature tree, including `--all-features` for this manifest, did not show the
`rand` 0.7 `log` feature. [RUSTSEC-2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097.html)
requires that feature and other conditions; this observation applies only to the
inspected dependency graph and is not a blanket safety claim.

A textual scan of `.rs` files in the 693 packages returned by `cargo metadata`
found no `array_iter_str` or `VariantStrIter` references outside `glib` itself.
That is not a reachability proof: aliases, generated code, native calls, other
features, targets or future versions could change the result. [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
identifies the affected iterator and lists 0.20 as patched. The current Tauri
GTK3 dependency chain constrains the older version; a coordinated upstream
upgrade or independently reviewed backport and cross-platform validation are
still needed. These findings remain visible release review items.

### SQLx SQLite patch provenance

On 2026-11-14, the vendored SQLx SQLite 0.8.6 tree was compared with the
published crate after verifying its SHA-256. Its only non-whitespace source
change is the documented `libsqlite3-sys` dependency pin; the patch notes are
an additional local file. `bash scripts/test sqlx-patch` now repeats this check,
including offline operation with `--archive`; a disposable tampering fixture was
rejected. This proves provenance, not compatibility or safety.

The [published SQLx SQLite 0.9.0 manifest](https://docs.rs/crate/sqlx-sqlite/0.9.0/source/Cargo.toml)
still caps `libsqlite3-sys` below 0.38, while [RustSec](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
lists 0.39 or later as patched. The [upstream development manifest](https://github.com/transact-rs/sqlx/blob/main/sqlx-sqlite/Cargo.toml)
now permits versions below 0.40. A published, tested compatible upgrade and
cross-platform validation remain necessary before removing the local patch.

### Isolated live calculator workflow

On 2026-11-14, the previously ignored live calculator workflow passed one
iteration through completion against `qwen25-coder-7b-q4` (73.59 seconds). It
used a disposable database and repository. The test asserts its workflow stages
and a coding trace; it does not independently prove generated application
behavior. The test container received no Model Host token and ran on a Docker
internal network. A separate short-lived proxy received the token through
standard input, could reach the native host, and accepted only model discovery
and chat-completion routes. Probes from the test network could not connect to
the host's model port or an external address or resolve an external DNS name;
the proxy rejected a management route and a query-bearing route. The proxy and
network were removed afterward. This is evidence for these probes and Docker's
isolation on this machine, not a formal security proof or validation of other
network configurations.

### Follow-up on ignored Rust cases

Five cases ignored by the broad Linux Rust run passed separately on 2026-11-14:
the embedded Product Book renderer matched a fixture captured from the mocked
desktop browser, that fixture generated an EPUB, two real Chromium PDF tests
passed, and local Whisper transcribed synthetic Piper audio. EPUBCheck 5.4.0
reported no errors or warnings. The PDF tests ran as a non-root user in
read-only, network-isolated containers with dropped capabilities and Chromium's
own sandbox disabled; they are not evidence of the production sandbox. The
renderer fixture uses mocked Tauri calls and is not native-window acceptance.

The live calculator case was initially deferred over credential and network
isolation concerns; the subsequent isolated proxy run above exercised one
iteration without placing the host token in the workflow container.

### Current broad regression run

At branch revision `b448186` with the test-only fixes in this checkpoint, the
Linux Rust library suite passed 246 tests with six ignored; the frontend build
and frontend tests passed, and `npm audit --audit-level=high` reported no
vulnerabilities. The isolated UI command passed its deployed smoke test, 27
remote tests, one CI fixture test and 38 workflow tests. A separate host Chrome
run passed 42 mocked desktop browser tests. Those mocks do not validate a native
window; ignored Rust cases include native fixtures, browser-backed PDF, local
Whisper and live-model workflow tests.

The initial UI run revealed two stale test assumptions: a tree selector chose a
new collapse button rather than a node, and Playwright's `setChecked` raced an
intentional conflict rollback. The tests now target tree nodes and assert the
conflict/rollback directly. The terminology guard now permits CommonJS
`module.exports` while still rejecting legacy `module_id`, as checked with a
disposable fixture. None of these test changes changes application behavior.

### Native host synchronization timeouts

The optional foreground native-host watcher now bounds host plan/status commands
to 15 seconds and each Studio MCP subprocess to 30 seconds. It catches timeouts,
retries in watch mode, and avoids logging captured subprocess output. A timed-out
MCP request may already have completed on the server, so its outcome can be
uncertain. HTTP discovery now runs in a child with a 25-second total deadline;
the bearer token travels on stdin and is not included in process arguments.
Ten `bash scripts/test native-model-host-sync` fixture tests passed, including
slow-response cancellation, size limits, timeout recovery and credential
redaction. Bounded host discovery and MCP provider lookup also passed against the
running worktrees without changing configuration. No Model Host restart was
performed; the watcher remains foreground and does not start automatically.

### Agent model inspection in Studio

The browser Models page and MCP `aruvi_agents` action `inspect_agent_model`
now inspect the model actually selected by the workflow's shared Rust selection
logic, including role-based fallback. The result distinguishes advertised,
not-advertised and unknown discovery, with a 15-second bound and pagination
limit. It does not run inference or guarantee a model is healthy, and concurrent
configuration changes can make any inspection stale. Browser request ordering
prevents an earlier check from overwriting a later agent selection or saved binding.
All 38 isolated workflows passed, including explicit binding, fallback,
unauthorized access, missing model, browser and MCP checks against fixtures. On
the running branch, MCP reported the coding agent's explicit `qwen25-coder-7b-q4`
binding as advertised by the real Model Host. This is a discovery check, not
a new inference test.

### Agent model routing preflight

`python3 scripts/check-model-routing` checks active agents' explicit primary
bindings, enabled model/provider configuration, and provider model discovery
through the running worktree's authenticated MCP interface. It reports unbound
agents as needing review instead of guessing the role-based fallback, bounds
pagination and treats failed discovery as unknown. It performs no writes or
inference; discovery does not prove the model is healthy or suitable for a task.
`bash scripts/test model-routing` exercises pagination, missing models, disabled
providers, voice-only selectors, ambiguous priorities and discovery failure with
fixtures. On 2026-11-14, the running branch reported all nine active agents bound
to the advertised `qwen25-coder-7b-q4` Model Host model; these bindings are local
worktree data, not defaults for other deployments.

### Browser database settings

The authenticated browser Routing page now reads the active server database path,
SQLite migration history, saved path override, and whether `ARUVI_DB_PATH` takes
precedence. It can save or clear the override through the same Rust services used
by desktop and MCP, with a confirmation explaining the restart effect. A mobile
API token now permits these operations; protect it accordingly. Paths refer to the
server filesystem. Like desktop and MCP, concurrent override writes are last-write-
wins and the new path is not validated by opening or migrating it when saved.

All 37 isolated workflow tests passed, including unauthorized and invalid input,
missing path, browser save/clear, migration display, unchanged active database and
Docker environment precedence. The test did not restart into a new database or
exercise an actual migration. Native UI acceptance remains open.

### Current Model Host and RSA advisory revalidation

At branch revision `c9d704d`, the running worktree passed the MCP coding
fixture with `qwen25-coder-7b-q4`, including independent JavaScript assertions
and a token-limit error. MCP also generated synthetic speech with
`piper-ljspeech` and transcribed it with `whisper-tiny-metal`; the browser
recorded synthetic Piper audio and transcribed it with the real host. These do
not test physical audio or live third-party providers.

The Studio lockfile still includes `rsa` 0.9.10 through SQLx's optional MySQL
dependency. On 2026-11-14, `cargo tree --locked --offline --target all -i rsa`
and the same command with `--all-features` produced no selected dependency path
for this Studio manifest. This is dependency-resolution evidence, not a binary
audit or a guarantee for other manifests, feature selections or toolchains.
[RustSec RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html)
now lists `rsa >= 0.10.0` as patched; the lockfile's 0.9.10 remains affected.
A direct update to 0.10 cannot be assumed to satisfy SQLx's 0.9 dependency, and
upstream compatibility and remaining advisories still require review.

### Single-answer client integrity

The shared OpenAI-compatible client rejects multiple choices and a supplied
choice index other than zero in both buffered and streaming completions. This
avoids presenting a discarded or mixed choice as a complete answer. An absent
index remains accepted for compatible providers; earlier stream chunks can
already have been delivered when an invalid later event fails. All 55 shared-client tests passed in Linux, using an offline-resolved temporary
lockfile. A separate consumer pinned to `a81f2a155bee2a8f52652aeb8a7d531503f4f0d2`
passed 22 tests after vendoring with network disabled, including an ambiguous
choice fixture. Its non-root container also passed real Model Host discovery,
completion, streaming and token-limit checks on `qwen25-coder-7b-q4`. All 36
isolated Studio workflows passed with the modified client.

### Browser planner voice view commands

The Rust planner voice endpoint recognizes the desktop trace and conversation
view phrases without calling a model, persists the exchange, and returns an
optional view hint. The browser opens a planner trace panel or returns to the
conversation accordingly. The panel shows the latest available trace in this
browser session and clears it when a new planner session is started; it is not
a persisted trace history. The isolated Docker build and all 36 workflow tests
passed, including a staged-design browser voice scenario for both view commands.
A focused Linux Rust test passed without a configured model. The browser also
supports expanding and collapsing the staged tree through individual branch
controls, global controls and desktop-equivalent voice phrases; selection is
preserved. Its isolated staged-design voice scenario covers both directions and
the global controls. A subsequent browser scenario verifies that “show draft”
opens the staged design and a spoken node selection reveals the selected node
inside collapsed ancestors. Targeted `expand`, `collapse` and `close` commands
now resolve a node in Rust, return its ID separately from the selected node, and
update that browser branch without changing the selection. All 36 isolated
workflow tests passed, including targeted expansion and collapse. Named-node
commands do not strip speech-added punctuation. This does not validate physical
microphones, native-window voice interaction, or every desktop voice command.

### Browser planner voice destination

The browser Voice tab can now send a reviewed or automatic transcript to the
planner voice endpoint as well as direct voice chat. Planner voice uses the
product scope and selected draft node from the Planner tab, updates the draft
tree, and displays the reply in both views. Direct voice chat remains the
default. All 36 isolated workflow tests passed, including a simulated browser
workflow that stages a design, selects a node and sends its product and node
with a `show draft` voice turn; the response retains that selection. The suite
also covers the existing synthetic microphone/direct-chat flow. Native view
switch commands, physical microphone input and live-provider voice quality
remain unverified.
The running branch also passed `bash scripts/test host-voice-browser --planner`
with synthetic Piper audio and real Model Host Whisper: the browser recorded and
transcribed the audio, then a corrected review transcript reached Planner. The
short command was misrecognized in one run, so this is not evidence of accurate
hands-free command recognition. An initial fixture also looped the short audio;
the test now pads it with silence. The existing longer synthetic recording
probe passed separately. Neither probe uses a physical microphone.
Exact planner voice commands now accept one trailing period, including apply
and clear commands, while question marks and named-node references are left
unchanged. The frontend build, six mocked desktop planner checks, five focused
native voice tests and all 36 isolated workflow tests passed; the browser case
uses `Show draft.` with a selected node. This does not improve recognition of
incorrectly transcribed words or validate physical microphones.

### Running branch Model Host revalidation — 2026-11-14

After the voice-only selection guards at branch revision `02ba802`, the running
worktree passed an MCP coding request to `qwen25-coder-7b-q4`, independent
JavaScript assertions, and a one-token truncation check. Authenticated browser
streaming chat also returned real coding-model output. A separate Studio MCP
round trip generated synthetic speech with `piper-ljspeech` and transcribed it
with `whisper-tiny-metal`. These checks exercised the configured native Model
Host environments without changing host configuration; they do not prove broad
coding reliability, physical audio, or native-window interaction.

### Broad regression and dependency scan — 2026-11-14

At branch revision `8715d9c`, the native Studio Rust library suite passed 242
tests with six ignored, and the frontend suite passed 125 tests. `npm audit`
reported zero vulnerabilities. `cargo-audit` scanned the Studio lockfile using
RustSec commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` (updated
2026-11-13) and reported RUSTSEC-2023-0071 for `rsa` 0.9.10, 13 unmaintained
warnings and two unsound warnings (`glib` and `rand`). The current selected
feature set produced no `cargo tree --target all -i rsa` path; this does not
establish safety for other feature sets or inspect the locally patched SQLx
source. The independent consumer lockfile scan reported no advisories or
warnings. The scanner could not open its local crates.io index. Ignored native
tests, physical hardware, native-window acceptance and live external providers
remain separate checks.

### Current requirement audit — 2026-11-14

For mobile/browser chat API calls without a provider ID, a registered model name
now selects its unique enabled text provider. Duplicate enabled registrations
require an explicit provider; a registered model with no enabled binding fails
instead of being sent elsewhere. Explicit provider selection retains precedence.
For compatibility, an unregistered model name still falls back to the first
enabled provider; callers that need deterministic routing should specify one.
The focused Rust routing test and 41 isolated workflows passed. A model-only
mobile request to the real `qwen25-coder-7b-q4` Model Host also returned a
response, and the completed-call ledger recorded the Model Host provider.

The browser Workspaces page can now create a workspace for its selected product
or area in a specified absolute server directory, attach it as default and
retain existing attachments. The full 40-test isolated workflow suite passed,
including browser creation and repeat rejection, unauthorized and relative-path
requests, foreign-area rejection, and the created Git branch. The default
`/repositories` parent is the worktree container's persistent volume; other
deployments must select a suitable server path. Browser work-item-specific
workspace creation and native dialog behavior have not been compared here.

The workspace parity audit found that desktop and MCP workspace creation could
overwrite files in an existing destination and remove a previous default
repository attachment. Workspace creation now refuses existing destinations,
including symlinks, checks the product-area scope, retains old attachments, and
initializes a new Git repository on `main`. The isolated workflow suite passed
40 tests, including existing-file preservation, mismatched scope, empty slugs,
new repository creation and retry, attachment preservation, and branch checks.
Workspace provisioning can still leave a newly created partial directory after
an I/O or database failure; an operator must inspect it before retrying.

At branch revision `aba6e547aadf8573ea0c3b07920c26348d6862b6`, the current
shared-client source passed all 56 tests on native macOS ARM64 with Rust 1.99.1.
The source was copied to a temporary directory and tested offline using a lock
derived from Studio's lockfile; every resolved package/version/source was
present in that lockfile. The native `stream_probe` example then streamed a real
response from the loopback Model Host's `qwen25-coder-7b-q4` model, receiving
nine text chunks. Its token was passed on stdin. This is one local model stream,
not live validation of the cloud providers or physical voice hardware.

The model-call parity audit found that MCP previously required an already-known
call ID to read a snapshot. MCP now lists recent calls and retrieves call details
using the same ledger as the desktop and browser. The isolated workflow suite
passed all 39 cases, including discovery and comparison of MCP ledger results
with a browser workflow record and its snapshots. MCP list results remain bounded
to 500 and do not yet support filtering or pagination. The refreshed worktree
deployment was healthy and its live MCP list action passed a one-record check.

This table summarizes the current evidence; dated sections below are historical
and may describe issues that were subsequently fixed. A passing fixture proves
only the stated scenario. Unchecked checkpoints remain open until their full
scope is reviewed, not merely until an example passes.

| Requirement | Current evidence | Remaining work |
| --- | --- | --- |
| Reusable Rust library | `crates/aruvi-model-client` is consumed by Studio and by a separate project pinned to commit `dd614f4a38a10e49017b09dc71f7140dce3a5452`; 23 independent tests passed offline after vendoring. Its non-root Linux probe passed real Model Host discovery, completion, streaming and token-limit handling. A previous pin passed a synthetic speech round trip on a host offering those models; the coding host used for the new pin did not advertise them. | Decide independent ownership, versioning and a migration plan for other projects. The current pin is a checkpoint, not a published release. |
| OpenAI-compatible and Anthropic protocols | Shared clients cover text, streaming, discovery and error handling; OpenAI-compatible voice is implemented. Contract fixtures cover both protocols and the reasoning request profile. | Live third-party OpenAI, Anthropic, DeepSeek and LM Studio sessions remain unverified; compatibility with each model and provider cannot be inferred from adapter capabilities. |
| Local coding integration | The isolated 7B Model Host passed real MCP completion and an extend-module workflow through code review and a disposable local Git remote. A later new-module attempt produced a missing function definition; independent behavior checks caught it before delivery. | Investigate reliability on broader coding tasks and actual authenticated network delivery; syntax-only checks do not establish correctness. |
| Speech and local fallback | Real Model Host speech generation and transcription and checksum-pinned local Whisper have been exercised with synthetic audio; browser and MCP fixtures exist. | Physical microphone permission/capture, audible playback, other languages and recognition quality remain unverified. |
| Desktop, browser and MCP parity | Browser and MCP support many model, planner, agent, repository and product workflows; the detailed action inventory is in `web-parity-audit.md`. Recent native Rust, frontend and isolated web suites passed. | Complete action-by-action comparison, especially native IDE/dialogs, runtime management, settings and CI. A locked Mac prevented native window interaction; simulated Tauri tests are not native acceptance. |
| Data, configuration and credentials | The branch has isolated Docker data and a loopback endpoint; the model-host setup stores its token outside Git. Inspected Model Host source uses the same token for inference and management. Migration 053 passed backup, integrity and foreign-key checks. | Inference-only host credentials are not established; review host restart operations, cross-platform behavior and upgrade paths. Existing external-channel bindings remain unscoped until explicitly migrated or replaced. |
| Security and release evidence | Independent client and npm scans found no advisories; the latest Studio lockfile scan found an RSA advisory and maintenance/soundness warnings. | Review those advisories and the locally patched SQLx dependency; verify target platforms and complete release acceptance. No release or master merge is authorized by this goal. |

The currently running branch app is a worktree deployment. Its port is dynamic;
use `bash scripts/env worktree status` to identify it rather than relying on a
historical checkpoint address. Staging, production promotion and release are
outside this work until separately requested.

- [x] Inspect the model-host API, deployments and installed models, and the
  current AruviStudio provider and speech paths.
- [x] Decide and implement a genuinely shareable library location and versioning
  mechanism that works both for other repositories and Docker builds. Record
  dependency provenance and reproducible builds; avoid an unpublished dependency
  that only works on this Mac.
- [ ] Define a provider-neutral client API with explicit capabilities, errors,
  bounded timeouts, configuration and credential handling. Preserve response
  metadata and existing Studio behavior. Specify streaming and cancellation.
- [ ] Implement and test OpenAI-compatible text/model discovery (including local
  model host, LM Studio and DeepSeek), native Anthropic protocol, and any other
  provider protocol the inventory shows is actually required. Cover protocol
  differences, malformed responses, authentication and provider failures.
- [ ] Implement and test supported remote voice transcription and speech output,
  model-host-specific format and capability constraints, and applicable native
  fallbacks. Assess image generation separately rather than assuming parity.
- [ ] Integrate the shared library into Studio's chat, agent, planner, speech,
  provider management and MCP paths without changing product-owned workflow,
  secret-store or telemetry semantics. Preserve existing configurations.
- [ ] Provide safe, documented configuration for the local model host in the
  branch Docker environment and native desktop. Do not copy host secrets into Git
  or expose them to unrelated processes. Verify networking and restart behavior.
- [ ] Validate real text inference through Studio and MCP against an available
  model-host model; verify actual response, error handling and telemetry. Test real
  voice where fixtures are available. Record model IDs and quality limitations.
- [ ] Run library tests, Studio Rust/frontend/build/browser/MCP/workflow checks,
  plus simulated external-provider contract tests. Record skipped live services,
  remaining desktop parity gaps and any limitations with evidence.
- [ ] Document integration, provider setup, client versioning and migration for
  other projects; leave the branch deployment running. Prepare release evidence,
  without publishing a release or changing other projects' deployments.

## Discovered model tasks

The desktop and browser model selectors show a provider-reported `task` from
discovery metadata alongside the model ID. This helps distinguish Model Host's
text, speech, transcription and image entries before registration. The label
is informational: provider task vocabulary is not standardized, and it does not
automatically assign capabilities or establish that Studio supports the task.
Missing or non-string task values are ignored, and long labels are bounded.
Desktop browser tests cover paging, labels and the selected model ID; the remote
workflow tests cover the label, selection and stale discovery responses.
On 2026-11-14, `npm run build`, all 6 desktop model-provider Playwright tests,
and all 23 isolated remote Playwright tests passed. The rebuilt worktree app and
its PDF service passed their health checks; its dynamic endpoint was
`127.0.0.1:55079` at this checkpoint.

## Web model definition fields

The browser now creates and edits the desktop model definition fields: context
window, capability tags and notes. An authenticated browser workflow verified
creation, editing and a tag-only concurrent MCP edit returning a conflict while
preserving the browser's draft. Older API clients that omit tags continue to
leave them unchanged; they do not receive tag conflict detection. The isolated
remote suite passed 23 tests on 2026-11-14, and the rebuilt branch services were
healthy at `127.0.0.1:55080`. Provider/model parity as a whole remains open.

## Unsaved file-aware Copilot chat

The web workspace can ask Copilot about the current unsaved editor buffer, as
the desktop can. The backend verifies the saved file's original hash first,
bounds the submitted buffer, labels it as unsaved and truncates the excerpt
sent to the provider. Conversation history resets when editor content changes.
The request and response remain subject to the model-call Activity ledger.
Patch proposals still require a saved file and reject unsaved content at the
API; asking about a buffer does not write it. The isolated remote suite passed
all 26 cases with a simulated provider. A separate disposable browser fixture
used the real `qwen25-coder-7b-q4` Model Host to identify a function present
only in the unsaved buffer, discarded the edit, then reviewed and applied a
saved-file patch. Real web and MCP saved-file follow-ups and patch previews
also passed. This is one fixture, not a general coding-quality guarantee. The
rebuilt branch was healthy at `127.0.0.1:55087` on 2026-11-14.

## Web Copilot generation controls

Web and MCP file Copilot now accept a file-context switch, bounded context
budget, additional system instruction, temperature and output token limit.
The server retains its own instruction and validates these values. When file
context is disabled, neither the file path nor its contents are sent in the
model messages. The browser resets conversation history when these settings
change. The isolated remote suite passed 26 cases using a simulated provider,
including inspection of outgoing requests, MCP context truncation and rejection
of an invalid budget. Defaults preserve previous behavior. Provider-specific
limits may be narrower than the server's accepted bounds. The rebuilt branch
was healthy at `127.0.0.1:55089` on 2026-11-14. The existing real 7B Model
Host browser and MCP Copilot workflow also passed with default settings.

## Desktop Copilot patch safety

The native IDE no longer refreshes a patched open file over unsaved editor
content. It checks every target before starting and checks each target again
between patches. If the user edits a file while its patch is being applied,
the editor preserves that buffer and reports that the disk patch must be
reconciled. Multi-file application can still be partial if a later target
changes or another operation fails. Four frontend regression tests cover the
dirty-before-start, dirty-during-apply, later-target and clean-refresh cases;
the frontend build, all 106 frontend tests and three mocked desktop navigation
checks passed. Native window interaction remains unverified. The isolated
worktree app remained healthy at `127.0.0.1:55095` on 2026-11-14.

## Desktop Copilot prompt construction

Desktop Copilot now sends structured history, mode, settings and optional
editor content to a Rust command that builds the model messages and uses the
existing streaming and telemetry path. File context is bounded to 2 MiB, the
context budget to 200,000 characters, and truncation is Unicode-safe; very
large context requests that the old frontend accepted are now rejected. A
native test checked message construction and a second used a simulated
streaming provider to verify events and the `desktop_ide` Activity record. A
mocked desktop browser test exercised the structured command and patch output;
the frontend build and all 106 frontend tests passed. The browser/MCP Copilot
prompt builder is still separate, and native-window interaction remains
unverified. The rebuilt worktree was healthy at `127.0.0.1:55097` on 2026-11-14.

## Automatic speech model selection

When the browser sends no transcription model, the server now selects the
first enabled registered transcription model for the chosen provider using
the same name/tag predicate as desktop and MCP; if none exists, it retains the
`whisper-1` fallback. Blank saved model and provider values are treated as
unset by all three paths. Explicit browser model IDs continue to be forwarded
as before. A disposable browser recording and MCP transcription test passed
with a registered synthetic speech model and an empty saved model setting;
all 31 isolated workflow cases passed. The rebuilt branch was healthy at
`127.0.0.1:55091` on 2026-11-14. The native speech-focused run passed five
tests and ignored its installed-model fixture.

## Web server transcription defaults

The Voice screen can now edit server-wide transcription provider, model and
locale together. The authenticated endpoint uses optimistic checks in one
transaction; browser-specific selections and the documented server environment
variables can override the saved values. A disposable browser/MCP fixture
verified a concurrent edit rolls back all three fields and that a recording
with no browser overrides uses the server's registered speech model and locale.
All 31 isolated workflow tests passed. A separate browser test recorded synthetic
Piper audio and transcribed it through real Model Host Whisper. Physical
microphone input and recognition quality remain unverified. The rebuilt branch
was healthy at `127.0.0.1:55092` on 2026-11-14.

`bash scripts/test host-speech-defaults` additionally starts a disposable
Studio fixture, configures its defaults through the browser, generates synthetic
Piper audio and transcribes it using the real native Model Host Whisper model.
The browser checked both omitted and explicit per-browser settings, and MCP
transcribed using the saved defaults. The fixture passed on 2026-11-14 and
removed its own container and volumes; it did not alter the branch database
or the Model Host deployment.

When a client explicitly chooses a different provider from the configured
speech default and omits a model, desktop, browser and MCP now ignore the
other provider's default model and select an enabled registered speech model
for the chosen provider. An explicit model remains authoritative; a model-only
default remains usable if no default provider is configured. A native unit test
and all 31 isolated workflow tests passed, including browser and MCP requests
with different configured and selected providers. The real Model Host speech
defaults fixture also passed. The rebuilt branch was healthy at
`127.0.0.1:55095` on 2026-11-14.

## External planner defaults in the browser

Routing now exposes the server-wide provider ID and model name for external
planner sessions. Both settings are saved in one transaction with optimistic
checks; the disposable browser/MCP workflow verified authentication, persistence,
and rollback of both fields when an MCP edit makes one value stale. The 24-test
isolated remote suite passed on 2026-11-14. Environment variables
`ARUVI_PLANNER_PROVIDER_ID` and `ARUVI_PLANNER_MODEL_NAME` take precedence in
external channel resolution; browser session settings remain separate. This
does not validate a live WhatsApp or voice-channel session.

## External planner webhook authentication

The WhatsApp and voice-gather routes share signature validation. Previously,
either missing the Twilio auth token or missing the configured webhook base URL
silently disabled signature verification. They now reject requests until both
values are configured, and verify the decoded signature with the HMAC verifier.
This changes existing deployments that relied on unsigned channel webhooks:
configure `ARUVI_TWILIO_AUTH_TOKEN` and `ARUVI_WEBHOOK_BASE_URL` (or their
corresponding settings) before enabling external channels. The Twilio voice
entry route and other channel behavior are separate from the gather callback.
On 2026-11-14, the native signature unit test and all 25 isolated remote tests
passed; the HTTP fixture covered missing configuration, an invalid signature
and a valid signature. The 7B coding fixture and token-limit error also passed
through Studio MCP, and browser speech generation loaded real Model Host WAV
audio before this webhook-only change. Native window and physical audio checks
were unavailable because the console was locked. The rebuilt branch was healthy
at `127.0.0.1:55083`.

## External channel product scope

An end-to-end signed callback test found that external planner turns previously
stopped at the product-selection prompt before calling the configured model.
Routing now offers an explicit default product for new external conversations.
Migration 053 stores the selected product on the channel binding, so later
default changes do not redirect existing conversations. Existing unscoped
bindings remain unscoped; deleting the bound product clears the binding's
product reference. No product is selected implicitly. The browser, signed
WhatsApp and signed voice-gather fixture exercised the configured provider,
model and product with a simulated model, and checked that an existing
conversation retained its original scope after a default change. Live Twilio
and live model behavior through these channels remain unverified.

Before applying migration 053 to the branch, a SQLite online backup was saved
to `/tmp/aruvi-before-053.db` and passed `quick_check`. Afterward the worktree
database reported migration 53 successful, `quick_check` OK and no foreign-key
violations. The backup is local and is not a release artifact.
The final rebuilt branch was healthy at `127.0.0.1:55085`; all 26 isolated
remote tests passed after the product dropdown was added.

## Current integration revalidation

On 2026-11-14, the current native macOS ARM64 desktop and server debug binaries
built with locked dependencies. A separately configured native Studio server
discovered and queried the real 7B coding host through MCP and generated and
transcribed synthetic speech through the native Model Host integration
environment. The host setup and validation scripts now accept the integration
environment name. This does not verify the native desktop window or physical
audio; the Mac console was locked during this check.

On 2026-11-14, the isolated real-model `extend-module` workflow ran on the
current branch image with `qwen25-coder-7b-q4`. It reached plan approval,
inspected the existing JavaScript module, generated the requested extension,
passed the workflow syntax gate and independent network-isolated behavioral
assertions, reached `done`, and delivered the verified file to a disposable
local Git remote. This is a single task and does not establish general coding
reliability or validate an authenticated network Git push.

On 2026-11-14 the native Rust library suite passed 224 tests with six ignored;
the frontend suite passed 102 tests. `npm audit --omit=dev --audit-level=high`
reported no vulnerabilities. The independent Git consumer built from its pinned
client revision with network disabled after vendoring and passed 20 tests. Its
non-root, read-only Linux probe also passed real Model Host discovery, completion,
streaming and token-limit checks. Studio's MCP coding fixture passed against the
same running 7B model.

The normal `host-shared-client` script initially could not query the older
Model Host validation checkout because its staging reservation had changed.
The checkout's local `deploy/product.json` was updated from staging port 20680
to the current reservation 21680; this is an uncommitted adjustment in the
detached validation checkout and does not move or restart its running worktree.
Afterward native worktree status and the standard `host-shared-client` command
passed against the live 7B model. Studio's six native-host synchronization
unit tests passed. The Model Host main checkout and deployed services were not
changed. The Studio branch app remained healthy at `127.0.0.1:55085`.
An additional native migration test upgraded a pre-053 database containing a
legacy channel binding, confirmed it remains unscoped, rejected an invalid
product reference, and verified product deletion clears the reference.

## Managed local model installation

- Local Whisper providers support transcription, not agent text completion.
  Browser model cards now select them for transcription without replacing the
  browser's chat model, and they are excluded from the browser agent selector.
  The shared binding repository rejects local Whisper assignments from browser,
  desktop or MCP without removing existing bindings. New browser-created local
  models use the same speech capability tags as desktop. Native Rust passed
  222 tests with six ignored; 23 remote UI and 31 isolated workflow cases
  passed, including actual local Whisper transcription with synthetic audio and
  MCP rejection preserving the previous binding. Remote provider capability
  labels do not prove the selected endpoint or model supports a task.
- Migration 052 closes indirect paths around that guard: a bound text model
  cannot be moved to a local Whisper provider, and a provider with bound agent
  models cannot be changed to local runtime. Direct binding inserts and model
  changes are guarded too. An upgrade test preserved an existing invalid legacy
  binding and allowed it to be repaired; it is not repaired automatically.
  Native Rust passed 223 tests with six ignored, and all 31 isolated workflow
  cases passed, including browser and MCP rejection and real local transcription.
  The backed-up branch database upgraded to migration 52 with integrity and
  foreign-key checks passing; nine Model Host bindings remained, and real MCP
  coding and token-limit checks passed.
- The desktop agent selector now marks local Whisper models as transcription
  only and disables their selection; it also disables binding while provider
  data is unavailable. Six mocked desktop browser cases, 102 frontend unit
  tests and the frontend build passed. The real native window remains untested.

- Branch runtime routing checkpoint: all nine seeded agents were bound to
  `deepseek-coder` on the container-local LM Studio endpoint, which failed its
  connectivity check. After confirming these were untouched default bindings,
  all nine were changed to `qwen25-coder-7b-q4` on the configured native Model
  Host and read back through MCP. A disposable workflow using the same model
  for all stages extended an existing module, passed syntax and independently
  executed behavior checks, and delivered to a disposable Git remote. This is
  evidence for that fixture, not a general quality guarantee. These branch-only
  bindings are persistent runtime data, not established by a Git commit or
  applied to other environments. Reconfigure individual agents if different
  provider models are desired.

- Native Model Host setup now only accepts an installed model advertised for
  text, explicitly restores the standard completion profile, and checks the
  existing provider's host, port and URL shape before updating it. Fixed setup
  errors are actionable without echoing credentials. Four helper tests passed;
  the helper refreshed the real 7B host provider and subsequent MCP coding,
  token-limit handling and browser streaming checks passed. The helper does not
  assign agent bindings or start automatically after a host restart.
- Native host discovery now rejects HTTP redirects instead of forwarding its
  bearer token to the redirect destination. A two-server fixture verified the
  redirect target received no request; five helper tests passed. The updated
  helper then refreshed the real 7B provider, and its MCP coding and token-limit
  checks passed. The stable native staging host advertises speech, transcription,
  image and small text fixtures, but not the validated 7B coding model; the
  current coding provider uses the dedicated native worktree and its dynamic port.
- The optional host watcher now updates a changed port without rotating an
  unchanged token; a changed token still rotates the credential. Six helper
  tests cover retry, redirection and both change types. A real host refresh
  preserved the Studio credential revision and the MCP coding fixture passed.
  The watcher remains opt-in and does not survive a machine restart by itself.

- Desktop and MCP now share the same managed installer. It rejects paths in
  model filenames, refuses symlink destinations and symlink model directories,
  bounds downloads to 8 GiB and one hour, and publishes a nonempty file only
  after the download finishes. Failed downloads are removed; an existing
  nonempty regular file is reused. Download URLs can still access endpoints
  reachable from the server and follow redirects. Checksums were not checked at
  that checkpoint; the later preset checksum change is described below.
  Installations refer to the server's filesystem.
  Native Rust passed 222 tests with six ignored, including interrupted download
  and MCP path checks. The rebuilt container passed 29 isolated workflow cases,
  including MCP install, rejection and idempotent reuse with a fixture server.
- Browser model setup now registers an existing server-side Whisper file using
  the same Rust operation as desktop and MCP. It rejects directories and makes
  the container path requirement explicit. Native Rust passed 222 tests with
  six ignored, and 30 isolated workflow cases passed, including browser
  authentication, registration, invalid path, secret-reference omission and
  repeat registration. This does not upload a local browser file.
- The browser also offers the three desktop Whisper presets through a fixed
  server-side allowlist and the shared installer. The isolated suite passed
  31 workflow cases, including authentication, invalid preset rejection and
  repeated installation using a pre-existing synthetic file. Actual preset
  downloads were not exercised in this test; the shared installer separately
  passed success and interrupted-download fixture tests. Browser file upload
  remained open at that checkpoint.
- Desktop and browser presets now pin Hugging Face revision
  `5359861c739e955e79d9a303bcbc70fb988958b1` and the upstream LFS SHA-256 for
  each file. The shared installer verifies downloaded and existing preset files,
  reports mismatches without deleting existing files, and publishes new files
  only after verification. MCP custom installs may supply `expected_sha256`;
  custom installs without it and manually registered files are not verified.
  Native Rust passed 222 tests with six ignored, and five mocked desktop
  browser tests passed. Two 31-case isolated workflow runs passed: one rejected
  a corrupt existing file, while the other downloaded the real Tiny English
  preset and verified reuse. Source metadata was checked against the
  [upstream model repository](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1).
  The test covers that preset, not transcription quality or all three downloads.
- The SHA-256-verified Tiny English preset also transcribed a synthetic Piper
  sentence through Studio's native local Whisper fallback. Repeat with
  `bash scripts/test local-whisper-preset /path/to/ggml-tiny.en.bin`; the check
  verifies the preset hash, generates audio through the configured native
  staging Model Host, and runs the ignored native Whisper integration test.
  An invalid model was rejected before inference. This checks synthetic English
  audio, not physical capture or broad transcription accuracy.
- The same check with `--container` copies the verified preset and generated
  WAV into disposable fixture containers. All 31 isolated workflow cases passed,
  including preset registration and real local Whisper transcription through
  the browser Voice screen with synthetic media, the authenticated browser API,
  and MCP. This exercises Linux CPU inference and synthetic audio; it does not
  establish physical microphone behavior.

## Product Book export data

- Browser/MCP EPUB and PDF, and native EPUB, now support embedded PNG and JPEG
  data images. Images are decoded with strict count, dimension and byte limits;
  EPUB assets are deduplicated. The real browser/MCP fixture covered both image
  types and a duplicate, passed EPUBCheck 5.4.0, and produced a PDF containing
  both image types. Repository paths, remote image URLs, animated PNG and other
  image formats remain unsupported and fail explicitly for EPUB/PDF.
  RustSec reported no advisory for the added image decoder; the existing RSA
  advisory and other warnings remain, and the crates.io index refresh failed.
  Native and Linux Rust suites each passed 214 tests with six ignored, and the
  isolated browser/MCP suite passed 23 tests. EPUBCheck reported no findings;
  the generated PDF was inspected for both embedded image types. Native desktop
  UI and actual EPUB reader compatibility have not been validated for images.
- Worktree PDF rendering now runs in a separate network-disabled, read-only,
  non-root container with no app data, repositories or API tokens; the only
  shared resource is a private Unix socket. Its own Chromium sandbox remains
  disabled due to the Mac Docker namespace restriction. The isolated suite
  verifies the container settings and that a stopped renderer fails closed;
  staging and production sidecar support remain unverified.
  Native and Linux suites each passed 212 tests with six ignored, and isolated
  browser/MCP, workflow and CI suites passed 23, 23 and one case respectively.
  Two worktree environments were run concurrently with distinct
  ports, credentials, database volumes and PDF socket volumes. A product created
  only in the second was absent from the first, and its PDF export succeeded;
  destroying the second environment left the first healthy.
- Browser and MCP Product Book exports now include PDF through a bounded
  Chromium worker. The isolated browser suite downloaded PDFs in all three trim
  sizes and invoked MCP PDF export. Captured PDFs had the expected dimensions,
  tags and searchable, product-scoped text; an A5 chapter was visually checked.
  Native real-browser tests verified external requests and scripts are blocked.
  At that checkpoint PDF and EPUB rejected books containing images. The worktree's Chromium
  sandbox is disabled because this Mac's container runtime rejects its required
  namespace operation; the image defaults to sandboxing. The legacy desktop PDF
  writer is not yet unified with this worker. These are release limitations.
  The native and Linux Rust suites each passed 211 tests, with six ignored;
  two real Chromium PDF tests passed separately, as did the 23-case remote suite.
- The authenticated browser and `aruvi_catalog.export_product_book` can now
  produce Docs HTML, Book HTML and EPUB from the shared backend. MCP returns
  base64 bytes; browser responses are attachments. The real browser download
  and MCP tests checked product isolation and missing authentication, and a
  captured EPUB passed EPUBCheck 5.4.0. The backend also rendered a 2,102-item
  HTML book. At that checkpoint image packaging and browser/MCP PDF export
  remained open. Native and Linux suites
  each passed 210 tests with four ignored, and the remote suite passed 23.
- Desktop Product Book exports now invoke the shared Rust backend renderer
  before using the existing native HTML, PDF and EPUB file writers. A Tauri mock
  IPC test exercises the actual command and database with camel-case frontend
  arguments; desktop UI tests cover all formats, selected PDF trim dimensions
  and rendering failures. This does not establish actual window, file reveal,
  or reader behavior. The shared renderer and browser/MCP artifact gaps remain
  tracked in `docs/product-book-migration.md`. Native and Linux Rust suites
  each passed 209 tests with four ignored; 94 frontend unit tests, 11 focused
  desktop UI cases and the renderer parity harness passed. The captured EPUB
  fixture also passed EPUBCheck 5.4.0, and the isolated remote suite passed
  23 cases against the rebuilt branch image.
- The Rust backend can render Docs or Book HTML from a consistent product
  snapshot using the existing renderer in an isolated, resource-limited embedded
  JavaScript engine; MCP exposes `aruvi_catalog.render_product_book`. A
  501-item fixture compares desktop and backend output for Docs, web Book, EPUB
  and print modes, excluding generation timestamps. This is shared rendering,
  not completed web/desktop/MCP export parity; see the Product Book checklist.
  RustSec reported no advisory for the added QuickJS bindings; the previously
  documented RSA advisory and other warnings remain. The scanner fetched its
  advisory database but could not refresh the crates.io index.
  Native and Linux Rust suites each passed 208 tests (four ignored); the
  501-item renderer parity harness and all 94 frontend tests passed. The
  isolated container suite passed 23 tests, including authenticated MCP
  rendering, all trim presets, isolation, and invalid argument handling.
- A 501-item book generated by the desktop renderer and packaged by the Rust
  EPUB service passes EPUBCheck 5.4.0 with no findings. This caught and fixed
  broken archive links for repository-relative references. The repeatable
  `scripts/test-product-book-epub` harness exercises the renderer, packager and
  validator; image packaging and actual reader compatibility remain open.
- Desktop EPUB packaging now uses a shared Rust archive service instead of a
  system `zip` process and temporary directory. A real archive structure test
  checks its mimetype, contents, nested navigation and escaped metadata. The
  service packages already-rendered XHTML; complete renderer and reader checks
  remain open. The 2026-11-13 RustSec scan of the updated lockfile reports the
  existing RSA advisory and 15 warnings, with no ZIP advisory; its crates.io
  index refresh failed, although the advisory database was fetched.
  With the patched ZIP dependency, both native and Linux Rust suites passed
  207 tests, with two live tests ignored in each suite. The isolated remote
  UI and MCP suite passed 23 tests against the rebuilt image.
- Desktop exports now read the product, hierarchy, product-scoped references and
  all work items through one SQLite snapshot. The same source is exposed as the
  authenticated MCP catalog action `get_product_book_source`. A concurrent-write
  database test covers snapshot consistency and reference isolation. This does
  not yet provide browser or MCP Product Book artifact exports; see
  `docs/product-book-migration.md` for the remaining renderer and download work.
  Validation: native and Linux Rust suites each passed 206 tests (two ignored),
  the mocked desktop suite passed 10 tests, and the isolated remote suite passed
  23 tests, including MCP authentication and cross-product reference isolation.
- The desktop Product Book previously passed an empty work-item array to its
  view and exports. A product-scoped Rust query now returns all work items,
  including child tasks, and is available through desktop and MCP. The view
  loads delivery items on request and indicates when they are not loaded;
  exports fetch them automatically and report read failures rather than
  producing an incomplete file. A 2,102-item database fixture and mocked
  desktop HTML/EPUB/PDF request checks cover the source data and product scope.
  Very large products can consume substantial memory when explicitly loaded or
  exported, and the separately loaded product tree, references and work items
  do not form one consistent snapshot. The browser/MCP Product Book renderer
  and artifact exports remain open; see [the migration plan](product-book-migration.md).

## Anthropic stream validation

- The shared client rejects malformed Anthropic event envelopes and non-object
  message/delta payloads after partial text, and validates an explicit stop
  reason in a message-start payload. Unknown named event types remain ignored,
  consistent with Anthropic's streaming guidance. The macOS client suite passed
  43 tests; the Linux Studio suite passed 204 with two live tests ignored. All
  23 isolated workflow cases passed, including a simulated Claude stream that
  fails after emitting text. The independent consumer now pins
  `d46befd707d8fc1defd156bdef251e5ea9ce3157` and passed 16 offline tests,
  including the malformed Anthropic stream; its separate local Model Host probe
  also passed. Live Anthropic validation remains open.

## OpenAI-compatible stream validation

- The shared client rejects malformed choice containers, message/delta objects
  and content types rather than silently discarding malformed stream data after
  earlier text. Missing optional fields and null intermediate content remain
  supported. The standalone macOS client passed 42 tests, including buffered
  and incremental malformed-stream cases; the Linux Studio suite passed 204
  tests with two live tests ignored. All 23 isolated workflow cases passed,
  including a browser stream that records an error after partial output. The
  real 7B Model Host browser and MCP Copilot fixture also passed. The public
  independent consumer now pins `abd7eb31a544b599e2ebe10f84a98d5311508b0d`;
  its 15 offline tests include malformed buffered and streaming responses, and
  its separate real-host probe passed discovery, completion, streaming, JSON
  mode and token-limit handling.

## Browser workflow automation settings

- The browser Routing screen now exposes desktop's auto-start, plan approval,
  and test-review approval settings. Reads preserve the backend's existing true
  defaults and boolean parsing; a save compares the raw value so a concurrent
  desktop or MCP change returns a conflict. The mobile API permits only these
  three settings. All 23 remote UI cases passed, including authorization,
  allowlist, unchanged neighboring settings and an MCP-versus-browser conflict;
  all 23 isolated workflow cases passed. macOS and Linux Rust suites each passed
  204 tests with two live tests ignored.

## Real browser voice revalidation

- Current-branch macOS ARM64 desktop and server debug binaries built with the
  locked dependencies; the native window remains untested because the console
  was locked. `scripts/test native-model-host BINARY HOST_WORKTREE --model ID`
  starts an isolated native server with a temporary database and private
  credential file, confirms the listener belongs to the started process, and
  checks discovery and coding completion through MCP. An initial shorter prompt
  produced one unclassified completion failure; after increasing its token
  budget and making the output instruction explicit, two separate runs passed.
  With `--speech-host HOST_CHECKOUT --speech-environment staging`, the same
  native fixture also generated Piper audio and transcribed it with Whisper on
  the deployed native host. The audio was synthetic English. This is not a
  model-quality, physical-device or native-window validation.

- On the current branch after the provider-profile migration, real Model Host
  `piper-ljspeech` generation and `whisper-tiny-metal` transcription passed as
  an MCP round trip. A separate browser check recorded synthetic Piper audio at
  16 kHz and transcribed it with the real host. This does not verify a physical
  microphone, audible playback or model quality for other languages.

- Manual desktop chat and planner recording now stop after 30 seconds or
  the corresponding sample count, even if timers are delayed. The captured
  audio follows the same transcription flow as a manual stop, and the UI shows
  the limit. Shared capture tests verify a bounded 48 kHz input produces a
  30-second 16 kHz WAV. All 102 frontend tests passed; four desktop-style
  browser cases covered manual and automatic stops, single transcription and
  track cleanup in chat and planner. These use mocked audio and Tauri; real
  native-window and physical microphone behavior remains unverified.

- Desktop PCM capture now releases microphone tracks if context setup or
  audio-node connection fails. Manual stop, silence and timeout share one
  finalization result; automatic close failures reject Voice Chat completion
  instead of leaving it pending. Planner displays voice errors in its composer.
  Five mocked audio lifecycle tests, 99 frontend tests, a desktop-style browser
  error/cleanup test, frontend build, and 23 isolated browser/MCP cases passed.
  The browser test uses a mocked Tauri bridge and microphone; real native-window
  behavior and physical hardware remain unverified. Manual desktop recording
  was uncapped at that earlier checkpoint; the later limit is described above.

- Browser recording now requests a 16 kHz AudioContext so a 30-second
  recording remains within the existing 600,000-sample cap. If the browser
  cannot create that rate, it uses its default and displays the actual shorter
  duration (12.5 seconds at 48 kHz). The isolated workflow suite passed 25
  cases, including automatic stopping, 48 kHz WAV contents, rate fallback,
  bounded payload size and track cleanup. A real headless Chrome capture used
  16 kHz synthetic Piper audio and Model Host Whisper transcribed it. Physical
  microphones, other browsers and speech recognition quality remain unverified.

- After the dependency update, the running branch passed real Model Host Piper
  speech generation in Chrome and a Piper-to-Whisper transcription round trip
  through MCP. A new `bash scripts/test host-voice-browser` check also drives
  browser recording using a synthetic Piper WAV as Chrome's fake microphone and
  verifies `whisper-tiny-metal` returns the expected `fox` and `dog` anchors.
  It uses a temporary audio file and cleans it up. These are synthetic English
  fixtures; physical microphone capture, audible playback, other languages and
  recognition quality remain unverified.

## Dependency advisory update

- Rechecked on 2026-11-14 against RustSec commit
  `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`: the independent consumer
  lockfile had no vulnerabilities or warnings. Studio still reports
  RUSTSEC-2023-0071 for `rsa` 0.9.10, 13 unmaintained warnings, and the
  unsound warnings RUSTSEC-2024-0429 (`glib` 0.18.5) and RUSTSEC-2026-0097
  (`rand` 0.7.3). `cargo tree --locked --target all -i rsa` found no selected
  path under the current feature configuration. This does not rule out other
  feature combinations; the local SQLx patch is outside the scanner's source
  review. Full `npm audit` reported zero vulnerabilities. These findings
  remain release review items rather than a claim of complete safety.

- On 2026-11-14, cargo-audit 0.23.0 refreshed RustSec to commit
  `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` (database update timestamp
  2026-11-13) and scanned the current lockfiles. The independent consumer had
  no vulnerabilities or warnings. Studio reported RUSTSEC-2023-0071 (`rsa`),
  13 unmaintained warnings and two unsound warnings: RUSTSEC-2024-0429 (`glib`)
  and RUSTSEC-2026-0097 (`rand` 0.7.3). `cargo tree --locked --target all -i
  rsa` found no selected path for the current features; target/feature changes
  can change that. The old `rand` is in Tauri's build dependency chain, and
  Linux ARM64 selects `glib` through GTK/Tauri. These are not proof of either
  exploitability or safety. The scanner does not audit our locally patched SQLx
  source. A fresh `npm audit` reported zero advisories. Review these findings
  and the patch provenance before release.

- On macOS ARM64, revision `0275579` passed 204 Studio Rust library tests with
  two live tests ignored, and the independently Git-pinned consumer passed 14
  tests. Both native Studio desktop and server debug binaries built with the
  locked dependencies and were identified as ARM64 Mach-O executables. The
  toolchain, build target and test data were isolated in temporary directories.
  The native desktop window was not launched or interactively tested, and the
  binaries are debug builds rather than release artifacts.
- Updated Studio's lockfile to `anyhow` 1.0.103, `event-listener` 5.4.2,
  `rustix` 1.1.6 and `rand` 0.8.6, addressing four previously reported
  unsoundness warnings. The locked update also changes some Windows transitive
  bindings; Windows builds were not rerun in this environment.
- A fresh `cargo-audit` 0.23.0 scan of the final lockfile reported one RSA
  vulnerability, 13 unmaintained warnings and two unsound warnings: `glib`
  0.18.5 and `rand` 0.7.3. `cargo tree --locked --target all` showed the old
  `rand` in Tauri's build-dependency chain and `glib` in GTK/Tauri; the pinned
  RSA has no selected path for the current feature set across targets. Different
  feature combinations may differ. The scanner does not audit the locally
  patched SQLx source. `npm audit` reported no advisories.
- Linux Rust passed 204 tests with two live tests ignored; the rebuilt image
  passed 22 remote UI cases, 23 workflow cases and a real 7B Model Host browser
  and MCP Copilot fixture. These results do not prove exploitability or its
  absence, and the remaining advisories require review before release.

## OpenAI-compatible completion parameters

- The shared client can explicitly select `max_completion_tokens` instead of
  `max_tokens`, and independently omit `temperature`, for ordinary, JSON and
  streamed completions. Existing defaults remain unchanged for Model Host and
  DeepSeek. A simulated HTTP server verifies the fields for all three paths.
  The [OpenAI reference](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
  says `max_tokens` is incompatible with o-series models; the
  [DeepSeek reference](https://api-docs.deepseek.com/api/create-chat-completion/)
  specifies `max_tokens`. Model-specific support for JSON, streaming and other
  parameters remains the endpoint's responsibility. Studio stores a per-provider
  completion profile: `standard` preserves existing requests and
  `openai_reasoning` selects `max_completion_tokens` and omits temperature.
  The desktop and browser forms expose the profile as a provider type, and MCP
  `aruvi_models` create/update actions accept `completion_profile` with either
  value. The reasoning profile requires an OpenAI-compatible provider and
  protocol. Migration 051 defaults existing providers to `standard`; a migration
  test checks existing providers, models, bindings, credentials and foreign
  keys. An isolated simulated endpoint checks ordinary and streamed browser
  calls and MCP completion requests. No live OpenAI call was run.
  The client passed 46 tests; Studio passed 219 native tests with six ignored
  and 27 isolated workflow cases. Real Model Host coding and token-limit
  handling passed through MCP using the unchanged default request shape.
- Provider profile integration checkpoint: the client passed 46 tests; Studio
  passed 220 native tests with six ignored; frontend passed 102 tests, isolated
  browser UI passed 23 cases, and isolated workflows passed 28 cases including
  profile creation, edits, conflict handling and simulated reasoning completions.
  The branch database upgraded to migration 51 with integrity and foreign-key
  checks passing; all three existing providers retained the standard profile.
  The real Model Host coding and token-limit MCP fixture passed again after the
  branch deployment. The OpenAI reasoning endpoint was simulated, not called live.
- Desktop provider profile checkpoint: a real Tauri mock-runtime IPC test now
  exercises camel-case create/update arguments, persisted profile reads, legacy
  defaults and rejection of incompatible protocol changes. Four Chrome tests
  of the desktop provider page passed with a mocked Tauri bridge, including
  creating the reasoning profile and restoring it when editing. These tests do
  not exercise a native window or a live OpenAI endpoint.

## Structured provider HTTP errors

- The Anthropic adapter marks its API-key header sensitive, preventing it from
  appearing in request debug formatting. Invalid HTTP header values are rejected
  at client construction without echoing the key. The standalone client passed
  47 tests, including redaction and authenticated requests; the Studio simulated
  Anthropic MCP and browser integration test also passed. The rebuilt branch
  passed all 28 isolated workflow cases, including simulated Anthropic streaming,
  and the real Model Host coding/token-limit MCP fixture. This does not guarantee
  redaction by external proxies or provider services; live Anthropic was not used.

- Reqwest transport errors now omit request URLs, protecting proxy paths and
  discovery cursors from those error messages while retaining the error category.
  The configured provider URL is still stored and displayed by Studio and may
  appear in its telemetry; this change does not protect secrets placed there.
  A regression test first reproduced the leak, then passed for text, streaming,
  discovery, health and voice operations across both transports. An isolated
  MCP fixture also verifies error text. The client passed 45 tests, Studio
  passed 217 native tests with six ignored, and all 26 isolated workflow cases
  passed. Real `qwen25-coder-7b-q4` coding and token-limit behavior passed via
  MCP on the rebuilt branch. Live third-party services remain unverified.

- Non-success provider HTTP responses now expose a numeric status via
  `ClientError::Http` and `http_status()` for completion, streaming, discovery,
  transcription and speech. Existing error text is retained; provider error
  bodies are not stored. Health checks still return a boolean, and the existing
  unsupported-transcription special case remains unchanged. Other failures may
  have no status; callers must not assume a status alone makes retries safe.
  This adds an enum variant, so exhaustive consumer matches must be updated.
  The independently recommended Git pin was subsequently advanced to this
  change and tested as documented in the independent consumer checkpoint.
  The shared client passed 44 native tests including simulated 429 responses
  across both provider protocols and the supported operations; Studio passed
  214 native Rust tests with six ignored, and 23 isolated browser/MCP cases
  passed. The branch Docker deployment rebuilt
  and both its app and isolated PDF service passed health checks.

## Repeated coding edits

Two fresh 7B `extend-module` workflow attempts reached planning and coding but
failed the syntax gate before delivery. In the captured second trace the model
repeated the same successful line-range replacement three times, duplicating code.
The coding tool loop now rejects an identical successful replacement until the
file is reread or another edit changes it, and gives the model an actionable
observation. A simulated tool-loop regression verifies the repeated call is not
applied; the check does not detect all semantically duplicate edits or guarantee
model correctness. Line-range replacement now treats a terminal newline as a
line terminator instead of adding a blank line; focused start/middle/end and
coding-loop tests passed. Mixed line-ending preservation remains unverified.
After rebuilding, a fresh 7B `extend-module` run passed its syntax
and isolated behavioral checks and delivered to the disposable Git remote. The
model did not repeat its edit in that run, so the guard itself was exercised by
the simulated regression, not the successful live run. The two preceding failures
remain evidence of nondeterministic coding reliability.
A subsequent 7B run after the newline fix passed the syntax and behavioral gates
and delivered, but the generated replacement duplicated an existing `sumEven`
function declaration. The gates did not detect that code-quality defect, so this
is not evidence of clean code or general coding reliability. The full native Rust
suite passed 236 tests with six ignored on 2026-11-14.
The disposable evaluator now checks for repeated named `sumEven` and `clamp`
function declarations before approving delivery. Its negative control supplied
behaviorally correct code with a duplicate and verified rejection with the Git
remote unchanged. This is a fixture-specific textual check, not a general parser,
product-level quality gate, or proof against duplicate code in other forms.
The coding tool now rejects a range replacement that exactly repeats a
substantial immediately adjacent line, with feedback to adjust the range or set
`allow_adjacent_duplicate=true` when intentional. A simulated coding-loop test
covers both rejection and override. This is a heuristic: it can require an explicit
override for legitimate duplication and cannot detect semantic duplicates or
nonadjacent context. It is not a general quality gate.
In a subsequent real 7B run, syntax passed but the independent behavioral
check rejected the generated module because it omitted the `sumEven` export;
the run stopped before delivery. Its trace included malformed tool-call envelopes.
The live run did not demonstrate the adjacent-overlap guard, and broader coding
reliability and malformed-output recovery remain open.
The coding-loop parser still refuses malformed flat tool calls, but for an
unambiguous allowed call it now provides a bounded corrected envelope as a
suggestion; only a later model response can trigger an action. Multiple actions,
including mixed flat and valid envelopes, are rejected. Parser and simulated
coding-loop tests confirm the malformed call itself is not executed. This does
not guarantee that a model follows the correction.
A subsequent real 7B `extend-module` run passed syntax, behavior and the
fixture-specific duplicate check and delivered to its disposable remote. The
model did not emit a malformed envelope in that run, so recovery remains
verified by deterministic tests only.

## Independent client revalidation

- The independent Git consumer now pins
  `5d49cfd5bd0cf55294cccdabba81f9121be9051e`, including the Anthropic
  credential-header fix. All 20 consumer tests passed offline on macOS and in
  a network-disabled Linux build after vendoring. A separate non-root,
  read-only container used that pinned client against the real 7B Model Host
  and passed discovery, completion, streaming and token-limit handling. The
  Anthropic credential behavior was simulated, not tested against live Claude.

- The independent consumer pins `0496499073bcd0a6771a485a02b326ab05fcad27`
  and passed 19 network-disabled vendored tests, including explicit completion
  request fields. A separate live Model Host probe passed discovery, text,
  streaming, JSON-object mode and token-limit handling using the default mode.
  The new OpenAI request mode was tested against a simulated server only.

- At the previous checkpoint the independent consumer pinned `bc8bfaa1df483a8eae380c34931c1b8d0966f5b8`
  and passed 18 vendored, network-disabled tests, including the URL-redaction
  regression. A separate live Model Host check passed discovery, completion,
  streaming, JSON-object mode and token-limit handling with
  `qwen25-coder-7b-q4`. This does not validate live third-party providers.

- At the earlier checkpoint the independent consumer pinned `89a63f60ac41ca78d304d6a517bf394a79f3d146`.
  Its clean vendored, network-disabled test passed all 17 cases, including a
  simulated 401 that checks numeric status and response-body privacy. A separate
  live Model Host probe passed discovery, ordinary and streamed completion,
  JSON mode and token-limit handling on `qwen25-coder-7b-q4`. This does not
  validate live third-party services.

- At the previous checkpoint, the independent consumer and documented dependency
  pinned public commit `8a0b3fa422ee77d66f4d04519826854e47b112cf`, incorporating the later
  malformed-response validation fixes. Its clean container vendored the
  locked dependency and passed all 16 tests with networking disabled during
  the test build. A separate container tested the pinned client against the
  running native `qwen25-coder-7b-q4`: discovery, completion, streaming,
  JSON-object mode and token-limit handling passed. This validates one local
  model and does not establish live third-party provider compatibility.

- The public Git-pinned consumer at `ecad3179607859044ffeca6de2139f70e0cb9236`
  was tested at that checkpoint. Its clean vendored, network-disabled build
  passed 14 tests. Its
  separate container then used the native `qwen25-coder-7b-q4` Model Host for
  discovery, text completion, streamed completion, JSON-object mode and a
  token-limit error. The live probe now checks the requested answer and verifies
  that concatenated stream callbacks equal the final response. This is one local
  model and does not validate live third-party providers or general quality.

## Browser model discovery

- Authenticated MCP `aruvi_models.get_provider_transport_capabilities` and the
  browser provider card expose the shared client's text, streaming, discovery,
  transcription, speech and JSON-object transport flags. These describe the
  client adapter, not the actual endpoint or a particular model, and they do not
  contact the provider. Creating the selected transport still requires its normal
  configuration and credentials; disabled providers are rejected.
  The Linux Rust suite passed 204 tests with two live tests ignored. The 22
  remote UI cases passed with synthetic OpenAI-compatible and Anthropic
  configurations, comparing the MCP and browser response and checking that
  unauthenticated access is rejected. These are adapter checks, not live provider
  capability discovery.

- Model discovery now invalidates in-flight requests when the provider changes
  or configuration reloads. Only the current request may populate results,
  report an error or re-enable controls. All 22 isolated remote UI cases passed,
  including a deliberately delayed response across an A → B → A selection and a
  subsequent discovery against the simulated OpenAI-compatible provider. This
  tests browser request coordination, not every provider's discovery behavior.

## Browser work-item parity

- The browser work-item editor now covers the desktop editor's status field.
  When a client supplies status, the update compares both its original status
  and the other editable fields in one SQL statement; older clients that omit
  status retain it unchanged. Changing a status does not execute or approve a
  workflow. The 22 remote UI cases passed, including invalid status, concurrent
  MCP status changes, retained unsaved edits and older-client compatibility; the
  Linux Rust suite passed 204 tests with two live tests ignored.

## Browser file-aware Copilot

- Browser file Copilot can optionally stream questions and patch proposals,
  with a stop control. The streaming route shares file hash, context, provider
  and history validation with buffered requests and records the `mobile_ide`
  Activity source. Only completed streams enter conversation history or become
  reviewable proposals. The 22 remote UI cases passed, covering authentication,
  stale hashes, invalid history, successful streaming, failure, cancellation,
  and streamed patch review. A headless browser streamed and applied a simple
  proposal from real `qwen25-coder-7b-q4`; the Linux Rust suite passed 204 tests
  with two live tests ignored. MCP file Copilot remains buffered. The branch
  container was healthy on port 55030 after this validation.

- A disposable real Model Host test now asks `qwen25-coder-7b-q4` for a
  one-line JavaScript change through both the browser API and MCP, previews and
  applies each proposal through its corresponding API, and checks the result. A
  headless Chrome run also generates, reviews and applies that change in the UI.
  With `ARUVI_TEST_MULTI_FILE_PROPOSAL=1`, a real model generated an existing-file
  edit and a new text file, and both were previewed and applied through MCP.
  The first prompt produced an invalid array-shaped patch with incorrect hunk
  counts; an explicit JSON string example and hunk-count guidance fixed this
  fixture. These checks cover simple synthetic changes with one model and do not
  establish general model reliability. Repeat via `bash scripts/test
  host-file-copilot` with the validation host running; the browser test requires
  Playwright and Chrome on the Mac.

- Browser Copilot can request and review multi-file patch proposals, preview each
  patch against the repository, and apply the reviewed patches sequentially.
  MCP `run_file_copilot` supports `patch_mode`; callers can preview and use
  `apply_reviewed_repository_patch` with the reviewed hash and existence flag.
  File existence and hash are checked before each write. Application is not an
  atomic multi-file transaction and can race external filesystem writers; the
  browser reports how many files were applied if a later patch fails. Review
  validates the active file's original hash, while other files are based on
  their contents at preview time. Model output remains untrusted. The 22 remote UI cases passed with a simulated provider,
  covering review without writes, stale files, existence conflicts, partial and
  successful application, new-file creation and MCP generation; the Linux Rust
  suite passed 204 tests with two live tests ignored. That checkpoint used a
  simulated provider; see the real-model test above.

- Patch previews are shared by the authenticated browser API and MCP
  `preview_repository_patch`: they validate the target and patch without writing
  and return the existing-file flag, original and resulting hashes, and resulting
  text. Existing and resulting files are limited to 2 MiB. The preview is not a
  transaction: external changes may race a later write, and a hash alone cannot
  distinguish a missing file from an existing empty file; the reviewed apply
  action also requires the existence flag.
- The Linux Rust suite passed 204 tests with two live tests ignored; all 22
  isolated remote UI cases passed, including matching browser and MCP previews
  of an existing file and an MCP preview of a new file with no writes. The
  rebuilt worktree container is healthy on port 55020.

- The shared repository patch parser now validates both hunk ranges and line
  counts, rejects out-of-bounds or overlapping edits, preserves uniform CRLF and
  trailing spaces, and handles explicit no-final-newline markers. The existing
  SHA-256 precondition still applies; this does not make multi-file patching
  atomic or protect against non-cooperating external filesystem changes.
  At this earlier checkpoint, browser patch proposals remained open.
- The Linux Rust suite passed 203 tests with two live tests ignored, including
  malformed-count and position rejection, multiple hunks, newline handling,
  CRLF preservation and verification that rejected edits leave files unchanged.
  All 22 isolated remote UI cases passed, including an authenticated MCP test
  that rejects a malformed patch without changing the file and applies a valid
  insertion. The rebuilt worktree container is healthy on port 55019.

- Browser and MCP file Copilot now accept follow-ups with at most ten
  user/assistant pairs and 24,000 history characters. The browser keeps history
  only in the current page for the same saved file and provider/model, resets it
  after a save or explicit reset, and excludes failed turns. MCP callers supply
  and scope their own history. The server rejects invalid role ordering and
  oversized history before model access. At that checkpoint it was non-streaming.
- The 22 remote UI, 23 workflow and one simulated CI cases passed; the Linux Rust
  suite passed 198 tests with two live tests ignored. Browser checks cover model
  changes, reset, failed turns and oversized transcript handling. A disposable
  real `qwen25-coder-7b-q4` fixture passed web and MCP follow-ups. Other models
  and live third-party providers remain unverified.

- The workspace browser can ask questions about the selected
  saved file using an explicitly selected provider and model. The server confines
  the read to the registered repository, checks the saved file's SHA-256 against
  the opened version, and sends the question, relative path and at most 12,000
  file characters as untrusted context. Unsaved edits disable the action. The
  request and response are recorded in Activity; users must review the selected
  file and provider before sending. Prompt-injection resistance is not guaranteed.
- The 22 remote UI cases cover authenticated use, a changed-file conflict, path
  confinement, exclusion of an absolute path, context truncation, and telemetry;
  the simulated CI case and 23 workflow cases passed. The Linux Rust suite passed
  198 tests with two live tests ignored. A disposable real Model Host fixture
  identified a function using `qwen25-coder-7b-q4` through the new API. This is
  not a general model-quality or live third-party-provider guarantee.
- MCP `aruvi_models` action `run_file_copilot` uses the same file and model service
  with a distinct Activity source. Authenticated browser and MCP tests cover
  matching hashes, stale-file rejection and telemetry; a disposable real 7B
  Model Host fixture passed both interfaces. The 22 remote and 23 workflow cases
  passed, and the Linux Rust suite passed 198 tests with two live tests ignored.
  This was not full desktop IDE parity at that checkpoint.

## Browser planner node review

- Desktop and browser now use a shared Rust matcher for plan actions associated
  with the selected node, also exposed as MCP `match_planner_actions`. The
  browser shows the latest plan held by the current page; this is not durable
  history. Matching preserves the desktop's name-based rules, so duplicate node
  names can be conflated and results should not be treated as an audit record.
  The Linux matcher unit test, 90 frontend tests, five mocked desktop planner
  cases, and 22 remote UI cases passed. The remote suite compared authenticated
  web and MCP matches and checked the browser rendering of a draft edit.
- Suggested prompts now come from one Rust service used by the native desktop
  command, authenticated browser API and MCP `suggest_planner_prompts` action.
  Selecting a browser suggestion fills the composer without submitting it. The
  browser ignores responses for a previously selected node. Legacy product-area
  metadata is now classified before generic product metadata.
- The Rust suggestion test, 90 frontend unit tests, 22 remote UI cases and five
  mocked desktop planner cases passed. Browser and MCP responses were compared
  in an authenticated fixture, and the running branch MCP action returned the
  expected result. The desktop harness mocks the native command; actual native
  window interaction remains unverified.
- The browser displays the selected draft node's path, type, summary, source,
  confidence and evidence as text. A response explicitly clearing the selected
  node now clears both the browser state and persisted selection instead of
  retaining a stale node for a subsequent turn. The concurrency guidance in
  the UI reflects the existing cooperating-process lock limitations.
- The 22-case isolated remote UI suite passed, including safe rendering, cleared
  selection, and details from an authenticated draft edit. The rebuilt worktree
  started healthy. A disposable browser repository-analysis flow used the real
  `qwen25-coder-7b-q4` Model Host and confirmed its staged draft through MCP
  (one model call, no format retries). Other repositories and model behavior
  remain unverified.

## Mixed provider content validation

- The shared client rejects malformed or unsupported OpenAI-compatible content
  parts and multipart streaming deltas instead of silently returning incomplete
  text. Anthropic responses reject invalid or unsupported content blocks and
  deltas; documented thinking blocks remain omitted from text-only responses,
  and tool use remains unsupported. A stream may have delivered partial text
  before returning an error.
- All 41 shared-client tests and 23 isolated workflow cases passed, including
  simulated malformed responses through both web and MCP. The rebuilt worktree
  is healthy; real 7B Model Host browser streaming returned coding output. These
  tests do not establish live Anthropic or other third-party behavior.
- The independent consumer pins `ecad3179607859044ffeca6de2139f70e0cb9236`;
  its clean Docker build passed 14 tests with networking disabled during tests,
  including rejection of a mixed-content response through the public gateway.
- Anthropic's [Messages reference](https://platform.claude.com/docs/en/api/messages/create)
  describes the text, thinking, redacted-thinking and tool content blocks.

## Real coding workflow revalidation

- On the current branch, a real `qwen25-coder-7b-q4` JSON-mode extend-module
  workflow reached test review and passed its configured syntax check, but the
  independently sandboxed behavioral assertions failed because the generated
  module did not export the existing `sumEven` function. The fixture did not
  approve delivery. A second run of the same task preserved both exports, passed
  syntax and independent behavioral assertions, and delivered the verified file
  to the disposable Git remote. These two runs show model variability, not a
  general reliability guarantee.
- `--verify-behavior-rejection` substitutes deliberately invalid exports only
  for the independent sandbox review and verifies that review rejects them before
  approval and the disposable remote remains unchanged. A real 7B new-module
  run passed this negative control. A syntax-only configured test does not prove
  functional behavior; use meaningful tests and review before delivery.
- `npm audit` reported zero advisories for the current lockfile. Other previously
  recorded Rust advisory and live-provider limitations still apply.

- Desktop chat now offers Stop and cancels its shared-client future; timeout
  and navigation also request cancellation. Cancelled and failed exchanges
  show an incomplete marker and are excluded from subsequent model history.
  Completed exchanges remain in history. Active stream IDs cannot be reused;
  cancellation is recorded as a failed model call, consistent with web streams.
  A native HTTP fixture verified upstream disconnect, partial telemetry and
  duplicate-ID rejection. Six desktop-style browser cases covered completion,
  errors, Stop, timeout and navigation races with a mocked Tauri bridge. The
  native suite passed 219 tests with six ignored, all 102 frontend tests and
  23 browser/MCP cases passed, and the frontend build passed. A real coding
  model streamed through the web browser; real desktop-window cancellation
  remains unverified.

## Browser chat streaming

- A simulated Anthropic Messages endpoint now verifies browser streaming with
  API-key and request-shape checks, incremental output, token-limit failure and
  recorded input/output usage. All 22 isolated workflow cases passed; this does
  not establish behavior against a live Claude account. The current Linux Rust
  library suite passed 196 tests with two live tests ignored.
- After a Mac restart, the branch container and the dedicated native Model Host
  validation worktree were restarted. Studio's configured provider was refreshed
  to the host's new dynamic port, and the real 7B coding and token-limit MCP
  checks passed again. The helper's watch mode is still opt-in.
- Browser chat has an opt-in streaming mode with a Stop control. The authenticated
  server uses the shared provider client and records completed, failed and
  cancelled calls in the existing model-call ledger. Partial replies are visibly
  marked incomplete and excluded from history; the client requires an explicit
  terminal event. A bounded event queue fails a stream rather than dropping text
  if the browser cannot keep up. Closing the browser response cancels the upstream
  request. Ordinary browser chat and voice remain non-streaming.
- The isolated browser fixture covers authentication, incremental text, token-limit
  failure, cancellation observed at the simulated provider, and telemetry for all
  three outcomes. The browser smoke, 21 remote, one simulated CI and 21 isolated
  workflow cases passed. On the rebuilt worktree,
  `ARUVI_CODING_TEST_MODEL=qwen25-coder-7b-q4 bash scripts/test host-coding-ui --stream`
  returned real coding output through the browser. This is one model and prompt,
  not a quality guarantee or validation of live third-party services.

## Browser provider credential detachment

- New remote provider URLs are validated consistently before desktop or MCP
  credential storage and again at the shared persistence boundary. HTTP(S)
  URLs containing user-info, a query or a fragment are rejected without
  echoing their contents; local-runtime file paths remain supported. Existing
  invalid legacy URLs are not migrated automatically and can still be disabled
  or renamed through the shared repository path; replacing the URL or type
  requires a valid value. Secrets in URL paths cannot be detected by this
  validation and provider URLs remain visible in settings and telemetry.
  The native Studio suite passed 218 tests with six ignored; 27 isolated
  workflow cases passed, including web/MCP rejection and a rejected edit that
  left an existing credential usable. Real `qwen25-coder-7b-q4` coding and
  token-limit behavior also passed through MCP after the branch rebuild.

- MCP now exposes `get_provider_credential_state` and
  `detach_provider_credential` with the same revision check used by the browser.
  An isolated cross-interface test rotates through the web API, rejects a stale
  MCP detach, and confirms a current MCP detach prevents authentication. All 20
  isolated workflow cases passed; the rebuilt worktree server is healthy and
  advertises both actions through MCP discovery.
- Browser provider management can explicitly detach a key and disable legacy
  provider/global fallback keys for that provider. It retains stored credentials
  because references can be shared; this does not revoke keys. Setting a new key
  restores credential use. A random database revision changes on credential
  reference updates, so a stale browser cannot detach a concurrently rotated key.
  External changes to a secret's value at an unchanged reference are not detected.
- Migration 050 was applied to the existing branch worktree database; its server
  started healthy. The isolated workflow test checks authentication, stale writes,
  browser detachment and preservation of a shared credential. The Linux targeted
  fallback test passed, as did the browser smoke, 21 remote, one simulated CI and
  20 workflow cases. Real Model Host coding and token-limit checks passed via MCP.

## Speech response container validation

- The OpenAI-compatible client rejects WAV responses missing a format chunk or
  nonempty data, truncated chunks, and inconsistent declared RIFF lengths. It
  accepts common unknown-length streaming sentinels. This verifies container
  structure, not codec decodability or speech quality.
- Linux shared-client tests passed 39 cases. The worktree release image rebuilt
  and started healthy; real `piper-ljspeech` browser generation and the MCP
  Piper-to-`whisper-tiny-metal` speech round trip both passed. Physical audio
  and other providers and codecs remain unverified.
- The independent consumer is pinned to client revision
  `bb31ba263e86ab635397f57b5c18d1f06f5975e9`; its clean Docker build passed
  13 tests with networking disabled during the test step.

## Remote transcription input validation

- The shared OpenAI-compatible transport rejects an empty model name or empty
  audio before network access. It does not impose a provider-wide file-size or
  format policy. Web and MCP tests verify empty audio is reported as invalid;
  the real Piper-to-Whisper Model Host round trip still passes.
- The client suite passed 38 tests, the macOS Studio suite passed 195 with two
  live tests ignored, and the isolated workflow suite passed 20 cases. The
  worktree image rebuilt and started healthy. Running server SHA-256:
  `68b196271d7b6a3259cd1e5e55b1c39c38448a8da3b6b904d0b83f91d24f01c1`.
- OpenAI's [file transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text)
  deprecates `whisper-1` and recommends `gpt-transcribe` for new OpenAI usage.
  No generic provider default was changed; Model Host and other compatible
  services need their own supported model names. The independent consumer now
  pins `8a6b7d209a2af6555afb960f8e3d59d3fea227f9`; its network-isolated Docker
  build passed 13 tests, including empty-audio validation, and its separate
  Docker probe passed real 7B Model Host discovery, completion, streaming and
  token-limit checks.

## Parity inventory reconciliation

- Updated the leading findings in `web-parity-audit.md` to reflect the later
  real Model Host browser and MCP checks, repository analysis, hierarchy placement
  and two-process planner fixture. Older dated results remain historical evidence,
  not current deployment status. Native interaction, physical audio, third-party
  live services and real Aruvici integration remain open.

## Atomic planner conversation exchanges

- Local voice-command responses (including clear and draft selection), text
  clarifications and reports now persist the resulting session state and both
  conversation entries in one SQLite transaction. Injected assistant-entry
  failures verify that voice clear/selection and text clarification/report leave
  no partial exchange or session-state change. Model proposals use the same
  transaction helper. Input selection and draft seeding before a model call remain
  separate, and action side effects are not claimed to share this transaction.
- The macOS Rust suite passed 195 tests with two live tests ignored. The rebuilt
  worktree image passed the two-process planner fixture, a real 7B Model Host
  planner proposal and confirmation, and browser suites (one smoke, 21 remote,
  one CI, 20 workflow). Running server SHA-256:
  `7c10582cde217a627c85890b00a03f10d1bc57b79204a21b734bd8adaac55642`.

## Atomic model-driven planner proposals

- Text-turn proposals, confirmation-requiring plans and repository-analysis
  proposals now persist the pending plan, draft, selection and both conversation
  entries in one SQLite transaction. An injected assistant-entry failure rolls
  all of those writes back; a retry test verifies the complete proposal is saved.
  If applying model actions to the draft fails, the user entry is no longer saved
  alone. Selection and draft seeding before the model call, clarification/report
  paths, voice-only handling and immediate external actions remain separate steps;
  this is not a claim that every planner turn is crash-atomic.
- The macOS Rust suite passed 193 tests with two live tests ignored. The rebuilt
  worktree image passed the two-server planner fixture, browser suites (one smoke,
  21 remote, one CI, 20 workflow), and real 7B browser repository analysis and
  MCP confirmation. The live model made two calls with one format retry. Running
  server SHA-256:
  `98512849317912c4cd4d2b9b7876c18bdd194b69867916f6fbf3d1d916423937`.

## Atomic manual planner edits

- Rename, add and delete draft operations now save the draft, selection and
  pending-plan removal in one SQLite update. An injected database failure test
  verifies all three operations preserve the original plan, draft and selection.
  This covers manual draft edits; model-driven turns still have separate
  persistence steps and are not claimed to be fully crash-atomic.
- The macOS library suite passed 192 tests, with two live tests ignored. The
  rebuilt worktree image passed the two-server planner fixture, browser suites
  (one smoke, 21 remote, one CI, 20 workflow), and a real 7B Model Host planner
  proposal and confirmation through MCP. Running server SHA-256:
  `aa891322671708d66bbeec44d1a7184e79caaacb3ce6817a7616a67595ae5bd9`.

## Planner coordination across Unix processes

- Cooperating macOS and Linux Studio processes running as the same OS user and
  sharing a SQLite database now take a per-session advisory file lock before
  planner reads or mutations, then reload the session from SQLite. This prevents
  an old process-local snapshot from overwriting a completed edit. Locks are held
  through model turns, so other requests to that session wait; unrelated sessions
  remain independent. The lock directory is private and rejects symlinks and
  non-regular lock files. Cancellation and process termination release locks.
- `bash scripts/test planner-multiprocess` creates two disposable Linux servers
  sharing one volume, primes both caches, concurrently adds two draft areas,
  checks both views, and confirms once while ensuring the second server cannot
  duplicate the apply. The previous image failed this fixture; the new image
  passed it five times. The macOS suite passed 189 tests with two live tests
  ignored before the final lock-open hardening; subsequent macOS tests covered
  killed-process release and symlink rejection. Linux passed the two subprocess
  lock and killed-process tests. The final image passed browser suites (one smoke,
  21 remote, one CI and 20 workflow) and real 7B planner proposal/confirmation.
- Guarantees require a writable shared database directory, working filesystem
  advisory locks, the same OS identity, and all participants using this version.
  Direct database writers, mixed old/new Studio versions, and Windows processes do
  not participate. This does not make every multi-statement planner operation
  crash-atomic; the separately implemented design apply and clear operations are
  transactional. Running server SHA-256:
  `fe6df8dab7d555cf71cb7e6a6df934d7860103a40a56751a798971cc3470318f`.

## Planner sessions after service restart

- Updating model settings or clearing a persisted planner session now loads it
  when the current service has no cached copy. Clearing pending and draft state
  uses one database update, and both operations update the in-memory cache only
  after the database write succeeds. Tests use a fresh service instance and
  injected write failures to verify restart recovery and preservation of both
  pending plan and draft. This does not solve concurrent edits from separate
  server processes, which can still hold stale cached snapshots.
- The macOS Rust suite passed 184 tests with two live tests ignored; the enhanced
  failed-clear case passed separately. The rebuilt worktree image passed browser
  suites (one smoke, 21 remote, one CI, 20 workflow) and a real 7B Model Host
  planner proposal and confirmation through MCP. Running server SHA-256:
  `808f24dbb08a53e170e733542531fdb595bb8a30deedeb6498de1cc174f6bf00`.

## Streaming cancellation contract

- A transport-level regression test now holds an incomplete response open,
  receives a text delta, drops the client future and verifies that the server
  sees the HTTP connection close. It passed for both OpenAI-compatible and
  Anthropic transports on macOS and Linux; all 37 shared-client tests passed on
  macOS. This checks client-side connection cancellation, not whether a remote
  provider immediately stops its own inference or billing after disconnection.
- The change adds only test code and test dependency features. The independent
  consumer's previously verified runtime pin remains unchanged.

## Dependency audit revalidation

- On 2026-11-13, `npm audit` reported no advisories. `cargo-audit` against
  Studio's current lockfile reported `RUSTSEC-2023-0071` for `rsa` 0.9.10, plus
  13 unmaintained and six unsound warnings. `cargo tree --locked --offline
  --target all -i rsa` showed no selected dependency path for the current
  manifest features; this does not cover other feature combinations. The
  separately pinned consumer lockfile reported no vulnerabilities or warnings.
- The advisory database reported its last update as 2026-11-13 13:33:03 +01:00.
  Audit results do not assess the locally patched SQLx SQLite source or prove
  exploitability. The native desktop UI remains inaccessible to computer control;
  this is not evidence of desktop parity.

## Native local Whisper inference

- On 2026-11-13 Studio's local transcription service loaded the existing native
  `whisper-tiny-metal.bin` model read-only and transcribed a synthetic English WAV
  generated by Model Host Piper. The recognized text contained the fixture's
  `fox` and `dog` anchors. This exercised base64 decoding, WAV parsing,
  resampling and actual local Whisper inference on macOS; it did not exercise the
  desktop window, a microphone, or large recordings.
- The opt-in Rust test `local_whisper_transcribes_synthetic_fixture` is ignored
  by default. Set `ARUVI_LOCAL_WHISPER_MODEL` to an installed compatible model
  file and `ARUVI_LOCAL_WHISPER_AUDIO` to a synthetic WAV of “The quick brown
  fox jumps over the lazy dog”, then run it with `cargo test --manifest-path
  src-tauri/Cargo.toml --lib local_whisper_transcribes_synthetic_fixture --
  --ignored`. Local model and fixture files are not included in this repository.
  Recognition is nondeterministic and this fixture does not establish accuracy
  for other voices, languages or environments.

## Local Whisper input-size bounds

- Local Whisper now rejects base64 audio over a decoded 32 MiB limit, checking
  encoded length before allocating decoded audio and checking decoded length
  again for base64 padding edge cases. WAV parsing now borrows the decoded bytes
  rather than copying the full file. The existing ten-minute duration limit also
  applies. Remote provider upload limits are unchanged.
- The size boundary, invalid base64 and existing WAV checks passed in the macOS
  Rust suite: 181 tests passed, one live test ignored. The worktree image rebuilt
  and started healthy; real remote Piper-to-Whisper MCP speech still passed.
  Actual local Whisper inference and unusually large recordings were not run.
  Running server SHA-256:
  `e241bb7da1b044073a0bef92203a6a646496109ed977c91587178f5569c6b9e8`.

## Local Whisper WAV bounds

- Local Whisper rejects zero sample rates and recordings longer than ten minutes
  before resampling. This prevents a low-rate WAV from expanding into an
  unbounded 16 kHz allocation. The limit is specific to Studio's local Whisper
  path and is an intentional change for longer recordings; remote transcription
  continues to follow its provider's limits.
- Crafted WAV and resampling tests cover invalid rates, excessive duration and
  normal 8/16 kHz input. The macOS Rust suite passed 180 tests with one live test
  ignored. The rebuilt worktree container remained healthy and passed the real
  remote Piper-to-Whisper MCP round trip. A local Whisper model was not loaded for
  these crafted-file checks. Running server SHA-256:
  `59ff6ee9727e5039544ba3ccc8ffd6c653ffe4f42189c83e2c7359fe8c9cfd15`.

## Planner draft-action repair

- Repository analysis now also validates proposed actions against a scoped,
  annotated draft copy and can request correction or clarification. Its
  existing three-model-call limit includes both format and action retries.
  The simulated MCP fixture verifies a successful correction, three-call
  exhaustion, unchanged catalog before confirmation and confirmed persistence.
  The native suite passed 217 tests with six ignored, and all 25 isolated
  workflow cases passed. A real `qwen25-coder-7b-q4` browser repository
  analysis created and confirmed a draft on its first response; it did not
  exercise action repair. Provider and model output remain variable.

- Model-generated draft actions are now checked on a clone of the selected
  session draft, after the same product scoping used for application. When
  validation fails, the model receives the error and may return a corrected
  proposal or ask for clarification. At most two action repairs are allowed
  within the existing six-step limit. Nothing is persisted or executed by
  the validation pass; repeated failures return an error. This checkpoint
  initially covered text planner turns; repository analysis was added later.
- A simulated provider test verifies correction, bounded repeated failures and
  unchanged draft state. The isolated MCP workflow fixture verifies a repaired
  proposal, no catalog mutation before confirmation, and persistence after
  confirmation. Studio passed 217 native tests with six ignored, 23 browser/MCP
  cases and 24 isolated workflow cases. A real `qwen25-coder-7b-q4` planner
  turn produced and confirmed the requested draft after one format retry; it
  did not exercise the new action-repair retry in that run. Model behavior
  remains variable and the earlier failed attempt remains historical evidence.

## Disabled agent model bindings

- An explicitly bound agent model is now checked for both model and provider
  enablement before the agent run starts. Disabled bindings fail with a clear
  validation error rather than silently selecting another shared model. The
  existing first-priority binding policy remains unchanged. A model disabled
  after it was selected for a running stage is not rechecked between calls.
  A database test covers disabled model, disabled provider, recovery and the
  presence of an enabled fallback; 216 native Studio tests passed with six
  ignored. The isolated workflow suite passed 23 cases, including a failed
  MCP workflow start that makes no provider request followed by a successful
  workflow after re-enabling the model.
- The real-model coding, planner and file-Copilot fixture scripts now supply
  the private PDF-socket volume required by the current test Compose file,
  check for name collisions and clean up their own volumes. These changes do
  not configure a PDF worker for fixtures that do not need PDF export.
  The disposable coding workflow reached plan and test review with the real
  `qwen25-coder-7b-q4`, passed its syntax gate and separate network-disabled
  generated-code assertions for the new-module JSON-mode scenario. This is
  one model and task, not a general model-quality guarantee.
  The file-Copilot fixture also passed real web and MCP patch proposals and
  follow-up questions. The real planner fixture started, but its text-turn
  test failed after a format retry: the model requested a capability under a
  product area that was not present, and Studio rejected the draft mutation.
  Planner recovery for this case remains open; this failure does not establish
  a general conclusion about other models or prompts.

## Disabled shared model providers

- Studio now rejects disabled OpenAI-compatible and Anthropic providers at
  shared gateway creation, before credentials are resolved or a request is
  made. This covers desktop, MCP, planner, agent and web paths that construct
  that gateway. Connectivity testing through the remote gateway also requires
  the provider to be enabled; existing local-runtime connectivity inspection
  remains separate. Disabling does not cancel in-flight requests or prevent a
  request that already loaded an enabled provider from continuing.
  Native Studio passed 215 tests with six ignored. The isolated workflow suite
  passed 23 cases, including disabled web/MCP transcription, disabled MCP chat
  and successful MCP chat after re-enabling the provider. On the rebuilt
  branch, real `qwen25-coder-7b-q4` coding and a token-limit error passed
  through MCP. This does not prove native window interaction or live third-party
  provider behavior.

## Disabled transcription providers

- Transcription now rejects disabled providers at the shared Studio service
  boundary before decoding audio, contacting a remote endpoint or loading a local
  Whisper model. Desktop, web and MCP use this boundary; enabled providers are
  unaffected. A Rust test covers both provider types and an isolated workflow
  test covers rejection through the web API and MCP.
- The macOS Rust suite passed 178 tests with one live test ignored; the isolated
  workflow suite passed 20 tests. The rebuilt worktree container passed the real
  MCP Piper-to-Whisper speech round trip and remained healthy. Its server SHA-256
  is `9760104441b96b115529d2826bfc7971ef509dbf0b3eb71f320787a94d15b0c7`.

## Real speech and browser coding checkpoint

- On 2026-11-13 the standard worktree image generated real `piper-ljspeech`
  WAV audio in the browser and returned real `qwen25-coder-7b-q4` coding output
  in authenticated browser chat. `bash scripts/test host-speech-cycle` now
  exercises a synthetic English phrase through MCP speech generation and
  `whisper-tiny-metal` transcription, validates WAV structure, and checks the
  recognized words with a 70% sequence-similarity threshold and required "fox"
  and "dog" anchors. Provider and model names can be overridden with the
  `ARUVI_SPEECH_TEST_PROVIDER`, `ARUVI_SPEECH_TEST_VOICE_MODEL`, and
  `ARUVI_SPEECH_TEST_STT_MODEL` environment variables.
- The real speech round trip passed for the test sentence. A separate phrase
  containing the product name was misheard, so this is a connectivity and fixture
  check, not a claim of general transcription quality. Exact matching failed in
  repeated runs; the thresholded test passed five consecutive runs. Physical microphone input,
  audible playback, other languages, and native desktop interaction remain open.

## Standard Docker and Linux revalidation

- On 2026-11-13 Docker's shared disk became available again (46 GiB free after
  these checks). The standard `bash scripts/build worktree` built source revision
  `c20e1bcfa8f5951b604ac6914cdcbf4b7713695e`. Its worktree image
  `sha256:d60ac3e0a2c4cd5236265a0f8367c4ed11129866cb10832d35e15f0ac4d7ffde`
  was started healthy, retaining the existing volume. The prior validated image
  was retained as a rollback tag. Running server SHA-256:
  `82a02ef1ab1b2ecdb4d25cb8b0a42ae7e7d0a116ffa956d9ce6464b63b77c5bd`.
- The Linux library suite passed 177 tests with one live test ignored. Against
  the standard image, browser tests passed one smoke, 21 remote, one CI and 19
  workflow cases; a real 7B Model Host coding completion and token-limit check
  also passed through Studio MCP.
- The independently pinned consumer passed all 12 tests in the repository's
  normal clean Docker build with vendoring and network-disabled tests. A separate
  probe built from that consumer contacted the native 7B Model Host from Docker
  and passed discovery, completion, streaming and token-limit handling. Physical
  desktop interaction and the other previously recorded limitations remain open.

## Current worktree deployment checkpoint

- On 2026-11-13 the frontend and locked Linux release server were built from
  `feac4c22f063df36f318b7e05504c9be190e937b`. Because Docker's shared virtual
  disk was nearly full, the Linux toolchain and Cargo artifacts were mounted from
  the Mac; the resulting binary was packaged into the existing worktree runtime
  image. The binary SHA-256 is
  `668e4eac48f52f5e0c6dc6e103e947d2787d9a021e2e8301020095ddbef52f5a`
  and matched the running container. The previous image was retained for rollback.
  The worktree container is healthy; its loopback port is dynamically assigned.
- The macOS Rust suite passed 177 tests with one live test ignored using the
  default stack. Browser tests passed one smoke, 21 remote, one CI and 19 workflow
  cases. Through Studio MCP, the real `qwen25-coder-7b-q4` model passed coding
  completion and token-limit handling and staged and confirmed a planner design
  (two model calls, one format retry). Model-generated behavior is nondeterministic.
- The Docker disk still has less than 1 GiB available. This was a worktree-only
  build and deployment; staging, production, physical desktop interaction and
  the independent consumer's Docker build were not revalidated here.

## Buffered SSE compatibility checkpoint

- Buffered OpenAI-compatible completions now share the bounded SSE parser used
  by live streams, including BOM and CR-only line support. The buffered response
  retains its 8 MiB HTTP limit and compatibility with an omitted final blank
  separator; a `[DONE]` marker is still required. The 36 shared-client tests
  passed, including an HTTP response exercising the buffered fallback.
- The independent consumer and documented Git dependency pin now target
  `dced95c2204580d79ee1eb627aa94f938a64eae3`. Its 12 native, offline tests passed,
  including a CR-delimited buffered HTTP fixture. Running the pinned consumer on
  macOS against the native `qwen25-coder-7b-q4` Model Host passed discovery,
  completion, streaming (two chunks), and token-limit checks. The independent
  Docker build remains unverified for this revision because Docker is nearly full.

## Streaming parser checkpoint

- The shared streaming client now processes SSE bytes in linear time across
  network chunks, accepts CR, LF and CRLF line endings, strips an initial UTF-8
  BOM, and handles empty `data` fields. The 2 MiB total stream limit and explicit
  provider terminal-event requirement remain. See the [SSE parsing standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream).
- On 2026-11-13 all 35 standalone shared-client tests passed, including an HTTP
  fixture with a BOM and CR-delimited events and parser tests with one-byte
  fragments and an oversized stream. Three Studio desktop stream-adapter tests
  also passed. This has not yet been rebuilt in the
  worktree container or independently re-pinned and tested by the sample consumer;
  Docker's shared virtual disk remains nearly full. The buffered completion SSE
  fallback was subsequently unified as noted above.

## Progress — completion response metadata

- The shared client preserves provider response ID, model, native finish/stop
  reason and usage in successful ordinary and streamed completions. Anthropic
  streaming usage is merged across events. Metadata is optional when absent;
  incomplete and failed responses still return errors. Desktop stream completion
  events expose the metadata without changing telemetry storage. Existing JSON
  without metadata remains compatible, while Rust struct literals require the
  new field or `..Default::default()`.
- The client suite passed 28 tests, including OpenAI-compatible and Anthropic
  metadata fixtures, fragmented streams and legacy JSON. Both native desktop
  stream adapter tests passed with metadata and unchanged usage telemetry, and
  the frontend production build passed. These are simulated-provider checks;
  the updated branch container was subsequently rebuilt and validated below.
- The independent consumer now pins `bf759c5a69c693252306c7946bd5d5f35738d9e8`;
  its eight network-isolated tests passed, including a public-API metadata
  assertion. A previously hanging oversized-response fixture now has a bounded
  server write timeout on macOS. The pinned client also passed real discovery,
  completion, streaming and token-limit checks against the isolated
  `qwen25-coder-7b-q4` host.
- The branch release server was rebuilt and its running binary hash verified.
  Linux Studio tests passed 151 with one live-provider test ignored; UI checks
  passed 1 smoke, 20 remote, 1 CI and 19 isolated workflow cases. Real MCP
  coding and token-limit checks, browser coding chat and generated voice audio
  passed against Model Host. Docker storage filled during the rebuild; removing
  only identified Aruvi consumer-test cache restored space and the branch
  container is healthy again on its Docker-assigned port.

## Progress — model request timeout configuration

- Studio supports `ARUVI_MODEL_CONNECT_TIMEOUT_SECS` and
  `ARUVI_MODEL_REQUEST_TIMEOUT_SECS` for its shared gateway, with unchanged
  15-second and 300-second defaults. The settings cover chat, discovery, agents,
  streaming and remote voice for both supported protocols; they are service-wide,
  not per-provider. Invalid, zero, inconsistent and over-24-hour settings are
  rejected. Compose propagates explicit values across its three configurations.
- Three native gateway tests passed, including a stalled HTTP response with a
  one-second timeout. Both native streaming adapter tests still passed, and a
  Compose configuration check verified custom value propagation in all three
  environment files. The locked Linux release server was rebuilt, its deployed
  binary hash matched, and the worktree container was healthy. Its defaults were
  present in the process environment; real 7B MCP completion and token-limit,
  authenticated browser chat, and browser voice generation passed. Staging and
  production deployments were not changed or exercised.

## Progress — Anthropic server tool blocks

- Anthropic messages can contain server-side tool calls and tool-result blocks
  even in a completed turn. The text-only adapter now rejects these blocks in
  ordinary and streamed responses instead of silently discarding them; a stream
  may already have delivered partial text. The shared client passed 29 tests,
  including fixtures for client and server tool calls and result blocks. See
  [Anthropic streaming](https://platform.claude.com/docs/en/build-with-claude/streaming)
  and [server tools](https://platform.claude.com/docs/en/agents-and-tools/tool-use/server-tools).
  The independent consumer now pins `78095c0173cd7f8b58fba21db96bc0f484ca0616`
  and passed nine offline native tests, including a completed server-tool turn.
  A native desktop stream adapter test verified an error event and failed
  telemetry after partial text; both existing stream success tests also passed.
  The locked Linux server was rebuilt and its deployed binary hash verified.
  The healthy branch passed real 7B MCP coding and token-limit checks and
  authenticated browser chat.

## Progress — real model workflow revalidation

- The independent consumer pinned at `78095c0173cd7f8b58fba21db96bc0f484ca0616`
  passed real native Model Host discovery, completion, streaming and token-limit
  handling with `qwen25-coder-7b-q4`. The probe used the private token over stdin;
  this run was native rather than the non-root container probe.
- Against the rebuilt branch, a disposable product-to-code-to-Git workflow
  passed with the 7B model: the generated JavaScript passed isolated assertions
  and the same file was present in the disposable Git remote. A preceding run
  generated the requested file but failed before test review due to a truncated
  model response. The original failure lacked enough ledger detail to attribute
  its stage or token budget. The fixture now prints safe stage, status and token
  budget diagnostics on that failure path, without prompts or credentials. One
  pass and one failure are not a general reliability estimate.

## Progress — coding tool feedback and native verification

- The isolated host had both `qwen25-coder-7b-q4` and `qwen25-coder-15b-q4`
  workers ready. Two 15B new-module workflows before the change and one after it
  failed to produce a valid completed coding tool loop within eight steps. The
  traces showed malformed or multiple JSON responses, repeated reads of a missing
  file and false completion claims. These runs did not establish 15B reliability.
- The tool prompt had shown mutually exclusive read ranges in one example. It now
  gives alternatives and explains how to handle missing files. Workspace reads
  use repository-confined path checks and return a relative missing-file message,
  without exposing the internal absolute workspace path. Symlinked and hard-linked
  files are rejected; as with the existing confinement checks, this is not a lock
  against concurrent filesystem changes. Existing model tool permissions remain
  enforced. A 7B workflow after the change generated code that passed isolated
  assertions and reached the disposable Git remote, using forced finalization
  after the eighth tool step. This single pass does not prove model reliability.
- The full native Rust suite initially had one CI socket test fail because macOS
  temporary paths exceeded the Unix socket length limit. A shorter test-only name
  fixed it; the complete rerun passed 158 tests with one live test ignored. The
  rebuilt worktree server passed the remote UI smoke, 20 remote, one CI and 19
  isolated workflow tests. The running binary hash matched the release build.

## Real coding workflow test-gate checkpoint

- The disposable real-model workflow now configures `node --check src/math.js`
  as its in-workflow test instead of checking only whether the file exists. It
  copies the Node executable from the existing Node fixture image to an
  executable temporary path in the disposable container; syntax checking does
  not execute generated JavaScript. Separate behavior assertions still execute
  the generated code in a network-disabled container without the Studio fixture
  credentials. The script checks that the workflow test artifact reports success
  before it performs those assertions and approves delivery.
- A real 7B JSON-mode workflow passed syntax checking and independent behavioral
  assertions, then pushed the verified file to its disposable remote. With
  `--verify-gate-rejects-invalid`, a separate 7B run fed a deterministic invalid
  JavaScript file to the workflow test gate; the test artifact reported failure
  and the script verified that the generated file was absent from the remote.
  A 15B run exhausted its coding steps before reaching the test gate. This is
  a fixture-specific JavaScript syntax check, not a general guarantee that
  generated code is correct or safe.

## Coding model repair feedback checkpoint

- When a single coding response fails the tool/final schema, Studio now includes
  a bounded, JSON-quoted excerpt in the next observation and, only when writing
  is permitted, tells the model how to submit an intended file using the write
  tool. It does not execute the malformed response. Ambiguous multiple-action
  responses remain rejected without feeding back their content. Focused tests
  cover the bound, tool permissions, ambiguous output and no accidental write.
- A real 15B JSON-mode workflow with 7B on other stages previously exhausted
  its steps after missing reads and malformed finals. With the feedback it
  reached test review and wrote a file, but the generated JavaScript contained
  literal newline escapes and failed isolated execution. The fixture was stopped
  before approval or delivery. This is an observed failure, not evidence of 15B
  reliability; in that run the fixture's in-workflow command only checked file
  existence, so its later executable assertions provided the code-quality check. A 7B
  JSON-mode control run generated executable code, passed assertions, and
  delivered it to the disposable Git remote.
- macOS and Linux suites each passed 171 Rust tests (one live test ignored);
  browser suites passed one smoke, 21 remote, one CI and 19 workflow cases.

## Progress — ambiguous coding actions

- The coding response parser now rejects multiple complete action-shaped JSON
  objects rather than accepting the first fenced action. It gives the model a
  retry instruction and skips the legacy-file fallback for that response. A
  fixture verifies that neither the first write nor a legacy file block is
  executed, then successfully continues with a later valid action. Single-action
  responses, nested JSON, quoted braces and ordinary legacy code blocks have
  regression coverage. This guard does not repair arbitrary malformed JSON.
- Focused native parser and workflow tests passed; the Linux suite passed 161
  tests with one live test ignored. The branch browser suites passed one smoke,
  20 remote, one CI and 19 isolated workflow cases. The release binary hash
  matched the deployed worktree server. A real 7B workflow generated code that
  passed isolated assertions and reached a disposable Git remote. A 15B retry
  still failed after repeating missing-file reads and producing malformed final
  responses; the model's tool-loop reliability remains unresolved.

## Progress — coding recovery and JSON-mode investigation

- The coding prompt now sorts the allowed tool list, distinguishes final-step
  guidance when no edits exist, and gives bounded, permission-aware feedback for
  missing-file reads. Focused tests passed on macOS; the Linux suite passed 163
  with one live test ignored. The rebuilt branch passed one remote smoke, 20
  remote, one CI and 19 isolated workflow cases. A 7B coding workflow generated
  code that passed isolated assertions and was delivered to a disposable Git
  remote. A 15B run still exhausted eight tool steps with repeated reads and
  malformed output; this does not establish reliable model behavior.
- A direct synthetic request to the isolated 15B Model Host using
  `response_format: {"type":"json_object"}` returned a parseable JSON object and
  a successful finish. This is a single protocol probe, not proof of schema
  adherence or full workflow support. At that checkpoint, OpenAI-compatible
  JSON mode was not yet exposed by the shared client or Studio. JSON validity
  alone cannot ensure safe or correct tool actions.

## Progress — opt-in coding JSON-object mode

- The shared gateway offers a separate `run_json_completion` operation. The
  OpenAI-compatible adapter sends `response_format: {"type":"json_object"}`
  and rejects responses that are not JSON objects. Ordinary completions remain
  unchanged. The default trait implementation, including Anthropic, reports an
  explicit unsupported operation; incompatible remote models may reject the
  parameter. JSON syntax is not action-schema or permission validation.
- Studio permits `json_object_output: true` in a coding agent's execution
  boundaries. It applies only to tool-loop and forced-final coding calls and
  defaults off; it does not change planning, ordinary chat or other agents.
  Both desktop and remote agent editors display the option. The simulated
  end-to-end provider test verified the actual coding request format and that
  non-coding calls did not receive it. The client suite passed 30 tests, the
  macOS and Linux Studio suites each passed 163 with one live test ignored,
  the frontend built, and the browser suites passed one smoke, 20 remote, one
  CI and 19 isolated workflow cases.
- With JSON mode enabled, a real 7B workflow generated code that passed isolated
  assertions and reached the disposable Git remote. A 15B coding run used 7B
  for other stages; its JSON responses still repeated reads until the step
  limit. Two additional all-15B attempts failed during planning with token-limit
  errors before JSON mode applied. These examples do not prove general model
  quality. The branch server's binary hash matched the locked release build.
  The independent consumer pins `20e92bd275d5ac4beb5999ab4ed56acf3313c0ba`;
  ten tests passed offline on macOS and with networking disabled in Linux. Its
  separate dependency audit found no vulnerabilities or warnings. A non-root,
  read-only container built offline from that pin successfully invoked JSON mode,
  discovery, completion, streaming and token-limit handling on the real 15B
  host. This confirms transport behavior, not reliable tool-loop behavior.

## Image capability assessment

- The deployed Model Host advertises an image model (`sd-v1-5-metal`) and
  documents `POST /v1/images/generations`, which returns PNG data as base64 and
  supports host-specific size, step and seed controls. The isolated coding host
  currently advertises only its two text models. These are discovery results,
  not an image-generation test by Studio.
- No Studio image-generation UI, MCP action or existing desktop image workflow
  was found, and the shared Rust client currently has no image operation. Thus
  image generation is a new capability rather than a demonstrated desktop
  parity regression. Adding it requires its own API and security/size limits;
  do not assume Model Host controls or response fields apply to all providers.
  For example, OpenAI's current image API returns base64 for GPT image models
  and does not accept the legacy `response_format` parameter for those models.
  See [Model Host image contract](https://github.com/vivasaayi/aruvi-model-host/blob/main/docs/images.md) and
  [OpenAI image generation](https://developers.openai.com/api/reference/resources/images/methods/generate).

## Initial evidence — 2026-11-13

- Native model-host staging is healthy at `127.0.0.1:20680`, revision
  `583b730d85853b361585959a35d6ec3deb94ab59`; its separately deployed Docker
  worktree is healthy on a dynamic port. Neither is production acceptance.
- Both expose authenticated `/v1/models`, `/v1/chat/completions`,
  `/v1/audio/transcriptions` and `/v1/audio/speech`. The Studio container reached
  native staging at `host.docker.internal:20680` and received HTTP 200 for an
  authenticated model-list request. No token was printed or copied to Git.
- Native staging has installed `smol-metal-test` (text, 135M test fixture),
  `whisper-tiny-metal` (transcription), `piper-ljspeech` (speech), and an image
  fixture. These do not establish that a suitable local coding model is installed.
- A real native text request succeeded, but the small test model gave an
  irrelevant answer to a greeting. Studio's worktree now has a separately named
  native-staging provider and `smol-metal-test` model definition; its MCP provider
  connectivity check and an MCP text completion (32 output tokens) succeeded.
  This uses the existing Studio gateway, not the planned shared client. The test
  model has not been assigned to agents. The model-host bearer token has broad
  service authority, so credential scope and rotation still need design.
- Studio currently has an application-specific OpenAI-compatible Rust gateway,
  separate remote transcription implementation and local Whisper/native speech
  fallbacks. Its provider enum does not currently implement native Claude HTTP.
  The existing default LM Studio endpoint was unreachable from the container.
- Existing Studio MCP and UI tests cover simulated model workflows. They are a
  baseline, not evidence of shared-library integration or real model inference.

## Progress — shared client extraction

- A standalone crate at `crates/aruvi-model-client` now owns the existing
  OpenAI-compatible completion client and data types; Studio adapts its error
  type and retains its existing credential and telemetry handling. The crate's
  current location is provisional, not the final distribution decision.
- The crate's two independent tests passed, covering an authenticated local HTTP
  completion request, token usage parsing, structured content parts and buffered SSE
  fallback. The worktree Docker image built using the locked dependencies.
- Studio's Rust library suite passed 147 tests with one live-provider test ignored.
  The branch UI suite passed 19 remote UI tests, one simulated CI test, and 18
  isolated workflow tests; simulated providers do not verify external services.
- After deployment of this extraction, Studio MCP invoked the real native model
  host and received a non-empty response and token counts using `smol-metal-test`.
  This validates transport, not coding quality or all provider capabilities.
- Remote multipart transcription now uses the shared client while local Whisper
  retains its existing path. An authenticated MCP request through Studio to the
  native `whisper-tiny-metal` model correctly transcribed a synthetic English WAV.
  Four client tests and 18 isolated workflow tests (including simulated browser
  transcription) passed. Physical microphone and other languages remain untested.
- The shared client now generates size-limited WAV speech, exposed through Studio's
  mobile API, browser Voice controls and `aruvi_speech` MCP action. A real native
  `piper-ljspeech` request returned WAV through MCP; Chrome generated and loaded
  audio using the same provider. English fixture playback quality, physical sound
  output and other languages remain unverified. `bash scripts/test host-speech`
  repeats the browser check against the configured native-staging fixture. Six
  shared-client tests, 147 Studio Rust tests (one ignored), 19 remote UI tests,
  and 19 isolated workflow tests passed after this change.
- Native Anthropic is implemented in the shared crate for text, with simulated HTTP tests
  for request mapping, usage, unsupported roles and redirect credential protection.
  Studio now routes non-streaming MCP, web chat, planner, agent and desktop
  completion calls through a protocol-selected gateway. An additive protocol column
  preserves existing providers, models, secrets and agent bindings without rebuilding
  the tables; an upgrade test and 148 Studio Rust tests passed (one live test
  ignored). After upgrading the branch container, MCP connectivity and a real
  `smol-metal-test` completion still succeeded. MCP and the web provider form can
  create an Anthropic provider; a simulated Messages endpoint verified MCP and
  authenticated web chat, including Anthropic headers and token usage. A browser
  test created and removed an Anthropic provider; the current run passed 149 Rust
  tests (one ignored), 20 isolated remote UI tests, one CI test and 19 workflow
  tests. MCP also completed another real request to the native Model Host.
  Claude requires an explicitly configured key. The shared client now supports
  Anthropic text streaming with bounded responses, explicit stream errors,
  truncation detection, token usage and cancellation by dropping the future.
  Event handling follows the [Anthropic streaming reference](https://platform.claude.com/docs/en/build-with-claude/streaming).
  Simulated tests cover fragmented UTF-8 and SSE, error and tool-use events;
  a Tauri mock-runtime test verifies desktop stream events and telemetry. A
  browser mock test checks desktop Anthropic provider setup. OpenAI-compatible
  desktop streaming now uses the shared bounded event reader and Studio adapter.
  Simulated tests cover fragmented streams, usage, errors, truncation and the
  desktop event bridge. A separate container running the crate's probe received
  20 text chunks from the native Model Host's `smol-metal-test`. Physical desktop
  and live Claude tests, durable versioning, and broader voice
  validation remain open.
- The streaming extraction passed 13 client tests, 150 Studio Rust tests (one
  ignored), and a separate OpenAI desktop adapter test. After rebuilding the
  branch image, real MCP completion and 20 remote UI, one CI and 19 isolated
  workflow tests passed. These fixture tests do not replace a physical desktop run.
- The library is independently consumable from the public Studio Git repository
  using a full commit pin. The separate `validation/model-client-consumer` fixture
  pins `d2560cfe561e625a2658f233ac5e4feb4b8d2beb`, commits its dependency lock,
  vendors the dependency graph, and tests authenticated completion and discovery
  inside a network-disabled Docker build. `bash scripts/test shared-client-consumer`
  repeats this check. Cargo's package listing includes the applicable license;
  there is no registry release. A separate repository and release policy remain
  open decisions rather than prerequisites for consuming this pinned snapshot.
- Model discovery now uses the shared client for OpenAI-compatible providers
  and Anthropic, including bounded responses and explicit Anthropic pagination.
  Provider-specific metadata is preserved without assuming client capability.
  MCP, desktop setup and authenticated web setup expose discovery separately
  from model registration. Simulated provider tests cover authentication,
  pagination, malformed and oversized responses; desktop browser tests exercise
  selecting models across pages. Live MCP discovery returned the native host's
  `smol-metal-test`, `whisper-tiny-metal`, `piper-ljspeech` and `sd-v1-5-metal`
  fixtures without registering them. The client suite passed 17 tests; the
  Studio suite passed 151 (one ignored), plus targeted simulated Anthropic
  MCP/web pagination tests, 20 remote UI tests, one CI test and 19 workflow tests.
- An isolated native Model Host worktree now runs the pinned Qwen2.5-Coder 1.5B
  and 7B Q4 fixtures, with the 7B worker currently loaded. Studio's separate
  `Aruvi Model Host (Studio Coding Validation)` provider and model definitions
  use this host; no agent bindings were changed. The staging host remains on its
  original four ready workers. The 1.5B and 7B each generated a small JavaScript
  function through MCP that passed isolated assertions; authenticated browser
  chat also returned real coding output. The host's dynamic port changed on
  restart and `scripts/configure-native-model-host` refreshed Studio's provider
  without duplicating it. This refresh is manual, not automatic.
- A disposable real-model workflow exposed false success when malformed output
  was treated as a legacy file-block response, and when an agent claimed it had
  changed files without recording a write. Coding now retries both cases and
  accepts an explicit reasoned no-change result. Tests cover those paths, and
  missing-directory tool feedback explains that writing creates parents. The
  Initial 1.5B and 7B workflow runs produced no file, invalid JavaScript, or
  exhausted the tool-step limit. The
  independent sandbox assertions and meaningful fixture check caught those
  failures. Do not treat the short function check as workflow-quality approval.
  After these changes, 151 Rust tests passed (one ignored), along with 20 remote
  UI tests, one CI test and 19 simulated workflow tests. At that checkpoint, the
  real coding workflow acceptance check was still failing.
- The agent now receives a bounded preview of content it submitted for a
  successful edit, without acquiring extra repository read access. The 7B model
  subsequently completed three disposable product/work-item workflows and passed
  generated JavaScript assertions; the latest run also verified that the checked
  file reached its disposable Git remote. These runs exercised one small task,
  and the model still repeated directory checks or writes in some traces, so
  broader coding quality and reliability remain unproven. The workflow fixture
  checks file existence inside Studio and runs functional assertions in a separate
  network-isolated container before approving test review. This change passed 151
  Rust tests (one ignored), 20 remote UI tests, one CI test, 19 simulated workflow
  tests, and real MCP and browser chat checks.
- OpenAI-compatible completion and transcription responses now have explicit
  byte limits, including chunked transfers; provider error bodies and malformed
  response previews are not propagated. Redirects and base URLs containing
  credentials, queries or fragments are rejected, and HTTP client initialization
  does not silently fall back to an unbounded client. Tests cover oversized
  advertised and chunked responses, error-body redaction and redirect refusal.
  The independent consumer passed three tests from the pushed commit, including
  rejection of credential-bearing URLs. After rebuilding the branch image, 151
  Studio Rust tests passed (one ignored), the browser suites passed (20 remote,
  one CI and 19 workflow cases), and real MCP coding, authenticated browser chat,
  and browser speech generation passed. One further disposable 7B coding
  workflow passed generated-code assertions and delivery; this remains a narrow
  example rather than a general coding reliability assessment.

- OpenAI-compatible buffered SSE now requires a terminal event and rejects
  malformed or error events and tool calls instead of returning partial text.
  JSON completions likewise reject provider errors, missing choices or text,
  refusals and tool calls; live streams reject refusals and empty output. Empty
  tool-call arrays remain compatible. The shared-client suite passed 22 tests,
  including these cases. The independently pinned consumer passed four tests
  in its network-disabled build, including an incomplete buffered stream. After
  rebuilding Studio, real Model Host coding passed through MCP and browser
  chat; the simulated browser suite passed 20 remote UI, one CI and 19 workflow
  tests.

- Existing provider protocol can now be edited from desktop, web and MCP while
  preserving provider IDs, models and credentials. The web editor rejects stale
  protocol edits; invalid protocol/type combinations are rejected. Desktop and MCP
  credential updates now write a new reference before changing the provider, so a
  rejected update cannot replace the active credential; unused keys may remain.
  Studio's Rust suite passed 151 tests (one ignored), the focused mocked desktop
  provider tests passed, and the browser/MCP test switched a disposable provider's
  protocol and rejected a stale edit. The full browser suites passed (20 remote,
  one CI and 19 workflow cases), as did a real Model Host coding request through
  MCP. Native desktop and live Claude service behavior remain unverified.

- Both shared HTTP clients accept explicit connection and total request timeouts
  through `ClientOptions`, while the existing constructors retain their 15-second
  and five-minute defaults and redirect refusal. Invalid or unbounded settings
  are rejected. The shared-client suite passed 24 tests, including stalled
  response bodies for both provider protocols. An independent consumer passed
  five tests from the pinned revision in a network-disabled build. Studio rebuilt
  and passed real Model Host coding through MCP and browser speech generation;
  Studio now exposes service-wide timeouts; per-provider settings remain open.

- The native-host setup helper supports an opt-in foreground watcher, which
  retries outages and refreshes the same Studio provider when a worktree's port
  or token changes. The isolated native coding host was stopped and restarted;
  its port changed from 50165 to 57470, the watcher updated the provider, and a
  real 7B coding request then passed through Studio MCP and isolated JavaScript
  assertions. Two Python regression tests cover retry, change detection and
  secret-free logs. Native staging remained untouched. The watcher is not
  installed as a login service, and staging currently has no suitable coding
  model; this does not establish an always-on centralized coding deployment.

- Anthropic responses now reject reported truncation, paused turns, refusals,
  tool-use stops and unknown stop reasons in non-streaming and streaming paths.
  A stream can deliver text callbacks before a terminal error, so consumers must
  inspect its final result. For compatibility, absent stop reasons are accepted
  and cannot establish completeness. The shared-client suite passed 25 tests;
  a simulated Studio integration test verified MCP and authenticated web chat
  report truncation as an error. An independently pinned consumer passed six
  tests in its offline build, including the Anthropic truncation case. The rebuilt
  branch deployment passed a real Model Host coding request through MCP.
  Behavior follows the [Anthropic stop-reason reference](https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons).

- OpenAI-compatible clients now reject reported token truncation, content filtering,
  tool calls and unknown finish reasons for JSON, buffered SSE and live streams.
  Missing or null reasons remain accepted for compatible hosts. This follows the
  [OpenAI Chat Completion reference](https://developers.openai.com/api/reference/resources/chat).
  All 26 shared-client tests passed, including partial stream callbacks; the
  pinned independent consumer passed seven offline tests. After rebuilding the
  branch, the real 7B Model Host returned normal coding output and a one-token
  request produced a truncation error through Studio MCP. `bash scripts/test
  host-coding` now checks both behaviors.

- The shared `ModelGateway` now supports streaming, transcription and WAV
  speech through an object-safe interface, with explicit adapter capabilities
  and unsupported-operation errors. Adapter capabilities do not claim that an
  endpoint or model supports an operation. Both OpenAI-compatible and Anthropic
  streams and OpenAI-compatible voice are exercised through trait objects in
  the 27 passing client tests. Studio chat, desktop streams and remote speech now
  construct transports through the same provider/credential selection function.
  The full Studio suite passed 151 tests with one ignored on rerun; an initial run
  failed an unrelated external-CLI log assertion. Mock desktop tests verified
  stream events and usage for both protocols. After the branch rebuild, 20 remote
  UI, one CI and 19 workflow tests passed, as did real 7B coding and speech
  synthesis. A real Piper-to-Whisper MCP round trip returned a nonempty transcript
  but misheard “host” as “hopes”; this verifies the path, not recognition quality.
  The separate pinned consumer passed eight offline tests. The unused private
  Studio adapter has since been removed.

- The external-CLI session log append now flushes Tokio's pending file write
  before returning, addressing a plausible cause of the intermittent immediate-
  read failure. The regression test checks 40 immediate reads after append. The
  complete Studio Rust suite then passed 151 tests with one live test ignored,
  without the prior dead-code warnings. Flush does not claim durable disk sync.

- The independently pinned Git consumer now has an offline-built host probe
  using `Box<dyn ModelGateway>`. In a read-only non-root container it reached
  the isolated native `qwen25-coder-7b-q4` host and passed authenticated model
  discovery, completion, streaming and token-limit handling. The host token
  was provided through stdin, not command arguments or image layers.
  `bash scripts/test host-shared-client /path/to/host --model MODEL_ID` repeats
  this check. The eight simulated consumer tests also passed offline. This is a
  single small-model integration check, not proof of broader coding quality.

- A native macOS ARM64 debug binary was built from the branch with Rust 1.99.1
  and a separate temporary toolchain and application data profile. Its own
  authenticated MCP endpoint registered a disposable provider and returned a
  real `qwen25-coder-7b-q4` completion from the native Model Host. Real Piper
  synthesis and Whisper transcription also passed through that native process;
  the short transcript was approximate. Two native mocked Tauri stream-event
  tests passed, one per protocol. The Mac was locked during the attempted UI
  inspection, so actual desktop window interaction, physical audio devices and
  desktop feature parity remain unverified. The native test app, Vite server
  and private test data are temporary; the branch Docker deployment is separate.

- The isolated real-model workflow fixture now also tests editing an existing
  tracked JavaScript module. A `qwen25-coder-7b-q4` run preserved the original
  `sumEven` behavior, implemented and exported `clamp`, passed boundary and
  invalid-range assertions in a separate network-isolated container, and delivered
  the verified file to a disposable Git remote. This is one additional task, not
  a general reliability claim. `bash scripts/test host-coding-workflow --scenario
  extend-module` repeats it; set `ARUVI_CODING_TEST_MODEL` for the installed model.

## Progress — SQLite security backport

- A temporary, locally vendored copy of the published SQLx SQLite 0.8.6 driver
  changes only its `libsqlite3-sys` dependency to 0.39.1 (bundled SQLite 3.54.1),
  addressing the version affected by RUSTSEC-2026-0459. Provenance, upstream
  licenses and the maintenance/removal plan are in `vendor/sqlx-sqlite`.
  This dependency combination is not supported upstream; Windows and other
  untested targets require validation before release.
- Full Rust suites passed on macOS and Linux: 159 tests each, with one live test
  ignored. A runtime SQL query test verifies SQLite is at least 3.54.0. A fresh
  cargo-audit no longer reports the SQLite advisory; it reports one RSA advisory
  in the lockfile, 13 unmaintained, 10 unsound and three yanked warnings. Cargo
  audit does not independently review the locally patched SQLx source.
- The worktree data volume was stopped and backed up to a private, owner-only
  archive before upgrading. After deployment, the database passed integrity and
  foreign-key checks; provider, agent, product and work-item counts matched the
  backup. The binary hash matched the locked release build. The browser suites
  passed one remote smoke, 20 remote, one CI and 19 isolated workflow cases;
  real 7B MCP coding and browser speech checks passed. Existing deployments
  retain their previous SQLite until rebuilt; staging and production were not
  upgraded. The branch build used the existing host-mounted Linux toolchain and
  Docker runtime stage to accommodate Docker Desktop's limited free space.

## Progress — Git dependency security update

- Updated `git2` from 0.19.0 to 0.21.0 and preserved both HTTPS and SSH build
  features explicitly, since the newer crate disables them by default. Adapted
  reference, status-path and commit-summary handling to the changed APIs while
  retaining existing fallbacks. A runtime feature test passed on macOS and Linux;
  actual authenticated remote SSH and HTTPS pushes were not tested.
- `cargo-audit` no longer reports the three `git2` soundness advisories
  RUSTSEC-2026-0008, RUSTSEC-2026-0183 and RUSTSEC-2026-0184. It still reports
  RUSTSEC-2023-0071 for RSA in the lockfile, 13 unmaintained, seven unsound and
  three yanked warnings. `cargo tree --target all -i rsa` found no selected
  dependency path; disabling SQLx default features in an isolated experiment did
  not remove its optional RSA dependency from the lockfile. This does not prove
  safety under every possible feature configuration.
- Native Rust tests passed 159 with one ignored, plus the new Git transport
  feature test. Linux Rust tests passed 160 with one live test ignored. The
  rebuilt branch passed one remote smoke, 20 remote UI, one CI and 19 isolated
  workflow checks, including commit and push to a disposable local Git remote.
  Real 7B MCP completion and browser chat passed. A real 7B workflow generated
  code that passed isolated assertions and reached its disposable Git remote.
  These are limited checks, not a general model reliability claim. Windows and
  other untested target builds remain open.

## Progress — async runtime and yanked dependency updates

- Updated the locked Tokio version from 1.49.0 to 1.54.1, addressing
  [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html),
  and updated the yanked `async-executor`, `spin` and `uds_windows` releases to
  1.14.1, 0.9.9 and 1.2.1 respectively. The Tokio upgrade also updated its
  compatible macro, system and socket dependencies. The final audit reports no
  yanked packages, one RSA vulnerability in the lockfile, 13 unmaintained and six
  unsound warnings. The scanner does not review the local SQLx patch itself.
- The full macOS and Linux Rust suites each passed 160 tests with one live test
  ignored. The worktree release binary hash matched the deployed binary; the
  browser suites passed one remote smoke, 20 remote, one CI and 19 isolated
  workflow cases. Real 7B MCP coding and browser speech checks passed. Windows
  builds and `uds_windows` runtime behavior remain unverified.

## Dependency security checkpoint — 2026-11-13

- `npm audit --json` reported no npm advisories. `cargo-audit` 0.23.0
  against Studio's committed `src-tauri/Cargo.lock` reported 23 vulnerabilities
  and additional unmaintained/unsound warnings. The separately pinned consumer's
  lockfile reported no vulnerabilities. These scans describe dependency
  advisories, not confirmed exploitability in this application.
- The Studio lockfile update resolves 21 of those vulnerability reports by
  upgrading `h2`, `libz-sys`, `openssl`, `plist`, `quick-xml`, `rustls` and
  `rustls-webpki` within existing manifest constraints. A repeat audit reports two
  remaining vulnerabilities and 23 unmaintained/unsound warnings; do not treat
  the result as release-ready.
- [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
  concerns the bundled SQLite version when opening an attacker-controlled database
  and WAL pair. The previous Studio build reached `libsqlite3-sys` 0.30.1 through SQLx
  0.8.6; the advisory's fixed bundled version is 0.39.0 or later. Verify the
  actual database trust boundary and a compatible upstream upgrade or other
  mitigation before release. As checked on 2026-11-13, the latest published
  SQLx 0.9.0 still requires `libsqlite3-sys <0.38`, so that upgrade alone would
  not reach the fixed version. SQLx supports dynamic SQLite linkage, but that
  requires a patched system library on every target and has not been adopted or
  verified here. See [SQLx SQLite linkage](https://docs.rs/crate/sqlx-sqlite/0.8.6).
  A feature-tree check confirmed the previous build selected `libsqlite3-sys/bundled`,
  whose source declares SQLite 3.46.0. `bootstrap` opens `ARUVI_DB_PATH`, then
  the saved desktop override, then the application data directory, and enables
  WAL mode. In the running worktree container `/data` is owned by `aruvi`, mode
  0755, so other container users cannot write the database directory; Docker
  administrators and the owning service account still can. The isolated native
  test data directory is mode 0700. These observations do not establish the
  permissions of a user's desktop override or any other deployment, and they do
  did not fix the bundled SQLite vulnerability. Do not open untrusted database/WAL
  pairs; protect both the database directory and override configuration.
- [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html)
  reports an RSA timing issue. `rsa` 0.9.10 appears in the lockfile, but `cargo
  tree --target all -i rsa` printed no path for Studio's selected features. This
  is not a claim about every possible build configuration.
- Docker Desktop's shared filesystem had approximately 1.2 GiB free, so compiler
  artifacts were stored on the Mac filesystem using the same pinned Linux Rust
  toolchain and build dependencies. The first test run put temporary test data on
  the Mac-shared mount and failed an atomic-rename secrets test; with test data on
  Docker's native filesystem, 151 tests passed and one live test was ignored.
  The release binary built with `--locked`, its frontend assets matched the normal
  container build, and the matching binary was packaged with the existing runtime
  dependencies and deployed to the isolated worktree Compose project. The browser
  suites passed (20 remote, one CI, 19 workflow cases), as did real Model Host
  coding and browser speech checks. Native desktop and Windows dependency paths
  remain unverified. No other products' data were pruned.

## Complete staged packet details checkpoint

- Review packets now include every staged node's full details in the human
  readable change section and in `draftNodes` in the structured change set.
  These remain available when manual edits clear the latest pending model plan.
  Pending model actions are labeled separately. The packet explicitly says the
  draft is not a complete catalog snapshot and that missing nodes do not imply
  deletion; it does not claim to reconstruct an exact database diff.
- Tests cover manual edits, preservation of node details, HTML escaping and the
  warning against inferred deletion. The persisted catalog remains unchanged by
  export; approval is still a separate operation. macOS and Linux each passed
  174 Rust tests (one live test ignored), all browser suites passed, and a real
  7B browser repository-analysis and packet-download fixture passed.

## Shared desktop review packet checkpoint

- Active desktop planner sessions now request the same backend packet used by
  browser and MCP, then use the existing native file save/reveal flow. The
  request includes the visible draft and selected node; the service rejects a
  changed tree or missing selection instead of exporting a stale packet.
- The packet now has section links, readable action summaries, and a self-contained
  SVG hierarchy plus the full accessible tree. The diagram shows at most 100
  nodes, while the full tree remains in the document. The older frontend renderer
  is retained only for export without a session. Its Mermaid layout and historical
  action summaries are not byte-for-byte identical to the shared output.
- The frontend build, 90 frontend unit tests, four headless planner flows, and
  174 Rust tests each on macOS and Linux passed (one live test ignored). A real
  7B browser workflow generated and downloaded the packet before MCP confirmation.
  The native desktop UI remains unverified because the Mac was locked.

## Browser and MCP design-review packet checkpoint

- The Rust planner service can generate a standalone HTML review packet from a
  staged session. The authenticated browser downloads it, and MCP action
  `export_planner_review_packet` returns the filename and HTML. It includes the
  staged tree, bounded current catalog context, validation issues, pending
  actions, and review/approval sections. Export does not apply changes or write
  a server file. User-provided content is HTML-escaped; no external scripts are
  loaded. If manual edits have cleared the pending plan, the tree remains present
  and the packet explicitly notes that pending actions are unavailable.
- Desktop now uses the shared backend renderer for active sessions and retains
  native file/reveal behavior. A stale visible tree or missing selected node is
  rejected before export. The old frontend renderer remains as a no-session
  fallback. The shared renderer adds an offline SVG hierarchy, section navigation
  and readable action summaries, but it differs from the older Mermaid styling
  and cannot reconstruct pending action details after manual edits clear the plan.
  Native UI verification and complete removal of the fallback remain open. A real 7B browser fixture
  analyzed a disposable repository, downloaded the packet, and confirmed the
  staged design through MCP. Tests cover authorization, missing drafts, HTML
  escaping, MCP output, browser download and unchanged catalog before apply.
- macOS and Linux each passed 174 Rust tests (one live test ignored); the browser
  suites passed one smoke, 21 remote, one CI and 19 workflow cases. A real 7B
  coding and token-limit MCP check passed on the rebuilt branch.

## Shared planner validation checkpoint

- The Rust planner service now computes draft readiness, node counts, duplicate
  sibling warnings and empty-branch warnings using the existing desktop rules.
  `aruvi_planner` exposes `validate_planner_draft`, and the authenticated browser
  can request and display the result. The browser marks its result stale after
  its own draft changes; another client can change a session after validation.
- Tests cover empty and duplicate trees, authentication, persisted draft state,
  matching HTTP/MCP results and the browser interaction. The desktop frontend
  now requests the shared Rust validation command for its displayed tree, with
  its existing local calculation as a fallback while the command is pending or
  unavailable. Headless desktop-frontend tests cover both the command result
  and existing planner workflows; native UI verification remains open because
  the Mac was locked. Design-review packet export and other parity gaps remain
  open. The desktop command accepts legacy tree nodes without evidence and uses
  the same metadata fallback as the previous frontend calculation.
- For the desktop integration checkpoint, macOS and Linux each passed 173 Rust
  tests (one live test ignored); 90 frontend tests and four headless desktop
  planner flows passed. The rebuilt branch passed a real 7B coding MCP check.
- At the earlier web-validation checkpoint, macOS and Linux each passed 172 Rust tests (one live test ignored); browser
  suites passed one smoke, 21 remote, one CI and 19 workflow cases. The real 7B
  browser repository-analysis and MCP confirmation flow also passed.

## OpenAI-compatible request validation checkpoint

- Ordinary, JSON-object and streaming completion calls now consistently reject
  blank model IDs, empty message lists, non-positive explicit token limits and
  non-finite temperatures before network I/O. Tests use an unreachable endpoint
  to verify that each mode returns input errors rather than sending the request.
  Provider-specific finite temperature ranges remain provider-validated.
- The client suite passed 32 tests; Studio passed 171 tests each on macOS and
  Linux (one live test ignored). The independent Git consumer pinned to
  `b460a7f77ee2ed835d2ebc486504eeef71475889` passed 11 tests on macOS and in
  network-disabled Linux with vendored dependencies. The rebuilt branch passed
  a real 7B Model Host coding and token-limit check through MCP.

## JSON mode capability checkpoint

- The shared `ModelGateway` trait now exposes
  `supports_json_object_completion()`. It defaults to false for existing custom
  implementations and returns true for the OpenAI-compatible transport. This is
  transport support only; an endpoint or model can still reject JSON-object mode.
  Anthropic reports false and continues returning `UnsupportedOperation`.
- The crate's 31 tests passed. An independent Git consumer was repinned to
  `fcd94c6a2490689436da800da5c870cb0a7b64bd` and its 10 locked, offline macOS
  tests passed, including trait-object capability checks for both providers. The
  same independent consumer passed on Linux with vendored dependencies and
  networking disabled. Studio's Linux and macOS suites each passed 170 tests
  (one live test ignored). This snapshot remains a provisional Git pin, not a release. Real 7B Model Host
  coding MCP and browser repository-analysis/confirmation tests passed after
  rebuilding the branch image.

## Planner action validation checkpoint

- Planner responses reject unknown action types and reject a mixture of staged
  draft changes with workflow operations or status reports. Previously the draft
  application path could silently skip those other actions. Repository analysis
  additionally rejects action types outside its supported design operations.
  Bounded correction requests include the validation error, and no partial plan
  is staged from an invalid response. Tests cover mixed actions, supported
  report-tree combinations and model correction of unsupported actions.
- Validation: macOS and Linux each passed 170 Rust tests (one live test
  ignored), all browser suites passed, and the real 7B browser repository
  analysis and MCP confirmation fixture passed.

## Repository snapshot privacy checkpoint

- Repository analysis no longer sends the configured remote URL or absolute
  local path to the model, its request trace, or request snapshot. A simulated
  provider asserts that a URL containing credentials and a query token, and the
  local fixture path, do not appear in the actual request. Preferred files are
  selected from the existing non-hidden, non-symlink tree instead of following
  a preferred filename symlink into a hidden file. A regression test covers a
  README symlink to a hidden file.
- The browser identifies the data categories sent to the configured model.
  File paths, README excerpts, manifest summaries and product/draft content can
  still contain sensitive information; these changes are not general secret
  detection or a guarantee against a concurrently modified filesystem. The model
  prompt treats repository text as untrusted, but prompt injection is not proven
  impossible. Review the workspace and provider before analysis.
- On 2026-11-13 the macOS and Linux suites each passed 169 tests (one live test
  ignored), all browser suites passed, and the disposable real 7B browser
  repository-analysis and MCP confirmation flow passed with the reduced snapshot.

## Real Model Host repository analysis checkpoint

- `bash scripts/test host-planner --repository --browser` drives an actual
  headless browser against a disposable Studio container and synthetic inventory
  repository. It uses the local 7B model, checks a staged inventory area is shown,
  verifies the catalog is unchanged before confirmation, confirms through MCP,
  and checks the resulting persisted area. The API-only variant is
  `bash scripts/test host-planner --repository`; each test removes its fixture.
- Two initial 7B attempts returned draft-node objects with nested action types.
  The old parser silently discarded those actions. It now rejects a response if
  any action lacks a top-level type, preventing partial or empty success, and
  repository analysis asks for a corrected envelope at most twice. A mock
  provider test covers recovery and draft persistence. The prompt includes a
  concrete action example; subsequent real API and browser runs both passed on
  their first model response. These are narrow, nondeterministic observations,
  not a quality guarantee for other codebases, models, or partial plans.
- macOS and Linux Rust suites each passed 168 tests, with one live test ignored.

## Repository analysis parity checkpoint

- The browser planner can load registered workspaces and request repository
  analysis for its selected product and draft node. The UI states that a snapshot
  is sent to the configured model. The authenticated server operation is the same
  Rust service used by MCP and desktop; it stages results for review rather than
  immediately changing the product catalog.
- An isolated simulated provider test exercises authorization, a missing
  repository, successful draft persistence, and the unchanged product catalog.
  Browser tests cover product selection and disabling controls while analysis is
  pending. Repository scanning or model failures now keep the cached planner
  session aligned with already persisted seed/selection state. Native desktop
  interaction remains unverified.
- Validation on 2026-11-13: macOS and Linux suites each passed 167 tests
  (one live test ignored); browser suites passed one smoke, 21 remote, one CI,
  and 19 workflow cases. A real 7B Model Host planner proposal and confirmation
  also passed through MCP after the rebuild.

## Real Model Host planner checkpoint

- `bash scripts/test host-planner` creates a disposable Studio fixture, binds the
  registered `qwen25-coder-7b-q4` Model Host provider/model explicitly, creates
  a product, requests an inventory product area through MCP, checks a model
  completion and staged draft, verifies no area exists before confirmation, then
  confirms and checks the stored area. It removes its fixture and socket volume.
  The host can be selected by the optional path argument; `ARUVI_CODING_TEST_MODEL`
  selects an installed model. This is a nondeterministic real-model test, not a
  guarantee of planner quality or evidence for every model and operation.
- On 2026-11-13 the first 7B run returned a malformed final envelope, and 15B
  hit the 1,800-token limit. Studio now retries malformed planner envelopes at
  most twice within the existing six-step limit, asking the model to correct its
  own response; it does not invent or commit actions. A mock provider checks both
  successful recovery and the retry bound. After the change, two 7B runs staged
  and confirmed the requested area; the second recorded two model calls and one
  format retry. A token-limit error is still reported, not retried or accepted.
- Explicit planner provider/model choices now fail validation if unavailable or
  ambiguous instead of silently using another default provider. Regression tests
  exercise valid and invalid explicit selections; defaults still apply when no
  explicit choice is supplied. This avoids accidentally sending prompts to an
  unintended provider. Existing configurations with an unregistered or disabled
  explicitly selected pair now need to select an enabled, registered model.
- The rebuilt worktree image passed 166 Rust tests on each of macOS and Linux
  (one live-provider test ignored), the browser smoke, 20 remote, one CI and 19
  workflow tests. The real planner checks above use a separate disposable database.

## Planner concurrency checkpoint

- Browser, MCP and native planner mutations sharing one `PlannerService` now
  serialize by session, including draft edits, settings, clear, repository analysis,
  and text and voice turns. Other sessions remain independent. A regression test
  checks that concurrent edits both persist and that an unrelated session is not
  blocked. This prevents a stale cloned draft from overwriting another request in
  the same server process.
- This is not database-level concurrency control: independently running server
  processes pointed at the same SQLite database retain separate caches and can
  overwrite each other. A multi-process revision or locking protocol, atomic
  persistence and end-to-end conflict testing remain open before claiming broader
  multi-client safety. Voice and model calls serialize other edits to that session
  for their duration.
- Validation on 2026-11-13: macOS and Linux library suites each passed 164
  tests (one live-provider test ignored). The rebuilt isolated worktree container
  passed the browser smoke, 20 remote, one CI and 19 workflow cases; the real
  Model Host 7B coding MCP smoke also passed.

## Iterative workflow execution checkpoint

- Automatic workflow stages now return the next stage to an iterative driver
  instead of recursively polling the entire preceding stage chain. Coordinator
  review retains its one-stage bypass and approval gates still stop execution.
  This avoids a reproducible default-stack overflow during the twelve-stage
  automatic test-review and Git-delivery workflow. Repository preflight errors
  mark the actual current stage failed; a targeted regression test verifies this.
- On 2026-11-13 the macOS Rust library suite passed 176 tests with one live
  test ignored using the default test-thread stack. The existing end-to-end
  workflow test reproduced the overflow before the change and passes afterward.
- The worktree container remains on the preceding image: Docker's shared virtual
  disk has under 1 GB available. Rebuild and browser/MCP workflow validation of
  this change remain necessary once sufficient space is available. Other projects'
  containers, volumes, and caches were not removed to make room.

## Atomic planner application checkpoint

- Applying a staged planner design now uses one SQLite transaction for product,
  area, capability and work-item changes, conversation entries, and clearing the
  staged session. The cached session changes only after the database commit.
  Failure-injection tests cover a late hierarchy error and a failed assistant
  conversation insert, verifying catalog changes roll back and the draft remains
  available for retry.
- This fixes partial application within a single server request; it does not solve
  stale planner caches or concurrent writers in separate server processes. Existing
  catalog matching and the creation of new child nodes are unchanged.
- On 2026-11-13 the macOS Rust library suite passed 176 tests with one live
  test ignored using `RUST_MIN_STACK=16777216`. A default-stack run aborted in a
  workflow test with stack overflow; it is not evidence of a passing default run.
  A real `qwen25-coder-7b-q4` Model Host planner proposal and confirmation
  passed over MCP in an isolated fixture (two model calls, one format retry).
  Browser suites passed one smoke, 21 remote, one CI, and 19 workflow cases.
- Rebuilding filled Docker's shared virtual disk and the worktree server initially
  failed with a SQLite I/O error. Removing only obsolete AruviStudio build-cache
  records restored free space; the existing worktree volume was retained and the
  rebuilt container restarted healthy. The dynamically assigned port can change.

## Independent client revalidation

The separate consumer fixture now pins Studio commit
`0ba3aa08b2840adbb327b435042eed86b2f076b9`, including the recent shared-client
error handling. After vendoring, its network-disabled build and 20 tests passed.
Its non-root, read-only Linux probe then reached the real Model Host and passed
model discovery, completion, streaming and token-limit handling with
`qwen25-coder-7b-q4`. This verifies the pinned revision; later client changes
require a new pin and revalidation. No release was published.

## Real Model Host new-module workflow

On 2026-11-14, two disposable MCP workflows used the native Model Host's
`qwen25-coder-7b-q4` to create and export `sumEven` in a new JavaScript module.
The ordinary profile wrote only an export referencing an undefined function.
Its configured `node --check` syntax gate passed, but the separate networkless
behavioral assertions failed and verified that the disposable Git remote was
unchanged. This independent review is part of the test fixture, not an
automatic general Studio workflow gate; a real workflow needs an appropriate
project test command and review before approval.

One subsequent run with the coding agent's JSON-object output enabled passed
the syntax and independent behavior checks and delivered the checked module to
the disposable local Git remote. It also modified the fixture README, which the
task-specific assertions did not evaluate. One run per profile does not prove
relative reliability or cover arbitrary generated changes. Neither run used a
real external Git service.

## Model Host credential scope review

The validation host checkout at `583b730` applies the same authorization
middleware and bearer token to `/v1` inference and `/api/models` registration,
installation, loading and unloading. The inspected main checkout at `6edb849`
also shares its middleware across inference and management routes. This is a
source review, not proof of the revision or settings of every deployed host.
Studio's setup helper stores that token, so its secret-store readers and anyone
with both the token and host network access must be trusted with host management.
Neither an inference-only token nor an independently enforced proxy has been
validated here. Host deployment and authorization were not changed.

## DeepSeek desktop presets

On 2026-11-14, the desktop model presets were updated to `deepseek-flash` and
`deepseek-v4-pro`, the model IDs in [DeepSeek's current API guide](https://api-docs.deepseek.com/).
The old `deepseek-chat` preset was listed for retirement in the provider's
[V4 announcement](https://api-docs.deepseek.com/news/news260424/); the older
coder preset is also no longer a current recommended model. Existing model
definitions and agent bindings are not migrated automatically. Users should
verify model availability, capabilities and pricing before changing bindings.
The frontend build and nine mocked desktop provider checks passed. An
unauthenticated `/models` request returned 401 at both root and `/v1`, which
does not validate either authenticated endpoint or a real model call.

## DeepSeek standalone voice client

DeepSeek's November 2026 voice APIs use dedicated `/v1/tools/asr` and
`/v1/tools/tts` routes, not the OpenAI-compatible audio routes. The shared
library now has an explicit client for multipart transcription and buffered WAV
generation, with bounded inputs/responses, sensitive bearer headers, redirect
rejection and sanitized errors. Synthetic HTTP fixtures verify request shape,
voice selection and a 429 without response-body leakage. DeepSeek's service
enforces audio codec and duration; unknown-length WAV responses cannot prove
complete synthesis. Studio selects this client through the explicit speech IDs
`deepseek-voice-transcription` and `deepseek-voice-synthesis`; these are Studio
selectors, not DeepSeek model IDs. Transcription automatically detects language
and ignores the locale hint. Synthesis supports `stella` and `tide`, and rejects
speed control. All 35 isolated workflow tests passed, including synthetic
transcription via API and MCP and synthesis via browser, API and MCP. Native
speech-service tests passed eight cases with one local Whisper case ignored.
No live DeepSeek voice call or physical audio test has been made.
Browser speech generation now offers optional voice and speed fields. An
isolated fixture verified DeepSeek `tide`, a compatible provider's voice and
speed, and DeepSeek's unsupported-speed error; all 35 workflow cases passed.
This does not establish voice availability on arbitrary providers.
Desktop model setup now includes a transcription preset, and Settings and
planner recognize the selector even without capability tags. The frontend build,
five planner selection tests and ten mocked desktop provider checks passed;
native-window and real-provider acceptance remain open.
Desktop Settings also preserves a saved speech selector while the model list
loads or when the selector is not registered locally; changing the provider
clears the unsaved model selection. The frontend build and eleven mocked
desktop provider checks passed, including delayed model loading. The provider
and model settings are saved separately.
DeepSeek voice selectors now ignore surrounding whitespace and ASCII case,
matching desktop model recognition. Its 8 MiB transcription limit is checked
before base64 decoding as well as by the shared client; other Studio
transcription retains its 32 MiB limit. Nine native speech tests passed with
one local Whisper case ignored, and all 35 isolated workflow tests passed with
mixed-case DeepSeek selectors through the simulated API and MCP fixture.
Known DeepSeek voice selectors and local Whisper providers are excluded from
automatic agent text fallback. New agent bindings reject voice-only selectors;
an existing invalid binding fails explicitly rather than silently choosing a
different model. This guard does not classify arbitrary model capability tags
as voice-only. A native test covered fallback, rejected binding and legacy
binding behavior; the frontend build, eleven mocked desktop provider tests,
and all 35 isolated workflow tests passed. MCP tool discovery now describes
the DeepSeek voice selectors and their constraints.
Planner and browser chat automatic selection also skip known voice-only
selectors; planner additionally excludes local runtime providers. Explicit
voice-selector text requests are rejected in buffered Studio model calls,
desktop streaming and browser chat. The frontend build, six planner picker
tests, native planner resolution test and all 35 isolated workflows passed,
including simulated browser and MCP chat rejection. Other models are not
classified as voice-only solely from capability tags.
When neither provider nor model is supplied to browser chat or planner chat,
Studio now prefers the first enabled provider that has an enabled non-voice
model, rather than stopping at a voice-only provider. Explicit provider and
model inputs keep their prior routing behavior. A native selection test and all
35 isolated workflow cases passed; the fallback does not prove the selected
provider is reachable or that its model is suitable for the requested task.

## Shared-client example pin

The client README's Git dependency example now matches the independent
consumer's `0cabaeb392db9c88d1aecea29f1c0296dca9254e` manifest and lockfile.
The previous example pointed to an older commit, predating some behavior
described by the current README. The three revisions were compared locally;
this documentation correction does not publish a release or retest live
providers. The independently tested pin remains provisional.

## MCP provider HTTP error metadata

Studio now preserves a shared-client provider HTTP status through its buffered
model adapter and remote speech errors. MCP tool error results include an
optional `structuredContent.provider_http_status` while retaining the existing
text and `isError` flag. The upstream response body is not included. The value
is an upstream status, not an automatic retry recommendation; provider errors
without an HTTP status omit it. Browser HTTP status behavior is unchanged; web
and desktop streaming error events include an optional `provider_http_status`
when the provider returned an HTTP error. Targeted Rust tests and all 34 isolated workflow tests
passed; the MCP provider fixture validates end-to-end 429 cases for buffered
chat, remote transcription and remote speech generation without returning the
synthetic provider body. The web stream fixture and native Tauri mock-runtime
test also verify the 429 field and body redaction; the desktop cancellation
test verifies the field is omitted without an upstream HTTP status. This is
simulated-provider coverage, not a live rate limit or native-window test.

## Independent shared-client voice probe

On 2026-11-14 the independent consumer advanced to `3b97f0aaeb9293868b7d04deb6d7aa3c60e1f989`.
All 21 tests passed in a vendored, network-disabled Docker build, including a
simulated DeepSeek voice call. A non-root read-only probe passed discovery,
completion, streaming and token-limit checks against the 7B coding worktree;
against the integration host it passed those text checks with
`qwen-05b-comparison` and synthetic `piper-ljspeech` to `whisper-tiny-metal`
speech. The initial request for voice models on the coding-only host failed
because its model list contains no voice entries; neither host was reconfigured.
The helper now classifies selected known failures without printing child output.
These checks do not establish live DeepSeek voice or physical-audio behavior.

The separate Git-pinned consumer now exercises synthetic speech generation and
transcription as optional public `ModelGateway` operations. With the client
pinned to `0cabaeb392db9c88d1aecea29f1c0296dca9254e`, its 20 offline tests
passed; a non-root, read-only Linux probe reached the existing Model Host
integration environment and passed text discovery/completion/streaming and a
`piper-ljspeech` to `whisper-tiny-metal` round trip. This uses synthetic English
audio, not a physical microphone or audible playback. The host deployment was
not reconfigured.

## Empty completion handling

OpenAI-compatible buffered text and JSON-object completions now reject an empty
text result instead of reporting it as success, matching the existing streaming
and Anthropic behavior. This changes behavior for callers that previously treated
an empty answer as valid. All 52 standalone client tests passed, including both
buffered modes. All 33 isolated workflow tests passed, including web and MCP
error reporting against a simulated empty-response provider.

## Shared-client debug payloads

Debug formatting for chat messages, speech and transcription requests, and
completion responses now reports payload sizes without printing their contents.
Serialization, fields, provider metadata and Studio telemetry are unchanged and
must still be handled appropriately by consumers. All 51 standalone client
tests passed, including synthetic redaction checks.

## Model Host setup in provider UI

The desktop quick start includes an Aruvi Model Host preset with a blank endpoint
and masked token field; the browser provider form explains that the address must
be reachable from the Studio server. Neither assumes a fixed port or configures
the host automatically. Eight simulated desktop provider checks and 124 frontend
unit tests passed; real native-window interaction remains unverified.

## Studio transcription upload bounds

Studio now applies the existing 32 MiB decoded-audio limit to remote as well as
local transcription, checking base64 size before allocation and decoded size
before upload. This is a new limit for desktop remote transcription; HTTP request
limits may be lower, and providers may impose their own limits. The reusable
client does not impose this Studio-specific upload limit. Focused tests cover
both bounds, malformed base64, and rejection before network access. A real MCP
speech generation/transcription round trip passed after the branch rebuild.

## Desktop provider key entry

Desktop provider creation and rotation inputs now mask typed credentials and
clear their input state on close; successful rotation also clears it. The
frontend build, 124 unit tests and seven simulated desktop provider checks
passed. Masking does not establish native-window or keychain behavior, which
remain separate validation items.

## Browser provider credential rotation

Browser key rotation now sends the opaque credential revision and completion
profile it loaded; the backend checks these along with the provider fields before
storing a new secret. A stale browser gets a conflict and keeps its typed input.
For compatibility, older API clients may omit the revision and therefore do not
receive this stale-credential protection. In an isolated run all 32 workflow tests
passed, including a second client rotating before the browser save. The first run
also exposed a stale synthetic voice fixture expecting `en-US`; it was updated to
verify the normalized `en` multipart field before the passing rerun.

## Browser example-catalog settings

The Routing page exposes the desktop's hide-example-products preference and
seed/repair action. Changes use authenticated Rust endpoints and compare the
previous setting value so a concurrent MCP edit is not silently overwritten;
seed/repair calls the same catalog service as desktop and MCP. The backend test
covers authentication, conflicts, persisted settings and repeat seeding. All 27
isolated browser tests passed, including an MCP edit between browser load and
save and a confirmed seed/repair action.

## Remote speech locale compatibility

Studio converts locale hints such as `en-US` and `pt_BR` to their primary
language component for OpenAI-compatible transcription, and omits `auto` or
empty hints. The reusable transport still sends its caller-provided language
unchanged. This matches Studio's existing local Whisper locale behavior and the
[OpenAI transcription API](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)
language-code guidance; a provider that relies on regional subtags will now
receive only the primary language through Studio. Synthetic multipart and locale
unit tests passed. Actual provider language support and transcription quality
remain provider/model-specific.
A real MCP speech round trip using `en-US`, `piper-ljspeech` and
`whisper-tiny-metal` passed on 2026-11-14 with synthetic audio.

## Shared-client embedded error handling

Anthropic completion, both providers' model discovery and OpenAI-compatible
transcription now reject successful HTTP JSON responses that contain a non-null
provider error, even if partial text or model data is also present. Errors do not
include the provider's response body. OpenAI-compatible text and stream paths
already checked embedded errors. All 50 standalone shared-client tests passed
on 2026-11-14 with local fixtures; this is not live-provider validation.

## Desktop and MCP database settings parity

Desktop and MCP now share Rust operations for database health, active path and
path overrides. Their validation and whitespace handling are consistent. The
settings inventory in `web-parity-audit.md` distinguishes MCP coverage from
browser controls and records container path and restart limitations. This does
not establish parity for all settings or validate a real database migration.
The desktop settings tests and an MCP validation/persistence round trip passed;
the native Rust library suite passed 229 tests with six ignored on 2026-11-14.

## Desktop patch-review checkpoint

Desktop Copilot now previews each proposed file through the Rust repository
service, displays the complete patch, and applies only against the reviewed
file hash and existence state. It rejects duplicate targets and avoids applying
proposals after a workspace change. Files are applied individually, so an error
can leave earlier files applied; editor buffers with unsaved changes are still
protected. Native-window acceptance is pending access to the locked Mac.

On 2026-11-14, the frontend build and 108 unit tests passed, as did four mocked
desktop browser checks and six native repository-service tests. The mocked
browser checks are not a substitute for applying a patch in a native window.

A follow-up rejects a whole desktop proposal when any target has malformed
fields, rather than silently dropping the bad entries and presenting a partial
proposal. The frontend build, 115 unit tests and the mocked Copilot browser
check passed. Native patch application remains unverified.

Desktop review and application reject path spellings containing dot segments,
empty segments or backslashes. The backend also compares file identities against
open editor paths, covering existing symlinks and hard links; dirty aliases block
application and clean aliases are refreshed. For a new target, dirty open buffers
must be saved or closed first. Identity checks and writes are separate filesystem
operations, so external concurrent renames or replacements can still race them.
Open editor buffers in other registered repositories are included in the identity
check and refreshed through their owning workspace. Windows requires native
validation. On the Mac case-insensitive temporary filesystem, a Tauri mock-runtime
IPC test verified case variants and cross-workspace hard links, then previewed a
patch, rejected a stale base and successfully applied the unchanged base. This
is not a native-window interaction test.
On 2026-11-14, the native symlink/hard-link test, 123 frontend tests,
the frontend build and a mocked desktop Copilot browser check passed.
A cross-workspace follow-up passed the native hard-link identity fixture and all
124 frontend tests, including dirty-buffer protection and clean alias refresh.

## Open design questions

- Decide whether to move the Git-pinned client to a separately maintained
  repository, and establish release and semantic-versioning policy.
- Determine whether native model-host loopback forwarding from Docker is an
  intentional supported network path and how narrowly to provision its token.
- Establish provider-specific requirements from official protocol documentation
  before implementing them. Do not equate ChatGPT subscriptions with API access.
- Preserve the previously audited planner concurrency and parity gaps; this goal
  does not imply those have already been solved.
