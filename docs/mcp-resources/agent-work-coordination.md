# Aruvi Agent Work Coordination

## Purpose

Use the `agent_work.*` MCP tools as the durable coordination layer for multi-agent roadmap execution.

This replaces file-first checkpoint protocols where agents edit local status files or private SQLite ledgers. Agents should treat the MCP server as the source of truth for:

- roadmap run identity
- feature/work rows
- batch claims
- lease heartbeats
- conflict-zone locks
- status checkpoints
- append-only event history
- commit linkage

## Execution Loop

1. Read `aruvi://guides/product-philosophy` and the relevant product tree before creating or modifying work.
2. Use `agent_work.runs.upsert` to create or resume a run.
3. Seed or update feature rows with `agent_work.items.upsert`.
4. Record dependency edges with `agent_work.dependencies.upsert` when one row must wait for another.
5. Inspect implementation context with `agent_work.context.get_feature` before editing code.
6. Claim work with `agent_work.items.claim_next`.
7. Heartbeat active claims with `agent_work.items.heartbeat` while work is in progress.
8. Append structured validation and changed-file evidence with `agent_work.evidence.append`.
9. Append significant checkpoints with `agent_work.events.append`.
10. Move work through `agent_work.items.update_status`.
11. Link the final commit with `agent_work.commits.link`.
12. Use `agent_work.runs.health`, `agent_work.runs.summary`, and `agent_work.events.list` to resume after interruption.

## Claiming Rules

Agents must never pick work by scanning local files and assigning themselves a row.

Use `agent_work.items.claim_next` so the server can:

- select a pending row
- create or update a batch
- issue a claim token
- create lease-backed conflict locks
- append the claim event

The claim token is the agent's guard for heartbeats and status changes. Agents should include it whenever a tool accepts `claimToken`.

If an agent crashes or a lease expires, a coordinator should use `agent_work.items.requeue_expired` before assigning more work. Do not hand-edit claimed rows.

## Implementation Context

Before implementation, agents should call `agent_work.context.get_feature` using either `featureId` or `workItemId`.

The context payload provides:

- product and product area
- feature details
- feature ancestors, children, and siblings
- attached stories and tasks
- selected story/task parent chain and siblings
- scoped references
- dependency and evidence history for the run

Use `agent_work.context.export_feature` when a worker needs a durable handoff file for a large feature or full product tree.

## Conflict Zones

Every seeded row should include `conflictZones` when concurrent edits could collide. Use stable, coarse identifiers:

- `repo:<repository-id>`
- `module:<module-name>`
- `path:<repository-relative-path>`
- `product:<product-id>`
- `area:<product-area-id>`
- `feature:<feature-id>`

Prefer a small set of meaningful zones over one zone per tiny file. The goal is parallelism without merge churn.

## Status Model

Feature row statuses:

- `pending`
- `claimed`
- `in_progress`
- `implemented`
- `tests_passed`
- `committed`
- `blocked`
- `skipped`
- `cancelled`

Batch statuses:

- `claimed`
- `in_progress`
- `implemented`
- `tests_passed`
- `committed`
- `blocked`
- `skipped`
- `cancelled`

Use `blocked` only when the agent cannot make progress without external input. Include the blocking condition in `details`.

## Scaling Notes

For hundreds of agents:

- use pagination on list calls
- prefer `runs.summary` over dumping full ledgers
- heartbeat only active claims
- claim small batches but lock coarse conflict zones
- append events for meaningful milestones, not every token or file read
- record structured evidence for tests, diffs, changed files, and validation outcomes
- link commits immediately after verification
- keep local checkpoint files as disposable caches, not authoritative state
