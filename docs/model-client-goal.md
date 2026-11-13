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
- [ ] Decide and implement a genuinely shareable library location and versioning
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

## Open design questions

- Select a maintainable library repository/distribution approach compatible with
  Cargo, Docker build contexts, independent projects and offline reproducibility.
- Determine whether native model-host loopback forwarding from Docker is an
  intentional supported network path and how narrowly to provision its token.
- Establish provider-specific requirements from official protocol documentation
  before implementing them. Do not equate ChatGPT subscriptions with API access.
- Preserve the previously audited planner concurrency and parity gaps; this goal
  does not imply those have already been solved.
