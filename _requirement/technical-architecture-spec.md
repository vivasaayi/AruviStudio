# Technical Architecture Specification

## 1. Document Control

- Product Name: AI Coding Assistant
- Document Type: Technical Architecture Specification
- Version: 1.0
- Status: Draft for implementation
- Target Platform: macOS desktop

## 2. Architectural Goals

The system must:

- run as a local-first desktop application,
- support a single user managing multiple products and repositories,
- coordinate autonomous multi-agent software delivery,
- isolate task execution and test execution,
- maintain deterministic workflow control,
- and preserve full auditability across prompts, actions, and outputs.

## 3. High-Level Architecture

The application is composed of eight primary layers:

1. Presentation Layer
2. Tauri Application Boundary
3. Rust Application Core
4. Persistence Layer
5. Agent Execution Layer
6. Model Gateway Layer
7. Repository / Workspace Layer
8. Sandbox and Docker Execution Layer
9. Observability and Artifact Layer

These layers should be implemented as explicit internal modules, not as a monolithic set of commands.

## 4. Runtime Topology

### 4.1 Desktop Runtime

- Frontend: React + TypeScript
- Host Shell: Tauri
- Native Backend: Rust
- Local Database: SQLite
- Local Artifact Store: filesystem-backed
- Test Runtime: Docker
- Model Providers: remote or localhost API endpoints (OpenAI-compatible initially)

### 4.2 Core Interaction Pattern

1. UI triggers a domain command.
2. Tauri bridges the request to the Rust backend.
3. Rust services validate the request, mutate persistence state, and emit domain events.
4. Workflow engine schedules or advances agent stages.
5. Agent execution services gather context, call the model gateway, and produce structured outputs.
6. Sandbox/repo services apply code changes and run commands as allowed.
7. Docker service executes tests.
8. Observability layer records prompts, outputs, transitions, artifacts, and logs.
9. Events are streamed back to UI for live updates.

## 5. Technology Stack

### 5.1 Frontend

- React
- TypeScript
- Tauri frontend runtime
- Monaco Editor
- TanStack Query for async data synchronization
- Zustand or Redux Toolkit for client state
- A lightweight component system styled for an IDE-like shell

### 5.2 Backend

- Rust
- Tauri command/event system
- Tokio for async orchestration
- Serde for serialization
- SQLx or rusqlite for SQLite access
- Git via `git2` or carefully controlled shell integration
- Structured logging via `tracing`

### 5.3 Infrastructure Dependencies

- SQLite database file
- Local artifact directories
- Docker daemon for test execution
- Network access to configured model providers
- Network access to git remotes when pushing

## 6. Frontend Architecture

## 6.1 Frontend Responsibilities

- render the product hierarchy,
- render task state and approvals,
- render the IDE/editor experience,
- render agent activity and artifacts,
- allow configuration of models, agents, and repositories,
- and provide clear user controls for approval, pause, resume, retry, and review.

## 6.2 Frontend Shell Layout

Recommended primary layout:

- Left Sidebar:
  - Product tree
  - Modules
  - Features
  - Tasks
- Main Pane:
  - Editor
  - Diff view
  - Artifact detail
- Right Sidebar:
  - Task status
  - Agent timeline
  - Approval panel
  - Findings
- Bottom Panel:
  - Terminal
  - Logs
  - Test results

## 6.3 Frontend Module Boundaries

- `app-shell`
- `product-hierarchy`
- `task-center`
- `ide-workspace`
- `agent-activity`
- `approvals`
- `artifacts`
- `settings`

## 6.4 Frontend State Strategy

Use a split approach:

- Server/domain state via TanStack Query
- UI/session state via Zustand or Redux Toolkit

Recommended domain caches:

- products
- modules
- features
- tasks
- repositories
- agent definitions
- workflow runs
- artifacts
- settings

## 7. Tauri Boundary Design

## 7.1 Purpose

Tauri is the boundary between UI and local native capabilities.

It is responsible for:

- exposing command handlers,
- streaming events to the UI,
- mediating access to filesystem and process capabilities,
- and ensuring frontend code does not directly own privileged operations.

## 7.2 Tauri Command Categories

- product management commands
- task management commands
- repository commands
- agent/workflow commands
- model provider commands
- settings commands
- artifact retrieval commands
- test execution commands

## 7.3 Tauri Event Categories

- workflow status updates
- task status updates
- agent stage progress
- test execution updates
- log stream updates
- artifact creation notifications

## 8. Rust Application Core

## 8.1 Core Principles

- Keep business logic in Rust services, not in the UI.
- Organize features by domain/service boundaries.
- Implement workflow as a state machine.
- Prefer structured domain models over ad hoc JSON blobs.

## 8.2 Core Services

### `product_service`

Responsibilities:

- manage products, modules, features
- enforce hierarchy limits
- manage repository attachments at product/module level

### `task_service`

Responsibilities:

- manage tasks and subtasks
- maintain task metadata and state
- resolve inheritance and overrides
- track approvals

### `workflow_service`

Responsibilities:

- own the workflow state machine
- schedule next stages
- enforce approval gates
- coordinate retries, pause, resume, failover

### `agent_service`

Responsibilities:

- manage agent definitions
- select the agent for a stage
- build structured agent requests
- validate agent outputs

### `model_service`

Responsibilities:

- store provider configurations
- store model definitions
- route inference requests to the model gateway

### `repo_service`

Responsibilities:

- manage repository registration
- resolve execution repository
- handle branch creation
- manage working directories
- capture diffs and push changes

### `sandbox_service`

Responsibilities:

- create isolated task workspaces
- constrain code-writing operations to task workspace
- manage workspace cleanup rules

### `docker_service`

Responsibilities:

- run test commands in Docker
- manage test container lifecycle
- collect logs and structured test results

### `artifact_service`

Responsibilities:

- store and index artifacts
- map artifacts to tasks and agent runs
- retrieve artifacts for UI

### `audit_service`

Responsibilities:

- persist logs, trace records, prompt snapshots, output history
- support replay and forensic inspection

## 8.3 Internal Communication Pattern

Recommended design:

- Command enters a service
- Service validates state
- Service writes to database
- Service emits domain event
- Workflow service consumes domain events to determine next action

This pattern keeps the app deterministic and easier to test.

## 9. Persistence Architecture

## 9.1 Primary Store

- SQLite as the primary relational store

Rationale:

- local-first,
- simple deployment,
- sufficient for single-user scale,
- mature support in Rust.

## 9.2 Artifact Storage

Store large artifacts on disk, indexed by database metadata.

Artifact examples:

- plan documents
- prompt snapshots
- model outputs
- diffs
- test reports
- QA reports
- security findings
- performance findings
- logs

## 9.3 Recommended Database Domains

- Product domain
- Repository domain
- Task domain
- Workflow domain
- Agent domain
- Approval domain
- Artifact domain
- Settings domain

## 9.4 Core Entities

### Product

- id
- name
- description
- vision
- goals
- status
- tags
- created_at
- updated_at

### Module

- id
- product_id
- name
- description
- purpose
- sort_order
- created_at
- updated_at

### Feature

- id
- module_id
- parent_feature_id nullable
- level
- name
- description
- acceptance_criteria
- priority
- risk
- status

### Task

- id
- product_id
- module_id nullable
- feature_id nullable
- parent_task_id nullable
- title
- problem_statement
- description
- acceptance_criteria
- constraints
- task_type
- priority
- complexity
- status
- repo_override_id nullable
- active_repo_id nullable
- branch_name nullable

### Repository

- id
- name
- local_path
- remote_url
- default_branch
- auth_profile nullable

### RepositoryAttachment

- id
- scope_type (product/module)
- scope_id
- repository_id
- is_default

### AgentDefinition

- id
- name
- role
- description
- prompt_template_ref
- allowed_tools
- boundaries
- enabled

### ModelProvider

- id
- name
- provider_type
- base_url
- auth_secret_ref
- enabled

### ModelDefinition

- id
- provider_id
- name
- context_window nullable
- capability_tags
- enabled

### AgentModelBinding

- id
- agent_definition_id
- model_definition_id

### WorkflowRun

- id
- task_id
- workflow_version
- status
- current_stage
- retry_count
- started_at
- ended_at nullable

### AgentRun

- id
- workflow_run_id
- task_id
- agent_definition_id
- model_definition_id
- stage_name
- status
- started_at
- ended_at nullable
- duration_ms nullable

### Approval

- id
- task_id
- approval_type
- status
- notes
- acted_at

### Artifact

- id
- task_id
- workflow_run_id nullable
- agent_run_id nullable
- artifact_type
- storage_path
- summary
- created_at

### Finding

- id
- task_id
- source_agent_run_id
- category
- severity
- title
- description
- status
- linked_followup_task_id nullable

## 10. Workflow Engine Architecture

## 10.1 Core Requirement

The workflow engine must be implemented as a deterministic state machine with explicit transition rules.

Avoid building this as a loose chain of prompts and callbacks.

## 10.2 Stage Model

Proposed stages:

1. `draft`
2. `pending_task_approval`
3. `requirement_analysis`
4. `planning`
5. `pending_plan_approval`
6. `coding`
7. `unit_test_generation`
8. `integration_test_generation`
9. `ui_test_planning`
10. `docker_test_execution`
11. `qa_validation`
12. `security_review`
13. `performance_review`
14. `pending_test_review`
15. `push_preparation`
16. `git_push`
17. `done`
18. `blocked`
19. `failed`
20. `cancelled`

## 10.3 Transition Rules

Transitions must validate:

- current stage
- required approvals
- required artifacts
- retry policy
- blocking findings
- external execution results

Examples:

- `pending_task_approval -> requirement_analysis` only after user approval
- `planning -> pending_plan_approval` only after plan artifact exists
- `docker_test_execution -> qa_validation` only after test results captured
- `pending_test_review -> push_preparation` only after user review approval

## 10.4 Rework Loop

The engine must support returning from downstream stages to `coding` when:

- tests fail,
- QA fails,
- blocking findings require code changes.

## 10.5 Failure Handling

Each stage must define:

- max retries
- retry backoff policy
- escalation behavior
- terminal failure conditions

On repeated failure:

- pause the workflow or mark it blocked
- preserve full failure context
- allow explicit user resume or takeover

## 11. Agent Execution Architecture

## 11.1 Agent Registry

Agent definitions are persisted configuration objects, not hard-coded only in UI.

Each definition should include:

- role
- intent
- prompt template reference
- allowed tools
- boundaries
- enabled flag

## 11.2 Agent Request Lifecycle

1. Workflow service requests stage execution.
2. Agent service resolves the agent definition.
3. Context builder collects task, repo, artifact, and policy context.
4. Prompt builder creates the prompt payload.
5. Model gateway sends the inference request.
6. Output parser validates and normalizes the response.
7. Agent artifacts are stored.
8. Workflow transitions based on structured result.

## 11.3 Structured Output Contract

Each agent should return structured data where possible.

Examples:

- Requirement analysis:
  - clarified scope
  - missing constraints
  - risks
- Planning:
  - implementation steps
  - likely files
  - test plan
- Coding:
  - file modifications
  - rationale
  - unresolved issues
- Security review:
  - findings list
  - severity
  - must-fix flag

This can be represented as JSON internally even if displayed as markdown in the UI.

## 11.4 Tool Permission Model

Each agent must operate within allowed capabilities.

Examples:

- Planner:
  - read task context
  - read relevant files
  - no file writes
  - no shell execution
- Coding agent:
  - read/write files in task workspace
  - limited shell access
  - no direct push without workflow authorization
- Review agents:
  - read code, diffs, and reports
  - no direct file writes

The permission model must be enforced by the Rust backend, not merely by prompt instructions.

## 12. Model Gateway Architecture

## 12.1 Purpose

Abstract model providers from workflow and agent logic.

## 12.2 Responsibilities

- store provider configurations
- validate connectivity
- send inference requests
- normalize responses
- record usage metadata

## 12.3 Provider Interface

Suggested internal interface:

- `health_check(provider_id)`
- `list_models(provider_id)` where supported
- `run_completion(request)`
- `run_structured_completion(request)`

## 12.4 Initial Provider Support

- OpenAI-compatible HTTP providers
- LM Studio endpoint
- DeepSeek-class model configured through compatible endpoints

## 12.5 Future Compatibility

The gateway must be designed so that later local runtime management can provide the same interface without changing agent/workflow logic.

## 13. Repository and Workspace Architecture

## 13.1 Repository Resolution

Execution repository must resolve using:

1. task-level override
2. module-level override
3. product-level default

If no repository resolves, the task must be blocked.

## 13.2 Working Directory Strategy

Do not write directly in the canonical source directory during execution.

Recommended approach:

- source repo remains registered as the canonical reference
- each workflow run gets a dedicated working directory
- code changes are made in the task workspace
- diffs are generated against the branch state

This reduces risk and makes retries safer.

## 13.3 Git Operations

Supported operations:

- inspect repo status
- ensure branch base exists
- create task branch
- diff changes
- prepare commit
- push to remote

Use `git2` where it is stable enough; otherwise encapsulate shell-based git commands behind a strict service interface.

## 14. Sandbox and Docker Architecture

## 14.1 Sandboxing Strategy

Separate coding isolation from test isolation.

### Coding Isolation

- Per-task working directories
- Restrict write operations to the task workspace
- Restrict agent file access to resolved workspace and approved context files

### Test Isolation

- Run tests in Docker
- Mount only required workspace paths
- Collect outputs back into local artifact storage

## 14.2 Docker Service Responsibilities

- select test image/profile
- mount workspace
- execute configured test command
- capture stdout/stderr
- capture exit codes
- summarize results into structured artifacts

## 14.3 Runtime Profiles

The system should eventually support repo-specific or task-specific Docker profiles.

Initial profile data should include:

- image
- workdir
- command
- env vars
- mount paths
- timeout

## 15. Observability and Audit Architecture

## 15.1 Logging

Use structured logging across all services.

Minimum fields:

- timestamp
- service
- task_id
- workflow_run_id
- stage
- severity
- message

## 15.2 Prompt and Output Capture

For each agent run, record:

- prompt template version
- resolved prompt snapshot
- referenced context artifacts
- provider
- model
- raw output
- parsed output

## 15.3 Replay

Replay should allow:

- viewing a full workflow timeline,
- inspecting each stage and artifact,
- understanding the exact inputs and outputs used.

This is essential for debugging and trust.

## 16. Security Architecture (Initial Product Scope)

## 16.1 Trust Boundaries

- UI is untrusted relative to privileged operations.
- Rust backend enforces permissions.
- Agent prompts are advisory; backend enforcement is authoritative.
- Docker execution is isolated from the host workflow as much as feasible within local constraints.

## 16.2 Secret Handling

For initial phases:

- keep secrets local,
- store references in settings,
- avoid exposing full secrets to the UI after initial entry.

Later, move to stronger OS-keychain-backed storage if not adopted in the first implementation.

## 17. API and Event Design

## 17.1 Command Shape

Prefer explicit domain commands over generic command payloads.

Examples:

- `create_product`
- `create_task`
- `approve_task`
- `approve_plan`
- `start_workflow`
- `pause_workflow`
- `resume_workflow`
- `review_test_results`
- `push_task_changes`

## 17.2 Event Shape

Events should be normalized and UI-friendly.

Examples:

- `task.updated`
- `workflow.stage_changed`
- `agent.run_started`
- `agent.run_completed`
- `test.run_completed`
- `artifact.created`
- `workflow.blocked`

## 18. Extensibility Strategy

The architecture must allow future changes without major rewrites:

- new agent roles
- new workflow stages
- new model providers
- built-in local model runtimes
- richer GitHub integrations
- stronger sandboxing profiles

This means:

- keep provider and agent definitions data-driven,
- keep workflow transitions declarative where possible,
- isolate side-effectful services behind stable interfaces.

## 19. Operational Risks

### Risk: Service sprawl in Rust

Mitigation:

- define strict service boundaries and shared domain types early.

### Risk: Workflow complexity becomes brittle

Mitigation:

- implement explicit transition tests for the state machine.

### Risk: Docker introduces environment variance

Mitigation:

- use explicit test profiles and artifactized outputs.

### Risk: Excessive artifact volume

Mitigation:

- separate metadata in SQLite from large file payloads on disk.

## 20. Recommended Initial Repository Structure

Suggested high-level project structure:

```text
src-ui/
  app/
  features/
  components/
  hooks/
  state/

src-tauri/
  src/
    commands/
    services/
    domain/
    workflows/
    providers/
    persistence/
    execution/
    observability/
```

This keeps frontend and backend concerns separated cleanly.

## 21. Architecture Acceptance Criteria

The architecture is acceptable when:

- UI logic is not responsible for core workflow decisions,
- the workflow engine is deterministic and testable,
- agent capabilities are enforceable by backend permissions,
- coding and test execution are isolated by design,
- repositories resolve correctly through scoped overrides,
- and all critical actions are logged, persisted, and replayable.
