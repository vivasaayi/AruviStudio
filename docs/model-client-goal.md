# Shared model client and Aruvi Model Host integration

This is the active implementation checkpoint. Completed items require the
evidence named below; simulated providers do not prove real-provider behavior.

## Objective

Create a reusable, independently consumable Rust library for model access, and
use it in AruviStudio while preserving existing desktop, web, MCP and workflow
behavior. Route local model inference to Aruvi Model Host. Do not move third-party
provider hosting into the model-host service as part of this work.

## Checkpoints

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
  pins `0a27533a35ce25f6bd8abbbd193521b20ea9c9ba`, commits its dependency lock,
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
  Studio configuration does not yet expose per-provider timeout settings.

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
  The shared-client tests cover these outcomes, including partial stream callbacks.

## Open design questions

- Decide whether to move the Git-pinned client to a separately maintained
  repository, and establish release and semantic-versioning policy.
- Determine whether native model-host loopback forwarding from Docker is an
  intentional supported network path and how narrowly to provision its token.
- Establish provider-specific requirements from official protocol documentation
  before implementing them. Do not equate ChatGPT subscriptions with API access.
- Preserve the previously audited planner concurrency and parity gaps; this goal
  does not imply those have already been solved.
