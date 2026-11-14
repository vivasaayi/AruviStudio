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

## Browser planner node review

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

## Open design questions

- Decide whether to move the Git-pinned client to a separately maintained
  repository, and establish release and semantic-versioning policy.
- Determine whether native model-host loopback forwarding from Docker is an
  intentional supported network path and how narrowly to provision its token.
- Establish provider-specific requirements from official protocol documentation
  before implementing them. Do not equate ChatGPT subscriptions with API access.
- Preserve the previously audited planner concurrency and parity gaps; this goal
  does not imply those have already been solved.
