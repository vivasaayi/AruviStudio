# Model integration handoff

This is a historical handoff. Work has resumed since it was written; use the
latest checkpoints in [model-client-goal.md](model-client-goal.md) for the
current deployment, disk capacity, validation evidence and remaining work.
In particular, the later clean-image checkpoint supersedes the full-disk and
branch-image statements below. Worktree ports are dynamically assigned.

Paused on 2026-11-14. The goal is not complete and no release has been
published. The detailed chronological evidence is in
[model-client-goal.md](model-client-goal.md); the desktop/web/MCP inventory is
in [web-parity-audit.md](web-parity-audit.md).

## Current state

- The active Studio checkout is `/Users/rajanthegoat/work/AruviStudio`, branch
  `codex/aruvi-web-docker`. Its last implementation commit before this handoff
  was `60be2ca`, pushed to the same remote branch. Merge into master was not
  verified as part of this handoff.
- The isolated worktree Docker app and PDF services were healthy at pause. The
  app address was `http://127.0.0.1:55246/remote`; the port can change after a
  restart. Run `bash scripts/env worktree status` in the active checkout for
  the current address.
- `crates/aruvi-model-client` is the shared Rust transport used by Studio. It
  covers OpenAI-compatible text, streaming, discovery and remote voice,
  Anthropic text, and DeepSeek's separate voice endpoints. It is consumed from
  the Studio Git repository at an immutable commit; it is not a separately
  published crate. The independent fixture and its lockfile currently pin
  `12791e9fcd018ea4847ff7a2591535d4de07914e`.
- Studio's browser and MCP have exercised real local Model Host text and
  synthetic speech; disposable workflows also exercised coding and delivery to
  a local Git remote. A later successful 7B coding run is one sample, not proof
  that arbitrary tasks or external Git delivery are reliable.
- The last broad checks included 267 native Rust tests passing with seven
  ignored, 128 frontend tests, 51 isolated browser/MCP workflow tests, and a
  mocked desktop provider-control test. The independent client passed 28 Linux
  tests and a real host text and synthetic Piper/Whisper probe. These checks
  cover their recorded scenarios, not all application features.

## Remaining work

1. Finish the action-by-action desktop, browser and MCP parity audit. Native
   window interaction, physical microphone capture and playback are unverified;
   the Mac console was locked when last checked. Mocked desktop UI tests are
   not native-window acceptance.
2. Validate real third-party OpenAI, Anthropic, DeepSeek and LM Studio sessions
   when appropriate services and API credentials are available. Existing
   contract tests use simulated providers. Confirm model-specific capabilities
   rather than assuming each endpoint supports every adapter operation.
3. Continue coding-workflow reliability and safety validation with meaningful
   project tests and review. Prompt requirements are included in bounded
   initial and follow-up contexts, but long fields are truncated and a model
   may still produce incorrect code or an inaccurate completion claim.
4. Resolve Model Host credential scope. The current running integration uses
   an existing token with management capability; an inference-only host change
   has been tested as a candidate but is not deployed. The candidate patch and
   its limitations are documented in the detailed checkpoint.
5. Review security and release evidence. The latest npm and independent-client
   scans reported no advisories; Studio's lockfile still reported RSA
   `RUSTSEC-2023-0071`, 13 unmaintained warnings and two unsoundness warnings.
   The inspected all-features/all-targets tree did not select an RSA path, but
   that does not prove safety for other configurations. The local SQLx patch
   passed provenance comparison and still requires release review.
6. Decide long-term ownership, versioning and migration for the shared client.
   The Git pin is a tested checkpoint, not a semantic-versioned release. Do not
   publish, deploy another project's service, or promote a release as part of
   resuming this goal unless separately requested.

## Resume safely

1. Inspect the current checkout, Git status and running environments before
   changing anything. The Model Host checkout at
   `/Users/rajanthegoat/work/aruvi-model-host` has unrelated uncommitted work;
   preserve it. The separate `aruvi-model-host-studio-validation` checkout
   backs a running validation host and should not be changed casually.
2. Docker's shared filesystem was nearly full. After the last scoped cleanup,
   it reported approximately 998 MB available. This can change independently;
   check it before a build. Do not run machine-wide Docker pruning or remove
   other projects' data. The Studio Rust build cache was removed during recovery
   and a fresh build may need substantially more space.
3. The current worktree's database survived the disk-full restart and a
   read-only SQLite `quick_check` returned `ok`; this was not a full data audit.
   Keep the worktree's persistent data and repository volumes intact.
4. Review the unchecked requirements and latest checkpoints in
   [model-client-goal.md](model-client-goal.md), then select the next task and
   record its tests and remaining limitations there. Use the repository's
   environment instructions and product-environments skill for worktree,
   full-stack test or deployment changes.

## Resumed checkpoint: 2026-11-17

Work resumed on the feature branch after this handoff was written. See the
latest evidence in [model-client-goal.md](model-client-goal.md). The isolated
branch Docker app is healthy at `http://127.0.0.1:55029/remote`; the Docker
daemon filesystem is full, so database writes and real inference through that
container can fail. The branch image itself built successfully.

The fresh browser/MCP suites, native Rust suite, standalone client tests and
real local Model Host checks passed, including native Studio MCP text and
synthetic speech. The native validation script now selects the host environment
explicitly. A new high-severity `picomatch` advisory affects the current npm
development tree and has no published upstream fix as of this checkpoint; the
production-only npm audit is clean. Native Tauri window and physical audio
acceptance, cloud-provider calls, inference-only host credentials, daemon
capacity recovery, and release review are still open. The full objective is
therefore not complete.
