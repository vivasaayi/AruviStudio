# Shared model client and Aruvi Model Host integration

This is the active implementation checkpoint. Completed items require the
evidence named below; simulated providers do not prove real-provider behavior.

## Objective

Create a reusable, independently consumable Rust library for model access, and
use it in AruviStudio while preserving existing desktop, web, MCP and workflow
behavior. Route local model inference to Aruvi Model Host. Do not move third-party
provider hosting into the model-host service as part of this work.

## Checkpoints

### Connected browser workflow diagram

The browser stage review now offers an optional connected diagram using the
desktop workflow's intake, planning split/merge, execution, verification
split/merge and delivery topology. Selecting a stage in the diagram updates the
existing stage details; the existing labeled buttons provide keyboard and
screen-reader access. An optional focused view follows the selected runtime
stage, including scrolling it into view at mobile widths. Current and recorded
entry colors do not assert completion; planning branches still share one runtime
stage, while their selected diagram node remains distinct; the visual topology is explanatory. The diagram's presentation data
is presently maintained alongside the desktop definition, so the two can drift.

A disposable native Studio server and Chromium fixture verified 27 connections,
diagram and keyboard selection, focus behavior, unknown stages, stage history and
artifact isolation. The mobile test checked that the page does not widen and
the focused node is visible. Desktop and 390-pixel screenshots were reviewed;
the first layout and focus failures were corrected and the fixture rerun. The
frontend build and 128 frontend tests passed; the Rust library suite passed 333
tests with seven ignored. This is not full desktop workflow
parity or a physical native-window comparison. The deployed branch image predates
this checkpoint until a clean image is built, validated and deployed.

### Manual browser microphone cleanup

Leaving the Voice tab or hiding the page now discards an unfinished manual
recording and stops its microphone. A pending microphone permission request can
finish after navigation; its returned stream is closed and cannot affect a new
recording. Pending transcription is aborted on navigation and its result is not
submitted. Once a transcript has already been submitted to chat or the planner,
this change does not cancel the model request. The existing manual review and
automatic-submit behavior remain available. Fourteen isolated browser cases
passed across manual and continuous sessions, including navigation, delayed
permission, restart, transcription cancellation and page hiding. The frontend
build and 128 frontend tests passed. A rebuilt native Studio server with a
simulated provider passed the existing manual review/submission and continuous
voice browser flows, as well as a navigation-and-discard check. Physical microphone and browser/device behavior remain
unverified. At `ca879db`, the complete local browser suite passed 67 tests,
the Rust library suite passed 333 with seven ignored, and the terminology check
passed. Both hosted test runs and the clean ARM64 build passed. The archive
checksum and revision were verified; its server binary (SHA-256
`b2567de6b94c528aec1d2fc59d89f8a579b86a85800283a50fdf5f0318668aa8`)
was layered onto the prior runtime to produce derived image
`sha256:95a6e192fcfc6f86e8d578f3b18c1e5bcf209bbd167ba47235e0debe13608250`,
not the full clean artifact. It passed 56 isolated workflow tests, 45 browser
tests with one skipped, and the PDF outage check. A verified database backup and
the prior image were retained. Product, MCP, mobile and PDF acceptance passed
across restart, as did published-port origin checks, real 7B MCP coding, real
MCP speech, synthetic browser transcription and continuous browser voice through
real Model Host Whisper and the local coding model. The branch endpoint was
`127.0.0.1:55288`; physical audio was not tested.

### Continuous browser voice conversation

The browser Voice screen now offers a continuous voice-chat session alongside
manual recording and transcript review. It records up to 18 seconds per turn,
detects a speech pause using a fixed audio threshold, transcribes through the
configured Studio service, submits to chat, and waits for device speech to end
before listening again. This session explicitly sends transcripts without review
and targets voice chat, independently of the manual recorder's target and review
settings. Ending the session or leaving Voice stops capture, aborts pending browser
requests, cancels browser speech and discards late results; aborting the browser
request does not prove a model server stopped processing it. A microphone permission
prompt cannot be cancelled programmatically, so a late stream is closed on return.
Speech-recognition thresholds, noise and physical devices need real-world testing;
this does not establish complete desktop voice parity.

A disposable native Studio server, simulated OpenAI-compatible transcription and
chat provider, and headless Chromium exercised a full turn, reply playback completion,
restart of listening and cleanup, and also passed the existing manual recording,
review and submission regression test. Nine isolated browser cases cover repeated
turns, speech waiting, late microphone permission, cancellation during transcription,
transcription failure, navigation and stopping while speaking. The frontend build,
128 frontend tests, and Rust library suite (333 passed, seven ignored) passed; the
full Rust run preceded the final browser-only cancellation adjustment. The new button
was visually inspected at mobile width without horizontal overflow. Neither fixture
uses physical audio or a live external provider.

Both hosted test runs and a clean ARM64 build passed for `cecec05`. The archive
checksum and source revision were verified; its server binary (SHA-256
`ed53dea6b22dca4ed778b38f586e55fd9cdb7dbbece45f3e381878769f6fd8cd`)
was layered onto the prior branch runtime to produce derived image
`sha256:6b6e4c09b31ac364926b232a48084c6d5fd82ba4b9b3a87130abb8b752c8330f`,
not the full clean artifact. This candidate passed 55 isolated workflow tests,
including continuous browser voice through a simulated provider, 45 browser
tests with one skipped, and the PDF outage check. A checked database backup and
the previous image were retained. Product, MCP, mobile and PDF checks passed
across restart, as did published-port origin checks, real 7B MCP coding, real
MCP speech and manual browser transcription. The deployed continuous browser
session also sent synthetic Piper audio through real Model Host Whisper to the
local 7B coding model; the recognized greeting and a nonempty reply were checked.
The branch endpoint was `127.0.0.1:55288`. Audio was synthetic and no physical
microphone or speaker was verified.

### Desktop Voice Chat transcription configuration

Voice Chat now asks the backend to resolve its transcription provider and
model before opening the microphone, and sends only its legacy `en-US` locale
fallback when transcribing. Server environment and saved configuration therefore
take precedence, including the official OpenAI model default. The preflight
checks disabled providers, incompatible protocols, synthesis-only selectors and
configured language mode locally; it cannot prove a remote model is available.
The displayed model is identified as the value resolved at session start; each
transcription resolves configuration again. A capture or transcription failure
ends the session instead of repeatedly opening the microphone. Stopping a
session discards pending capture or transcription results, including after a
new session starts; already-submitted model streams and native speech are not
cancelled by this change.

The frontend build and 128 frontend tests passed, as did six mocked Voice Chat
browser tests and four existing capture tests. The full Rust library suite passed
333 tests with seven ignored; focused command coverage includes legacy and new
Tauri IPC payloads and environment precedence.
These tests do not validate a native desktop window, microphone, speaker or a
live third-party account. Both hosted test runs and the clean ARM64 image build
passed for `6ebe923`; the later browser voice checkpoint records deployment of
a subsequent revision that includes it.

### Desktop Planner transcription precedence

Planner previously sent saved speech settings and a default `en-US` as explicit
request fields, which could mask the server's environment overrides. It now
lets Rust resolve configured provider, model and locale first; inferred Planner
provider/model values and the prior `en-US` locale are supplied only as
fallbacks. A fallback model is used only with its corresponding provider.
Explicit Tauri arguments retain highest precedence; an explicitly blank locale
environment setting still suppresses the saved value. A configured provider
also works when the frontend cannot infer any speech selection. The focused
Rust tests and mocked desktop voice tests pass for environment precedence and
fallback selection. A Tauri mock-IPC test also confirmed both legacy payloads
without the new fallback fields and the new camel-case fallback fields reach
the command, and the environment fixture covered both stale Planner fallbacks
and no frontend selection. The frontend suite passed 128 tests and its build,
four mocked desktop voice tests passed, and the full Rust suite passed 333
tests with seven ignored. Both hosted test runs and the clean ARM64 build passed
at `ef2e5c9`; the archive checksum and source revision were verified. Its server
binary (SHA-256
`eed30fc1f98fc8152c0c91d67e2bfd99cd405b8996b14c065c4522e1ea3e5e3e`)
was layered onto the prior runtime, creating derived image
`sha256:d8e6621f7263fc025eb5967c44eea93d834199f129e45a674956b54ee180cd66`,
not the full clean artifact. The candidate passed 54 isolated workflow tests,
45 remote browser tests with one skipped and PDF outage checks. A checked
database backup and prior image were retained; product, MCP, mobile and PDF
checks passed after restart, as did published-port origin and real local 7B MCP
coding checks. A separate disposable fixture saved server speech defaults and
used them through the browser and MCP for real synthetic Model Host transcription.
The branch endpoint was `127.0.0.1:55288`. This does not validate a native
desktop window, physical audio devices or live OpenAI access.

### OpenAI transcription default migration

OpenAI's documented file-transcription path now recommends `gpt-transcribe` and
states that `whisper-1` shuts down on February 26, 2027. When no transcription
model is configured or registered, Studio now selects `gpt-transcribe` only for
the official OpenAI HTTPS API root or `/v1` endpoint; compatible proxies and
local providers retain the existing `whisper-1` fallback. Explicit and
registered models still take precedence. The desktop Planner no longer sends a
hard-coded fallback model, so the same server choice applies to desktop, web and
MCP. In the shared client, automatic official `gpt-transcribe` locale hints such
as `en-US` are reduced to `en`; supported Chinese regional hints are preserved.
Other providers and explicit format overrides keep their original hints. See
[OpenAI's transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text)
and [deprecation guidance](https://developers.openai.com/api/docs/guides/transcription).
The packaged client passed 66 tests; the Studio speech tests passed 15 with one
local-model test ignored, the frontend passed 128 tests and its production build,
and four mocked desktop voice-capture tests passed. The mock confirms Planner
omits the fallback model and delegates selection to Rust. The independent consumer
was pinned to `1d8df226a22e7c4e0ddd21c3e46a654c410ca21f`; its Linux offline
contract suite passed, and its non-root, read-only probe passed real Model Host
discovery, completion, streaming, token-limit handling and synthetic
`piper-ljspeech` / `whisper-tiny-metal` speech against `qwen-05b-comparison`.
The test's exact disposable Docker cache entries were removed afterward to
recover space; running images and data volumes were retained. Linux branch
validation used a clean ARM64 build at `f9bdb3e` and the later test fixture at
`f523688`; both hosted test runs passed. Its archive checksum and source revision
were verified. The extracted server binary (SHA-256
`f7c638cbb764f3d1071a90913cd08e5dc9997cc5a3bde26607496979feb0582f`)
was layered onto the prior runtime, producing derived image
`sha256:6047abf6602cdf8bd9cd4d6a95c17f27fd945e9e396cff18a7b611fed8923694`;
this is not the full clean artifact. The candidate passed 54 isolated workflow
tests, including a compatible-provider fallback through the browser API and MCP,
45 remote browser tests with one skipped, and the PDF outage check. The native
Rust library suite passed 330 tests with seven ignored. A checked database backup
and prior image were retained; product, MCP, mobile and PDF acceptance passed
after restart, as did published-port origin checks, a real local 7B MCP coding
request, an MCP Piper/Whisper round trip and a headless browser recording using
synthetic Piper audio. The branch URL was `127.0.0.1:55288`. Physical audio,
live OpenAI access and actual OpenAI account/model availability remain unverified.

### Workflow map stage indicators

The browser map labels the current stage, stages entered according to recorded
transition history, and stages without recorded entry. These labels do not
assert completion or prove an unrecorded stage never ran; planning labels share
one runtime stage and do not establish that individual planning branches ran.
Map selection is exposed through `aria-pressed` and has a visible focus-style
outline. The native server build and disposable browser fixture passed for
current/entered/unrecorded states, mouse and keyboard selection, workflow
isolation, and all map controls at a 390-pixel viewport. A headless browser
capture was visually reviewed at that width and map spacing was increased;
a desktop-width capture was also reviewed. This is not a complete layout audit.
Both hosted test runs and the clean ARM64 build passed at `1a34cbf`; the archive
checksum and source revision were verified. Its extracted server binary (SHA-256
`3197c3ffb0594fd0f36378194eb0b89716a7abac670b14b78b8e683d1c6b03a5`)
was layered onto the prior branch image to produce derived image
`sha256:394d98e8d6563641e02bd5ccaedc59fad27b09fe3eb1bf2c940ce9de05725da7`,
not the full clean artifact. The candidate passed 53 isolated workflow tests,
45 browser tests with one skipped, and the PDF outage check. A checked database
backup and prior image were retained. After deployment, product, MCP, mobile and
PDF checks passed across restart, as did the published-port origin check and a
real `qwen25-coder-7b-q4` MCP coding request. The branch endpoint was
`127.0.0.1:55288`. The desktop SVG connections and focused-node layout remain open.

### Browser workflow map

The recorded-stage review now includes an explanatory workflow map with the
desktop's intake, planning, execution, verification and delivery lanes. Its
stage buttons select the corresponding recorded-stage details, including stages
with no recorded events; structural split nodes are labeled as explanation,
not evidence of execution. Multiple planning labels share the `planning`
runtime stage, as on desktop. The native server build and disposable browser
fixture passed, including navigation to a stage with no recorded events. The selected-stage view also
groups recorded model calls by their stage or associated agent run; the native
fixture verified the selected workflow's call and excluded the other workflow's
call. Hosted tests and the clean ARM64 build passed for `ba058cd`; the archive
checksum and revision were verified. Its server binary (SHA-256
`4e34cae630373a437f3c33809748ab3d376b0d063513df8bf293b5b9d1dbd444`)
was applied to the prior branch image, producing derived image
`sha256:d126dd21b740db55f86e7a8c583bbb1c02397612305efa304226f3894c362fb7`;
this is not the full clean artifact. The candidate passed 53 isolated workflow
tests, 45 browser tests with one skipped, and PDF outage checks. A checked
database backup and prior image were retained; product, MCP, mobile and PDF
acceptance passed after restart, as did published-port origin checks and a real
`qwen25-coder-7b-q4` MCP coding request. The branch endpoint at this checkpoint
was `127.0.0.1:55288`. This map does not yet reproduce the desktop SVG
connections, stage status colors or focused-node layout.
A 390-pixel-wide native browser fixture also confirmed that the Coding map
control fits the viewport; it is not a comprehensive mobile layout audit.

### Workflow stage review

The browser can request transition history and agent runs for the selected
workflow, select a recorded stage, and view that stage's transitions, runs and
associated work-item artifacts. The history endpoint uses the same repository
query as desktop and MCP and requires the mobile token. A disposable native
fixture verified authentication, MCP/HTTP agreement, workflow isolation,
stage selection, and reading an associated artifact. The native Rust suite
passed 327 tests with seven ignored. Hosted tests and the clean ARM64 build
passed at `58d39ec`; its archive checksum and source revision were verified.
The extracted server binary (SHA-256
`0386d12c395bbac68ae4af90610ba53bedc6bb85c6f8b51693ec8fe76af0e413`)
was layered onto the preceding branch image, yielding derived image
`sha256:c5f2aaf59ccf17d8335bdf31943cd107af30c52bab127b2f9262fd03a9ed23f1`;
this is not the full clean artifact. The candidate passed 53 isolated workflow
tests, 45 browser tests with one skipped, and PDF outage checks. A checked
database backup and prior image were retained. The deployed branch passed
product, MCP, mobile and PDF acceptance across restart, published-port origin
checks, and a real `qwen25-coder-7b-q4` MCP coding request; it was available
at `127.0.0.1:55288` at this checkpoint. This is a recorded-stage view, not yet a replacement
for the desktop's explanatory split/merge diagram. The history query, like the
desktop query, is unpaginated.

### Workflow usage and agent-run visibility

The workflow-call browser control also retrieves the selected workflow's agent
runs and summarizes input/output tokens, failures and duration. When no
per-call records exist, it labels available agent-run token totals as legacy
telemetry instead. The corresponding authenticated HTTP route uses the same
agent-run query as desktop and MCP. The native server build and disposable
browser fixture passed for both per-call and legacy totals. Hosted tests and the
clean ARM64 build passed at `ac2144d`; the archive checksum and source revision
were verified. Its server binary (SHA-256
`97598f3cfd8e66d3cbd4e67edea5d470bd99873637a8116a3e2d1244d3d4adc9`)
was layered onto the preceding branch image, creating derived image
`sha256:0a057f8ba93cebc8196aadd40bfa411438c0893c457cfcf13b60824c40a50877`.
This is a derived image, not the full clean artifact. The candidate passed 53
isolated workflow tests, 45 browser tests with one skipped and PDF outage
checks. A checked database backup and prior image were retained; the deployed
branch passed product, MCP, mobile and PDF acceptance across restart, published
port origin checks, and a real `qwen25-coder-7b-q4` MCP coding request. Its URL
was `127.0.0.1:55288` at this checkpoint. This does not add the desktop's full workflow diagram
or stage-by-stage artifact review. The desktop diagram contains structural
split/merge nodes as well as actual runtime stages; its selected-stage panel
associates runs and artifacts and shows recent transition history. MCP already
provides `get_workflow_history`; the newer stage-review checkpoint above adds
browser history and stage selection. A follow-up must distinguish structural nodes
from recorded runtime events and keep the selected workflow's runs, transitions
and artifacts scoped together rather than presenting the diagram as proof of
execution.

### Workflow-call branch deployment

The browser now displays recorded model-call summaries for the selected work
item's latest workflow, and MCP exposes `list_agent_model_calls_for_workflow`.
A disposable native fixture verified two-workflow isolation, authentication,
MCP/HTTP agreement and the browser display. The isolated Linux simulated-provider
workflow suite passed 53 tests, including the new check; the remote browser suite
passed 45 with one skipped, and its PDF outage check passed. The native Rust
library suite passed 327 tests with seven ignored; frontend tests passed 128 and
the frontend production build passed.

Hosted tests and the clean Linux ARM64 build passed for `de9c626`. Its archive
checksum and source revision were checked, and the extracted server binary
(`a847c10b02aeac7c33090146fee2ea3c2e39ae24f1024c8039233f7e7743a018`)
was layered onto the prior branch image, yielding derived image
`sha256:e93edfb0d9985fc8baa927011a8b66e42490653501cf62dea3558602adcf8091`.
This is a derived image, not the full hosted artifact. A checked database backup
and prior image were retained before deployment. Product, MCP, mobile, PDF and
persistence acceptance passed across restart; the published-port origin test
and a real `qwen25-coder-7b-q4` MCP coding request also passed. The branch is
available at `127.0.0.1:55288` at this checkpoint. Model-call retrieval uses the
existing unpaginated desktop query, and this summary is not complete desktop
workflow-review parity. Mac integration and VPS staging remain unverified.

### Current real-model workflow revalidation

On the deployed branch image derived from runtime `a850602`, a disposable
existing-module workflow used the real `qwen25-coder-7b-q4` Model Host model,
JSON-object coding mode, an alternate work-item workspace, and a 4,096-token
non-coding budget. Headless Chrome approved both the plan and test-review gates.
The generated edit passed the workflow syntax test, independent behavioral and
duplicate-declaration checks, and was delivered to a disposable Git remote.
The product's default workspace was untouched. The coding trace confirmed the
duplicate-definition guidance reached the model; it also recorded a rejected
repeat replacement before the model completed. This is one successful sample,
not a guarantee of model quality, physical desktop behavior or staging readiness.
The fixture containers and volumes were removed; the branch remained healthy.

A separate new-module run on the same image and 7B model, with the same JSON
mode, workspace override, token budget and browser approvals, also passed syntax
and independent `sumEven` behavior assertions before delivery to its disposable
Git remote. The default workspace remained untouched. This is one successful
new-module sample and does not erase the earlier incomplete-function failure or
establish general coding reliability.

### Historical image archive and Docker headroom

To restore working space without removing the current branch image, immediate
rollback image, containers or volumes, 14 older Studio-only validation image
records (27 tags) from a separate, unused image chain were exported and then
removed from Docker. Before removal, all tags and their root filesystem layer
identities were checked against the archive, and no container referenced those
images. Afterward all 94 OCI blob content hashes and the archive checksum were
verified. The private archive, checksum and tag-to-layer inventory are in
`~/.local/share/work-environments/aruvi-studio-worktree-0e3d3e51014c/image-archives/`.
The archive `historical-studio-20261121.tar.gz` has SHA-256
`4bb1e65b23b5e2c37a1853ec2456cc65063f2e97512bd1c6b7a51f6e0f0d7b7d`;
`docker load -i <archive>` restores its archived tags and layers when needed.
Historical checkpoints below mentioning those tags describe their earlier state;
the inventory identifies which are now archived. Docker VM free space increased
from about 642 MiB to 2.4 GiB, and the branch app and PDF service remained
healthy. The active image and `pre-a850602` rollback image remain in Docker.

### Browser and MCP workspace editing

Browser Workspaces and `aruvi_repositories.update_repository` now share the
desktop backend operation for name, path, remote URL and default branch updates.
A changed browser path must be an accessible Git working repository, matching
the browser registration constraint; MCP and desktop retain their existing path
validation. Both remote interfaces require a repository snapshot and reject stale changes;
the desktop command remains compatible without one. Workspace changes and
inherited work-item branch updates commit together. Active/paused workflows or
running CLI operations associated with the workspace block path, remote or
branch changes. Registration and workspace creation use the same SQLite write
lock to coordinate duplicate path checks. This protection requires cooperating
versions sharing a database; legacy/direct writers and external filesystem
changes are outside it. MCP write protection requires clients to supply the
opened root, and desktop buffers do not yet have this guard.

A disposable native browser/MCP fixture tested stale snapshots and preserved
drafts, duplicate and concurrent registrations, active/paused workflow and CLI
guards, atomic rollback, propagation of default but not custom branches, and
rejection of an open file save after changing its workspace path. The same
fixture ran the new remote Playwright case on a native server. A Rust endpoint
test and existing native editor, Copilot and root-guard fixtures passed. The
native server and frontend build passed, as did all 128 frontend tests and 327
Rust library tests (seven ignored) at `a797d7d`. The later changed-path Git
validation passed the focused Rust and native browser/MCP tests, including
non-Git and bare repositories; the Linux browser case also checks rejection of a
non-Git path. Its full hosted checks remain pending. The clean
ARM64 image and both hosted test runs passed for `a797d7d`, but its isolated Linux browser suite found an
ambiguous workspace-name label in an existing registration test: 44 passed,
one skipped and one failed. The initial replacement label still contained the
registration label as a substring, which the existing Playwright selector
matches. The edit field now uses a non-overlapping label; updated native
workspace and editor browser fixtures passed, including a check using the same
non-exact registration selector. The final runtime revision `a85060285f9cf418101fa4a0d180233b2971a68e` passed
both hosted test runs and a clean ARM64 image build. Its archive checksum and
revision matched. The server binary (`sha256:d24d9b11a0261c82852c21eb78e428b42ea5e38d38a80f3ec5dee095ca40a897`)
was layered onto the previous branch runtime to create derived image
`sha256:232d46fa3392d460c7f2aab4f5b9dc53a286d8cdb2b4dcd2b38c45c372423208`;
this is not the full hosted image. It passed 45 Linux browser tests (one skipped),
the PDF outage check and all 53 workflow tests. After a checked SQLite backup
and retaining the previous image, the derived candidate was deployed to the
branch. Product, MCP/mobile authorization, PDF and persistence acceptance passed
across restart, along with browser MCP-origin checks and real 7B general and
coding requests. A branch browser edited a uniquely named unattached workspace;
MCP verified the changes and rejected a stale snapshot, and the fixture record
was deleted. A browser catalog import was also verified through MCP and its
fixture product archived. The previous image is tagged
`aruvi-studio-worktree-backup:pre-a850602` and the checked backup is in
`/private/tmp/aruvi-workspace-a850602`. Docker VM free space was approximately
643 MiB after validation. Physical desktop acceptance and staging remain
pending.

### Browser workspace root guard

Browser file saves and reviewed patch applications now carry the workspace
path captured when it was opened. Before writing, the server checks that path
under a SQLite write lock, coordinating with database updates from other
processes using the same database. A stale browser buffer or reviewed patch is
rejected if the workspace path changed, even when the destination file has the
same contents. Reads and patch previews also check the expected path, but their
check and filesystem operation are separate; this is not a complete snapshot
or protection against external filesystem changes. MCP file reads return the
registered root; callers can pass it as `expected_root` to file writes and both
patch application actions. Those actions hold the same SQLite write lock through
the mutation. Older API callers can omit the path and retain their former
behavior. Desktop buffers and external filesystem changes are not covered.

A disposable native browser test used a separate process holding a SQLite write
lock to change the registered path while a save waited; it verified conflict,
unchanged files, rejection of a stale reviewed patch, a valid current-root
patch, and a legacy save. The fixture also checked MCP read metadata, stale
writes and patches, a valid write, and an MCP write waiting on a separate
process that changes the path. A disposable two-server Linux fixture also
verified stale browser-API and MCP writes after a path change, including an MCP
write waiting for a separate SQLite writer; it passed twice on the deployed
branch image. A Rust endpoint test and native editor and general Copilot
regression fixtures passed. The server build and full Rust library suite
passed (326 tests, seven ignored). Linux container and branch-deployment checks
remain pending, and workspace editing has not yet been exposed through the
browser or MCP.

### Remote catalog bulk import

The browser Products screen and `aruvi_catalog.submit_inline_bulk_import` can
submit UTF-8 JSON or CSV without a server-side file path. Requests carry at
most 1 MiB of content; the backend records the source as `<inline>` and uses
the existing asynchronous import jobs. Existing path-based imports continue to
work. Import is an upsert: matching IDs can change existing records, and a
failure after a committed batch can leave earlier batches applied. The browser
asks for confirmation and displays job errors; it does not provide an atomic
preview or rollback. The authenticated browser endpoint and MCP share the
same parser and persistence implementation.

A disposable native server and headless Chrome fixture verified JSON and CSV
submissions, persisted products, the legacy path import, malformed input,
size limits, unauthorized access and error status; it also ran the remote
Playwright import test against the native server. The Rust bulk-import tests,
server build and frontend build passed; the full Rust library suite passed 325
tests with seven ignored, and all 128 frontend tests passed. Both hosted tests
and the clean ARM64 image build passed for `b09491f01f530062cf6a6828ce90d2603e18e590`.
The archive checksum and revision matched. Its server binary was layered onto
the prior branch runtime, producing derived image
`sha256:f021d9aabbab22fcfca222e0ab2d971e9f205f9484875c8bc5921d7c678cf980`;
this is not the full hosted image. The candidate passed 44 Linux browser tests
(one skipped), the PDF outage check, and 53 workflow tests. After a checked
SQLite backup and retaining the previous image, it was deployed to the branch;
product, MCP/mobile authorization, PDF and persistence acceptance passed across
restart, as did MCP-origin checks and real 7B general and coding requests. A
headless browser imported a uniquely named fixture into the branch; MCP verified
the resulting product and then archived it. An initial cleanup used the wrong
MCP argument name; that fixture was subsequently archived as well. The prior
image is tagged `aruvi-studio-worktree-backup:pre-b09491f`, and the checked
backup is in `/private/tmp/aruvi-import-b09491f`.

### Browser workspace file copies and Copilot tabs

The browser workspace can download the open UTF-8 file copy, including unsaved
edits, without writing to the repository. Consistent CRLF files retain their
line endings; read-only mixed-line-ending files download their original bytes.
This is a snapshot of the open buffer, which may differ from a later disk change;
it does not reveal the server folder or support binary and oversized files. A
fix also preserves the general Copilot transcript when closing an active tab
while other file tabs remain open. A disposable native server and headless
Chrome test verified these cases, the unchanged disk contents, and existing
simulated Copilot streaming and patch flows. A separate disposable native
server test also passed the same browser flow with the optional Monaco editor,
including downloads, unsaved tabs, guarded saves and line endings. The test
refocuses the editor before invoking its save shortcut after a download. Linux
image and branch-deployment validation for this change subsequently passed as
follows.

The clean hosted ARM64 build of `b704d5b2f7f53e6026db7f709945eb3ef00027db`
and both hosted test runs passed; the later `d67c01e` commit changes only tests,
the test entry point and this checkpoint, and its two hosted test runs passed.
The hosted image archive passed its checksum and source-revision checks. Its
server binary (`sha256:95e5fe962748d8829a84f5ced8ce67c77e6fe483f799dd453aba7c8c78cdbff6`)
was overlaid onto the existing worktree runtime, producing derived image
`sha256:fe656b9d4cfd2aea46f3ab491539aa80758326180ccc1edcf6af90c161d0b806`;
this is not the complete clean hosted image. The candidate passed 43 Linux
browser tests (one intentionally skipped), PDF isolation and outage checks,
and 53 workflow tests. After a checked SQLite backup and preserving the prior
image, the branch app and PDF services were deployed with the derived image.
Product, MCP/mobile authorization, PDF and persistence checks passed across a
restart, as did browser MCP-origin checks and real 7B general Copilot and coding
requests. An authenticated headless browser also streamed a real 7B coding
answer from this deployment, and both the MCP Piper-to-Whisper round trip and
the browser synthetic-microphone transcription passed against Model Host. These
checks do not establish physical microphone or audible speaker behavior.
The previous image is tagged `aruvi-studio-worktree-backup:pre-b704d5b`
and the checked database backup is in `/private/tmp/aruvi-browser-b704d5b`.
Docker VM free space was approximately 759 MiB after tests. Staging and native
window acceptance remain outstanding.

### Browser IDE Copilot without a file

The general Copilot HTTP (buffered and streaming) and MCP actions now share
server-side request limits: at most 100 messages and 256,000 Unicode
characters, a nonblank user message, supported chat roles, temperature 0–2 and
`max_tokens` 1–32,768. The browser already applies smaller history limits.
Direct general-Copilot requests outside these limits, which were previously
accepted, now fail before contacting the provider; the separate general model
chat operation is unchanged. Native boundary tests and authenticated HTTP/MCP
tests covered the validator. A disposable native browser fixture passed its
simulated-provider chat and patch flows, and both browser and MCP general
Copilot received a nonempty reply from the real 7B Model Host. The native Rust
library suite passed 323 tests with seven ignored. Native-window acceptance
remains pending.
The native fixture also sent invalid token limits, temperatures, roles and
oversized messages through buffered HTTP, streaming HTTP and MCP, and verified
that none reached its simulated provider. Equivalent Linux container tests and
checks against the deployed worktree rejected the same invalid cases through all
three interfaces.

For runtime revision `c88773fda2de69a1726b15065c72dbb0fa68d25e`, both hosted
test runs and the clean Linux ARM64 image build passed. The image archive's
checksum and revision matched; its server binary was layered onto the previous
unchanged worktree runtime, producing derived image
`sha256:b9ff10932cebc416558b497ce6e9fb9fbb9b31241ad45e54eb8e1d2f0c88f8a9`,
not the complete hosted image. Its disposable Linux browser suite passed 43
tests (one skipped), the workflow suite passed 53, and the PDF outage check
passed. The branch deployment passed product, mobile/MCP authorization, PDF
and persistence acceptance across restart, MCP-origin checks and real 7B
coding and general-Copilot requests. A checked database backup and prior image
were retained before deployment. This is not staging or physical desktop
acceptance.

The clean hosted ARM64 build and both hosted test runs for
`6d23c1d3e4acc8e068af8dddf0fd5bd54ebea9a6` passed. Its downloaded image archive
passed its checksum and revision checks. The extracted server binary was layered
over the unchanged previous worktree runtime, producing derived image
`sha256:f9e9b4217734878d242aafbaf8f796416ea5710fa0b8c954cbae5e873c4d6d9f`;
this is not the complete hosted image. In disposable Linux containers the
browser suite passed 43 tests (one skipped), including general MCP Copilot and
no-file patch review, concurrent-change rejection and streaming; 53 workflow
tests, PDF outage isolation and artifact confinement/paging also passed. After
a checked SQLite backup, the candidate was deployed to the worktree. Product,
MCP, mobile authorization, PDF and persistence acceptance passed across restart,
as did browser MCP-origin checks and real 7B coding and general-Copilot MCP
requests. The prior worktree image was retained for rollback. These checks do
not establish staging or physical desktop parity.

The desktop IDE can chat with Copilot without selecting a file. The browser now
also offers a separate general conversation, including streaming and reset, and
restores it when the last open file is closed. General requests send the prompt,
instructions and bounded history rather than a repository path or file content.
They use authenticated browser IDE endpoints and are recorded as IDE calls.
With a workspace selected, the browser can also propose a patch without an
open file. The server supplies the patch format instruction; review previews
each file and guarded application checks the previewed base. File conversations
remain separate, and an open file must be saved before proposing a patch.

A disposable native server and headless browser exercised both completion modes,
authentication, conversation history and switching into and out of a file using
a simulated provider (`bash scripts/test native-general-copilot`, after building
the native server). A mock-browser fixture and an Anthropic protocol fixture
also covered the new paths. The fixture additionally exercised buffered and
streamed proposals without an open file, rejection after a file changed between
review and application, and the existing open-file path. Application of a
multi-file proposal is still sequential and may stop after partial success.
The native Model Host fixture with `--browser` also exercised general Copilot
without a file using the real `qwen25-coder-7b-q4` host, after an MCP coding
completion; it checked that the browser received a nonempty answer, not its
correctness. MCP now also exposes `aruvi_models/run_general_copilot`, including
the server's patch-format instruction and IDE activity recording. A simulated
Anthropic test checked the instruction and telemetry; the native fixture also
received a nonempty response through this action from the real 7B host. This
does not establish native-window, live cloud-provider,
container deployment or complete IDE parity.

### Shared artifact reading

Desktop, MCP and browser artifact text reads share a confined reader. The
full-text API has a 2 MiB UTF-8 limit; separate 64 KiB pages provide access to
larger text artifacts. Browser test results keep their smaller 256 KiB full-read
limit and work-item/type checks. This closes a difference where desktop and MCP followed
stored paths outside the configured artifact directory. Unix opens walk from an
artifact-directory descriptor and reject symlink components, including a
parent swapped after path resolution; non-Unix retains a canonicalize/open
race. The page version is metadata-based rather than a content hash: ordinary
file changes are rejected, but it is not a snapshot against a malicious writer.
Hard links and in-place content changes are not prevented. Existing records
outside the current directory and non-text files require separate recovery.

The desktop artifact modal and browser Work view can now read a large text
artifact in bounded pages; MCP exposes `read_artifact_page`. A first page
returns a version and each subsequent request must supply it. A page ends on a
UTF-8 boundary and detected changes require a restart. The existing full-read
APIs and their limits are unchanged. Desktop's paged view displays one page at
a time; its full-content trace timeline still requires a full read. The
version is derived from file metadata, not a content hash or an immutable
snapshot, and non-Unix metadata offers weaker change detection. A native
desktop IPC test, a desktop-modal browser fixture, and a disposable native
server/headless browser test exercised paging, including a 2 MiB-plus file and
a changed-file rejection. Real native-window interaction remains unverified.

For `8abf3ba0145bdb971496d80aa7dce9390f3ceb43`, the native Rust suite passed
322 tests (seven ignored), the frontend passed 128 tests and its build, and
both hosted test runs and the clean Linux ARM64 build passed. The hosted image
archive checksum and source revision were verified; its server binary was
layered onto the unchanged previous runtime to produce derived image
`sha256:a5556abfea7279ce7c22ebcedb0862d3eda00a02a70496c88f3d54d99e53b088`.
The disposable Linux tests reconstructed a text artifact larger than 2 MiB via
MCP, paged it in a browser, and rejected a same-size edit between pages. The
workflow and remote browser suites passed (43 browser tests, one skipped), as
did the PDF outage check. The worktree deployment passed restart acceptance,
MCP origin and authorization checks, and a local coding-model fixture. A
checked SQLite backup and the previous image were retained before deployment.
These results do not establish staging readiness or native-window parity.

An existing parallel Rust test failure was traced to a wildcard model-response
queue shared between unrelated test databases. Test-only wildcard responses are
now scoped to their SQLite pool; the previously interfering tests passed
together and a focused pool-isolation test passed. This changes no production
model routing.

The final macOS library suite passed 321 tests with seven ignored; both hosted
test runs and the clean Linux ARM64 build of
`d72db2e198ad06374859dd960cf097ed5c6a2769` passed. The image archive checksum
and source revision were verified. Its extracted server binary (SHA-256
`3c619237a4190fe52ebe12152b3901d2b988b0e050ba1fa11ec29384bce047f9`)
was layered over the unchanged previous branch runtime, yielding derived image
`sha256:db5a852f728518337d9d56043fa6a6c58e43748047fa15f3f7d0db13fad1ee42`;
this is not the complete hosted image. The candidate passed 53 browser/workflow
scenarios and the two-container transition and Git-delivery races. A disposable
Linux fixture checked valid, outside-directory, symlink and oversized artifact
reads through browser and MCP; the preceding image failed that fixture by
allowing the outside-directory MCP read. The fixture is available as
`bash scripts/test container-artifact-boundaries` and can select a candidate via
`ARUVI_TEST_APP_IMAGE`.

After deployment, product, MCP/mobile authorization, PDF and persistence
acceptance passed across restart, as did browser MCP-origin and real 7B MCP
coding checks. A checked SQLite backup and the prior image are retained. A
read-only compatibility count found no artifact records in this worktree
database, so it does not establish compatibility with historical user data.
The branch endpoint after acceptance was `127.0.0.1:55273`, subject to change
on restart. Staging, non-Unix race behavior and complete desktop parity remain
unverified.

### Current independent client, voice and dependency revalidation

The current worktree provider inventory contains the two Model Host entries and
the seeded LM Studio entry, with no configured OpenAI, Anthropic or DeepSeek
provider. The LM Studio connectivity check failed from the container using its
saved endpoint; this does not establish whether LM Studio is running on the Mac.
A separate TCP probe found that Mac loopback port 1234 was not accepting
connections at the time. Live LM Studio and third-party provider behavior
remain unverified.

After the Copilot branch deployment, the running container again completed a
real Model Host `piper-ljspeech` to `whisper-tiny-metal` round trip through MCP.
Headless Chrome also recorded synthetic Piper audio and sent it through the
browser transcription flow to Whisper. These checks do not test a physical
microphone or audible output. Desktop Execution settings now state that the
saved Docker host and maximum retry values are not currently used or enforced;
configured test commands run as the Studio server user. Implementing actual
retry policy and execution isolation remains open.

At `6d23c1d`, the shared-client pin/source check, running worktree MCP-origin
check, npm audit, and SQLx vendor-provenance check passed again. A refreshed
RustSec scan at database commit `55e1c3dfbd834237a05bbbf7b50ba3b5be27b662`
again reported `RUSTSEC-2023-0071`, 14 unmaintained warnings and two unsound
warnings. These are lockfile findings and retain the release-review limitations
described below. The Mac console reported locked during this revalidation.

On 2026-11-21, the independent consumer's manifest, lockfile and source-pin
checks passed; the pinned client source and manifest match Studio's current
crate. The Linux consumer rebuilt with 31 offline contract tests passing. Against
the running native Model Host integration environment, the consumer exercised
`qwen-05b-comparison` discovery, completion, streaming and token-limit errors,
and a synthetic `piper-ljspeech` / `whisper-tiny-metal` round trip. The running
Studio branch separately passed the real MCP speech round trip and a headless
Chrome recording of synthetic Piper audio followed by Whisper transcription.
These do not establish physical microphone or speaker behavior. An attempted
combined probe against the separate coding-validation host reported that the
voice models were not advertised there; the integration host advertises the
models used for the successful combined probe.
The Mac's console again reported a locked screen, so native-window acceptance
could not be repeated. The consumer build temporarily reduced the shared Docker
VM's available space below 1 GiB; its four identified build-cache records were
removed by ID, restoring approximately 1.9 GiB without changing the running
Studio containers or volumes.

`npm audit` reported no vulnerabilities, and `bash scripts/test sqlx-patch`
verified the vendored patch's pinned-source provenance. Cargo-audit 0.23.0 against
the Studio lockfile and RustSec commit
`55e1c3dfbd834237a05bbbf7b50ba3b5be27b662` reported the existing
`RUSTSEC-2023-0071` RSA lockfile vulnerability, 14 unmaintained and two unsound
warnings. The separately pinned consumer lockfile had no advisories or warnings.
The `chrono` unmaintained warning, `RUSTSEC-2026-0543`, is newly reported in
this scan. Studio uses chrono directly for timestamps, EPUB metadata and local
quiet-hours evaluation; a migration must preserve timestamp and timezone
behavior, including daylight-saving transitions. Cargo's all-target,
all-features tree selected chrono only through Studio and did not select RSA;
this is not a binary-level proof or coverage of other manifests. The local SQLx
patch remains outside cargo-audit's source review. These findings remain open
release-review items.

### Quiet-hours migration investigation

A native and Linux comparison of chrono 0.4.44 with Jiff 0.2.38 found that
the proposed replacement is not a drop-in change: with an invalid `TZ`, chrono
used the machine's configured local zone while Jiff fell back to UTC, and a
100 ms UTC timestamp formatted with three fractional digits in chrono but one
in Jiff. Valid New York DST transition examples and the tested hour/minute
strings agreed. The comparison was limited to those cases and is not a general
compatibility proof. The dependency has therefore not been replaced.

Quiet-hours evaluation now has native regression coverage for inclusive start,
exclusive end, midnight crossing, equal or missing boundaries and invalid
times. A separate process with an explicit POSIX timezone verifies both
occurrences of 01:30 during fall-back and the spring-forward gap. This protects
the tested existing behavior; Windows and other timezone configurations remain
to be validated when selecting a maintained replacement.

The macOS Rust library suite on `938391c0f8fb53e623013e25cc430b80a2b76f69`
passed 319 tests with seven ignored; both hosted test runs and the clean Linux
ARM64 container build passed. The archive checksum and source revision were
verified. Its extracted server binary (SHA-256
`583976fd3da24afae97a1e6e0f73fa73b3f0ce4e95c75893b50674da511ebe76`)
was layered over the unchanged preceding branch runtime, producing derived
image `sha256:835a7b810ecb947dc3e2b36fc2298e826fea5e91ad0b24249c952952662c27bd`;
this is not the complete hosted image. The candidate passed 53 workflow
scenarios, including browser quiet-hours configuration and enforcement, plus
the two-container workflow-decision and Git-delivery races. After deployment,
product, MCP/mobile authorization, PDF and persistence acceptance passed across
restart, as did browser MCP-origin and real 7B MCP coding checks. A checked
SQLite backup and the prior image are retained. The branch endpoint after
acceptance was `127.0.0.1:55261`, subject to change on restart. This does not
establish staging, complete desktop parity or a replacement for chrono.

### Real-model code review and test scope

Two consecutive disposable `qwen25-coder-7b-q4` extend-module workflows with
a work-item workspace override, prompt template and 8,192-token non-coding
budget reached test review. Both passed the configured JavaScript syntax check,
and the prompt template and edit guidance reached the model. Independent
network-isolated behavioral assertions rejected both: one omitted the existing
export and the other exported a function it had not defined. The disposable
remote remained unchanged, and the product's default workspace was untouched.
These are two failed samples, not a general measurement of the model.
A subsequent run used the same 7B model, extend-module task and workspace
override with JSON-object output enabled for the coding agent, no prompt
template, and a 4,096-token non-coding budget. It passed the syntax gate and
independent network-isolated behavioral assertions, preserved the default
workspace, and delivered the verified file to the disposable Git remote. The
configuration differs in several respects, so this single success does not
establish that JSON mode caused it or that it will reliably succeed. JSON mode
is an explicit per-agent boundary and depends on model support.

Browser Work now explains that a passing command covers only its checks and
that syntax alone does not verify behavior. Test review repeats this guidance;
Routing explains that automatic test-review approval depends on the configured
command. The setting's behavior is unchanged. The fixture's behavioral review
runs generated code in a separate network-free container. Studio's configured
workflow test command still runs as the server user with access to its files and
network; it is not an equivalent sandbox. Review and meaningful project tests
remain necessary before delivery.

On the guidance change, a disposable native server and headless Chrome checked
configuration, pending review and automatic-approval guidance. Both hosted test
runs and the clean Linux ARM64 image build of
`c8890530f5e42caaff20eae730e77554f474204a` passed; the later `0f78e5f`
commit adds a browser assertion without changing the runtime. The archive
checksum and revision were verified. The server binary (SHA-256
`3f575324a3ac51db6f45272e26895ec95f4b69c7873fd615f3ad50f8007508d4`) was
layered over the unchanged prior runtime to create derived image
`sha256:b0b21861165fc1db109c49bf8482656588978faddf5ef67fa518cabbf71340df`;
this is not the full hosted artifact. The candidate passed 53 workflow and 43
general browser tests; the separately covered token-rotation case was skipped
in the latter. PDF isolation and outage checks passed. After deployment,
product, MCP/mobile authorization, PDF and persistence checks passed across
restart, as did browser MCP-origin and real 7B MCP coding checks. A checked
SQLite backup and the previous image are retained. The branch endpoint after
acceptance was `127.0.0.1:55265`, subject to change on restart. These checks do
not establish staging or complete desktop parity.

### Database override restart validation

The native fixture now reconstructs a migration-52 database from a freshly
created database, adds a legacy channel binding, backs it up, and starts the
server to apply migration 53. The product and binding survived; integrity and
foreign-key checks passed. This is one reconstructed migration scenario, not a
test of all historical user databases or staging.

The expanded browser test exposed a race in which a delayed database-settings
response erased a path draft. Routing now tracks edits and ignores stale reads,
preserving a draft made before or during refresh and while a save is completing.
The test reproduced the original failure against the older binary and passed
with the native fix, including stable-origin restart and migration checks. The
running branch Docker image has not yet been rebuilt with this UI change.

The disposable native server fixture `scripts/test native-database-override`
now checks the authenticated database settings API across restarts. It verified
that an override takes effect only after restart, each temporary database keeps
its own products, clearing the override returns to the original database, and
`ARUVI_DB_PATH` takes precedence. Fresh databases reported applied migrations;
unauthenticated access was rejected. That initial run did not test a browser
session over restart, migration of an existing database, or staging.

The fixture was then extended with an optional persistent headless Chrome
session. It passed save and clear through Routing, checked the active path after
each server restart, and showed the environment-precedence message. Each
restart used a new loopback port, so the fixture re-entered its synthetic token
for that origin. A later run reused one dynamically chosen loopback port across
all restarts and passed the same checks after browser reloads without re-entering
the saved token. Existing-database migration and staging remain unverified.

### Isolated build filesystem cleanup finding

After the `e07ea27` host-backed Linux build, the loop device and build
container were gone, but macOS `lsof` still showed Docker Desktop's shared
Virtualization process holding the deleted 12 GiB sparse image open. Host free
space fell by roughly 3 GiB during that build and had not recovered when
checked; the exact allocated space held by the open descriptor was not measured.
The shared VM was not restarted because it also runs other projects.

A separate 64 MiB disposable ext4 probe wrote data, unmounted cleanly, then
truncated its backing file to zero inside the guest before unlinking it. The Mac
reported zero size and allocated blocks and no loop device remained. Future
isolated builds should truncate their owned backing image only after successful
unmount and confirmed loop release, before removing the host file. This probe
does not recover the already-deleted image or establish enough capacity for a
clean current Dockerfile build.

### Host-browser CI fixture

`bash scripts/test ci-host-fixture` runs the disposable Docker application and
simulated CI socket with host Playwright/Chrome, avoiding the browser-container
dependency installation. Against the current worktree image, its CI browser/MCP
test passed, including authenticated artifact download, streamed save with a
simulated picker, byte/digest verification and unauthenticated rejection. The
PDF renderer isolation check also passed. The CI service and save picker in this
fixture are simulated; real native-service and large-transfer evidence is
recorded separately below.

### Native real-browser PDF tests

At `806ee55`, the three ignored PDF tests were run explicitly on macOS with
the installed Chrome and local Playwright: desktop PDF export, database-backed
Product Book PDF generation, and blocking external fetches and page scripts all
passed. This uses a real headless browser and native worker; it is not a native
window interaction, actual reader compatibility check, staging acceptance, or
the network-isolated PDF container test.

### Reusing a worktree image without rebuilding

`bash scripts/env worktree up --no-build` now validates the worktree with the
shared launcher, requires the configured image locally, and starts it with
Compose build and pull disabled. It refuses configurations requiring database,
migration or seed hooks rather than silently skipping them. A fake-Docker test
covered image selection, scoped configuration, missing images, unsafe ports and
credential output; a live invocation left the existing app and PDF container
IDs and image unchanged and healthy. It does not establish that the selected
image matches the current source or replace a clean Dockerfile build.
At `c84bfe1`, a second disposable worktree started with this command alongside
the original, with distinct loopback ports, credentials and Compose volumes.
A product created in the disposable worktree was absent from the original.
After destroying the disposable project and removing its worktree, the original
services remained healthy and its authenticated product API responded. The
fixture image tag and private temporary state were removed.

### CI artifact and context-guard branch deployment

Source `e07ea27` built as an optimized Linux ARM64 server in a disposable
host-backed ext4 filesystem. The server SHA-256 was
`42291f46cf34dc87f9df92c026a085d50f97329a28d7dfb0a94299b6c4e2e3a4`.
The filesystem unmounted and was removed; the binary and build log are retained
in the private worktree artifact directory. The frontend artifact matched the
unchanged tracked frontend source. Layering the server on the previous worktree
image produced image ID
`sha256:6643b6bf13d69a02889f5be4d843586e66799f30bb122167c20b4117858264f1`.
This is a derived image, not a clean Dockerfile build or staging artifact.

Before replacement, 41 host-browser remote tests, PDF isolation and outage
checks, and 51 workflow tests with repository post-checks passed against the
candidate. The worktree database had no workflow, CLI or import rows. After a
graceful stop, no container mounted the data volume and no SQLite WAL or journal
was present; an offline copy passed `quick_check` and `foreign_key_check` using
immutable read-only access. The copy is retained with the build artifacts, and
the prior image is tagged `aruvi-studio-worktree-backup:pre-ci-artifact`.

The shared launcher attempted a full build on `worktree up`; it was interrupted
before startup because Docker storage was low. The same worktree Compose project
was then started with the tested image using `--no-build`. Both services are
healthy. Authenticated catalog and new CI MCP discovery, published-origin and
token rejection, the served hash asset, database quick-check, and a real 7B
Model Host coding request with token-limit rejection passed. Chrome checked all
13 remote sections at phone and desktop widths without page errors or document
overflow. Hosted run `44893429786` also passed for `e07ea27`. Docker's shared
filesystem still had about 218 MiB free after deployment; a clean image build,
actual CI service integration, native-window comparison and staging remain open.

### Agent repository-context symlink check

On Unix, context-file reads now use descriptor-relative `openat` with
`O_NOFOLLOW` for each component, reject hard-linked or non-regular files, and
bound reads even if a file grows. Candidate manifest entries are checked through
the same path. Two native tests passed for external symlinks and hard links and
for directory/file replacement before opening. Directory enumeration itself is
still path-based, and concurrent mutation can affect which files are seen; this
is not a filesystem sandbox. The non-Unix fallback retains canonical-path checks
and has not been validated against replacement races.

Automatic agent repository-context collection previously followed symlinked
files and directories, potentially including content outside the workspace in
a model prompt. It now skips symlink entries and checks canonical paths before
directory traversal and snippet reads. A native test confirmed that ordinary
external file and directory symlinks are excluded while an in-workspace source
file remains visible. The later descriptor-based Unix changes are described
above. The running branch Docker image does not yet contain these changes.

The running worktree's authenticated MCP repository list contained zero
registered repositories when checked; this does not inspect other environments
or unregistered workspaces.

### Workspace dependency-directory regression

The hierarchy terminology migration had incorrectly changed the filesystem
name `node_modules` to `node_product_areas` in the new-workspace `.gitignore`
and agent repository path filter. New workspaces now ignore the real dependency
directory, and agent context and tool paths again exclude it. The shared
workflow test helper was corrected too. Targeted native tests passed for the
agent path guard and for an MCP-created workspace's actual Git ignore behavior;
the terminology guard also passed. Existing workspaces are not modified by this
change and may need their `.gitignore` corrected separately. The running Docker
image has not yet been rebuilt with this fix.
The terminology guard now also rejects that malformed dependency directory in
scanned source. A temporary negative fixture was rejected, and the two targeted
workspace and agent path tests passed again.

### Current Studio native Rust library checks

At Studio branch revision `1c16562`, `cargo test --locked --lib` passed on the
Mac: 291 tests passed and seven were ignored. Ignored cases include tests
requiring configured browser or live model infrastructure. This exercises the
current native library, not the older running Docker image or staging.

### Model Host scoped-token branch revalidation

At Model Host branch `codex/scoped-inference-integration` revision `1302b93`,
`cargo test --locked --offline --lib --bin aruvi-model-host --test lifecycle`
passed ten Rust unit tests and 18 simulated lifecycle cases on macOS, including
scoped-token denial of protected routes and simulated streaming/media. Draft
[#12](https://github.com/vivasaayi/aruvi-model-host/pull/12) now records this
result. No deployed host was changed, and this does not replace a merged build
and integration acceptance.

### Hosted terminology guard correction

Hosted run `44891200558` at `9dde6f4` failed its terminology guard because the
vendored upstream SHA-256 library uses `typeof module` in its CommonJS wrapper.
That run reported zero statuses for the other five test/build suites. The guard
now permits only that exact expression in that exact vendor file. It passes
locally and a temporary product-source `module_id` fixture was rejected.
Newer hosted checks and a full current-revision run still need to finish.
Hosted run [44892301702](https://github.com/vivasaayi/AruviStudio/actions/runs/44892301702)
subsequently passed at `f41fbdf`; the run for `200427c` was still in progress
when checked. Neither is a check of the later guard change.

### Current host package and frontend checks

At branch revision `2409402`, the desktop frontend `npm run build` passed and
`npm run test:frontend` passed 128 tests. The independently packaged Rust model client passed all 65
tests offline using its generated package lockfile. An initial direct
`cargo test --locked` on the library source could not run because that library
does not maintain its own lockfile; the package test supplies one. These checks
do not validate live third-party providers, the native desktop window, the
currently deployed older Docker image or staging.

### Branch build capacity check

The running worktree app and PDF renderer were healthy, but the Docker shared
filesystem had about 300 MiB free and the Mac reported about 12 GiB free. A
targeted cleanup selected only unshared BuildKit records for this project's
older server-copy step; three records were reclaimed, leaving roughly 433 MiB
free in Docker. The remaining selected records were retained by BuildKit. No
other project cache, runtime data or image was removed. This remains inadequate
headroom for a full Docker build, and the running image still predates the CI
artifact changes. The branch's hosted checks were still installing dependencies
at this checkpoint.

### CI artifact parity investigation

For browsers implementing the File System Access API, CI now offers a streaming
save that hashes bounded chunks incrementally with vendored MIT-licensed
js-sha256 1.1.0 and closes the temporary writable file only on digest match.
The native Studio and Aruvici fixture passed a multichunk Chrome test with a
simulated save picker, including abort on changed content. The real picker and
deployed service have not been tested. A separate synthetic browser check
verified 268,435,457 bytes in 4,097 writes without retaining output in browser
memory; both its provider responses and save picker were simulated. Browsers
without that API still have the 256 MiB buffered-download limit.

A subsequent disposable native debug Studio and Aruvici test attempted to
stream a real 268,435,457-byte file through the CI socket. It exceeded the
five-minute browser test timeout and did not establish completion; the fixture
and temporary file were removed. The JSON byte-array chunk path's performance
needs investigation, including comparison with optimized builds. The test now
reports periodic progress and timeout byte counts for a future run.

The protocol branch now buffers socket writes, uses blocking accepted sockets,
and offers an optional 512 KiB base64 chunk; Studio falls back to the legacy
64 KiB byte-array method. A repeat with native debug binaries successfully
streamed and verified the real 268,435,457-byte file through the socket in 513
writes, and the synthetic legacy fallback also passed. The save picker remained
simulated. The 11 Aruvici CI API tests passed. This does not validate the deployed
CI service, actual picker, Docker image or staging.

A separate disposable fixture sent a full 512 KiB artifact chunk through a
second native Studio server, the authenticated loopback relay and the Aruvici
protocol-branch service. It verified the decoded bytes and rejected an invalid
relay token and a disallowed deploy request. This exercises the relay path, but
not a deployed container-to-host relay or a full large transfer through it.

The separate Aruvici protocol branch now has draft PR
[#1](https://github.com/vivasaayi/aruvici/pull/1). At `33e5a08`, it also
requires chunk paths to belong to the artifact's owning run. Its
`cargo test --test ci_api` passed all eleven tests on the Mac, including a
cross-run artifact rejection, guarded reads and retry policy. This is a
reviewable dependency, not a merged or deployed service.
The real foreground-service test also verified a complete 512 KiB base64
response over the Unix socket; `cargo test --test ci_api --test ci_service`
passed all 12 tests.

The original buffered browser CI artifact download uses authenticated chunk
requests, verifies length and SHA-256 before saving, requires HTTPS or localhost,
and buffers at most 256 MiB. A native Studio and real Aruvici protocol-branch fixture passed a Chrome
download of a 76,800-byte file, rejected modified bytes and denied an
unauthenticated chunk request. The later streaming option above addresses larger
files on supporting browsers; the deployed service and staging remain unverified.

MCP now exposes artifact metadata and bounded chunk reads. A freshly built
native Studio server and the Aruvici protocol-branch binary passed a disposable
76,800-byte multi-chunk transfer with byte, length, offset and SHA-256 checks;
the fixture also rejected a negative offset. Deployed service validation remains
open. The native build emitted existing warnings.

The desktop can reveal a verified CI artifact, whereas the browser and MCP
previously exposed readiness but not its bytes. Aruvici supports bounded artifact
chunks, and the Studio relay allows that operation. The Studio
`promotion_readiness` protocol extension is in Aruvici branch
`codex/studio-ci-protocol`, not the inspected local `main` checkout.
The running service's version
and staging compatibility still need verification. See the parity audit.

### Repeatable host-browser fixtures

The disposable remote and workflow fixtures now have supported host-browser
entrypoints that share their setup and post-checks with the container tests.
Against the branch image described below, host Chrome passed 41 remote tests,
the PDF outage check, and 51 workflow tests with repository post-checks. The PDF
container's isolation settings were also verified. A port-conflict check failed
cleanly without displacing the existing listener and removed its fixture
resources. These tests use a mock provider; they do not establish a clean image
build, staging acceptance or complete desktop feature parity.

### Reference-management branch deployment

Source revision `cdca020` was compiled as an optimized Linux ARM64 server in a
disposable ext4 filesystem with the existing project Rust image. The binary
SHA-256 was `6dfd08da1b391c12a69f15f91f3d545ca5b902e6bae5ab97b238c6a8cdca5484`.
The filesystem unmounted cleanly and was removed. Layering the binary onto the
preceding branch runtime produced image ID
`sha256:7bb772361e9946da8ea8a96cb29c9b66256de5947edfe278e986291cbc14ce7f`;
this derived image is not a clean Dockerfile build or staging artifact.

The usual browser-container fixture filled the nearly full shared Docker store
and was stopped; its temporary containers and volumes were removed. A subsequent
fixture used host Chrome and the local Playwright installation with disposable
containers and a loopback mock provider. After correcting a missing fixture file
and its host/container networking, all 41 remote tests passed. A similarly
adapted fixture passed 51 workflow tests and repository post-checks. A separate
no-network, read-only PDF fixture generated a PDF and failed closed when its
renderer stopped. These adapted tests are not the unmodified container test
script; Docker's shared store remained low on space.

With no active workflow, import or CLI run, the SQLite backup passed
`quick_check`; the previous image is retained as
`aruvi-studio-worktree-backup:pre-reference-management`. After deployment, image
identity, authenticated product/MCP and PDF checks, a real local coding-model
request and browser-origin/authentication checks passed. The backup and build
log are stored in the private worktree artifact directory.

### Product-area purpose parity

The browser can now set and edit product-area purpose, matching the desktop
form. Updates compare the previously displayed purpose when supplied; older
name/description-only requests leave it unchanged. A disposable browser/MCP
hierarchy test passed creation, a concurrent purpose conflict, refresh and
successful editing, along with the existing capability detail checks. The
running branch image does not yet include this change.

### Browser reference management

The browser reference view now creates and deletes references at product, area,
capability and feature scopes. Creation checks product ownership and node kind in
the insert; deletion requires the displayed reference snapshot and rechecks
product ownership. A disposable browser/MCP fixture passed capability-scoped
creation and deletion, authentication, wrong-product and stale-delete rejection.
It also retained the earlier safe-link and pagination checks. The subsequent reference-management branch deployment is recorded above. Generic MCP and desktop reference actions
retain their existing behavior.

### Browser product reference viewing

The browser product overview and hierarchy now expose on-demand, authenticated
reference lists for product, product-area, capability and feature scopes. The
server verifies product ownership and node kind, and returns pages of 50. The
browser renders content as text and links only HTTP(S) URLs without credentials.
A disposable browser/MCP test covered product and capability references,
unauthorized and cross-product/wrong-kind requests, unsafe URL rendering and
52-item pagination. Offset paging can shift if records change between requests;
creation and deletion were subsequently added as recorded above. The native server built,
and the subsequent reference-management branch deployment is recorded above.

### Product editor branch deployment

Source revision `49fba35` was compiled as an optimized Linux ARM64 server in
a disposable ext4 filesystem with the existing project Rust image. The binary
SHA-256 was `6121df5085d9cff3ca3932a64480ba6a288abc779792778381606a5d7bc6c73d`.
The filesystem unmounted cleanly and was removed. The binary was layered onto
the prior branch runtime, producing image ID
`sha256:ed806bebdbd99bbbdb1e82ee30c30e088bb255aad40500a47086126e5965608f`.
This derived image is not a clean Dockerfile build or staging artifact. It
passed 40 browser tests, PDF isolation and outage checks, and 51 workflow tests.
With no active workflow, import or CLI run, the SQLite backup passed
`quick_check`; the previous image is retained as
`aruvi-studio-worktree-backup:pre-product-editor`. After deployment, image
identity, authenticated product/MCP and PDF checks, a real local coding-model
request and published-port origin/authentication checks passed. The backup and
build log are stored in the private worktree artifact directory.

### Concurrent partial product updates

Shared product updates now write only explicitly supplied fields in a single
SQL statement, avoiding the previous read-then-rewrite race between partial
MCP, desktop or planner updates. A native MCP test queued two different-field
updates behind a SQLite write lock and verified that both values survived; the
product-focused suite passed 33 tests with four ignored. Conflicting updates
to the same field remain last-write-wins outside the snapshot-checked browser
editor. The running branch image has not yet been rebuilt with this change.

### Browser product detail editing

Browser Products now edits the desktop product detail fields through an
authenticated, snapshot-checked update transaction. A disposable browser/MCP
fixture verified all fields, a concurrent update and preserved browser draft,
refresh and retry, unauthorized access, identity and status tampering, and an
empty name. The browser uses comma-separated goals and tags. Existing desktop
and generic MCP updates remain last-write-wins for the same field; partial updates
were subsequently hardened as recorded above. The native server built, but the running Docker
branch has not yet been rebuilt with this change.

### Hierarchy conversion audit

The existing shared kind-conversion operation previously validated and wrote
through separate database operations. Its reads, validation and writes now use
one immediate SQLite transaction. A native MCP test verifies that an invalid
kind conversion and an injected update failure preserve the existing child,
and that a same-kind operation succeeds. Product-related native tests passed
(32 passed, four ignored). The first proposed reparent test revealed a scope
constraint rather than reaching its write: canonical product areas accept only
capabilities and capabilities accept only features. Consequently, converting
between these kinds in a valid tree is rejected by the parent rule, and the
child-reparent branch has not been exercised. No browser conversion control
has been added; deciding whether conversion should move nodes or change the
canonical model remains open. The running branch image predates this change.

### Product hierarchy detail parity

The browser capability/feature editor now supports acceptance criteria and
technical notes, which were present on the desktop editor. The authenticated
server checks expected values and product ownership; legacy name/description
requests retain their previous behavior. The native server built and a
disposable browser/MCP test passed field editing, stale and cross-product
rejection, and missing-expected-value rejection. Other hierarchy actions remain
to be compared, and the running branch image does not yet include this change.

### Real Model Host new-module workflow revalidation

The running worktree image from `4f26c4b` completed the `new-module` coding
workflow using the native Model Host's `qwen25-coder-7b-q4` and a managed
coding prompt template. The disposable fixture passed the plan and test-review
gates, syntax checking and independent network-isolated behavioral assertions,
then delivered to a disposable local Git remote. The model initially requested
a missing directory and produced a final response before writing; the existing
workflow feedback led it to continue. This is one successful run after an
earlier failed new-module attempt, not a reliability estimate or proof of an
authenticated network Git push. The latest browser-only capability selector
was not in the running image used for this test.

### Portfolio dependency capability selection

The browser dependency form now loads named capability choices for each
selected product instead of requiring pasted IDs. A request generation check
discards late responses when the product changes, and failed loads disable
submission until retried. A disposable browser/MCP fixture verified both scoped
choices, late-response isolation, failure and retry, and the resulting
capability IDs. Existing dependency rows may still show IDs. This UI change has
not yet been rebuilt into the running branch container.

### Portfolio guard branch deployment

Source revision `4f26c4b` was compiled as an optimized Linux ARM64 server
in a disposable ext4 filesystem using the existing project Rust image. The
binary SHA-256 was `4165f372c04a74e9664dd583047f6792a04fb6db690de9e6b2d30b964fd036af`.
The temporary filesystem unmounted cleanly and was removed. The binary was
layered onto the preceding branch runtime, producing image ID
`sha256:a778fee372e60097281c59c68b64e530cf318b53e7a20552f326519a9f14bcaf`;
this is a derived image, not a fresh full Dockerfile build or staging artifact.
It passed 39 browser tests, including the portfolio guards, PDF isolation and
outage checks, and 51 workflow tests. With no active workflow, import or CLI
run, a SQLite backup passed `quick_check`; the previous image is retained as
`aruvi-studio-worktree-backup:pre-portfolio-guards`. After deployment, image
identity, authenticated product/MCP and PDF checks, a real local coding model
request, and browser-origin authentication tests passed. The backup and build
log are stored in the private worktree artifact directory.

### Branch validation against current default

At `fbc21a4`, the branch had no commits to incorporate from `origin/main` and
the PR was reported mergeable but remained a draft with CI still running. Local
checks passed the terminology guard, 128 frontend unit tests, 289 native Rust
library tests (seven ignored), and all 46 mocked-Tauri desktop browser tests.
The browser suite uses simulated desktop commands and is not a native-window
acceptance test. The subsequent portfolio-guard deployment and its container validation are
recorded above. These results are not staging acceptance.

### Standalone client package validation

At Studio commit `ca80520`, the reusable client was packaged and verified
offline with Cargo; its unpacked package then passed all 65 client tests offline
on macOS ARM64. The archive includes the crate source, example, README and
license. `bash scripts/test shared-client-package` repeats that local check
with cached dependencies; the separate Git-pinned consumer fixture and Linux
validation remain distinct. This did not publish a package or establish an
independent repository, release policy, live third-party compatibility or voice
device acceptance.

### Product strategy link conflict protection

Browser link and unlink actions submit the product's displayed link set. The
server requires it and compares it inside an immediate SQLite write transaction
before changing any link or primary assignment. Legacy desktop and MCP calls may
omit the optional snapshot and remain last-write-wins. Backend tests cover stale
link and unlink requests and verify the primary link is preserved. A disposable
browser/MCP fixture passed stale primary-assignment and unlink conflicts, followed
by successful refresh and retry. The subsequent portfolio-guard deployment is recorded above.

### Strategy subtree deletion guard

Browser strategy deletion now confirms the number of loaded descendant nodes
and linked products. The server requires that subtree and link snapshot, then
compares it and deletes in one immediate SQLite write transaction. A browser
fixture exercised a concurrent descendant addition, rejected the stale delete,
then refreshed and successfully deleted the confirmed subtree. A backend
regression covers both new descendants and new links. Legacy desktop and MCP
callers can still delete without snapshots; guarded MCP calls are optional.
Browser product-link operations were subsequently guarded as described above.
The running branch container does not yet include these changes.

### Portfolio branch deployment

Source commit `765a128` was compiled as an optimized Linux ARM64 server in a
disposable 18 GiB ext4 filesystem, using the existing project-owned Rust build
image. The binary SHA-256 was
`530670f1bff05889cd92ebf6ce568ab7256e64f8a1f6c85c615e89d642ad4737`.
The build completed; cleanup initially returned an unmount error because the
working directory was inside the mount. The build container exited, no loop
devices remained, and the temporary filesystem was removed. The frontend build
passed. This was a separate server build, not a fresh complete Dockerfile build.

The binary was layered onto the preceding branch runtime image. The candidate
image ID is `sha256:d44fb01481f69f16dbc459a69900b37bd4b1182e998f682e766e31c9fb29d30b`.
It passed 39 isolated browser tests, PDF isolation and outage checks, and 51
workflow tests. With no active workflows, CLI runs or imports, the SQLite
backup passed `quick_check`; the previous image is retained as
`aruvi-studio-worktree-backup:pre-portfolio`. The candidate was deployed to
the isolated worktree. After deployment, its image identity, product and PDF
acceptance, a real local Model Host coding request through MCP, and published
port origin/authentication checks passed. The backup and build log are in the
worktree environment's private artifact directory. This is not staging or a
release artifact; the derived image is distinct from the earlier clean OCI build.

### Strategy edit concurrency

Strategy creation and updates now validate hierarchy inside an immediate SQLite
write transaction, and browser strategy edits submit their displayed node as an
expected snapshot. The native backend regression and a disposable browser/MCP
fixture verified that a stale browser edit is rejected without losing the draft,
and that refresh permits a new edit. Desktop and MCP callers that omit the optional expected snapshot remain
last-write-wins; browser subtree deletion and product-link operations were
subsequently guarded as recorded above. This is not complete concurrency
protection. The subsequent portfolio branch deployment is recorded above.

### Browser portfolio management

Products now includes browser controls for strategy nodes, product links and
dependencies through an authenticated endpoint restricted to portfolio actions.
Dependency capabilities must belong to their respective products; strategy edits
reject cycles and incompatible child kinds. Tests cover authorized and denied
API actions, both wrong-product capability sides, valid references and invalid
hierarchy edits. A disposable browser fixture exercised strategy creation,
product linking, dependency creation and confirmed deletion via the browser,
with dependency reads checked through MCP. Editing and reparenting use the
existing last-write-wins backend; users should refresh before editing, and
concurrent hierarchy writes are not serialized. Capability selection was subsequently improved as recorded above. The subsequent portfolio branch deployment is
recorded above.

### Portfolio strategy MCP gap

A source audit found that the desktop Portfolio strategy hierarchy, product
links and dependencies were absent from the web/MCP parity inventory. The
`aruvi_catalog` namespace now exposes those shared backend operations; primary
strategy-link replacement is transactional. A disposable MCP-dispatch test
covers creation, editing, linking, dependency creation, deletion and validation
failures, including rollback of an invalid primary replacement. A disposable
native server also passed authenticated HTTP MCP discovery and strategy creation,
with unauthenticated access rejected. The full native Rust library suite passed
289 tests with seven ignored. The browser management interface and capability/product scope checks remain to be done.
The subsequent portfolio branch deployment is recorded above. See
`web-parity-audit.md` for the remaining scope.

### Deployment acceptance image identity

The environment acceptance script now verifies that the app and PDF containers
run the same Docker image ID and can require an explicit full expected ID,
checking it again after an optional restart. An intentionally incorrect ID was
rejected before creating any fixture. Against the running worktree, the expected
derived image ID matched and the authenticated MCP, mobile-read, PDF and
fixture-cleanup checks passed. This run did not restart the services or test
external ingress. An image ID must still be matched to a separately verified
source revision and target-platform artifact; this is not staging acceptance.

### Model registration form comparison

The desktop and browser forms both expose provider name, protocol/profile, base
URL and credential, plus model provider, name, context window, tags, notes and
enabled state when editing. Browser model registration now also offers the four
desktop model presets. Unlike the desktop shortcuts, these only fill model fields
and leave the selected provider unchanged; users must select a compatible
provider. A headless Chromium check loaded the updated HTML against the running
branch origin and verified all four presets, their fields and provider selection.
The rebuilt native server passed the browser model-configuration regression in
a disposable database, including all presets, model creation, editing and
conflict handling. It did not test third-party availability. The subsequent portfolio branch deployment is recorded above. Other management
operations and native-window behavior remain to be compared.

### Clean Dockerfile build and shutdown finding

At branch commit `7143479`, a no-cache Linux ARM64 build of the actual
Dockerfile succeeded using a temporary BuildKit worker and an isolated ext4
filesystem image on the Mac; the shared Docker store had less than 1 GiB free.
The resulting OCI manifest was
`sha256:cc06e802bfd20a52911e319bbed0c804946337166a29e8a197a8d61da85605a7`.
In a disposable OCI runtime fixture, the clean image started as its configured
non-root user, authenticated browser navigation and product creation passed,
and the separately launched PDF renderer generated a valid PDF. The fixture
retained its product after a forced restart; authenticated MCP work creation,
PDF export and unauthenticated-read rejection passed afterward. This was a
fixture, not a Compose deployment, staging acceptance, or a release artifact.

The restart check found that the server process, running as PID 1, remained
alive after SIGTERM and needed SIGKILL. Commit `e9d4b74` adds headless HTTP
graceful shutdown for SIGTERM and SIGINT. The native Rust suite passed (288
passed, seven ignored), including an in-flight HTTP response regression; the
headless process regression also passed both signals and verified SQLite and
product persistence after restart. Background workflows, imports and external
subprocesses are not drained by this HTTP shutdown path; check active work
before restarting.

A second no-cache Dockerfile build from `e9d4b74` produced Linux ARM64 OCI
manifest `sha256:cfd94c81e84812234435b5d1a42d58a0ac1c1ef9da0072d67f6187c6727458a8`.
The retained OCI archive has SHA-256
`fc31913e6e76f8442aacd58e7819eef603d6a746619c338a17199c9655d5298f`.
In an isolated OCI runtime fixture, eight browser scenarios, PDF export,
SIGTERM as PID 1, persistence after restart, authenticated MCP work creation
and unauthenticated access rejection passed. This clean artifact is retained in
the worktree environment's private artifacts directory; it is not a staging or
release artifact.

Because Docker Desktop's shared store lacked room to load the clean OCI image,
the branch deployment uses a derived image containing its verified server binary
on the prior branch image (image ID
`sha256:550de0b970369f053353b97393e1d83a77045220b34dcde8f55bf30e0031e690`).
The derived candidate passed 38 remote-interface and 51 workflow tests, PDF
isolation and outage checks, and a Docker PID 1 SIGTERM test with exit code 0.
Before deployment, active work was checked, a SQLite backup passed
`quick_check`, and the prior image was retained as
`aruvi-studio-worktree-backup:pre-signal-fix`. The branch Compose services are
healthy. The clean OCI fixture and derived branch deployment are distinct and
neither constitutes staging acceptance.

### Scoped Model Host candidate revalidation

Model Host [PR #12](https://github.com/vivasaayi/aruvi-model-host/pull/12)
remains a draft based on the current remote default branch. Its separate
candidate worktree now derives scoped-credential denial checks from all
protected router declarations and tests seven HTTP methods, with explicit
positive coverage for the five allowed inference method/path pairs. The check
also requires review if the router gains composed or nonliteral routes.
The refreshed macOS host build passed ten Rust unit tests, 18 simulated lifecycle
tests and seven native adapter tests. Disposable native and Docker fixtures using
the current Studio branch passed scoped MCP completion and management denial;
the watcher fixture retained its provider across a token change before restart
and recovered when the restarted host accepted it. No live host was changed.

The original Model Host main checkout still has its pre-existing uncommitted
environment work. A fresh comparison found those files retained in the candidate;
the differing README, native adapter and adapter test include the scoped-token
changes. The candidate is pushed through `1302b93` but has not been merged or
deployed. The current host still requires the management-capable token; use of
an inference-only credential remains contingent on a merged host revision and
integration acceptance under the environment contract.

### Anthropic stream initial-content guard

The shared client previously ignored `message_start.message.content`, so a
proxy could send initial text or a tool-use block followed by a text delta and
the client would report only the delta as a complete answer. A regression
fixture reproduced this. Initial content must now be absent, null, or an empty
array; a nonempty or malformed value fails the stream before dropping content.
Nested initial-message errors also fail without exposing provider details.
The native Claude API stream begins with an empty content list, as shown in the
[Anthropic streaming reference](https://platform.claude.com/docs/en/build-with-claude/streaming).
Some nonstandard proxies may now need to send content as block events instead.
The standalone shared-client suite passed 65 tests. The independent Git consumer
was updated to `dc711d0c7e5e869133544de17a811d6afdb560d3` and passed 30 native
tests and the same 30 tests as a non-root Linux user with networking disabled
after vendoring. A separately built, read-only non-root Linux probe of that pin
passed real Model Host discovery, completion, streaming and token-limit checks
against `qwen25-coder-7b-q4` and `qwen-05b-comparison`, plus a synthetic
`piper-ljspeech` to `whisper-tiny-metal` round trip. Studio's native library
suite passed 287 tests with seven ignored. The Linux ARM64 release binary from
source commit `dc711d0`, packaged on the previous worktree image rather than
built from the Dockerfile, passed 38 remote browser tests, PDF isolation and
outage checks, and 51 workflow tests; the workflow fixture confirmed the browser
reports the malformed Anthropic stream. Image
`sha256:356698a34954bc1963f37ac0fe24ecbd70ae56f47a928cd744bfc56cee5373c5`
was deployed to the isolated worktree after confirming no active CLI, import or
workflow jobs and taking a consistent database backup that passed `quick_check`.
The previous image remains tagged `aruvi-studio-worktree-backup:pre-initial-stream`.
Both services were healthy after restart; a real Model Host coding completion,
token-limit error and browser-origin MCP check passed. The loopback port is
dynamic. No live Anthropic credentials were configured, and this is not staging
acceptance or a clean Dockerfile build.

### Current real Model Host revalidation

On the branch deployment at source commit `67ecacd`, the real
`qwen25-coder-7b-q4` model returned an MCP coding completion that passed
independent JavaScript assertions; a one-token request produced the expected
truncation error. Authenticated browser streaming chat also returned real model
output, and the published-port MCP origin fixture rejected missing credentials
and the tested cross-origin requests. A synthetic `piper-ljspeech` to
`whisper-tiny-metal` round trip passed through Studio MCP.

The disposable real-model `extend-module` workflow now supports assigning a
managed coding prompt template and verifying its prompt artifact. The first
attempt reached coding but generated JavaScript rejected by the configured
syntax gate; the workflow stopped before its delivery approval. The first
version of the fixture did not independently check the remote on that error
path. The fixture was extended to check the prompt and remote on failure. A
second attempt verified the template in the coding prompt, passed the syntax
gate and independent behavioral assertions, and delivered the verified file to
the disposable local Git remote. A deliberate invalid-syntax control initially
failed its own remote check because it incorrectly expected the pre-existing
file to be absent. After fixing the fixture to compare the original file, a
repeat verified the template, confirmed the test gate rejected the invalid file,
and confirmed the disposable remote remained unchanged. The fixture containers
and their volumes were removed. These attempts are examples, not evidence of broad model reliability,
live third-party providers, physical audio, or native-window acceptance.

### Agent prompt templates

The existing template table was unused, and the branch database's nine seeded
agent references had no matching templates. Browser and MCP now manage versioned
templates, while shared agent execution applies only explicit `template:<id>`
references; older references retain their previous inactive behavior. A template
matches the agent role or `*`, and missing or mismatched templates fail a stage
instead of silently changing its behavior. Instructions are limited to 1,000
characters and retained during subsequent coding turns. Tool permissions and
execution boundaries remain independently enforced. An assigned template cannot
be deleted; concurrent template edits require the current version. The native
agent execution fixture verified the instructions in initial and later coding
turns, and the management fixture checked authorization, role mismatch, legacy
references, stale edits and assigned-template deletion. The Rust library suite
passed 287 tests with seven ignored; 128 frontend tests, the frontend build and
terminology check passed. The Linux ARM64 release binary from source commit
`67ecacd`, packaged on the previous image rather than built from the Dockerfile,
passed 38 remote browser tests, PDF isolation and outage checks, and 51 workflow
tests. The browser fixture also assigned a template to an agent and checked the
deletion guard. Image
`sha256:1ab28f89376d7e0c6a73c60dc75860ae9bc4d97eeb3484c5124f0404341b1115`
was deployed to the isolated worktree after confirming no active CLI, import or
workflow jobs and taking a consistent database backup that passed `quick_check`.
The previous image remains tagged `aruvi-studio-worktree-backup:pre-prompt-templates`.
Both services were healthy after restart; template UI controls, MCP discovery and
an authenticated template-list call passed. The loopback port is dynamic. This
is not staging acceptance or a clean Dockerfile build.

### Browser work-item ordering

The browser can drag or use move buttons to reorder an unfiltered product
backlog. A guarded browser/MCP move validates the current contiguous interval,
then normalizes product order in one transaction, retaining the relative order
of unloaded items. The old desktop/MCP reorder now commits its writes together,
but remains last-write-wins and does not validate product scope. Moves are
limited to 1,000 loaded rows at a time. Backend tests exercise authorization,
cross-product and stale conflicts, rollback, MCP, and a page-boundary move;
a native browser fixture exercises drag, buttons, filter restriction and stale
conflict. A separate in-memory SQLite check reordered 50,000 synthetic rows in
approximately 0.07 seconds on this Mac. The native Rust suite passed 286 tests
with seven ignored; 128 frontend tests, the frontend build and terminology check
passed. A Linux ARM64 release binary from source commit `3726819`, packaged on
the prior worktree image rather than built from the Dockerfile, passed 37 remote
browser tests, PDF isolation and outage checks, and 51 workflow tests. Image
`sha256:ca9973e87e7711f0518b67df33f95f54679e7b7c8195246bb8a6349efae2d35f`
was deployed to the isolated worktree after confirming no active CLI, import or
workflow jobs and taking a consistent database backup that passed `quick_check`.
The prior image is tagged `aruvi-studio-worktree-backup:pre-work-order`. Both
services were healthy after restart; browser controls, MCP move discovery and an
authenticated catalog call passed. The loopback port is dynamic. This is not
staging acceptance or a clean Dockerfile build.

### Browser backlog batch decisions

The Work view now filters by status and supports selecting up to 100 loaded
items for approval or rejection. The browser and MCP use one transactional
service that checks every full snapshot, product scope and active execution;
a conflict rolls back the entire batch. Automatic starts are requested after
commit according to the saved policy. A native browser fixture exercised the
filter, stale conflict, rollback and successful decisions. The Rust library
suite passed 285 tests with seven ignored; 128 frontend tests, the frontend
build and terminology check passed. The Linux ARM64 release binary from source
commit `81a49ad`, packaged on the previous worktree image rather than built
from the Dockerfile, passed 36 remote browser tests, PDF isolation and outage
checks, and 51 workflow tests. Image
`sha256:a5e98d3b33ad8fe6c40c062600e413de2e61f7a5c54205bc113deb3d0d9c272a`
is deployed to the isolated worktree. Before restart there were no active CLI,
import or workflow jobs, and a consistent database backup passed `quick_check`.
The previous image remains tagged `aruvi-studio-worktree-backup:pre-batch-decisions`.
After restart both services were healthy; browser controls and an authenticated
MCP catalog call passed. The loopback port is dynamic. At that checkpoint drag ordering, a clean
Dockerfile build and staging acceptance remain open.

### Browser work-item decisions

The browser Work view now supports individual approval and rejection as well
as the existing explicit approve-and-start action. Approval follows the saved
automatic-start policy; rejection requires a reason and returns the item to
draft. Browser decisions compare the full displayed work-item snapshot and
reject active workflow or CLI conflicts. The desktop, MCP and browser task
decision paths now update status and insert the approval record atomically.
Desktop and MCP retain their existing automatic-start behavior.

An injected database failure verified rollback through mobile API and MCP paths;
the native browser and isolated container browser fixtures exercised approval,
stale rejection and rejection after refresh. The native Rust suite passed 284
tests with seven ignored; 128 frontend tests, the frontend build and terminology
check passed. A host-backed Linux ARM64 release binary packaged onto the prior
worktree image passed 35 remote browser tests, the PDF outage check and 51
workflow tests. At that checkpoint, bulk decisions, status filtering and drag ordering remained open;
this is not a clean Dockerfile build or staging acceptance.
After source commit `c76ead8`, image
`sha256:fa1d7a23f1b314b5ac563fd53d02316d898f1a2603d2b3317e68702dac8bbc40`
was deployed to the isolated worktree. A consistent database backup passed
`quick_check` after confirming no active jobs; the previous image remains as
`aruvi-studio-worktree-backup:pre-task-decisions`. App and PDF health, the new
browser controls and authenticated MCP catalog access passed after restart.
The published loopback port is dynamic.

### Multi-process startup recovery

Starting a second Studio process against the same SQLite database previously
marked a live external CLI run and a running bulk import as interrupted. Unix
processes now hold a shared database-specific startup lock; only the first
cooperating process performs recovery. CLI recovery's event insertion and state
update are transactional. A native fixture reproduced the original bug, then
passed with two processes, a live fake CLI, a synthetic running import, and
simulated interrupted records after both processes stopped. A Linux ARM64
fixture passed the same checks using two containers and a shared named volume.
Lock takeover, symlink rejection and transactional rollback tests passed; the
native library suite passed 283 tests with seven ignored. The locking and
orphan-process limitations are described in `headless-server.md`.
The Linux candidate was built with host-backed storage and packaged on the
previous worktree image; it passed 34 isolated browser tests, the PDF outage
check and 51 workflow tests. This is not a clean Dockerfile build or staging
acceptance.
After source commit `9b11ceb`, image
`sha256:b7b6bb7dcbd72fb8e32b40b5c873f0c5e1be04720c4e4912fa0baec365068501`
was deployed to the isolated worktree. The previous image is retained as
`aruvi-studio-worktree-backup:pre-cli-recovery`; a consistent database backup
passed `quick_check` after confirming no active workflow, CLI run or import.
The app and PDF sidecar became healthy, authenticated MCP catalog access
passed, and the new lock file was present. The loopback port is dynamic.

### External CLI browser and MCP migration

The work item External CLI tab now has opt-in browser and MCP operations using
the shared Rust launcher. Remote execution requires an operator-supplied CLI
home and does not inherit server environment variables. Output and run history
are bounded, and same-item active runs block conflicting launches, workspace
changes, deletion and new workflows. This is not a sandbox or coordination
between different items that share a workspace. The image still has no actual
CLI executables or provider credentials; see `headless-server.md`.

A disposable fake-CLI fixture passed through native and Linux container browser
and MCP paths, including scope override, disabled mode, authentication,
output limits, failure and restart persistence. The final native library suite
passed 278 tests with seven ignored. The frontend suite passed 128 tests, and
the frontend build and terminology check passed. A host-backed Linux ARM64 release
binary was packaged on top of the previously built worktree image; that
candidate passed 34 remote browser tests, the PDF outage check and 51 workflow
tests. This is not a clean Dockerfile rebuild, real CLI-provider acceptance or
staging validation. The shared Docker disk remained near capacity.

The candidate was then deployed to the worktree as image
`sha256:d44c18821350ee96a39c685e058532953e232baac43effd063d3db36e3a1cc78`
after source commit `dee2555`. The prior image was retained as
`aruvi-studio-worktree-backup:pre-external-cli`, and a consistent SQLite
backup passed `quick_check` before the switch. App and PDF services became
healthy; the updated browser UI and authenticated MCP action discovery passed,
and an authenticated launch request confirmed remote execution is disabled by
default. The published loopback port is dynamic. This worktree deployment does
not establish real CLI-provider integration or stable staging acceptance.

### Integration adapter scoped-token preparation

The scoped-integration worktree's existing native candidate binary was tested
again with the current `c88773f`-based Studio worktree image and a temporary
copy of the real `qwen-05b-comparison` model. A fresh inference-only token was
denied Model Host management, while discovery, Studio MCP completion and
general Copilot, browser API streaming and headless browser general Copilot
streaming passed. The fixture now exercises both general-Copilot paths as well
as chat. Its temporary host, model state and Studio container were removed;
neither deployed service was changed. This is one small model and does not
establish production readiness or deployed scoped-credential support.

The existing untracked Model Host `deploy/compose.integration.yaml` now passes
the optional `MODEL_HOST_INFERENCE_TOKEN` into the app, matching the candidate
worktree, staging and production adapters. A temporary Compose configuration
check confirmed that a supplied inference token and the management token are
passed separately, and that an omitted inference token defaults to empty. This
is a configuration check only: the integration adapter and other host checkout
changes remain uncommitted, and the scoped-token host branch has not been merged
or deployed. No running service was changed. Reconcile and review the host
changes together before building and testing the merged integration environment.

A later disposable three-way reconciliation of host candidate `ffb4cfb` and
the host main checkout's tracked changes applied cleanly except for one README
conflict. The temporary resolution kept the newer integration/staging status
and the scoped-token instructions; the two untracked adapter/document files
were copied into the snapshot. Seven native adapter tests, the integration
Compose configuration check and 17 simulated-worker lifecycle tests passed.
The lifecycle test used the existing candidate host binary rather than rebuilding
the combined tree. The snapshot was removed; neither host checkout was merged
or committed, and this does not establish a deployed integration service.

The same reconciliation was subsequently prepared in the separate Model Host
worktree `aruvi-model-host-scoped-integration` and pushed as
`codex/scoped-inference-integration` at `0336d34`. It incorporates the existing
main-checkout environment changes without changing or clearing that checkout.
Seven native adapter tests and 17 simulated lifecycle tests passed there, again
using the existing candidate host binary. The native integration plan preserved
the legacy service identity; the command was read-only. Review the combined
branch and complete integration acceptance before deployment. A subsequent
fresh offline macOS ARM64 build of `0336d34` passed ten Rust unit tests and 17
simulated lifecycle tests. Disposable native and Docker Studio MCP fixtures
passed with a scoped credential and denied host management. A disposable host
using the freshly built executable and real `qwen-05b-comparison` CPU model
passed scoped discovery, management denial, MCP completion, browser API
streaming and headless browser streaming. This is one text model and does not
establish deployed integration acceptance. The branch has not been merged,
and no running service was changed.

The same freshly built host subsequently passed the disposable scoped-token
rotation fixture: Studio preserved its provider while the new credential was
not yet accepted and recovered after the host restarted. A real
`qwen25-coder-7b-q4` GPU fixture passed scoped discovery, management denial,
Docker Studio MCP completion, browser API and headless-browser streaming; the
generated JavaScript function also passed the fixture's isolated assertions.
This is one coding prompt and does not establish general model reliability.

### Studio inference-credential opt-in

`scripts/configure-native-model-host --credential-scope inference` now reads
only `MODEL_HOST_INFERENCE_TOKEN`, never silently falls back to the management
credential, and requires a 403 from `GET /api/models` before configuring Studio.
The check uses a bounded child process, does not follow redirects, and does not
print credentials or provider bodies. It checks one management route and does
not by itself prove the entire host policy. Eighteen helper tests passed; a
disposable candidate host accepted the inference token for discovery and denied
management while the helper rejected the management token. Against the currently
running host, the helper failed closed because no inference token is configured;
it made no provider change. A failed check leaves any previous provider credential
in place; it does not remove or revoke a management token. Default helper behavior
remains management-token compatibility until a scoped host is deployed and
explicitly selected. The independent-client probe accepts the same scope option;
it passed discovery, completion, streaming and token-limit checks against the
candidate with a simulated worker, and fails closed against the currently
running host when the scoped credential is absent. Its default compatibility
mode still passed real 7B-model discovery, completion, streaming and token-limit
checks against the running host.

### Candidate inference-only Model Host credential

The candidate has now been applied to an isolated Model Host worktree based on
`6edb84979f0b670c304192600052f98533a86439`, committed as `a927792` and pushed
on `codex/studio-inference-scope`. Seven Rust unit tests, six native adapter
tests, 17 simulated lifecycle tests and a disposable native Studio MCP test
passed; the latter verified completion with the inference token and denial of
host management. The Studio fixture required `PYTHONPATH` pointing to the host
worktree. The running host was not changed. The main host checkout still has
uncommitted changes, including an integration Compose adapter that is not in
this branch; reconcile those changes and independently review authorization,
configuration and deployment before using the scoped credential in the live
service. This candidate is not a deployment or a substitute for TLS.

The candidate branch was advanced to `ffb4cfb` to make all four cross-project
fixtures directly runnable without a `PYTHONPATH` workaround and to document
their required environment. Native Studio, Docker Studio, token rotation, and
the independent Docker client each passed against a simulated scoped host. The
rotation test verified that Studio retained its provider until the restarted
host accepted the new credential. These remain synthetic fixtures; deployment
and reconciliation with the host's separate integration adapter remain open.

The Model Host source at `6edb84979f0b670c304192600052f98533a86439` currently
uses the same bearer credential for inference and management. A candidate patch
at `docs/integration/model-host-inference-token-candidate.patch` adds an optional
separate credential accepted only on the explicit `/v1` model-listing, text,
audio and image inference routes. The management token remains required and
the two values must differ. Without the new credential, existing authorization
is retained. This controls API access, not inference resource consumption, and
does not replace TLS on an untrusted network.

The context-bearing patch applies to the committed host revision with `git apply`;
review the exact base revision before applying it. A disposable snapshot
passed 17 simulated lifecycle tests, seven Rust tests and six native-adapter
tests; worker fixtures verified credentials do not enter worker environments.
The scoped token also exercised streaming, transcription, speech and image
inference with simulated workers. A route matrix checked every currently
registered `/api` management method and rejected method mismatches on inference
routes; this must be maintained as the host API changes. The independent Docker
consumer also passed its scoped-token probe against a simulated worker. A separate
watcher fixture changed the private token file before restarting the candidate
host: Studio kept its existing provider and credential while the new token was
rejected, then updated the same provider after the host restarted. MCP completion
passed before and after rotation. This used synthetic credentials and simulated
workers; it did not restart the deployed host. The three tracked Compose adapters passed
synthetic configuration checks. Native and Docker Studio fixtures completed MCP
inference through the candidate host with the scoped token; the native fixture
verified management was denied, and the Docker fixture exercised idempotent
configuration through Studio's scope-checking helper. Those lifecycle and cross-project tests used synthetic models and credentials.
A separate temporary host built from the candidate then copied the installed
`qwen-05b-comparison` model into its own state, ran it with CPU inference, denied
management with a fresh scoped token, and passed Docker Studio MCP completion
and browser API streaming twice. A subsequent run with `--browser` also passed
through a headless Chrome chat session using the same scoped credential. No live
host deployment or data was modified; the temporary host and Studio container
were stopped. This tests one model and
prompt, not broad model quality or a deployed scoped host. The reusable
`scripts/test-scoped-real-host` command accepts explicit candidate-host binary,
worker binary, model file and Studio image paths. The snapshot's other probes require `MODEL_HOST_TEST_BIN` plus `ARUVI_STUDIO_TEST_BIN` or
`ARUVI_STUDIO_TEST_IMAGE` and `ARUVI_STUDIO_ROOT` respectively. Applying the
patch to the exact base reproduced all 15 candidate files byte for byte.

On 2026-11-14, the same isolated fixture also passed with the real
`qwen25-coder-7b-q4` model and GPU inference. It denied management access,
completed MCP and streaming requests, and exercised browser chat. The MCP
response generated a JavaScript function whose behavior passed two independent
checks in a network-isolated container. This is one coding prompt, not a broad
quality evaluation; the scoped host remains an undeployed candidate. The fixture
supports `--coding`, `--memory-mb`, `--gpu-layers` and `--install-timeout` for
larger models.

This remains a **candidate**, not a host commit or deployment. The host's main
checkout contains unrelated uncommitted changes, including a separate untracked
integration Compose adapter; those changes were not modified or incorporated.
The managed isolated checkout request timed out at the app tool; no new checkout
was found in Git's worktree list or chat attachments. Before adopting the
patch, reconcile it with that work, add the inference token to the integration
adapter, independently review authorization and validate the intended deployment. The running
Studio branch still uses the existing management-capable host credential.

A temporary snapshot combining the candidate with the current uncommitted host
files exposed a zero-context patch hazard: Git reported a clean application but
inserted a native-adapter test assertion before the variable it used. The patch
was first corrected and later regenerated with context in every hunk. On the exact
committed base, all 15 patched files still match the previously tested candidate
byte for byte and all seven native-adapter tests passed. Against a fresh snapshot
of the current local changes, the context-bearing patch correctly rejects the
README hunk; applying the other hunks passed the seven native-adapter tests.
The README needs manual reconciliation. A previously manually reconciled snapshot
also passed seven Rust tests and added the integration Compose token setting;
that setting is not part of the candidate patch because the adapter is currently
untracked. Review the full diff before adoption.

The manually reconciled snapshot was then built as a native host binary. All 17
simulated lifecycle tests passed. Disposable native Studio and Docker Studio
fixtures completed MCP requests using the inference-only credential; the Docker
fixture also exercised Studio's scope-checking configuration helper. A separate
watcher fixture retained its provider while a rotated credential was not yet
accepted, then recovered after restarting the disposable host. These used
synthetic workers and credentials, not the live host or real-model inference.

The reconciled snapshot was also built in release mode and tested with a
temporary copy of the real `qwen25-coder-7b-q4` model. Its scoped credential
was denied management access; disposable Docker Studio completed MCP coding,
independent JavaScript assertions, streaming API and headless browser chat.
The host source matched the earlier candidate's server source. This is one real
model and coding prompt in temporary state; neither the live host nor its data
was modified.

On 2026-11-14, a disposable copy of the untracked integration Compose adapter
was rendered with `MODEL_HOST_INFERENCE_TOKEN: ${MODEL_HOST_INFERENCE_TOKEN:-}`
added alongside its existing management-token setting. Docker Compose preserved
both synthetic credentials and rendered an empty inference value when unset. No
host file or deployment was changed. This validates configuration rendering only;
the adapter still needs reconciliation and runtime testing with the candidate host.

A subsequent Docker runtime check built the candidate host binary for Linux and
placed it in a disposable image based on the local host image. The current
integration adapter was copied to a temporary Compose project and given the
proposed inference-token setting. With synthetic credentials, scoped discovery
succeeded, management returned 403, the management credential succeeded, and
an unauthenticated management request returned 401. Docker's Linux filesystem
was full, so the first run failed initializing its named data volume; the
successful retry used a temporary in-memory `/data` mount and does not validate
the integration adapter's persistent-volume lifecycle. The fixture and its
volumes were removed, as were this test's temporary image and its identified
941 MB build cache entry. The remaining Docker filesystem was still nearly full.
`scripts/test-scoped-host-compose` can repeat the check with an explicit adapter
and candidate image; `--tmpfs-data` selects the storage substitution. The host
repository and deployed host were not changed.

### Current branch build and security checks

Agent model calls now re-read the selected model and provider before each call.
Disabled models or providers stop the next call, as do changes to the selected
model's name, provider or context window. This is a pre-request check, not an
atomic lock: an edit can race a request already in flight. The native regression
test verifies rejection and recovery across successive queued calls; all 260
native Rust tests passed with seven ignored. All 49 isolated workflows passed,
and a disposable real 7B `extend-module` workflow passed its syntax and
independent generated-code assertions. That run did not change the model during
execution and does not prove cancellation of an in-flight request.
An additional native coding-stage fixture used a synthetic HTTP provider that
disabled the selected model after its first response. The next call was rejected,
the agent run was marked failed, only one model call was recorded, and a staged
file was not synchronized to the repository. Coding failures before repository
synchronization now clean up their workspace when `keep_workspace` is false.
Failures after synchronization begins retain it for recovery even with that
setting; `keep_workspace=true` also retains failed workspaces. The native
fixture verified all three cases and confirmed a sync conflict left the original
repository file unchanged and retained the generated candidate. Cleanup failures
are logged without replacing the original execution error. Workspace creation
now exclusively creates its generated directory and attempts to remove it if the
repository copy fails, without deleting an existing directory on collision.
Native workspace tests covered failed copy, collision and successful copy; all
49 isolated workflows passed. Abrupt cancellation or a failed cleanup can still
leave a partial workspace.
After re-enabling the model, a separate new workflow completed with a queued
no-change response; automatic retry of the failed workflow was not tested.
All 260 native Rust tests passed with seven ignored, as did all 49 isolated
workflows; the rebuilt worktree was healthy.

The OpenAI-compatible client now handles full `message` content in SSE without
silently dropping it or duplicating an accumulated response. It emits a
message-only answer once, accepts a matching final message after deltas, and
rejects a conflict or later delta. All 61 native client tests passed. A
disposable native Studio and synthetic-provider browser fixture passed matching
and conflicting streams. After Docker's Linux filesystem fell to about 265 MB,
27 additional unshared Studio build-step caches were identified by this task's
completed logs and removed, restoring about 1.4 GB. The already-built branch image
was started without rebuilding and is healthy. All 49 isolated Docker workflows
passed, including matching and conflicting full-message streams. Real 7B MCP
coding, browser-origin MCP, and synthetic Piper-to-Whisper MCP checks passed
against the refreshed worktree. Disk space remains limited.
The independent consumer pins `873617965bdf455f4cd89accea64cb1b3b69931b`; all
27 native and non-root Linux offline tests passed, including streamed and
buffered full-message handling. Linux probes passed real 7B coding-host text and
integration-host text plus a synthetic Piper/Whisper speech round trip. The
integration-host probe first failed for an unclassified reason, then passed on
one retry; the original failure output was not retained, so reliability is not
established. Host-mounted temporary build files were removed.

Desktop transcription choices and planner auto-selection now follow the
backend's explicit `stt`/`asr` capability ranking and synthesis exclusions.
The reserved DeepSeek synthesis selector cannot be auto-selected for
transcription even if it has a conflicting tag. When a speech provider is saved
without a model, desktop planner fallback now uses `whisper-1` instead of an
empty string. The native ranking test, 128 frontend tests, frontend build and
13 mocked desktop provider tests passed. The rebuilt branch also passed the
synthetic Piper-to-Whisper round trip through MCP. Native-window and physical
audio acceptance remain open.

Studio's speech service now rejects using the DeepSeek transcription selector
for synthesis or the synthesis selector for transcription before making a
provider request. A native service test covers mixed-case selectors and remote
and local provider types; all 49 isolated workflows passed, including browser
API and MCP rejection checks and the existing simulated DeepSeek paths. The
rebuilt worktree also passed a real synthetic Piper-to-Whisper speech round trip
through MCP. Physical audio and live DeepSeek remain unverified.

The OpenAI-compatible SSE parser now validates both `message` and `delta`
fields. Previously, a later SSE message could contain tool calls or refusal
after an earlier text delta without causing the stream to fail; buffered SSE
fallback could also return message text without checking those fields. Sixty
native client tests passed, including malformed and non-text message fixtures.
All 49 isolated Studio workflows passed, including a browser stream that marks
the partial response incomplete when a later message contains tool calls.
Live third-party provider behavior remains unverified.
The independent Git consumer now pins `c4cac799d99ffef77e6199711c07b85d4db6fd1a`;
all 26 native offline tests passed, including streamed and buffered SSE tool
messages following partial text. On Linux ARM64, the same 26 tests passed as a
non-root user in a read-only container with networking disabled after vendoring
the pinned sources. To avoid Docker's nearly full filesystem, the temporary
vendor and build directories were bind-mounted from the Mac and then removed.
An independently built non-root Linux probe of that pin also passed real
`qwen25-coder-7b-q4` Model Host discovery, completion, streaming and token-limit
checks. A separate non-root Linux probe of the same pin passed a synthetic
`piper-ljspeech` to `whisper-tiny-metal` round trip and `qwen-05b-comparison`
text discovery, completion, streaming and token-limit checks against the
existing integration host. No release image was built; physical audio and live
third-party services remain unverified.

After the planner voice change at `2a7e433`, the isolated Docker workflow
suite passed all 49 cases on 2026-11-14. It covers simulated OpenAI-compatible
and Anthropic streaming, provider error and usage handling, DeepSeek voice,
browser voice, and MCP paths; it does not validate live third-party providers
or physical audio devices.

Named planner voice selection and expand/collapse now accept mixed-case node
types and retry a reference without one final period when its original spelling
finds no match. An existing punctuated node name takes precedence and ambiguous
matches still return an error. A native shared-service test exercised selection,
tree commands and a node whose name ends in a period. All 21 native planner-turn
tests passed; physical microphone and native-window testing remain open.

At revision `040092f`, the native macOS ARM64 Rust library suite passed 256
tests with seven ignored, and desktop, server and MCP debug binaries built. The
standard unit wrapper passed after setting the native Cargo environment, including
126 frontend tests; the terminology check and TypeScript/Vite build passed. The
Rust integration-test wrapper reported no integration tests in `src-tauri/tests`;
the separately recorded isolated browser/MCP workflows provide different
integration coverage. The wrapper previously failed silently when Cargo was not on
`PATH`; it now reports the spawn error. A deliberate missing-Cargo check confirmed
the message, followed by the successful configured run.

`npm audit --audit-level=high` reported zero vulnerabilities. Against locally
cached RustSec database commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`,
Studio's lockfile still reported `RUSTSEC-2023-0071` and 15 maintenance or
soundness warnings. The pinned independent-consumer lockfile reported no
advisories. Cargo audit could not open the local crates.io index, so these runs
did not check yanked status. The Studio all-features, all-targets dependency
tree selected no path to `rsa`; this does not remove the lockfile advisory or
establish safety for other manifests. Native-window acceptance, ignored cases,
live providers and release review remain open.

### Current real coding-host revalidation

After rebuilding Studio at `2a7e433` on 2026-11-14, the configured real
`qwen25-coder-7b-q4` host again generated JavaScript through Studio MCP that
passed three independent assertions in a network-isolated container. A
one-token request produced the expected truncation error. The browser-origin
MCP check passed on the newly assigned worktree port and rejected missing
credentials and the tested cross-origin requests. This is one coding fixture,
not a broad quality or full browser-workflow evaluation.

At Studio revision `ac0774a321525b36fd2bdaffe64f3568aea0c2c0`, the running
worktree used `qwen25-coder-7b-q4` for an MCP coding completion that passed
independent JavaScript assertions; a one-token request produced the expected
truncation error. An authenticated browser received a streamed coding response.
In disposable Studio and Git fixtures, browser and MCP file Copilot discussed
an unsaved file, reviewed and applied a patch, and MCP reviewed and applied a
two-file proposal. A separate real-model `extend-module` workflow reached test
review, passed its syntax gate and independent behavioral assertions, and
completed delivery to a disposable local Git remote. These are individual
fixture runs, not broad model-quality, live cloud-provider or remote Git
acceptance. The running worktree records telemetry for the MCP and browser checks.

### Direct browser contact actions

The browser contact form offers direct WhatsApp and voice actions alongside
planner routing. Direct actions bypass planner policy and quiet hours, which the
form and confirmation state explicitly. Provider acceptance is shown as submitted,
not confirmed delivery. The mobile API validates input and authentication and
uses the existing channel services. The 28 isolated browser tests passed,
covering cancellation, both simulated actions, authentication, invalid requests
and an unconfigured server. Actual Twilio delivery remains unverified; no real
message or call was made. The branch image restarted healthy.

### Outbound channel transport

The shared Twilio transport now rejects redirects, bounds connection time to
15 seconds and the full request to 30 seconds, and reports failure status without
returning provider bodies or request URLs. `TwilioConfig` debug output shows only
presence flags. These are observable error and timeout behavior changes for
browser, desktop and MCP. Native tests used loopback fixtures to verify success,
HTTP errors, redirect refusal, connection redaction and debug redaction; they did
not send a real message or call. All 49 isolated workflows passed, the branch
image restarted healthy, and real Model Host coding and truncation checks passed
through MCP afterward. Live Twilio behavior remains unverified.

### Browser Twilio configuration

Routing now edits the desktop's Twilio account, sender and webhook settings and
can keep, replace or clear the stored auth token without returning it through the
browser API. Environment overrides are indicated without exposing their values.
A keyed, process-local token revision and transactional comparisons reject stale
browser saves after an MCP or desktop setting change; a restart or a different
server process requires refresh. A stored malformed or credential-bearing
webhook URL is not echoed and must be corrected outside this browser form.
The auth token remains in the existing settings database (which desktop and generic
MCP settings can read); this change does not encrypt or migrate that storage, and
clearing it does not revoke it or override an environment-provided token. Refresh
or revisiting Routing clears an unsaved token input. Live Twilio delivery has not
been tested. The isolated 49-case workflow suite passed before a subsequent
button-label correction; after that correction the 28-case browser suite passed.
A disposable container verified environment override indicators without returning
synthetic values. The branch deployment restarted healthy. No messages were sent.

### Browser planner contact

The browser also edits the desktop's saved contact destination and opening
message through authenticated endpoints. Updates compare both previous values
in one transaction so an MCP edit triggers a conflict without a partial save.
The user explicitly applies saved values to the contact form; refresh leaves an
unsent contact draft intact. Refreshing the defaults themselves replaces edits
in that defaults form. All 48 isolated workflow tests passed and the rebuilt
worktree passed its browser-origin MCP check.

The Routing page now provides a confirmed planner-contact form backed by the
existing Rust contact policy. It supports channel preference and explicit quiet
hours override, and reports blocked or sent outcomes. The API requires the mobile
token and bounds inputs; provider failures avoid returning provider details and
warn that delivery may be uncertain. The isolated browser test mocks outbound
routing and verifies cancellation, request fields and blocked feedback; it also
checks authentication, invalid inputs and an unconfigured server. All 28 isolated
browser tests passed on the final image; the worktree deployment restarted healthy
and passed the browser-origin MCP check. Actual Twilio delivery, provider-error handling and actual direct sends remain
unverified. A separate browser-to-server fixture configured voice-only routing
and verified quiet hours blocked the fallback, with outbound HTTPS disabled.
No external message or call was made in validation.

### Candidate Aruvici CI protocol

A candidate patch against Aruvici `5851f25` is saved at
`docs/integration/aruvici-ci-protocol-candidate.patch`; it applies cleanly to
that revision. It adds target repository and approval metadata, retries of
failed, blocked, cancelled or interrupted runs with current approval and the
original plan digest, and artifact ownership and content verification. The
candidate always reports promotion eligibility as false because a promotion
policy has not been implemented; it does not authorize deployment. Symlink
checks and file opens are separate operations and can race a local filesystem
change. A disposable source copy passed all 59 native tests, including eight CI
API tests, plus its doc tests. No pipeline was queued during the Studio probe.

Native Studio and the candidate service passed MCP health, target and run
listing, browser API health and target binding. The optional
`bash scripts/test native-ci-service STUDIO_BINARY ARUVICI_BINARY --audit-protocol --exercise-protocol`
uses a disposable Git repository, a temporary approval and synthetic failed-run
and artifact database records. It verified artifact integrity via MCP and the
authenticated browser API, rejected unauthenticated access and tampered content,
and queued a retry with the original commit and context through MCP. The test
holds the temporary worker lock until the service stops, so no build executes.
This does not establish actual build execution, native UI behavior or promotion
readiness. The original Aruvici revision still fails the protocol audit. The
candidate is not committed to or deployed from Aruvici; its isolated checkout
request remains pending. Docker transport remains open.

The candidate has since been applied in the isolated Aruvici worktree
`/Users/rajanthegoat/work/aruvici-studio-ci-protocol` and pushed as branch
`codex/studio-ci-protocol`, commit `c2efef3`. Artifact readiness and chunk
reads open path components relative to directory descriptors with no-follow
flags and reject hard-linked regular files, reducing the earlier symlink
check/open race. The candidate passed 60 Rust tests, formatting and clippy;
the disposable native Studio fixture again passed health, target binding,
artifact integrity and guarded retry with worker execution locked. Promotion
eligibility remains false. The running Aruvici service was not changed; an
actual build, container-to-host transport, shared workspace identity and review
remain open. The patch in `docs/integration` reflects this branch.

The native fixture now has an explicit `--execute-build` mode. Against the
isolated Aruvici candidate `c2efef3` and a native Studio binary, it created a
temporary Git repository, approved its fixed Rust/Docker plan, and queued a
build through Studio MCP. The real gitleaks, formatting, clippy, Rust test,
OCI packaging and package-verification stages passed; the browser API reported
the passed run and the candidate verified the OCI artifact's integrity while
remaining ineligible for promotion. This used a minimal dependency-free Rust
fixture and a scratch image, not the Studio application or a native window.
Gitleaks 8.30.1 was obtained temporarily from its official release and checked
against that release's checksum; the binary was removed after testing. No image
was loaded or deployed. The fixture can be repeated with
`bash scripts/test native-ci-service STUDIO_BINARY ARUVICI_BINARY --audit-protocol --execute-build`
when gitleaks, the Rust toolchain and Docker Buildx are available on PATH.
The Docker-to-host CI transport and shared-workspace identity remain open.

An opt-in local CI relay now forwards a bounded allowlist of requests from a
token-authenticated TCP connection on host loopback to the private Aruvici Unix
socket. Studio accepts an explicitly configured local relay and does not fall
back to its socket if relay configuration fails. A disposable real Aruvici and
Docker Studio fixture passed MCP and authenticated browser API status reads;
incorrect credentials and a disallowed deploy method were rejected. The existing
simulated Unix-socket CI test and focused Rust socket tests passed. This is a
plaintext trusted-host transport, not TLS or an access boundary against Docker
administrators. It is not installed or deployed. The subsequent checkpoint
below covers the host/container mapping and a container-initiated build.

Studio now supports an explicit host/container repository-root mapping for CI.
It confines mapped target paths to the configured roots, resolves the container
path and checks it on binding and before queueing. A disposable Docker Studio
fixture mounted a real host Git repository read-only, used the authenticated
relay and candidate Aruvici service, bound the target and queued a real minimal
Rust/OCI build. All expected scan, formatting, lint, test, packaging and artifact
verification stages passed, and the browser API read the result. Unit tests
cover mismatches, traversal and symlink escape. An opt-in worktree Compose
override and `scripts/ci-shared-worktree-up` provide the read-only mount; its
rendered configuration was checked. The operator must ensure the roots are the
same files: path translation is not independent proof of shared identity. Live
CI deployment, stable-environment mounts, real product builds and writable
workspace delivery through this mount remain open.

The disposable container-to-host fixture was rerun with a real headless Chrome
browser against the completed minimal build. The CI page showed the target,
commit and passed run, exposed all expected stages, displayed a real Rust-test
log and requested OCI artifact verification. `scripts/test native-ci-service`
accepts `--browser` with `--execute-build`, and optionally
`--browser-executable` to use a local Chromium installation. This exercises
inspection, not cancellation or a deployed CI service. A follow-up run exposed
that the browser hid mapped targets by comparing host and container paths as
strings. The target selector now leaves identity validation to the backend, which
checks mapping and attachment during assignment and queueing. Chrome selected a
second commit and queued a distinct real build through the relay; it passed and
reported the selected commit. The simulated browser fixture also verified that a
mismatched target is rejected. Live deployment and other target types remain open.

### Disposable native Aruvici connection

Studio revision `f90987b` and Aruvici revision `5851f2594bbbd83193e259a566004088ac28c52e`
were built natively on macOS. A disposable Aruvici process with an empty temporary
registry and private socket and a disposable native Studio process passed MCP
CI health, target and run listing plus browser API health. No pipeline was
queued. `bash scripts/test native-ci-service STUDIO_BINARY ARUVICI_BINARY`
repeats the check; `--docker-image IMAGE` also probes the temporary socket from
a network-isolated container as UID 10001. On this Mac, that Docker Desktop
probe returned `ENOTSUP`. This is one socket-mount observation, not proof about
all Docker configurations. The running Docker branch still lacks a real CI
transport; real targets, builds and browser rendering remain unverified. Both
temporary services and their temporary state were removed after the check.

The optional `--audit-protocol` fixture additionally revealed that this real
Aruvici version omits the repository metadata required by Studio's safe target
binding and does not implement Studio's retry or artifact-readiness methods.
Binding was rejected and no build was queued. A separate temporary TCP probe
reached Mac loopback from the branch image, but no relay was deployed: the
protocol and shared-path gaps need resolution first. See `web-parity-audit.md`.

### Shared guarded PDF export

On the development Mac, Playwright's default browser for the installed package
was absent. Without an explicit browser override, the native PDF worker now
tries installed Google Chrome or Chromium if the default launch fails. The
guarded native PDF test passed with no PDF-specific environment variables and
blocked its attempted external requests. The worktree image built and 27
isolated browser tests passed, including PDF export. An explicitly configured
browser does not fall back. Packaged runtime dependencies and other operating
systems remain unverified.

At revision `e0a572a`, the broad native Rust suite passed 253 tests with seven
ignored, and 43 mocked-desktop browser tests passed. The new guarded desktop PDF
test also passed separately with the macOS development defaults for Node and
Playwright and an explicitly selected installed Chrome. This does not exercise
the native window, a packaged application or Windows.

Desktop Product Book PDF export now uses the server's guarded renderer and image
validation while retaining the requested page dimensions, margins and header
labels. The obsolete separate Playwright export script was removed. A native
Chrome/Playwright test passed, including an attempted external stylesheet fetch
and script, and produced a tagged 432-by-648-point PDF. A native test rejected
remote images before rendering. The worktree image built and 27 isolated browser
tests passed, including browser/MCP exports and PDF sidecar failure handling.
This is not native-window acceptance: macOS development still needs available
Node, Playwright and Chromium, and packaged native dependencies and Windows
behavior remain unverified.

### Running branch model routing and inference

At revision `596c095`, the running worktree's nine active agents all had explicit
bindings to the advertised `qwen25-coder-7b-q4` on the native coding Model Host.
An authenticated MCP request returned coding output that passed independent
network-isolated JavaScript assertions; an intentional one-token completion
returned the expected truncation error. An authenticated Chrome session also
received a streamed coding answer through the running browser app. These checks
used the branch's persistent telemetry and prove only the tested fixture, not
general model quality or every provider. LM Studio remains a separate configured
provider and was not validated by these Model Host checks.

### Model Host watcher recovery

The watcher also tracks the process ID reported by native host status, so a
same-port, same-token restart between polls triggers revalidation when the ID
changes. The original two-value connection helper remains compatible with
other scripts. Thirteen isolated helper tests passed, including a simulated PID
change, and the running validation host reported a valid process ID. No live
host was restarted for this check. Older status formats without a PID, PID
reuse, and restarts not reflected in status can still evade this detection.

The foreground native-host watcher now rechecks Studio's provider and the host's
advertised model after a detected outage, including when the host returns with
the same port and token. It does not rotate an unchanged credential. Eleven
isolated helper tests passed, including a simulated same-endpoint outage and
recovery. This does not prove every host restart is detected between polls or
make the optional watcher start automatically.

### Current branch regression checks

At `dbcd245`, five of the six default-ignored native tests were also run
individually: local Whisper transcribed a macOS-generated synthetic English WAV
using the checksum-pinned Tiny.en model; native Chrome/Playwright rendered the
PDF and passed its external-fetch/script-blocking check; and freshly captured
mocked-Tauri Product Book fixtures passed embedded-renderer parity and EPUB
generation. EPUBCheck 5.4.0 reported no errors or warnings. The initial browser
fixture capture failed because Playwright's video recorder was unavailable; it
passed with lightweight mode, which disables video and tracing. These captures
do not prove native desktop-window behavior, physical audio, or broad speech
accuracy. The ignored live calculator test was not run; the separate disposable
Model Host workflow evidence is recorded below.

At revision `e844cd3`, the isolated native macOS Rust library suite passed 252
tests with six ignored. The ignored cases require configured Chromium/Playwright,
captured desktop Product Book fixtures, a local Whisper model and synthetic WAV,
or a live model provider. The frontend terminology check, 126 frontend tests,
and TypeScript/Vite production build passed; `npm audit --audit-level=high`
reported zero vulnerabilities. An isolated browser/MCP test against a simulated
CI socket passed. Recent branch checks separately covered 27 remote-interface
and 47 browser/MCP workflow cases; these do not establish native window or live
third-party acceptance. The previously documented RustSec findings remain open.

### Current pinned client against both Model Hosts

The independent consumer pinned to `c130a4bc1af205d64f1ea37df897ccd4193aee66`
passed 26 tests in Linux with build networking disabled after vendoring, including
an embedded Anthropic stream-error fixture. It also passed non-root, read-only
Linux probes against both existing native hosts. The
coding worktree's `qwen25-coder-7b-q4` passed discovery, completion, streaming,
JSON-object completion and token-limit handling. The integration host's
`qwen-05b-comparison` passed discovery, completion, streaming and token-limit
handling, and `piper-ljspeech` to `whisper-tiny-metal` passed a synthetic English
speech round trip. Neither host was reconfigured. Physical audio, live
third-party providers and broad model quality remain unverified.

### Browser workspace editor buffers

The initial browser patch guard compared path strings, so a hard link or other
alias could bypass protection for an unsaved tab. The browser now asks the
authenticated server to compare filesystem identity before applying reviewed
patches and invalidates matching clean tabs after each applied patch. The
isolated browser test verified an unsaved hard-link alias blocks application,
authorization is required, and a clean alias is invalidated after a partial
application; 27 browser and 47 workflow tests passed. Identity checking and file
writes are separate operations, so a non-cooperating process can still change
filesystem identities between them.

The subsequent worktree restart initially failed with SQLite disk I/O because
Docker's 128 GiB disk image was full. Removing only the rebuildable
`aruvistudio-cargo-target` build cache recovered 4.5 GB; the worktree restarted
healthy and its authenticated MCP provider listing succeeded. No data volume was
removed. Docker still had approximately 4.2 GB free, so future image builds
may require additional capacity; rebuilding the removed cache will take longer.
Subsequently, 36 obsolete unshared runtime cache chains identified by this
repository's Dockerfile steps were removed by explicit cache ID, in dependency
order. Docker then reported approximately 11 GB free; the worktree stayed
healthy and authenticated MCP provider listing still succeeded. Running images,
data volumes and other projects' images were not removed. Docker usage is
shared and may grow again during future builds.

The browser now keeps up to 25 open files per workspace with separate unsaved
buffers, dirty indicators and close confirmation. It blocks workspace changes
while any buffer is dirty, keeps the existing stale-content save check, and
refuses a reviewed patch if another affected open file has unsaved edits. Clean
affected tabs are invalidated after patch application. The worktree image built,
47 isolated workflow tests and 27 browser tests passed; browser cases cover two
unsaved files, switching, close cancellation, workspace protection, stale saves,
and a patch targeting an unsaved tab. Buffers are in memory and do not survive
page reload; the browser requests unload confirmation when any buffer is dirty.
Native window acceptance and other IDE parity gaps remain open.

### Browser workspace tree filtering

The browser can also refresh the workspace tree without clearing its open file
or unsaved editor content. An isolated browser/MCP case created a file through
MCP, refreshed the tree with an unsaved browser edit, verified both states, and
discarded the edit; all 27 remote browser tests passed. Refresh rebuilds the
displayed tree and does not preserve expanded folder state.

The browser workspace now filters its already loaded, bounded file tree by
case-insensitive name or relative path and opens matching ancestors. The worktree
image built and 27 isolated browser tests passed, including opening a nested
matching file, no-match feedback and returning to the editor after clearing the
filter. This closes one IDE navigation difference, not the full parity audit.

### Provider usage debug redaction

The shared client retains provider usage metadata in fields and serialization,
but its debug output now prints only whether usage is present. This prevents
arbitrary nested provider usage values from being copied into debug logs; other
metadata and application telemetry still require care. All 58 native client
tests passed and the Studio worktree image built. An independent consumer pinned
to `1c02a8d09738b89cfc82b8d6fbf18d2d42e7de27` vendored its dependencies and
passed 25 offline Linux tests, including the redaction and retained-data case.
The earlier real Model Host probe used the preceding pin.

### Current branch real Model Host coding revalidation

At branch revision `bf42f28`, the disposable Studio container ran the
`extend-module` coding workflow through MCP against the running native
`qwen25-coder-7b-q4` Model Host with JSON mode enabled. It reached review,
passed the configured syntax check and independent network-isolated behavioral
assertions, and delivered to a disposable local Git remote. This is one run;
earlier failures and variability remain relevant. The host deployment was not
modified, and live remote Git delivery and other models remain unverified.

### Browser outbound contact policy

Subsequent routing inspection found that a WhatsApp-to-voice availability
fallback occurred after the quiet-hours and destination checks. The final channel
is now checked again before I/O, preventing calls during quiet hours and calls to
WhatsApp-formatted destinations through that fallback. A native guard test and
47 isolated browser/MCP workflows passed, including a real MCP routing decision
with synthetic credentials. The fixture forces HTTPS through an unreachable
loopback proxy; no call or message was delivered. Live channel delivery remains
unverified.

The Routing page now edits the channel preference, ambiguity escalation and
quiet hours consumed by the existing server policy. Writes use a transaction
and compare the previously read values, including against changes made through
MCP. The isolated worktree image built and 46 browser/MCP workflows passed,
including authorization, invalid time ranges and rollback on a concurrent MCP
edit. Quiet hours use the server's local timezone. No real outbound call or
WhatsApp message was sent; live channel validation and other parity gaps remain
open in `web-parity-audit.md`.

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

The shared client now exposes structured timeout, connection, request, body
and other transport categories while retaining its previous error text and URL
redaction. MCP, browser streaming and desktop stream events expose the category
as optional metadata. Native Tauri event tests and six mocked desktop chat UI
tests passed; the UI still displays the text rather than using that category.
Buffered browser errors retain their plain-text bodies and expose optional
`x-aruvi-provider-transport-kind` or `x-aruvi-provider-http-status` headers;
MCP file Copilot preserves the corresponding structured provider error. Isolated
connection-failure and HTTP 429 fixtures verify both paths and provider-detail
redaction, and validation errors do not acquire a transport header. The worktree
image build, 45 browser/MCP workflow tests and 27 remote-interface tests passed.
The new `ClientError` variant
requires exhaustive-match consumers to
update when they opt into this revision; the category alone does not establish
retry safety. All 57 native client tests, a native MCP error test and 45 isolated
Studio workflows passed. The independent consumer pinned to
`56542efcbcc5089c1c595bb0af4f777e809b2745` passed 25 vendored offline tests,
including connection and stream-timeout classification. Its lockfile audit had
no reported findings; a non-root Linux probe also passed real Model Host text and
synthetic Piper/Whisper speech checks.

The shared DeepSeek voice client now implements the common `ModelGateway`
interface with transcription and WAV synthesis capabilities. It rejects
unsupported language hints, speed, output formats, discovery, health and text
operations rather than claiming support. Its generic request model field is not
sent because these voice endpoints do not accept one. Studio routes its existing
DeepSeek voice paths through the interface and continues to omit locale for
automatic language detection. All 57 native shared-client tests and 45 isolated
Studio workflows passed against fixtures; live DeepSeek voice remains unverified.
The native Studio Rust suite also passed 250 tests with six ignored by default,
and native ARM64 desktop, server and MCP debug binaries built successfully.
The previous independent consumer was pinned to
`6539f1613a7d1b71fb61d09f42509c419f56f82c`; all 24 consumer tests passed after
vendoring with build networking disabled, including a trait-object DeepSeek
voice check. Its non-root, read-only Linux probe passed real Model Host text,
streaming, token-limit and synthetic Piper/Whisper speech checks. Its lockfile
audit reported no vulnerabilities or warnings against the database below.

At branch revision `7215aaa`, a fresh npm audit reported zero vulnerabilities.
Cargo audit against RustSec advisory database commit
`4ef2df8e806a65e2f1ecc14f56f51249062b23d6` reported one vulnerability
(`RUSTSEC-2023-0071`, RSA 0.9.10), 13 unmaintained warnings and two unsoundness
warnings (`glib` and `rand`) in Studio's lockfile. The database commit matched
RustSec's remote `main` when checked. `cargo tree --locked --offline --all-features
--target all -i rsa` found no selected dependency path for this manifest; this
does not prove safety for different manifests or future features. The pinned
independent consumer lockfile had no reported vulnerabilities or warnings.
The SQLx vendor provenance check passed again, but cargo audit does not assess
the locally patched source and the existing release review items remain open.

At revision `fdccc96` with the planner test-fixture corrections in this
checkpoint, the native macOS ARM64 Rust library suite passed 250 tests with six
ignored; desktop, server and MCP debug binaries built as ARM64 executables.
The frontend suite passed 126 tests. Five ignored cases passed separately: two
real-Chrome PDF checks, the generated desktop-renderer comparison, an EPUB
fixture export, and local Whisper transcription of synthetic speech with the
checksum-verified Tiny.en model. A newly generated EPUB passed EPUBCheck 5.4.0
without errors or warnings. Browser UI checks used mocked Tauri and installed
Chrome, not a native desktop window. The live calculator test was not rerun in
this pass; its earlier isolated proxy run is recorded below. Physical audio,
live external providers and release acceptance remain open.

The independently vendored consumer pinned to
`dd614f4a38a10e49017b09dc71f7140dce3a5452` passed a real Model Host integration
probe against the native `integration` environment at revision
`01f934e378f9e07a8a2dcca94b1d5742e63966ae`. `qwen-05b-comparison` passed
discovery, completion, streaming and token-limit checks; `piper-ljspeech` and
`whisper-tiny-metal` passed a synthetic English speech generation/transcription
round trip. The client ran in a read-only, non-root Linux container with the
host token passed on stdin. This does not verify microphone capture, playback,
other languages, model quality in general or live third-party providers.

Planner model calls now revalidate the session's exact provider/model binding
before each request, including repository analysis and later tool-loop calls.
All 45 isolated workflows passed; a simulated provider confirmed that disabling
the model or provider, or renaming the model, stopped subsequent requests, and
that re-enabling the model allowed the existing session to continue. A change
after validation cannot cancel a request already in flight.

Planner defaults are now constraints rather than fallback hints: a configured
provider/model pair that does not resolve to one enabled text model returns an
error instead of silently choosing a different provider. Environment values
override the corresponding saved fields; explicit session selections retain
precedence. With no configured default, a sole eligible model is still selected
automatically. The focused Rust resolution test and all 44 isolated workflows
passed, including invalid saved defaults and explicit overrides through browser
API and MCP. `bash scripts/test planner-env-routing` also passed six disposable
server cases covering valid, invalid, partial and blank environment overrides
and explicit session precedence. Those checks stop at session creation; live
external channel delivery was not exercised.

Automatic transcription selection ranks explicit transcription/STT/ASR tags
above generic audio names and ignores clear synthesis-only names or tags, unless
the model also explicitly advertises transcription. Manual model choices remain
unchanged. The focused Rust selection test and all 43 isolated workflows passed,
including browser API and MCP requests verified against a simulated provider.
Generic name heuristics and the historical `whisper-1` fallback cannot establish
actual provider support; configure an explicit model when capability is known.

Explicit transcription model IDs now pass through desktop, browser and MCP
without requiring a naming heuristic; the selected provider still determines
whether the model is supported. Automatic selection continues to use enabled
speech-tagged or recognizable models. A simulated provider checked the exact
custom ID through the browser API, MCP and a focused desktop-command Rust test;
all 42 isolated workflows passed. This does not establish live third-party
support, native-window interaction or physical microphone capture.

For mobile/browser chat API calls without a provider ID, a registered model name
now selects its unique enabled text provider. Duplicate enabled registrations
require an explicit provider; a registered model with no enabled binding fails
instead of being sent elsewhere. Explicit provider selection retains precedence.
An unregistered model name can use the sole enabled provider; with multiple
providers the caller must select one explicitly, preventing an ambiguous prompt
from being sent to the first provider. The focused native Rust test and all 45
isolated workflows passed, including buffered and streaming rejection before
either simulated provider received a request. A model-only
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
to 500. The refreshed worktree
deployment was healthy and its live MCP list action passed a one-record check.

MCP `list_model_calls` additionally accepts an exact `source_kind` and a paired
`before_created_at` and `before_id` from the last returned call to continue in
descending creation-time/ID order. Existing requests and the returned array are
unchanged. A database fixture covers timestamp ties and invalid cursors; the
isolated workflow compares filtered MCP pages with the existing ledger. A cursor
is a position rather than a snapshot: new calls before that position are not
included in subsequent pages. The browser Activity view now loads older calls in
100-call pages and pauses auto-refresh while browsing history; Refresh returns to
the latest page. Its filters apply to loaded calls, not the whole ledger. An
isolated browser fixture covered failed-page retry and refresh, and an authenticated
API fixture compared filtered pages. All 29 isolated remote and 49 workflow tests
passed. Desktop now also loads 100-call pages, pauses refresh during history
browsing and filters loaded calls. The frontend build and mocked desktop browser
test passed, including failed-page retry and return to latest. A native Tauri
mock-runtime IPC test verified cursor arguments, timestamp ties and legacy calls
without cursor arguments. Native-window acceptance remains open.

A read-only `scripts/test-host-model-call-pages --model qwen25-coder-7b-q4`
check compared an older source-filtered page from the running worktree's
authenticated browser API and MCP ledger. The same ten records were returned
and the page included existing real-model calls. This checks API results, not
browser interaction or a snapshot-consistent view during concurrent writes.
`node scripts/test-host-model-call-ui.mjs` subsequently used a headless browser
against the running worktree, loaded 27 older records from its real ledger,
observed the paused-refresh state, and returned to the latest page. It did not
create calls or test native-window behavior.

This table summarizes the current evidence; dated sections below are historical
and may describe issues that were subsequently fixed. A passing fixture proves
only the stated scenario. Unchecked checkpoints remain open until their full
scope is reviewed, not merely until an example passes.

| Requirement | Current evidence | Remaining work |
| --- | --- | --- |
| Reusable Rust library | `crates/aruvi-model-client` is consumed by Studio and by a separate fixture pinned to `342520c95f4481d72646f83f9394c45134dc2864`; 31 independent native and Linux tests passed offline. A non-root, read-only Linux probe passed real Model Host text and synthetic Piper/Whisper speech. One unclassified integration-host failure occurred with a preceding pin before a successful retry. | Investigate an integration-host failure if it recurs; decide independent ownership, versioning and migration for other projects. The current pin is a checkpoint, not a published release. |
| OpenAI-compatible and Anthropic protocols | Shared clients cover text, streaming, discovery and error handling; OpenAI-compatible voice is implemented. Contract fixtures cover both protocols and the reasoning request profile. | Live third-party OpenAI, Anthropic, DeepSeek and LM Studio sessions remain unverified; compatibility with each model and provider cannot be inferred from adapter capabilities. |
| Local coding integration | The isolated 7B Model Host passed real MCP completion and an extend-module workflow through code review and a disposable local Git remote. A later new-module attempt produced a missing function definition; independent behavior checks caught it before delivery. | Investigate reliability on broader coding tasks and actual authenticated network delivery; syntax-only checks do not establish correctness. |
| Speech and local fallback | Real Model Host speech generation and transcription and checksum-pinned local Whisper have been exercised with synthetic audio; browser and MCP fixtures exist. | Physical microphone permission/capture, audible playback, other languages and recognition quality remain unverified. |
| Desktop, browser and MCP parity | Browser and MCP support many model, planner, agent, repository and product workflows; the detailed action inventory is in `web-parity-audit.md`. Recent native Rust, frontend and isolated web suites passed. | Complete action-by-action comparison, especially native IDE/dialogs, runtime management, settings and CI. A locked Mac prevented native window interaction; simulated Tauri tests are not native acceptance. |
| Data, configuration and credentials | The branch has isolated Docker data and a loopback endpoint; the model-host setup stores its token outside Git. Inspected Model Host source uses the same token for inference and management. Migration 053 passed backup, integrity and foreign-key checks. | Inference-only host credentials are not established; review host restart operations, cross-platform behavior and upgrade paths. Existing external-channel bindings remain unscoped until explicitly migrated or replaced. |
| Security and release evidence | On 2026-11-21, independent client and npm scans found no advisories; the Studio lockfile scan found an RSA advisory and maintenance/soundness warnings, including a newly reported chrono maintenance warning. | Review those advisories and the locally patched SQLx dependency; verify target platforms and complete release acceptance. No release or master merge is authorized by this goal. |

The currently running branch app is a worktree deployment. Its port is dynamic;
use `bash scripts/env worktree status` to identify it rather than relying on a
historical checkpoint address. Staging deployment has been requested and remains subject to the shared
integration and promotion gates. Production promotion and release remain
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

- The clean hosted ARM64 build and both hosted test runs for runtime revision
  `4afc0ee6ed97904446c69602118aea0b909d40d6` passed. Its archive checksum and
  revision matched. The extracted server binary was layered onto the previous
  unchanged worktree runtime, producing derived image
  `sha256:f8326af7a230b73d81de96d8779cae7694a13b2490ac160a8d0205edbca8f972`,
  not the complete hosted image. Disposable Linux tests passed 43 browser cases
  (one skipped), including HTTP/MCP scope matching, 53 workflow cases and the
  PDF outage check. After a checked SQLite backup, the worktree deployment
  passed product, authorization, PDF and persistence acceptance across restart,
  MCP-origin checks, a deployed scoped-match request and real 7B coding and
  general-Copilot MCP requests. The prior image was retained. To make space,
  two superseded downloaded archives (`1e5d3e5` and `c889053`) were removed;
  their candidate images and the current rollback image remain. This does not
  establish staging or physical desktop acceptance.
- The shared recent-action matcher can now use selected-node ancestors supplied
  by desktop, browser or MCP to reject explicit product/area conflicts, and
  checks a known parent capability when matching newly created child nodes.
  Callers that omit ancestors retain the earlier name-only behavior. Missing
  scope is still ambiguous; renamed ancestors can hide older matches, and
  equal names within the same scope may still match. This remains a page-session
  aid, not durable identity or an audit log. A Rust matcher test covers matching
  and conflicting product, area and parent names, normalized scope comparisons
  and legacy calls; authenticated HTTP and MCP return the same result. A
  simulated browser fixture and six mocked desktop planner tests verify the
  clients send ancestor scope. A disposable native server and headless browser
  selected two identically named nodes in different product areas, verified
  each displayed its own scoped action plus an ambiguous unscoped action, and
  checked the same result through MCP (`bash scripts/test native-planner-matches`
  after building the native server). The native Rust library suite passed 325 tests
  with seven ignored before the final scope-comparison normalization; focused
  matcher and HTTP/MCP tests passed again afterward. The frontend build and
  128 frontend tests passed. The later Linux deployment is recorded above;
  physical desktop acceptance remains open.
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
  existing first-priority binding policy remains unchanged. At this older
  checkpoint, a model disabled after selection was not rechecked between calls;
  the current checkpoint above records the subsequent revalidation change.
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

Anthropic streaming now also rejects a non-null error field in any event, even
when the event has another type and partial text has already been emitted. The
returned error omits the provider body. All 59 standalone shared-client tests
passed, including synthetic ping, message-delta and unknown-event failures;
a separate browser workflow then verified partial text is marked incomplete,
the error is shown without private provider details, and telemetry marks the call
failed. The full 47-case isolated workflow suite passed after correcting an initial
assertion that targeted the separate error bubble instead of the partial bubble.
This is fixture coverage, not live Anthropic acceptance. The worktree image was
rebuilt and restarted healthy; Chrome verified authenticated MCP access on its
published port and rejection of missing credentials and cross-origin requests.

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

## Agent run failure checkpoint

On 2026-11-14, agent preparation and successful-output persistence failures were
changed to mark the already-created agent run as failed and record the error.
A database failure during that status update, process termination, or task
cancellation can still leave a running record requiring recovery. A coding-stage
fixture forces a preparation database error before any model call and an output
artifact insertion failure after execution; both produce failed runs. The full
native library suite passed (261 passed, 7 ignored). After removing 30 explicitly
identified, unshared old Studio build cache entries, the branch image was rebuilt
from commit a53fe35 and deployed to the isolated worktree. Both services were
healthy, and all 49 isolated browser/MCP workflow checks passed against the new
image. The worktree port is dynamic; obtain the current address with
`bash scripts/env worktree status`. These checks use test providers and do not
replace the outstanding native-window and physical microphone acceptance.

## Shared-client adoption pin checkpoint

On 2026-11-14, the README's consumer example was corrected to the independent
fixture's existing Git revision `873617965bdf455f4cd89accea64cb1b3b69931b`.
The client Rust source at that revision matches the current source; this is not a
published release. The independent consumer passed 27 native tests with its
locked dependencies offline, and its native probe passed discovery, completion,
streaming and token-limit checks against the real 7B coding host. The probe used
the existing management-scoped credential and did not test cloud providers.
The consumer test entry point now checks that the README, manifest and lockfile
pins agree. Temporary mismatched README and lockfile controls were rejected.

## OpenAI transcription language shape

On 2026-11-14, the shared OpenAI-compatible client was updated for the
[OpenAI transcription contract](https://developers.openai.com/api/docs/guides/speech-to-text):
`gpt-transcribe` on the official HTTPS endpoint sends a supplied language hint
as `languages[]`, while legacy models and other endpoints retain `language`.
Consumers of compatible proxies can explicitly choose either field. A simulated
multipart endpoint verified the array request and exclusive field selection;
legacy multipart coverage and all 62 shared-client tests passed. Two Studio
transcription language tests also passed. This is contract-fixture coverage,
not a live OpenAI transcription; proxy support and language codes remain the
operator's responsibility. The rebuilt worktree image passed all 49 isolated
browser/MCP workflow tests, and its services restarted healthy. The independent
consumer was advanced to `12791e9fcd018ea4847ff7a2591535d4de07914e`: 28 native
offline tests passed, including an array-format proxy request, and its native
probe passed real 7B host text and streaming. The same pin subsequently passed
all 28 Linux consumer tests with the build's test step network-disabled. A
non-root, read-only Linux probe reached the existing native integration host:
`qwen-05b-comparison` passed text discovery, completion, streaming and token
limit checks, and synthetic English speech generated by `piper-ljspeech` was
transcribed by `whisper-tiny-metal`. This does not test a physical microphone,
playback, other languages or live OpenAI transcription.

## Provider transcription language format

Studio now exposes the shared client's `auto`, `singular` and `array`
transcription language formats per OpenAI-compatible provider in browser and
desktop model controls and via MCP get/set actions. Automatic retains existing
behavior; overrides support compatible proxies. Invalid settings fail closed,
the browser detects stale edits, and deleting a provider removes the setting.
Desktop settings writes remain last-write-wins. Local Whisper and DeepSeek's
separate voice transport do not use this field.

On 2026-11-14, the native suite passed 267 tests with seven ignored, the
frontend build and 128 frontend tests passed, and all 51 isolated browser/MCP
workflows passed. A simulated provider verified the array field through the
browser and MCP; a native desktop-command test verified it from the desktop
command, and a mocked desktop UI test verified the selector. A separate real
Model Host synthetic speech/transcription check passed using automatic mode.
Live third-party transcription, physical audio and native-window interaction
remain unverified.

The first worktree restart failed with SQLite disk I/O because Docker's shared
disk was full. Removing selected unshared Studio build entries and Studio's
Rust target cache recovered space, and the app restarted healthy. A read-only
SQLite `quick_check` returned `ok`; this is not a full data audit. Docker still
reported approximately 360 MB available, so subsequent builds may fail until
capacity is increased or more authorized cleanup is performed.

## Dependency revalidation after provider configuration

On 2026-11-14 at branch revision `7329e27`, `npm audit --audit-level=high`
reported zero vulnerabilities. Cargo audit against the locally cached RustSec
database at `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` reported one Studio
lockfile vulnerability (`RUSTSEC-2023-0071` in `rsa` 0.9.10), 13 unmaintained
warnings and two unsoundness warnings. The local advisory database commit
matched RustSec's remote `main` at the time of the check. The all-features,
all-targets dependency tree for this manifest selected no RSA path; this is
not a binary audit or a guarantee for another manifest or future features.
The independently pinned consumer lockfile reported no advisories or warnings.
The JSON scans explicitly skipped yanked-package checks. The SQLx vendor
provenance check passed, but the audit does not review the locally patched SQLx
source. These findings and the cross-platform review remain open for release.
The Mac console still reported locked during this check, so native-window and
physical microphone acceptance were not run.

## Transcription provider-only override

When `ARUVI_SPEECH_PROVIDER_ID` names a different nonblank provider from the
saved transcription provider and no environment model is supplied, the saved
model is no longer reused with the new provider. The selected provider's
automatic model selection applies instead. Matching providers and model-only
saved defaults without a saved provider retain their existing behavior; an
explicit environment or request model remains supported. This changes the
previous browser behavior for a conflicting provider-only override and the
newly unified desktop/MCP behavior. Operators who intentionally use the same
model name across providers can set `ARUVI_SPEECH_MODEL_NAME` explicitly.

On 2026-11-14, a focused native resolver test and a disposable simulated-provider
fixture passed for browser and MCP with provider-only and provider-plus-model
overrides, explicit requests, and a different explicit provider. All 50
isolated browser/MCP workflow checks passed; the rebuilt worktree was healthy.

## Blank transcription locale parity

On 2026-11-14, desktop and MCP transcription were aligned with the browser API
when a request supplies an empty or whitespace-only locale: each falls back to
its saved locale. A native desktop-command fixture and a simulated provider
accessed through browser API and MCP verified that a saved `ta-IN` produces the
`ta` multipart hint. All 50 isolated browser/MCP workflow checks passed, and
the rebuilt worktree services were healthy. These are synthetic requests, not
native-window or real Tamil recognition tests. Browser-only speech environment
overrides previously had different precedence from desktop and MCP; the
subsequent checkpoint below records the alignment.

## Transcription environment precedence

Browser API, desktop and MCP transcription now share the browser's existing
default precedence: explicit nonblank request values, then environment values
when present, then saved settings. A present blank environment value continues
to suppress its saved counterpart. An explicitly selected different provider
does not inherit the configured model. This changes desktop and MCP behavior
when `ARUVI_SPEECH_PROVIDER_ID`, `ARUVI_SPEECH_MODEL_NAME` or
`ARUVI_SPEECH_LOCALE` is set; deployments relying on the old divergence should
review their environment before upgrading. The current branch environment did
not define these keys when checked. Native speech playback settings are separate.

On 2026-11-14, a desktop command ran in a separate process with environment
overrides and conflicting saved settings. A disposable Linux server and a
simulated provider verified the browser API and MCP default path, explicit
overrides, and a different provider without the configured model. All 263
native library tests passed with seven ignored; all 50 isolated browser/MCP
workflow checks passed. These checks do not establish native-window interaction
or live external-provider recognition. A separate disposable fixture used the
real integration Model Host's synthetic Piper speech and Whisper transcription
through browser-configured server defaults and MCP; it did not set environment
overrides or use a physical microphone.

## Real coding retry guidance checkpoint

On 2026-11-14, the rebuilt worktree passed a real `qwen25-coder-7b-q4` MCP
coding completion with independent JavaScript assertions and a token-limit
error check; authenticated browser streaming also returned model output.
Three disposable extend-module workflow attempts did not complete delivery.
The ordinary profile failed its configured test command after the model first
returned multiple actions in one response. A JSON-mode attempt repeatedly put a
proposed tool action inside a final response and exhausted eight steps. A later
JSON-mode attempt asserted no changes were needed; its syntax check passed, but
independent behavioral assertions found the requested `clamp` export missing
and blocked delivery. These are small samples, not a comparative evaluation of
the modes. The syntax gate alone does not establish functional correctness.

Parser feedback now offers a bounded corrected tool-call envelope when a final
response contains a nested action for an allowed tool. It does not execute the
action: the model must issue a new call, which remains subject to normal tool
authorization and safety checks. A focused native test covers the correction
and omission for a disallowed tool. The later real attempt did not exercise
this particular correction, so improvement in live-model reliability remains
unverified. The native library suite passed 261 tests with seven ignored; the
rebuilt worktree services were healthy and all 49 isolated browser/MCP workflow
tests passed against that image.

## Coding work-item context checkpoint

On 2026-11-14, the coding prompt was corrected to use the actual context keys
for the work item's problem statement, acceptance criteria and constraints;
the work-item type is now included too. Previously those fields were omitted
from coding prompts despite being present in the context. An end-to-end native
coding-stage fixture checks the stored prompt for each field; a separate test
checks the existing acceptance and constraint length bounds. Long fields remain
truncated, and follow-up tool-loop turns use a bounded recap that can omit later
context in that revision; the follow-up checkpoint below improves the recap.
Inclusion is not a guarantee that a model follows the instructions.

The native suite passed 265 tests with seven ignored, and all 50 isolated
browser/MCP workflow tests passed. One disposable `qwen25-coder-7b-q4`
JSON-mode extend-module workflow passed syntax and independent behavioral
checks and delivered to a disposable local Git remote. Its task details were
also present in its description, so this single run does not establish that
the restored fields caused success or that model reliability is solved.
The worktree services were rebuilt and healthy after these checks.

## Coding follow-up recap checkpoint

The coding loop now builds a bounded recap from structured task context on each
follow-up turn, reserving space for the title, problem, description, acceptance
criteria, constraints, plan and skill instructions before adding any remaining
base-prompt excerpt. Individual fields are capped and marked when truncated;
the combined recap remains limited to 3,500 characters. This retains the start
of each field, not every detail of a long requirement, and does not guarantee
the model will obey it.

On 2026-11-14, a simulated HTTP provider verified acceptance and constraint
markers in both the first and second actual coding requests; multiline and long
context tests verified the recap and its bounds. The native suite passed 267
tests with seven ignored, the extended coding-stage test passed, and all 50
isolated browser/MCP workflows passed. One disposable 7B JSON-mode extend-module
workflow passed syntax and independent behavioral checks and delivered to a
disposable local remote. This is one model sample, not broad reliability or
proof that the recap caused success. The rebuilt worktree services were healthy.

## Coding step-limit finalization checkpoint

When the coding loop reaches its step limit after edits, its additional
finalization request now includes the same bounded task recap and asks the model
to identify unverified or unfinished requirements. A simulated HTTP provider
checked acceptance and constraint markers in the initial, follow-up and
forced-finalization requests in a two-step coding run. The native suite passed
267 tests with seven ignored; all 50 isolated browser/MCP workflow checks passed,
and the rebuilt worktree was healthy. This is simulated-model coverage. The
model can still produce an inaccurate summary; if finalization does not produce
a valid final response, the existing workflow can continue with collected edits,
which still require suitable tests and review before delivery.

The coding output now begins with an explicit no-files-changed notice when the
agent returns `no_changes_needed`, identifying it as the model's claim to
verify. The prefix remains visible in the 200-character artifact summary even
when the model supplies a long or misleading summary. The focused coding-stage
test verifies the stored artifact and the rebuilt image passed all 49 isolated
browser/MCP workflow checks; the worktree services were healthy afterward. This
does not verify the claim or force a manual review when workflow auto-approval
is enabled. Project-specific functional tests and an appropriate review policy
remain necessary.

## Resume checkpoint: 2026-11-17

The feature branch remains `codex/aruvi-web-docker`, based on pushed commit
`b955797`. The feature worktree was clean before this checkpoint. The separate
PDF-isolation checkout was left untouched. The branch Docker image rebuilt
successfully from the pinned sources. Its isolated app and PDF sidecar are
healthy at `http://127.0.0.1:55029/remote`, with separate project identity
`aruvi-studio-worktree-0e3d3e51014c` and its existing persistent data volume.
The legacy Studio Compose app on port 8787 was left running.

Docker's internal filesystem filled during startup. The failed startup did not
damage the named worktree database; a later isolated startup succeeded after
reclaiming only BuildKit caches identified as belonging to this Studio build.
The persistent volume was preserved. Docker filled again during browser/MCP
validation, and a subsequent text request failed with `No space left on device`
while SQLite attempted to write. The Docker app continues to report healthy,
but write-dependent checks against this container are unreliable until space is
freed. No machine-wide prune, other-project cleanup, or other project's
deployment change was performed. The product environment launcher rebuilds on
every worktree `up`; this resumed run started the already-built image directly
to avoid another cache expansion.

The isolated browser/MCP validation passed: the operational-section browser
check (1), the complete remote UI suite (29), CI browser/MCP flow (1), and all
workflow browser/MCP cases (51). The native Rust library suite passed 267 tests
with seven ignored; the model-client crate passed 62; and the independent
consumer passed all 28 tests against Git pin
`12791e9fcd018ea4847ff7a2591535d4de07914e`. The frontend production build,
server `cargo check`, terminology check and provider-routing tests passed. The
configured Rust integration wrapper found no tests in `src-tauri/tests`.
`npm audit --omit=dev --audit-level=high` found zero vulnerabilities.

Live local inference also passed outside Docker. The independent pinned client
used the native integration Model Host for discovery, text completion,
streaming, token-limit detection, and synthetic Piper-to-Whisper audio. An
isolated native Studio server then used the shared client through MCP to
discover `qwen-05b-comparison`, generate a `sumEven` function, and generate and
transcribe synthetic speech. The test used a temporary credential file and
database, which were removed on completion. `scripts/test-native-model-host`
now accepts `--host-environment` (`worktree`, `integration`, or `staging`) so
this validation can target the intended running host. The available integration
host currently advertises `qwen-05b-comparison`, `smol-metal-test`, Piper,
Whisper and an image model; it did not advertise a dedicated coding model at
the time of this check.

Fresh full `npm audit --audit-level=high` now reports six high findings through
`picomatch` 4.0.7, including GHSA-fc5q-39xc-vjpf. GitHub's advisory currently
marks versions through 4.0.7 affected and lists no patched version
([advisory](https://github.com/advisories/GHSA-fc5q-39xc-vjpf)). The npm registry
also currently has no 4.0.8 release. `npm audit fix --dry-run` cannot clear the
findings without a major Vitest downgrade to 3.1.4; no lockfile change was
applied. Treat the development-toolchain audit as a release blocker until an
upstream patch or a reviewed compatible dependency path is available. The
previous Rust advisory findings, SQLx vendor review, and ignored test cases
remain as recorded above.

Still open: native Tauri window acceptance and physical microphone/playback;
live third-party provider calls; broader model quality and project-level coding
validation; inference-only Model Host credentials (the deployed host still uses
its management-capable credential and the host checkout has unrelated user
changes); full Docker-backed real-model revalidation after daemon capacity is
restored; and release/dependency review. No release was created or deployed.

## Follow-up: 2026-11-18

The full `npm audit --audit-level=high` now reports zero vulnerabilities.
GitHub withdrew GHSA-fc5q-39xc-vjpf on November 18; its advisory links to an
upstream discussion and does not identify a patched picomatch release. This
withdrawal and clean audit supersede the prior scan result, but do not establish
that the reported behavior was fixed. Reassess the advisory and dependency tree
at release time.

The worktree app is unhealthy and `/health` returns 503 `database unavailable`.
Docker's internal filesystem reports zero available bytes. Removal was scoped to
identified unshared Studio build and runtime cache entries; the daemon still
reports no available space. Persistent volumes were not removed, and database
integrity has not been rechecked since this latest failure. Restore Docker
capacity before restarting the app, checking database integrity and repeating
write-dependent validation. Other services share this daemon.

## Recovery check: 2026-11-20

Docker reported 33 GB available and the existing worktree app and PDF service
were healthy. A read-only SQLite `quick_check` returned `ok` and
`foreign_key_check` returned no violations. A real Model Host text completion
and synthetic Piper-to-Whisper round trip passed through the Docker Studio MCP
endpoint. The browser-origin MCP check passed on the published port and rejected
missing credentials and tested cross-origin requests. The dedicated coding
fixture failed at its MCP call with its default model/provider; this check does
not identify the cause and coding-model validation remains open. These checks
do not establish native window, physical audio, or live cloud-provider parity.

## Staging adapter checkpoint

The staging Compose adapter now starts the same network-isolated PDF sidecar
as the worktree and connects the app to its Unix socket. A disposable local
Compose project using the worktree image reached healthy status for both
services; the app connected to the socket, and the renderer reported no network
and a read-only root filesystem. This did not test a complete export or a VPS
deployment. The fixture and its disposable volumes were removed.

On 2026-11-20, `origin/main` was `1edb9dd` and did not contain the feature
branch. The organization promotion process calls for a merged default-branch
revision and Mac integration acceptance before VPS staging. No staging
deployment has been made. The integration adapter, target platform, ingress,
real Model Host connectivity and persistent staging acceptance still need review.

In a later disposable Mac Compose run of the staging adapter using the current
worktree image, an authenticated browser API request and an MCP request each
generated a PDF with a valid PDF header. After restarting both services, the
fixture product remained present and another API PDF export succeeded. The
fixture and its volumes were removed. This checks the adapter locally; it is
not an acceptance test of the VPS staging host or its data.

## Mac integration adapter

The repository now has a separate integration Compose adapter with persistent
app data and an isolated PDF renderer. It binds loopback port 20280 on the Mac;
the organization registry reserves that port for this product's integration,
while the staging reservation applies on the VPS. Integration and staging use
separate project identities and generated credentials. The new
`scripts/test integration-ui` runs the existing section-navigation smoke test
against an already deployed integration stack; `scripts/test integration`
continues to mean Rust integration tests.

A disposable Compose project using the integration adapter and worktree image
started both services healthy, created a product and exported a PDF through the
authenticated API, then retained the product and generated another PDF after
both services restarted. Temporary credentials were distinct across integration
and staging and stored with mode 0600. This does not establish a merged revision,
real integration deployment, tailnet access or second-device acceptance. The
organization registry reservation lives in the separate `.org` checkout and
must be made available wherever that shared checkout is installed.

The production Compose adapter also specifies the isolated PDF renderer and
socket. A disposable Mac project using the worktree image started both services
healthy and generated a PDF for an authenticated test product. The fixture and
its volumes were removed; this is not a production deployment, VPS validation,
or acceptance of a release artifact.

## Default-branch reconciliation

On 2026-11-20, `origin/main` was `1edb9dd`. It already contained the earlier
Studio branch through `e35292c`; its only additional commit pinned the CI
runner to Ubuntu 24.04. The feature branch's later work had not been merged.
The CI change was merged into the feature branch without conflicts at
`3e44115`; it changed only the workflow runner configuration. The running
worktree app remained healthy. This is not a merge of the feature branch into
main, a CI result, or staging acceptance.

## Browser bridge configuration status

The browser Activity view now displays an authenticated, read-only bridge
configuration summary and derives the MCP URL from the browser's current
origin, rather than presenting the container's internal bind as an external
address. The summary identifies configured bind values and their sources,
whether an MCP token is configured, and whether configuration permits requests;
it does not return credentials. These bind values are configuration and can
differ from the active listener until restart. A native API test checked mobile
authentication and credential redaction. The deployed-branch navigation smoke,
29 remote UI tests, one simulated CI test and 51 workflow tests passed; the
browser smoke checked the actual published origin and token omission. The
worktree app and PDF renderer were healthy at port 55093. Browser editing of
bridge configuration, reverse-proxy deployment and native-window acceptance
remain open.

## Coding host recovery

The coding-validation Model Host worktree was no longer registered with
launchd; its endpoint file contained an old port. Its existing verified native
artifact was restarted on a new loopback port, and
`scripts/configure-native-model-host` refreshed the existing Studio provider
and discovered `qwen25-coder-7b-q4`. On 2026-11-20 the running Docker Studio
passed the real MCP coding fixture using that model: generated JavaScript passed
three independent assertions in a network-isolated container, and a one-token
request returned the expected truncation error. Authenticated browser streaming
chat also returned coding-model output. This addresses the earlier failed
fixture for this model and prompt; it does not establish general model quality.
The test explicitly set `ARUVI_CODING_TEST_MODEL=qwen25-coder-7b-q4`; the
fixture's default remains the separate 15B model. The validation host still
uses a management-capable token, and its dynamically assigned port can change
after a restart.

## Shared transport factory

The shared client exposes `GatewayConfig` and `create_gateway` for explicit
OpenAI-compatible, Anthropic and DeepSeek voice transport construction. Studio
now uses that factory for its text gateways while retaining application-specific
validation, credential resolution and provider settings. The config type does
not implement Debug or serialization. The client suite passed 63 tests, the
native Studio suite passed 267 with seven ignored, and all 51 isolated browser
and MCP workflow checks passed. The rebuilt branch passed real 7B coding and
token-limit checks and synthetic Piper/Whisper speech through MCP. The independent
consumer now pins `4152e1cad1836190190ebd033ff1672fd42dbe13` and its multipart
transcription fixture constructs the gateway through the new public factory.
All 28 independent tests passed on macOS and in a Linux Docker build whose test
step had networking disabled after vendoring. The live-host probes in this
checkpoint initially used Studio. The independent consumer's host probe now
also constructs its gateway through the factory. Running as a non-root user in
a read-only Linux container, it passed real `qwen-05b-comparison` discovery,
completion, streaming and token-limit handling plus synthetic Piper-to-Whisper
speech. A separate probe passed discovery, completion, streaming, JSON-object
completion and token-limit handling against `qwen25-coder-7b-q4`. The probe
uses the currently configured management-capable host credentials; scoped live
deployment and physical audio remain open.

Studio's DeepSeek voice gateway now also uses the shared factory; its existing
provider validation, secret lookup, speech selectors, and request mapping are
retained. Twelve native speech tests passed with one local Whisper fixture
ignored. The rebuilt worktree image passed all 51 isolated workflow tests,
including simulated DeepSeek speech and transcription through browser, API,
and MCP. Both app and PDF service were healthy at port 55091. This does not
validate the live DeepSeek service.

Anthropic streaming now rejects duplicate indexed content-block starts, indexed
text deltas without an open text block, stops for unopened indexed blocks, and
message termination while indexed blocks remain open. Unindexed proxy events
remain accepted and full event ordering is not enforced. The client suite passed
64 tests; the rebuilt Studio image passed all 51 isolated browser/MCP workflow
checks, including simulated Anthropic streaming. The branch services were
healthy on port 55092. Live Anthropic validation remains open.
The independent consumer now pins `b0b1087c1c0f41d77a08990d76ab31917fa728b1`;
its 29 tests passed on macOS and Linux with the Linux test step offline. The
new independent test confirms that a partial indexed stream ending with an
unclosed block returns an error after emitting its earlier text.

## Coding final-response guidance

A real 7B extend-module workflow removed the existing exports and put corrected
code only in its final response. Its syntax gate passed, but independent
behavioral checks rejected it before delivery. The coding tool prompt now
states that final-response content is a report, not a file edit, and asks the
model to preserve and update module exports in the actual file. The focused
prompt test passed. After rebuilding Studio, one repeat of the same real-model
workflow passed syntax, independent behavioral checks and delivery to a
disposable local Git remote. This is one follow-up sample; it does not prove
the guidance caused success or prevent inaccurate model claims. Project-level
behavioral tests and review remain necessary. The rebuilt worktree app and PDF
renderer were healthy on port 55085.

## Earlier agent template reference clarification

Desktop and browser agent editors now state that the prompt-template reference
is stored metadata and is not applied during execution. Seeded references have
no corresponding template resolution path at that checkpoint. The frontend build
passed and the rebuilt worktree app and PDF sidecar were healthy at the newly
assigned port `55079`; the UI text itself was not separately browser-tested.

The current branch's native desktop binary and macOS debug app bundle built on
2026-11-20. An isolated data directory and profile were used to start it, but
the Mac console reported locked and the computer interface returned
`cgWindowNotFound`; native-window acceptance was not performed. The fixture
processes were stopped and their temporary data directory removed. A successful
build is not evidence of native interaction or physical audio parity.

## Previously ignored native checks

On 2026-11-20, six of the seven ignored native library checks were run
individually and passed. Three used headless Chrome for real PDF generation,
including the desktop export command and checks that scripts and external
fetches do not run. The shared Product Book renderer matched fixture output
captured from the mocked desktop UI for Docs HTML, Book HTML, EPUB and PDF.
The EPUB fixture produced an archive from that UI capture; EPUBCheck was not
run in this pass. The native local Whisper test transcribed synthetic Piper
speech using the existing `whisper-tiny-metal` model. These are headless or
synthetic checks, not native-window or physical microphone acceptance.
The ignored multi-iteration live calculator test was not run in this pass;
separate disposable real-model coding workflows are recorded elsewhere.

## Staging browser smoke on Linux

The staging smoke launcher previously used Docker Desktop's host alias on all
platforms, which cannot reach a Linux host's loopback-bound app from the default
container bridge. On Linux it now runs the browser with host networking and a
loopback URL; Docker Desktop keeps its existing host alias. A mocked Docker
invocation checked Linux argument selection. On the Mac, the actual worktree
smoke, 29 remote browser tests, simulated CI test and 51 workflow tests passed.
This does not validate real Linux host networking, the VPS deployment, its
artifact identity or persistence. The environment contract still requires a
merged default-branch revision to pass Mac integration before VPS staging.

## Browser IDE save shortcut

The browser workspace editor now accepts ⌘/Ctrl+S while the editor is focused,
using the same conflict-checked save as its button. The rebuilt Docker image
passed all 29 isolated browser tests, including a keyboard save followed by an
MCP file readback; the PDF outage check also passed. The rebuilt worktree app
and PDF sidecar restarted healthy. At that checkpoint the browser used a plain
text area and native dialogs were unavailable; see the later editor checkpoint.

## Local speech provider editing

The browser could register a local Whisper provider but could not subsequently
edit or disable it because both the form and mobile API required an HTTP URL.
The form now accepts a server file path for local providers. The backend checks
a changed path against an existing regular file and preserves the optimistic
concurrency check, including the provider type. An unchanged missing file does
not prevent disabling a provider. The rebuilt image passed 51 isolated workflow
tests, including local provider disable/re-enable, valid and invalid replacement
paths, stale edits and existing hosted-provider URL validation. These fixtures
do not validate the replacement model's contents or physical audio. The native
Rust library suite passed 273 tests with seven ignored, and the branch app and
PDF renderer restarted healthy.

A follow-up Linux container fixture used the checksum-pinned Tiny English
Whisper model and synthetic audio generated by the configured Model Host. After
changing the local provider to a second copy of the verified model through the
browser, transcription returned the expected words through the browser Voice
screen, mobile API and MCP. All 51 isolated workflow tests passed in that run.
This checks synthetic English audio rather than physical capture or general
recognition quality.

## Browser workflow artifacts

The Work view can open generated workflow artifacts as bounded UTF-8 text. The
mobile endpoint requires authentication and matching work-item ownership, and
resolves paths within the artifact directory. Its 2 MiB limit is enforced while
reading. The Rust fixture covered invalid scope, outside paths and symlinks,
oversized and non-text files; a browser workflow opened a generated coding
artifact and compared it with MCP. All 51 isolated workflow tests passed using
the rebuilt image. Independent filesystem writers can race path checks, and
desktop/MCP artifact readers still have separate behavior.

## Browser code editor

The Docker image includes the pinned Monaco distribution and serves its assets
locally. The Workspaces view can switch between Monaco and the existing plain
editor without discarding open buffers; server-side conflict checks and mixed
line-ending restrictions remain in force. A browser fixture covered unavailable
asset fallback, JavaScript language detection, edits and undo across tabs, a
delayed file open, ⌘/Ctrl+S, concurrent-write rejection and a read-only mixed
line-ending file. A follow-up check also saved a CRLF file and verified that its
line endings remained CRLF through MCP. Requests in the fixture stayed on the Studio origin. Native
servers require an explicit asset-directory configuration; native application
packaging, other languages, mobile devices and full desktop IDE behavior remain
unvalidated. Per-file buffers still do not survive a page reload.

The rebuilt image passed 34 isolated remote-browser tests and 51 workflow
tests; the native Rust library suite passed 275 tests with seven ignored. A
subsequent worktree restart hit a SQLite disk I/O error because Docker's VM
filesystem was full. One identified AruviStudio Cargo build-cache entry was
removed, the existing database passed read-only `quick_check`, and the same
image restarted healthy without rebuilding. Authenticated MCP catalog reading
and local editor-asset delivery then passed. Docker VM capacity remained tight
at approximately 1.7 GiB free; future builds and writes need available space.

## Browser work list pagination

The Work list no longer silently stops at the first 100 work items. It uses
the existing mobile pagination metadata, retains loaded rows after a page
failure and permits retry, resets on refresh, and ignores delayed pages after
changing product. A browser fixture exercised 101 real work items, a failed
page, retry and an in-flight product switch. Offset pagination is not a snapshot;
changes to earlier rows can move or skip items until the list is refreshed.

## Work item workspace override

Desktop, browser and MCP now share a backend operation to set or clear a work
item's registered workspace and branch. Browser edits use an assignment baseline;
MCP can opt into the same comparison. A valid Git branch name is required when
provided, and assignment is rejected while a workflow is running or paused.
This changes the prior desktop behavior, which allowed editing during execution.
The choice does not check out a branch or require product attachment. Isolated
tests covered browser/MCP conflicts, clearing, invalid and missing inputs, and
active-run rejection. A simulated-model workflow selected an override different
from the product default, wrote its generated file there and left the default
workspace unchanged. Actual network Git delivery and non-cooperating database
writers remain outside this validation.

A real-model override fixture subsequently used `qwen25-coder-7b-q4` and a
disposable product default distinct from the assigned work-item workspace. Two
initial attempts stopped on truncated provider responses: one used the 7B model
throughout and the other used 15B for non-coding stages. A third used 7B with a
4,096-token non-coding-agent response budget and completed both approval gates,
syntax validation, independent network-isolated behavioral assertions, and
delivery of the verified file to a disposable local Git remote. The selected
override contained the file and the product-default workspace did not. The test
uses a management-capable credential on a temporary Studio container and synthetic
work; it is one successful sample, not a guarantee of model quality, delivery to
an authenticated network remote, or a deployed inference-only credential.
`scripts/test host-coding-workflow --workspace-override` enables this scenario;
`--non-coding-max-tokens` changes only disposable agent settings.

## Work item deletion

Browser Work now offers deletion with a confirmation explaining record and file
effects. Desktop, browser and MCP share a backend check that rejects deleting an
item or detaching a direct child while either has a running or paused workflow.
The browser compares previously loaded work-item fields; MCP can supply the
same baseline. The focused Rust test covered authorization, stale data, an
active child and retained detached child. Isolated browser and workflow tests
cover cancellation, stale MCP edits, confirmed deletion and active-run rejection.
Non-cooperating database writers and any separately retained filesystem artifacts
remain outside this check. The rebuilt image passed 33 remote UI and 51 workflow
tests; the native Rust library suite passed 275 tests with seven ignored.

## Independent browser Copilot model

Browser workspace Copilot can choose an enabled registered provider/model
without changing the Connection model, or use the Connection selection. The
choice is page-local. Refreshing the list preserves an unavailable choice and
blocks sending until the user chooses an available model. The isolated browser
fixture verified the selected model reaches a simulated provider, Connection
remains unchanged, and disabling/re-enabling the model is handled without an
implicit fallback. All 29 remote tests and the PDF outage check passed; the
rebuilt branch app and PDF sidecar restarted healthy. This is not live cloud
provider validation or complete desktop IDE parity.

## Browser MCP through a disposable TLS proxy

`bash scripts/test tls-mcp-proxy` started a separate worktree fixture and a
local HTTPS proxy preserving Host. Headless Chrome loaded the browser app and
successfully sent an authenticated same-origin MCP ping through the proxy. A
missing token returned 401, and a different hostname returned 403 even with the
token. The test uses a self-signed temporary certificate and disables browser
certificate verification; it does not validate trust or the actual VPS ingress.
The current npm audit reported zero vulnerabilities. LM Studio was not listening
on localhost:1234, and OpenAI, Anthropic and DeepSeek API keys were not present
in the current shell; this does not rule out credentials in other secret stores.

## Scoped credential with the real 7B coding model

The isolated Model Host candidate `ffb4cfb` was run with a temporary copy of
`qwen25-coder-7b-q4`, a fresh inference-only token and separate temporary data.
It denied that token on the management API and advertised the model through
discovery. Disposable Docker Studio completed an MCP coding request; the generated
JavaScript passed independent assertions in a network-isolated container. Browser
API streaming and a headless Chrome streaming session also completed. Temporary
services and data were removed. This checks one coding prompt and does not deploy
the candidate or establish general model quality; the existing live host still
uses a management-capable integration credential. GitHub Actions run 44854070337
passed for revision `3403738`; later revisions need their own completed checks.

## Existing VPS staging inspection

Read-only SSH inspection found healthy native staging and production services
on loopback ports 8788 and 8787, both at `2d1130f921e9967cc8dffa77953ebc1355802e25`.
The staging Nginx route targets the native service and does not route MCP. The
new Compose adapter uses 20280 with separate persistent volumes and is not
deployed there. The VPS is Linux x86_64 with a rootless Docker daemon; root's
default Docker context was unavailable and the default root-path organization
contract was absent. Both native SQLite databases were present. No service,
data, ingress, credential or host configuration was changed. The transition
and acceptance prerequisites are documented in `headless-server.md`; a clean
merged revision and Mac integration acceptance are still required first.

## Explicit MCP browser origins for staging

The server accepts optional comma-separated exact external MCP origins from
`ARUVI_MCP_ALLOWED_ORIGINS`, still requiring Origin to match Host and rejecting
forwarded-host headers. Any nonblank list requires an MCP token even for a
localhost-bound server. The four environment adapters pass the setting through.
Two native Rust origin tests passed; rust-objcopy reported a missing libLLVM
debug-stripping warning. An isolated container rejected tokenless localhost MCP
with an external origin configured. The rebuilt image passed 30 browser tests,
including proxy-header authorization, mismatched host and origin rejection,
and the PDF outage check. The worktree app and PDF sidecar restarted healthy.
Actual TLS ingress and VPS staging acceptance remain untested.

## CI terminology guard correction

GitHub Actions run 44852419796 failed the terminology guard because a coding
prompt used “module” for a source-code module. The same run reported 268 Rust
library tests passing with seven ignored and 46 desktop Playwright tests
passing. The guard now narrowly recognizes the source-code prompt; a disposable
fixture confirmed it still rejects the legacy `module_id` identifier, and the
full terminology check passed locally. A new CI run is needed for this revision.

## Real-model new-file workflow revalidation

With `qwen25-coder-7b-q4` on the native Model Host validation worktree, a
disposable Studio MCP workflow created `src/math.js`, reached both approval
gates, passed the workflow syntax check and independent network-isolated
functional assertions, and delivered the verified file to a disposable local
Git remote. This is a single successful new-file sample after an earlier
failure; it does not establish general coding reliability, external Git delivery,
or safety of model-generated code. The host credential remains management-capable.

## Product Book bounded serialization

The shared backend renderer now enforces its 64 MiB input limit while writing
JSON, rather than serializing the full value and checking afterward. Unit tests
covered exact byte limits, escaped multibyte content and rejection. The rebuilt
image passed 30 isolated browser tests, including browser and MCP Product Book
exports, and the PDF outage check; the worktree services restarted healthy. The
database snapshot was still loaded without a size bound at that checkpoint.

The snapshot now performs a product-scoped preflight in the same SQLite read
transaction, before loading the tree, work items and reference candidates. It
rejects more than 64 MiB of raw column values or 100,000 candidate rows without
truncation. It counts all table columns and candidate references, so it can reject
data that would be smaller after filtering; neither this check nor the JSON limit
is a strict process-memory cap. Focused tests covered byte and row limits,
product isolation and snapshot consistency; the existing 2,102-item export
passed. The rebuilt image passed all 30 isolated browser tests and the PDF outage
check. The direct work-item list query remains uncapped outside this snapshot.

## Compose-network browser smoke

The staging and integration browser smoke tests no longer rely on Docker host
networking or Docker Desktop's host alias. They check the published loopback
health endpoint from the host and run the browser on the Compose default network
against the app service. This also avoids assuming rootless Docker shares the
host's network namespace. On the Mac, the worktree smoke, 30 remote browser tests,
simulated CI test and 51 workflow tests passed. The browser portion does not
exercise published-port or external-ingress navigation, and the rootless VPS
deployment remains untested. This supersedes the earlier host-network approach.

On the VPS, a disposable fixture on the existing `dockerapps` rootless daemon
verified that a second container could resolve and reach the `app` network alias,
and that a dynamically published loopback port was reachable from the host. The
fixture used an already-cached Python-capable image, not Studio or Compose, and
was removed afterward. The existing native staging and production services
remained active. This is infrastructure evidence only; the actual rootless Studio
stack, browser, ingress and persistent staging acceptance remain untested.

## Environment product and restart acceptance

Added `scripts/test environment-acceptance PROFILE [--restart]` for worktree,
integration and staging. It creates a uniquely tagged product, area and task via
MCP, checks mobile authentication and product reads, and verifies a PDF export.
The explicit restart option waits for app and PDF health, rediscovers a dynamic
published port and repeats reads and export. The fixture product is archived in
cleanup; its records remain in the environment. Two successful worktree runs
passed after restart. An earlier run exposed a changing published port and left
a fixture active when cleanup used the old port; that specific fixture was then
archived through the current endpoint. The final script waits for both service
health checks and rediscovers the endpoint for cleanup. Real integration and
VPS staging, external ingress and model inference remain unvalidated by it.

## VPS AMD64 branch-image fixture

On the VPS, rootless Docker built the committed branch revision
`e45d63855f3213b79dee6f5a25fdb563f90c056e` for Linux AMD64 from a temporary
`git archive` source, with per-build-container resource limits of 6 GiB and two
CPU equivalents. The local image ID was
`sha256:443b6c861f892f520dedaf0cd33d11bf975bdea69976c8dac05cf3181060169e`.
A uniquely named disposable Compose project using the staging adapter started
the app and network-isolated PDF service on a dynamic loopback port. It created
a product and area via authenticated MCP, rejected unauthenticated mobile reads,
checked product reads and PDF output, restarted both services, and verified the
product and PDF again. The fixture's containers, network, volumes, temporary
source and image tag were removed. Build cache was not pruned. The existing
native staging and production services remained active.

This was a branch compatibility fixture, not a deployment or acceptance of the
stable staging environment. The image was removed and cannot be promoted. A
merged default-branch revision must still pass Mac integration, then a
target-compatible artifact from that revision needs real VPS staging, ingress,
browser, model-host and persistence acceptance.

## VPS Studio to Model Host HTTPS fixture

The VPS Model Host staging endpoint advertised `smol-test` (text) and
`sd-v1-5` (image); no coding model was advertised. The small text model returned
a nonempty completed response. A separate rootless Studio container built for
Linux AMD64 from branch revision `56bbeb28d5b4d0a7000584486807bd6da3174d67`
used the real staging Model Host HTTPS endpoint. Authenticated discovery listed
`smol-test`, MCP returned a nonempty completion, and mobile API streaming reached
its terminal event with text. The host's management-capable staging token was
used only in the disposable fixture; Studio data was on tmpfs and the container,
temporary source and uniquely tagged image were removed. Build cache was retained.
The Model Host staging container remained healthy, and native Studio staging and
production remained active. Inference may warm the existing host and record its
normal telemetry; no service configuration or deployment was changed.

This validates one real HTTPS path with a small text fixture, not a coding model,
physical browser, scoped deployed host token or stable Studio staging deployment.

## Clean branch image and CI browser origin

On 2026-11-21, an isolated BuildKit worker built the Dockerfile without cache
from committed revision `0fa6849922474837c95bfb0afc7fe308c107b32a` for
Linux ARM64. Its OCI manifest was
`sha256:e5e79ed0c14ae6f0f024a6e05c251a605f563fd1a30131b6bb1fd5a5293c3726`.
The private artifact and build log are retained under this worktree's environment
artifacts. The temporary filesystem was unmounted and truncated in the VM;
the previous branch image and a checked SQLite backup were retained for rollback.
Only unshared, reclaimable AruviStudio BuildKit records were removed to recover
capacity before this build; other products and persistent volumes were preserved.

The branch app and network-isolated PDF service started healthy using the clean
image. Product and work-item creation via MCP, mobile authorization, PDF export,
restart and persistence passed. Forty-one remote browser tests, PDF isolation
and outage checks, 51 workflow tests, and the simulated CI artifact test passed.
The published-port browser MCP test accepted an authenticated same-origin ping
and rejected missing credentials and cross-origin requests. Both hosted checks
for this revision passed. These are branch tests, not staging acceptance.

The initial combined UI test failed at CI artifact download because the
container browser used a non-local HTTP origin, where Chromium disables Web
Crypto. The CI fixture now forwards a loopback browser origin to the isolated
app and explicitly checks that the origin is secure; no application security
policy was relaxed. The corrected container CI test and the host-Chrome CI
fixture both passed. The original combined invocation did not finish; its
remaining workflow suite was run separately and passed.

The clean image predates the test-fixture and documentation changes in this
checkpoint, which do not change the runtime image. Complete desktop parity,
real-provider and device checks, Mac integration on a merged default revision,
and persistent VPS staging with ingress and Model Host acceptance remain open.

A further settings source audit found that desktop's `docker_host` and
`max_workflow_retries` values are persisted but have no backend consumers in
the current source. New workflow runs initialize their maximum to three; the
test stage executes the configured command in the server environment and the
Docker service is a stub. This is an existing behavior gap, not evidence that a
Docker sandbox or the configured retry limit is active. See the settings action
inventory in `web-parity-audit.md`; retry semantics and execution isolation still
need design and validation.

## Workflow test policy conflict protection

The browser now sends its loaded effective product test policy as an expected
snapshot. The server compares it with the current product override and inherited
default under an immediate SQLite transaction, and rejects stale changes with
HTTP 409. The browser retains its draft on conflict and offers an explicit
reload. The snapshot remains optional for existing API clients; legacy API and
generic MCP writes are still last-write-wins. A focused API test covered a
changed default, a changed product override and concurrent writes from the same
snapshot; a disposable native server and host Chrome exercised draft retention,
reload and retry. The native Rust library suite passed 293 tests with seven
ignored, and the terminology check passed. This protects policy changes but does not provide command
isolation or implement the unused Docker and retry settings noted above.

The clean Linux ARM64 Dockerfile build of commit
`b5004f779f5fc63c033af01856c0b4fdb4ad50f4` produced image manifest
`sha256:95f531d7de7625555467e55cf581d221aa2d5f0701f3f44eaccce9a5c8f52da8`.
The isolated worktree app and PDF sidecar are running that image, with the
previous image and a checked pre-deployment SQLite backup retained. The full
worktree UI command passed: section navigation, 41 remote tests, PDF isolation
and outage checks, the simulated CI test and 52 workflow tests including the
new conflict case. MCP product and work-item creation, authenticated mobile
reads, PDF export and restart persistence passed. Browser-origin MCP rejected
missing credentials and cross-origin requests, and the local 7B coding-model
fixture passed through MCP. The temporary BuildKit filesystem was released.
These checks do not establish complete desktop parity or staging acceptance.

## Workflow command child cleanup

A native regression test reproduced a successful test command leaving a
background child running. On Unix the test runner now signals the command's
process group on normal exit, timeout and future cancellation. On macOS and
Linux it observes the leader's exit without reaping before signaling, avoiding
normal-exit process-group ID reuse. Focused macOS tests cover all three paths;
the native Rust library suite passed 296 tests with seven ignored. An isolated
workflow fixture also checks for a surviving child. Processes can
change group or session, and this is not a sandbox; command execution isolation
and the unused Docker/retry settings remain open.

The clean Linux ARM64 image built from commit
`70525aa56780748cf1ce8482441d277d034e8bba` has manifest
`sha256:33e28ea9b963f0893094358784645216942a301cd82efc8d184b86cc370cde8a`.
Before deploying it, the isolated Linux workflow fixture passed 52 tests and
confirmed that a test command's delayed background child did not leave a marker.
After deployment, the app and PDF service passed product creation, authorization,
PDF and restart-persistence acceptance; 41 remote UI tests, PDF isolation and
outage checks, the simulated CI fixture, published-port browser MCP checks, and
the real local 7B coding-model MCP fixture passed. Timeout and cancellation
cleanup were tested natively on macOS; those two paths were not independently
exercised on Linux. The previous branch image and a checked database backup are
retained. An older image was removed from Docker after verifying its retained
OCI archive, and only old rebuildable Studio compiler outputs were deleted to
make build space. Staging and complete desktop parity remain open.

## OpenAI-compatible stream finish ordering

A regression fixture reproduced an OpenAI-compatible SSE stream accepting a
second choice after `finish_reason: stop`, including a later text delta or a
duplicate finish. The shared client now rejects either sequence for streamed
and buffered SSE, while allowing usage-only events followed by `[DONE]` and
still accepting compatible providers that omit a finish reason. Earlier
streaming callbacks may already have received text before a later error. All
66 standalone client tests passed, and the changed client streamed a real
`qwen25-coder-7b-q4` response from the local Model Host. This single host/model
sample does not validate other compatible services or live cloud providers.
The standalone package was built and its 66 tests passed after extraction;
Studio's native Rust suite passed 296 tests with seven ignored.

The documented independent-consumer pin now references the pushed client
commit `342520c95f4481d72646f83f9394c45134dc2864`. Cargo vendored its locked
dependencies, and a read-only, network-disabled Linux container passed all 30
independent consumer tests. The vendored client source matched the Studio
checkout. A separately built non-root, read-only Linux probe using that pin
reached the real local host and passed discovery, buffered completion, streaming
and token-limit checks with `qwen25-coder-7b-q4`. It used the existing
management-capable host credential; this does not resolve credential scope or
establish compatibility with other hosts and models.

The independent fixture subsequently added a regression case for a choice
arriving after `stop`. The full fixture passed 31 tests both on macOS offline
and in a read-only, network-disabled Linux container using vendored sources.

The clean Linux ARM64 image from Studio commit
`6e705cebeed762e3418aefbe194c68f22eaa313f` has manifest
`sha256:1db276f62096887775eb3809ddafa74d90d0e7210f49eca512bb63d2aaf55598`.
Its disposable workflow fixture passed 52 tests before the branch deployment
was refreshed. The running app and PDF sidecar passed product, MCP, mobile
authorization, PDF and restart-persistence acceptance; 41 remote browser tests,
PDF isolation/outage and simulated CI checks passed. Browser-origin MCP checks,
a real local 7B MCP coding request and authenticated real-model browser streaming
also passed. A checked database backup and the prior image were retained. This
does not establish live cloud-provider compatibility or staging acceptance.

## Real-model existing-module workflow revalidation

On the deployed `6d23c1d`-based branch image, a new disposable run used
`qwen25-coder-7b-q4`, JSON-object coding mode, a work-item workspace override,
and a 4,096-token non-coding budget. The real workflow reached plan approval
and test review; the model received the duplicate-definition guidance and its
generated edit passed the workflow syntax check. Independent assertions ran
the generated code in a network-disabled, read-only, non-root container and
checked both existing `sumEven` behavior and the new `clamp` cases, including
the invalid range. A task-specific duplicate-declaration check also passed.
The product default workspace remained untouched, and after review the
disposable Git remote contained the verified file. This single successful run
does not resolve the failures or establish general model reliability.

A second run with the same setup used headless Chrome for both the plan and
test-review approvals (`--browser-approvals`). The browser observed each gate
and completed the action against the isolated container. Independent syntax,
behavioral and duplicate-declaration checks passed before the browser approved
delivery; MCP then confirmed the workflow was done and the disposable Git
remote contained the reviewed file. An initial browser attempt failed before
approval because its test selector matched several controls; it confirmed that
the remote was unchanged. These runs are samples with a 7B model, not a claim
of general model or physical desktop reliability.

Against the refreshed branch image and real `qwen25-coder-7b-q4`, a disposable
existing-module workflow used an alternate work-item workspace and a 4,096-token
budget for non-coding agents. Its syntax test passed, but independent review
found a duplicate function declaration and stopped delivery; the product's
default workspace and disposable Git remote were unchanged. A second run with
the managed coding prompt template completed both approval gates, syntax and
independent behavioral and duplicate checks, and delivered the reviewed file
to the disposable local remote. These two samples use a management-capable host
credential and do not establish model reliability or authenticated network Git
delivery. The syntax-only test gate by itself does not detect duplicate
declarations.

The coding tool-loop prompt now asks agents to edit existing definitions in
place rather than duplicating them, and to inspect the result for duplicate
definitions and missing exports when repository reads are permitted. A prompt
test checks both the read-enabled and write-only configurations. This guidance
does not enforce correctness or replace project-specific behavioral tests and
review; its effect on real-model reliability requires further samples. The
native Rust library suite passed 296 tests with seven ignored.

On the candidate image with that guidance, a third real-model attempt showed
the guidance in the recorded coding request, but produced invalid JavaScript.
The syntax gate stopped the workflow and the disposable remote remained
unchanged. Its trace showed a successful multi-line replacement followed by a
different replacement using the original line number without a fresh read.

The coding tool loop now guards against this sequence when file reads are
permitted: after a successful write, patch or range replacement, another range
replacement on that path requires a complete, untruncated read covering the
current target lines. Reads of other files, truncated reads and unrelated line
ranges do not satisfy the check. Write-only agents retain their previous
distinct-edit behavior because they cannot perform the required read. This is
an in-process safeguard for cooperating tool calls, not protection from external
writers, model errors after a read, or every possible stale edit. A coding-stage
fixture exercises rejection of a changed replacement after an earlier write;
the native Rust library suite passed 298 tests with seven ignored.

The clean Linux ARM64 image from commit
`bf9d43f6e0f3379a98f55b1809236e6b60cadc19` has manifest
`sha256:73f264c49baded4ddb5b66a4bedd4aa8425486a0a76c125943bbae0f157c4377`.
Before deployment, 52 simulated workflow tests passed. A separate disposable
real `qwen25-coder-7b-q4` existing-module workflow confirmed that the new
guidance reached the coding model, passed syntax and independent behavior and
duplicate checks, and delivered the reviewed code to a local Git remote. The
work-item override was used and the product-default workspace stayed unchanged.
This run did not trigger the stale-line guard; its rejection path was exercised
by the deterministic coding-stage fixture. After deployment, product creation,
MCP and mobile authorization, PDF export and restart persistence passed. A
checked pre-deployment database backup and the previous image are retained. One
successful model sample does not prove general reliability or staging readiness.
The deployed image also passed 41 remote browser tests, PDF isolation/outage,
the simulated CI fixture, published-port browser MCP checks and a real local
7B MCP coding request. An additional focused guard test verified that an
offset read cannot satisfy the fresh-line requirement; that test-only change
postdates the built image.

## Database override conflict protection

Database path override changes now use a temporary file and atomic rename,
with a process mutex and an advisory file lock on Unix for cooperating Studio
processes. Browser saves compare their loaded override and preserve the draft
on conflict; an explicit discard-and-reload action retrieves the saved value.
MCP clients may supply the same expected snapshot. Existing desktop and legacy
API/MCP writes remain last-write-wins, direct file writers and older binaries
do not take the lock, and value comparison does not detect an A-to-B-to-A
change. The write still does not validate the database at the new path.

Focused tests covered stale and simultaneous API updates, MCP conflicts,
cross-process coordination and atomic reader visibility. A disposable native
server and host Chrome exercised a conflicting MCP write, browser draft
preservation and reload, then migration, override and clear across restarts and
environment precedence. The native Rust library suite passed 303 tests with
seven ignored. This does not establish Windows cross-process locking,
all historical migrations, or staging acceptance.

The clean Linux ARM64 image from commit
`029a5619d89ce60489e75430d6682835553cd13e` has manifest
`sha256:5de7577d779ae1f8d8fa560fdd039332755da3904d6c0acad1f218d4ede7fe06`.
A disposable two-container test sharing one data volume reproduced lost
updates on the prior image and verified ten concurrent guarded update pairs on
the candidate, each with one success and one conflict. The isolated browser
workflow suite passed 52 tests, including a browser draft conflicting with a
legacy MCP update. After refreshing the worktree deployment, product/MCP,
mobile authorization, PDF and restart-persistence acceptance passed, as did
41 remote UI tests, PDF isolation/outage, the simulated CI fixture, browser
MCP-origin checks and a real local 7B MCP coding request. The old image and a
checked pre-deployment database backup are retained. These are branch checks,
not validation of the stable integration or VPS staging environments.

## Docker Desktop host-local model connection

The seeded LM Studio provider in the current branch uses `localhost:1234`,
which refers to the Studio container in Docker. No LM Studio server was
listening on the Mac during this check. The browser Models form now offers an
explicit Docker Desktop host conversion for loopback HTTP(S) URLs; it preserves
the port and path, rejects credentials, queries and fragments, and only changes
the form until the user saves. Existing provider settings are not silently
rewritten. A native browser fixture checked new and existing provider forms,
IPv4/IPv6, invalid inputs and explicit save.

A disposable Docker Studio fixture created two synthetic providers against a
Mac-loopback OpenAI-compatible test service: container loopback discovery
failed as expected, while `host.docker.internal` supported model discovery and
MCP completion. This validates the Docker Desktop networking path, not a live
LM Studio server, HTTPS certificate, other operating systems or model quality.

The clean Linux ARM64 image built from commit
`7e53964a07ca39a0944b92c8271af3ddcb11de48` has manifest
`sha256:e6fd74e9591abe565a3d78b7f25f72425f9a4d2ed59cbb209930b4320a22aff3`.
The disposable Docker host fixture passed against this image, as did all 42
remote browser tests and the PDF outage check. After refreshing the worktree
deployment, product, MCP and mobile authorization, PDF and restart persistence
acceptance passed. Published-port browser MCP origin checks and a real local
`qwen25-coder-7b-q4` coding request and token-limit error also passed. A
checked pre-deployment database backup and the previous image are retained.
These checks do not establish live LM Studio compatibility, full desktop parity,
or integration and staging acceptance.

## Workflow start capacity

New workflow starts now atomically reserve the work item and team slot when
creating the assigned run. This prevents a capacity rejection from leaving a
phantom active run and prevents concurrent starts from taking the same final
slot. A SQLite fixture covered contention, rejected-run absence, slot reuse and
duplicate work-item rejection; the native Rust library suite passed 304 tests
with seven ignored. A subsequent change also checks capacity atomically when
an active run is assigned or moved to a team; a focused contention fixture
checks slot reuse. The native library suite passed 305 tests with seven ignored
after this change. Automatic stage retries and execution isolation remain open. This change has
not yet been deployed to the running worktree image or tested in staging.
An attempted clean Linux ARM64 image build from commit `5cff1f1` was stopped
when free host storage fell below the 3 GiB safety threshold. The isolated
builder exited and its scratch filesystem was unmounted and truncated; the
previous worktree image remains running. The later hosted build and its
validation are recorded below; deployment still needs Docker headroom.

## Independent client pin drift

The consumer pin check now compares the current client manifest, source file
set and license with the pinned Git revision, in addition to checking the
documented revision and lockfile. A temporary Git fixture verifies matching,
modified, added and removed source files. The current checkout matches the
independent consumer pin. The check requires that Git revision locally; it
does not replace the separate offline consumer build or live provider checks.
The normal GitHub test workflow now runs this guard. A shallow-clone fixture
verified that a missing pinned commit is reported and that fetching the exact
revision allows the checks to pass. The hosted workflow itself still needs a
completed run after this change.

## Hosted ARM64 candidate and Docker capacity

GitHub Actions run 44908077935 built commit
`b2e429737275b1f3f74d75608980aa0a4516aed0` without cache on Linux ARM64.
The downloaded archive passed its SHA-256 check and recorded that revision.
The hosted Docker image configuration was
`sha256:5e752d730adde021e49e60ad4428dd1b602c16ca306dbe9fad74b1061cdb5154`;
Docker Desktop represented the loaded image by manifest ID
`sha256:17662c938a300611f973cf3f298b718004e72b66340e6d9352994acb988560ad`.
Exporting the loaded image confirmed the same configuration digest. The
candidate passed 52 workflow tests, 42 browser tests, the simulated CI fixture,
PDF isolation/outage checks, and the Docker-to-Mac model-host fixture.

After the candidate was loaded, the shared Docker VM filesystem became full.
Both the candidate and previous image then failed SQLite startup with a disk
I/O error. The candidate image was removed from Docker, while its verified
archive was retained in the worktree artifact directory. With space restored,
the previous image `sha256:e6fd74e9591abe565a3d78b7f25f72425f9a4d2ed59cbb209930b4320a22aff3`
restarted successfully. Product, MCP/mobile authorization, PDF and restart
persistence acceptance, browser MCP origin checks, and a real local 7B MCP
coding request passed on the restored image. A checked database backup from
before the attempted deployment is retained. The candidate is not deployed;
its worktree deployment still requires sufficient Docker filesystem headroom
and acceptance. A separate derived image was subsequently deployed as described
below. This does not establish staging or full desktop parity.

## Branch deployment using the hosted server binary

The server binary was extracted from the verified hosted image archive and
installed over the previous branch runtime image. Its SHA-256 is
`ed157a11c317c8bbe208c1684d82d482d671fa6a74007e2830f84fb58ad42a82`;
the derived image's Docker Desktop manifest ID is
`sha256:089db6c1f6a4fba2f0ad2bea5419a616cc069da365420dfe6b4700bd4260fb09`.
Its source revision is `b2e429737275b1f3f74d75608980aa0a4516aed0`.
Between the previous image's source revision `7e53964` and that revision,
runtime code changes were limited to workflow service and repository files;
the Dockerfile, frontend, dependency manifests, lockfile, resources and
shared client source were unchanged. The derived image is not the clean hosted
image: it retains the previous image's runtime layers. Its Dockerfile, logs,
hosted archive and checked pre-deployment database backup are retained in the
worktree artifacts.

The derived image passed 52 isolated workflow and 42 browser tests, PDF
isolation and outage checks, and the Docker host-model fixture. After branch
deployment, product creation, MCP/mobile authorization, PDF and persistence
passed after restart; browser MCP origin checks and a real local 7B MCP coding
request also passed. Docker VM free space remained about 2.5 GiB. To recover
space after an initial derived build unpacked the base layers, one unused older
rollback image was removed from Docker after verifying its preserved OCI
archive and manifest; the current previous image is retained. Integration,
staging and complete desktop parity remain unverified.

The isolated workflow suite now also covers team capacity through MCP and the
authenticated browser API. With capacity one, it starts the first work item,
checks that a second start reports capacity without creating a workflow run,
raises capacity through MCP and starts the second item. All 53 workflow tests
passed against the deployed branch image with the simulated model provider.
This validates those interface flows; it does not establish automatic retries,
live third-party providers or staging acceptance.

The two-server Linux planner fixture now also primes one server with a staged
design, clears it through the other, and verifies the stale server's subsequent
confirmation does not apply the cleared product area. It passed three repeated
runs on the running branch image along with its existing concurrent-edit and duplicate-confirmation
checks. This is one ordered clear/confirm scenario for cooperating processes;
it does not cover every concurrent interleaving or non-cooperating writer.

## Atomic workflow transitions

The main workflow stage transitions and failure recording now update the
stage, lifecycle and history in one SQLite transaction, conditioned on the
observed stage and a running status. A conflicting or finalized run is not
overwritten. The separate advance path uses the same database operation and
marks a transition to `done` as completed. Fault-injection tests cover history
write rollback, concurrent stale transitions, failure recording and terminal
state protection. The native Rust library suite passed 311 tests with seven
ignored before the final terminal-guard fixture, which was checked separately.
This does not make agent side effects, coordinator pending-stage updates or Git
push atomic with SQLite. The advance path can perform a Git push before its
database transition and concurrent external delivery remains a separate risk.
Against a disposable native server and simulated provider, a browser/MCP
capacity workflow and a browser approval flow that changed coordinators and
reached test review both passed. Subsequent container validation and branch
deployment are recorded below.

A macOS subprocess fixture subsequently started two independent processes
against one temporary SQLite database and raced the same transition. Across
five runs exactly one process succeeded, the other received a conflict and one
history entry was recorded. This checks one transition on the local filesystem;
the later hosted Linux test run also passed. External side effects remain
separate checks.

The new `workflow-transition-multiprocess` fixture starts two disposable
Linux Studio containers against one temporary volume, holds a SQLite write lock
while both request the same gate rejection, and checks the resulting state and
history. Against the previously deployed image it reproduced two successful
decisions. The atomic-transition candidate passed five races with one success,
one rejected decision and one history entry per race.

The hosted clean Linux ARM64 build of
`68e46c9a059f6c99da26dd608367cafb6b32fa96` passed; its archive checksum and
source revision were verified. The extracted server binary (SHA-256
`c57411451542129709a081443362d7bcc2166d631d83c622b2703d17b87c5375`) was
installed over the preceding branch runtime, producing derived image
`sha256:6d5e6e0059ac96fad429b4a5d8f757cf51ba0ac181fe319dbab329b7059d527c`.
This is not the complete hosted image. In addition to the two-container race,
the candidate passed 53 workflow tests, 43 general browser tests, the dedicated
bridge-token browser fixture, PDF isolation/outage and the Docker host-model
fixture. After deployment, product, MCP/mobile authorization, PDF and
persistence acceptance passed after restart, as did browser MCP-origin and real
local 7B MCP coding checks. The verified archive, build records, checked
pre-deployment database backup and previous image are retained. Docker VM free
space remained about 2.0 GiB. Git pushes and other external effects are still
outside the SQLite transaction; integration and staging are unverified.

## Browser bridge bind settings

The browser Activity view now loads and edits the saved server bind host and
port. Its authenticated API validates inputs and atomically compares both
loaded settings before writing, preserving an unsaved draft when another client
changes either value. It explains restart, environment override and Docker port
publishing behavior. A disposable native server and headless browser exercised
a conflicting MCP edit, draft preservation, reload and successful save. A Rust
API test covered authorization, malformed input, transactional rollback and
simultaneous writes. Token administration remains a separate parity item; the
settings do not themselves change the currently bound listener. The native Rust
library suite passed 306 tests with seven ignored, and the frontend build and
terminology check passed. Subsequent branch image validation is recorded below.

`bash scripts/test native-bridge-bind <server-binary>` additionally starts a
disposable native server with temporary credentials and storage. It verified
unauthorized reads are rejected, the saved host and port persist across
restarts, environment overrides take precedence, and the server listens on the
saved port after the overrides are removed. This does not validate container
port publication or a remote network client.

The hosted clean Linux ARM64 build of commit
`7c47d8c2db46a0619c5bf3c90b6c51e922c12cb0` succeeded; its archive checksum
and recorded revision were verified. The server binary, SHA-256
`69d00c100f991cd13045fe7ec4b18742a8c9206db658fbf8e72f743aa08c41b2`,
was applied to the preceding branch runtime image to create the derived image
`sha256:8ddba210fef2a8b959b4239c66a17c79f7b8bd9a006b3f79a347d6226665879e`.
This is a derived image, not deployment of the complete hosted artifact. It
passed 43 isolated browser and 53 workflow tests, PDF isolation/outage and the
Docker host-model fixture. The refreshed worktree passed product, MCP/mobile
authorization, PDF and persistence acceptance after restart, browser MCP-origin
checks and a real local 7B MCP coding request. A checked database backup and
the previous image are retained. The Docker VM still had approximately 2.3 GiB
free; integration, staging and full desktop parity remain unverified.

## Browser bridge token administration

Activity now exposes write-only replacement and clearing for saved mobile and
MCP tokens. The authenticated API returns only configuration flags and opaque,
process-local keyed revisions; compare-and-swap rejects stale changes. Token
changes take effect immediately unless an environment token overrides them. A
browser replacing its active saved mobile token switches to the new credential;
clearing it disables browser/mobile access until an administrator configures a
token externally. The form warns before both actions and preserves a draft on a
conflict or request error. A response interrupted after a successful write can
leave the new credential active. Revisions are process-local and value-based;
they do not detect an A-to-B-to-A change, and another process or restart requires
a reload. Generic desktop/MCP setting operations still expose stored values.

A disposable native server and headless browser verified conflict preservation,
environment-provided MCP credential precedence, active mobile token rotation,
old-token rejection, persistence after restart, access removal on clear and
recovery using an environment token. The Rust library suite
passed 307 tests with seven ignored; frontend build and terminology checks
passed. The subsequent branch image and deployment checks are recorded below.

The clean hosted ARM64 build of `9a9d9c0ffa56723cd90760e79389d8d2acfb4f24`
passed and its archive checksum and source revision were verified. The server
binary (SHA-256 `b01ebbfbe762d46f3c25c6486010b6fd02678d41b05418f38f34f10e32de0941`)
was installed over the preceding branch runtime, yielding derived image
`sha256:c6918007d3051beb517d71d8cf75f2aeb4cc3abffafa47d31500ba7733bfdc03`.
This is not the complete hosted image. A disposable Docker volume, container
and headless browser verified token conflict, rotation, restart, clearing and
environment recovery; the same candidate passed 43 other browser tests, 53
workflow tests, PDF isolation/outage and the Docker host-model fixture. After
deployment, product, MCP/mobile authorization, PDF and persistence acceptance
passed after restart, as did browser MCP-origin and real local 7B MCP coding
checks. A read-only check confirmed the running branch's mobile and MCP tokens
are environment-controlled. The verified archive, build records, checked
pre-deployment backup and previous image are retained. Docker VM free space
remained about 2.2 GiB; this is not integration or staging acceptance.

## Workflow Git delivery coordination

Manual workflow advancement and the normal Git delivery stage now coordinate
through a per-run in-process lock and, on Unix with a file-backed SQLite database,
a private advisory lock beside the database. Advancement records entry to the Git
stage before delivery and completes only after the push succeeds; an error is
recorded as a failed Git stage. A persisted delivery commit permits retry after a
failed push. This changes manual advancement from push preparation to proceed
through delivery to completion in the same call. A failure from a delayed stage
executor is rejected if another request has already advanced the workflow.

Focused native tests exercised two concurrent advances against a local bare Git
remote, an unavailable delivery, and separate processes contending for the same
lock. The native Rust library suite passed 316 tests with seven ignored. The lock
requires cooperating versions and a filesystem that supports shared
advisory locks; Windows and in-memory databases have only in-process coordination.
It cannot make a remote Git push and a SQLite update one atomic operation: a
crash after a successful push may cause a retry of the same recorded commit.
These tests do not establish full parity or staging acceptance.

The final native Rust suite on `1e5d3e5677650d1f43bcea713c26c0f4fa391eb7`
passed 316 tests with seven ignored; 128 frontend tests, the frontend production
build and terminology check also passed. Both hosted test runs and the clean
Linux ARM64 image build passed for that revision. Its archive checksum and source
revision were verified. The extracted server binary (SHA-256
`c9744655773d871a5474aad3741c6ad8e650c9d9c65fe11889b77711ca1ed341`)
was applied to the preceding branch runtime, whose non-Rust runtime inputs were
unchanged, producing derived image
`sha256:dcd18b42b42845e3950d9e98215bd91ba0da575b5d5d535a68308ae14c9cbaf5`.
This is a derived image, not the full hosted artifact.

The candidate passed five two-container transition races, 53 isolated workflow
tests, 43 general browser tests and a separate token-administration browser
test, PDF isolation/outage checks, and Docker-to-host model discovery and MCP
completion against a fixture. After deployment, product, MCP/mobile authorization,
PDF and persistence acceptance passed across a restart; the published-port
browser MCP-origin checks and a real local 7B MCP coding request also passed.
The branch endpoint after acceptance was `127.0.0.1:55253`; its port can change
when restarted or recreated. A SQLite backup passed `quick_check`, and the prior image is
retained for rollback. Mac integration, staging, real external services and full
desktop parity remain unverified.

The two-container fixture was subsequently extended to hold the delivery lock
while two MCP clients advance the same workflow against a local bare Git remote.
The candidate waited and produced one delivery commit and a completed workflow;
the preceding image failed the fixture by bypassing the held lock. This proves the
tested cross-process scenario, not exactly-once delivery after a process crash.

### Workflow model-call parity (in validation)

The selected work item's latest workflow can now display its recorded model
calls in the browser. MCP exposes the same desktop query through
`aruvi_workflows` / `list_agent_model_calls_for_workflow` with a
`workflow_run_id` argument; the browser uses an authenticated workflow-specific
endpoint. The added isolated workflow test checks authorization, workflow
filtering, MCP/HTTP agreement, and the browser display. See the later deployment
checkpoint at the top of this file for the completed Linux and branch checks.
This adds call summaries; it is not proof
of complete desktop workflow-review parity. The query currently returns all
calls in the workflow, as does the desktop query, without pagination.
