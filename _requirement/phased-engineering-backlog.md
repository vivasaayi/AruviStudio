# Phased Engineering Backlog

## 1. Backlog Overview

This backlog translates the product vision, PRD, and architecture into phased execution. It is organized by:

- phases,
- epics,
- implementation stories,
- and completion intent.

The sequencing is designed to deliver a usable system early while preserving the ability to scale into the full agent-driven architecture.

## 2. Delivery Principles

- Prioritize the end-to-end task loop over broad surface area.
- Build deterministic infrastructure before advanced autonomy.
- Make the product useful as a structured product/workspace manager before the full agent swarm matures.
- Ship visibility and auditability early, not as a cleanup step.

## 3. Phase 0: Discovery and Technical Foundation

Goal:
- lock product scope,
- confirm stack choices,
- and remove architectural ambiguity before feature-heavy implementation.

## Epic 0.1: Product and Domain Finalization

### Story 0.1.1

Define the canonical domain entities for product, module, feature, task, repository, workflow run, agent run, artifact, finding, and approval.

Acceptance intent:
- entity boundaries are fixed,
- hierarchy and inheritance rules are explicit.

### Story 0.1.2

Define the task lifecycle and workflow stage model.

Acceptance intent:
- stage names, transitions, and terminal states are documented.

### Story 0.1.3

Define approval rules and rework loops.

Acceptance intent:
- task approval, plan approval, and test-review approval are formalized.

## Epic 0.2: Technical Spikes

### Story 0.2.1

Validate Tauri + React + Monaco integration with a minimal shell.

Acceptance intent:
- desktop shell launches,
- Monaco renders correctly.

### Story 0.2.2

Validate Rust-to-frontend command/event communication.

Acceptance intent:
- commands and event streaming work reliably.

### Story 0.2.3

Validate OpenAI-compatible provider connectivity against LM Studio.

Acceptance intent:
- provider config can be tested,
- completions can be retrieved.

### Story 0.2.4

Validate Docker-based test execution from the Rust backend.

Acceptance intent:
- backend can launch a test container,
- capture logs and exit code.

## 4. Phase 1: Core App Shell and Local Data Platform

Goal:
- establish the persistent local-first desktop foundation.

## Epic 1.1: Project Bootstrap

### Story 1.1.1

Create the Tauri project scaffold with React and TypeScript.

### Story 1.1.2

Set up frontend state architecture and routing shell.

### Story 1.1.3

Establish backend service/module structure in Rust.

### Story 1.1.4

Add baseline structured logging support.

## Epic 1.2: Persistence Foundation

### Story 1.2.1

Create the SQLite schema and migration strategy.

### Story 1.2.2

Implement repositories for product, task, repo, workflow, artifact, and settings domains.

### Story 1.2.3

Implement local artifact storage conventions and metadata indexing.

## Epic 1.3: Settings and Local Configuration

### Story 1.3.1

Build settings persistence for application defaults.

### Story 1.3.2

Implement local provider settings and secret references.

### Story 1.3.3

Implement logging and debug verbosity configuration.

## 5. Phase 2: Product Hierarchy and Task Management

Goal:
- make the app useful as a structured planning system even before agent execution.

## Epic 2.1: Product Hierarchy CRUD

### Story 2.1.1

Implement product create, read, update, archive flows.

### Story 2.1.2

Implement module CRUD under products.

### Story 2.1.3

Implement feature CRUD with 3-level nesting support.

### Story 2.1.4

Build the left-panel hierarchy tree UI.

## Epic 2.2: Task Management

### Story 2.2.1

Implement task CRUD and task-detail view.

### Story 2.2.2

Implement nested subtasks.

### Story 2.2.3

Implement task metadata fields, acceptance criteria, constraints, and task types.

### Story 2.2.4

Implement task status model and state transitions for non-execution states.

## Epic 2.3: Approval Records

### Story 2.3.1

Implement approval storage and approval history.

### Story 2.3.2

Implement task approval UI.

### Story 2.3.3

Implement plan approval placeholder UI for future workflow integration.

## 6. Phase 3: Repository and Workspace Management

Goal:
- connect planning objects to real codebases.

## Epic 3.1: Repository Registry

### Story 3.1.1

Implement repository registration with local path, remote URL, and default branch.

### Story 3.1.2

Implement repository list and detail views.

### Story 3.1.3

Implement repository health/status checks.

## Epic 3.2: Scoped Repository Attachments

### Story 3.2.1

Implement product-level repository attachments.

### Story 3.2.2

Implement module-level repository overrides.

### Story 3.2.3

Implement task-level repository override.

### Story 3.2.4

Implement repository resolution logic and validation.

## Epic 3.3: Workspace Context UX

### Story 3.3.1

Display active resolved repo/workspace on task detail screens.

### Story 3.3.2

Surface blocked state if no repository resolves.

## 7. Phase 4: IDE Shell

Goal:
- provide the central coding and review environment.

## Epic 4.1: IDE Layout

### Story 4.1.1

Implement the shell layout with sidebars, main pane, and bottom panel.

### Story 4.1.2

Implement tab management.

### Story 4.1.3

Implement resizable panes.

## Epic 4.2: Editor and Explorer

### Story 4.2.1

Integrate Monaco Editor.

### Story 4.2.2

Implement file explorer for the active workspace.

### Story 4.2.3

Implement file open/save in the local workspace.

### Story 4.2.4

Implement search within the active repository.

## Epic 4.3: Review Surfaces

### Story 4.3.1

Implement diff viewer.

### Story 4.3.2

Implement source-control panel for task changes.

### Story 4.3.3

Implement artifact preview pane.

## 8. Phase 5: Model Provider Integration

Goal:
- connect the app to external or localhost LLM providers.

## Epic 5.1: Provider Configuration

### Story 5.1.1

Implement provider create/edit/delete flows.

### Story 5.1.2

Implement OpenAI-compatible provider client in Rust.

### Story 5.1.3

Implement provider connectivity test.

## Epic 5.2: Model Catalog

### Story 5.2.1

Implement model definition and catalog storage.

### Story 5.2.2

Implement model capability tags.

### Story 5.2.3

Implement model selection UI.

## 9. Phase 6: Agent Registry and Workflow Engine

Goal:
- establish the orchestration backbone before autonomous coding.

## Epic 6.1: Agent Registry

### Story 6.1.1

Implement agent definition schema and storage.

### Story 6.1.2

Implement agent registry UI.

### Story 6.1.3

Implement agent enable/disable controls.

## Epic 6.2: Workflow State Machine

### Story 6.2.1

Implement canonical workflow states and transitions.

### Story 6.2.2

Implement approval-gated transitions.

### Story 6.2.3

Implement retry, pause, resume, abort, and blocked handling.

### Story 6.2.4

Implement workflow persistence and restart recovery.

## Epic 6.3: Execution Timeline

### Story 6.3.1

Implement workflow run and agent run persistence.

### Story 6.3.2

Implement task timeline UI.

### Story 6.3.3

Implement live workflow status events in the UI.

## 10. Phase 7: Planning Workflow

Goal:
- deliver the first practical agent-assisted task workflow.

## Epic 7.1: Requirement Analysis

### Story 7.1.1

Implement the requirement-analysis stage.

### Story 7.1.2

Build context assembly for task metadata and related hierarchy objects.

### Story 7.1.3

Persist requirement-analysis artifacts.

## Epic 7.2: Planning

### Story 7.2.1

Implement the planning stage.

### Story 7.2.2

Generate structured plan artifacts with steps, likely files, and test notes.

### Story 7.2.3

Implement plan approval UI and gating.

## Epic 7.3: Planning Review UX

### Story 7.3.1

Render plan artifacts clearly for approval.

### Story 7.3.2

Allow rejection with notes and re-run.

## 11. Phase 8: Autonomous Coding Foundation

Goal:
- enable autonomous file changes after plan approval.

## Epic 8.1: Task Workspace Isolation

### Story 8.1.1

Implement per-task isolated working directory creation.

### Story 8.1.2

Implement workspace lifecycle and cleanup rules.

### Story 8.1.3

Enforce file write boundaries to the isolated workspace only.

## Epic 8.2: Git Branch Workflow

### Story 8.2.1

Implement branch creation for task execution.

### Story 8.2.2

Implement diff generation against the branch baseline.

### Story 8.2.3

Implement commit preparation support.

## Epic 8.3: Coding Agent

### Story 8.3.1

Implement coding-agent prompt and context assembly.

### Story 8.3.2

Implement file-modification application flow.

### Story 8.3.3

Persist coding outputs and unresolved-issue artifacts.

### Story 8.3.4

Implement coding-stage transition logic.

## 12. Phase 9: Testing and Validation Automation

Goal:
- make autonomous output trustworthy enough to review.

## Epic 9.1: Test Generation

### Story 9.1.1

Implement unit test generation stage.

### Story 9.1.2

Implement integration test generation stage.

### Story 9.1.3

Implement UI test planning artifact stage.

## Epic 9.2: Docker Test Execution

### Story 9.2.1

Implement Docker runner service.

### Story 9.2.2

Implement repo/task test profile configuration.

### Story 9.2.3

Implement test log capture and structured result parsing.

## Epic 9.3: QA Validation

### Story 9.3.1

Implement QA validation stage based on acceptance criteria and test artifacts.

### Story 9.3.2

Implement QA report artifact generation.

## Epic 9.4: Human Test Review

### Story 9.4.1

Implement summarized test-result review UI.

### Story 9.4.2

Implement approve/reject test-review actions.

## 13. Phase 10: Security and Performance Review

Goal:
- convert non-functional quality checks into actionable outputs.

## Epic 10.1: Security Review

### Story 10.1.1

Implement security review stage over code diffs and dependencies.

### Story 10.1.2

Persist security findings.

### Story 10.1.3

Generate follow-up tasks from security findings.

## Epic 10.2: Performance Review

### Story 10.2.1

Implement performance review stage.

### Story 10.2.2

Persist performance findings.

### Story 10.2.3

Generate follow-up tasks from performance findings.

## 14. Phase 11: Autonomous Iteration and Recovery

Goal:
- let the system self-correct before requiring the human.

## Epic 11.1: Rework Loops

### Story 11.1.1

Implement workflow returns to coding on failed tests.

### Story 11.1.2

Implement workflow returns to coding on failed QA validation.

### Story 11.1.3

Implement blocking-finding rework rules.

## Epic 11.2: Failure Handling

### Story 11.2.1

Implement retry thresholds and stage failure policies.

### Story 11.2.2

Implement blocked and failed recovery controls.

### Story 11.2.3

Implement human takeover and manual resume hooks.

## 15. Phase 12: Observability, Audit, and Replay

Goal:
- make the autonomous system inspectable and debuggable.

## Epic 12.1: Structured Logs and Traces

### Story 12.1.1

Persist structured service and workflow logs.

### Story 12.1.2

Implement stage timing and execution trace records.

### Story 12.1.3

Render run timeline and log views in the UI.

## Epic 12.2: Prompt and Output History

### Story 12.2.1

Persist prompt template versions and resolved prompt snapshots.

### Story 12.2.2

Persist raw and parsed model outputs.

### Story 12.2.3

Implement prompt/output inspection UI.

## Epic 12.3: Replay

### Story 12.3.1

Implement workflow replay view.

### Story 12.3.2

Allow stage-by-stage artifact inspection.

## 16. Phase 13: Git Push Delivery

Goal:
- complete the first end-to-end value loop.

## Epic 13.1: Push Workflow

### Story 13.1.1

Implement push-preparation validation.

### Story 13.1.2

Implement Git push operation from the task branch.

### Story 13.1.3

Persist push success/failure state and diagnostics.

## Epic 13.2: Delivery Completion

### Story 13.2.1

Implement the final “done” transition rules.

### Story 13.2.2

Display push completion and delivery summary in task UI.

## 17. Phase 14: Hardening and Scale-Up

Goal:
- stabilize the product for broader real-world usage.

## Epic 14.1: Reliability Hardening

### Story 14.1.1

Add state-machine transition tests across all workflow stages.

### Story 14.1.2

Add integration coverage for restart recovery.

### Story 14.1.3

Add artifact consistency checks.

## Epic 14.2: UX Hardening

### Story 14.2.1

Improve performance for large product trees and task histories.

### Story 14.2.2

Refine usability of approvals, findings, and timeline review.

## Epic 14.3: Future Hooks

### Story 14.3.1

Add extension points for PR creation and GitHub actions integration.

### Story 14.3.2

Add extension points for built-in local model lifecycle management.

## 18. Suggested MVP Cut

A realistic MVP cut should include:

- Phase 1
- Phase 2
- Phase 3
- Phase 4
- Phase 5
- Phase 6
- Phase 7
- Phase 8
- essential pieces of Phase 9
- essential pieces of Phase 12
- Phase 13

In practical terms, MVP must support:

- product/task hierarchy,
- repo attachment and resolution,
- IDE shell,
- provider integration,
- task approval and plan approval,
- autonomous coding in isolated workspace,
- Docker test execution,
- test-result review,
- observability basics,
- Git push.

## 19. Dependency Notes

- Workflow engine should be in place before autonomous coding.
- Repository resolution must be complete before task execution.
- Docker execution should be validated before test automation is treated as reliable.
- Observability should be implemented incrementally from early phases, even if full replay comes later.

## 20. Backlog Management Guidance

When execution begins:

- track each epic as an execution milestone,
- convert stories into sprint-sized technical tasks,
- attach acceptance criteria and demo checkpoints,
- and keep a strict distinction between MVP-critical stories and future-hardening stories.

The core rule is simple:

If a story does not improve the approved-task-to-pushed-code loop, it should not outrank the current critical path.
