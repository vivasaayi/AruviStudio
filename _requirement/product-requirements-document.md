# Product Requirements Document (PRD)

## 1. Document Control

- Product Name: AI Coding Assistant
- Document Type: Formal Product Requirements Document
- Version: 1.0
- Status: Draft for execution planning
- Target Platform: macOS desktop
- Delivery Mode: Local-first desktop application
- Primary Tech Stack: Rust, Tauri, React

## 2. Executive Summary

AI Coding Assistant is a single-user, Mac-first, AI-native engineering workstation for solo developers. It combines structured product management, a VS Code-style IDE, multi-agent software delivery orchestration, and automated code execution workflows into one desktop application.

The product enables a solo developer to define products, modules, features, and tasks, then delegate implementation to a set of autonomous agents that refine requirements, plan work, write code, generate tests, validate outputs, and push approved changes to GitHub-backed repositories. The human remains responsible for task approval, plan approval, and test-result review, while the agents autonomously perform coding and technical iteration.

The initial release will support DeepSeek-class coding models through LM Studio or any OpenAI-compatible endpoint. Future phases will add built-in local model lifecycle management.

## 3. Product Vision

Build a local-first, AI-native software engineering desktop platform that lets a solo developer operate at the product level while autonomous agents perform the majority of planning, implementation, testing, and technical review work.

Vision principles:

- The human defines intent, priority, and approval.
- The human must be able to read the product structure as documentation.
- The agents execute engineering work autonomously.
- The system is transparent, auditable, and replayable.
- The product is useful even before full autonomy is perfect.
- The platform should reduce manual engineering effort, not just repackage it.
- The system should preserve human comprehension, not optimize it away.

## 4. Problem Statement

Solo developers often carry the responsibilities of product manager, architect, developer, tester, QA lead, and release manager. Existing IDEs and AI coding assistants help with snippets or isolated coding tasks but do not provide an integrated system that:

- manages the product hierarchy,
- translates product work into implementation plans,
- coordinates specialized agents across the SDLC,
- isolates execution safely,
- and returns reviewed, test-backed code changes as a traceable workflow.

The result is fragmented work, manual orchestration, and high cognitive overhead.

## 5. Goals

### 5.1 Business Goals

- Create an integrated desktop product that combines product management and AI-driven software delivery.
- Enable a solo developer to manage multiple products and repositories from one workspace.
- Reduce the amount of manual coding, planning, and validation work required per task.
- Establish a platform foundation for future multi-model and local-runtime support.

### 5.2 User Goals

- Define products, modules, features, and tasks in a structured hierarchy.
- Read the product hierarchy as a navigable system document or book.
- Export the hierarchy into a readable book-like artifact for review.
- Make progressive changes to one part of the structure while staying grounded in the wider product context.
- Attach multiple repositories and resolve the right workspace at runtime.
- Approve task scope and execution plans, then let agents code autonomously.
- Review test results and final diffs without manually implementing most changes.
- Track agent activity, prompts, artifacts, failures, and outcomes in one place.

### 5.3 Product Goals

- Deliver an end-to-end workflow from task creation to validated code push.
- Support autonomous coding with human approval gates.
- Provide Docker-based isolated test execution.
- Maintain full observability and auditability for trust and debugging.

## 6. Non-Goals (Initial Phases)

- Multi-user collaboration, permissions, or team workflows
- Enterprise security/compliance certification
- Full GitHub-equivalent collaboration platform
- Full VS Code parity, extension marketplace, or debugger parity
- Automatic deployment to production environments in v1
- Broad language-specific orchestration rules beyond prompt-driven handling
- Fully embedded local model downloading/serving in the initial release

## 7. Target User

### Primary User

- A solo developer building and maintaining multiple software products

### User Characteristics

- Manages product scope and technical implementation personally
- Works across multiple repositories or workspaces
- Wants AI to handle execution-heavy engineering work
- Values visibility and control over autonomous systems
- Accepts local-first desktop workflows on macOS

## 8. Core User Journeys

### 8.0 Product Reading and Progressive Refinement

1. User opens a product and reads its structure as a navigable architecture or rollout book.
2. User moves through modules, features, work items, and related context to understand the wider system.
3. User exports the current structure as a readable book artifact when deeper review is needed.
4. User identifies one section that is incomplete, unclear, or outdated.
5. User edits that section and verifies that the updated structure still makes sense in the wider product context.

### 8.1 Product Setup

1. User creates a product.
2. User defines modules under the product.
3. User defines features and nested subfeatures.
4. User attaches one or more repositories to the product and optionally to modules.
5. User reviews the product dashboard and backlog readiness.

### 8.2 Task Authoring and Approval

1. User creates a task under a product, module, or feature.
2. User adds problem statement, acceptance criteria, and constraints.
3. User optionally overrides the repository/workspace for the task.
4. User approves the task for agent processing.
5. Requirement-analysis and planning agents refine the task.
6. User reviews and approves the generated implementation plan.

### 8.3 Autonomous Execution

1. System resolves the target repository and creates an isolated task workspace.
2. Coding agent creates a dedicated branch and implements changes autonomously.
3. Test-generation agents create unit and integration tests.
4. UI test-planning agent prepares UI validation scenarios.
5. Tests execute in Docker.
6. QA, security, and performance agents review the output and surface findings.
7. Agents iterate if required based on failures or findings.
8. User reviews summarized test results.
9. System pushes validated code to the remote repository.

### 8.4 Review and Traceability

1. User opens the task detail view.
2. User reviews the task timeline, artifacts, prompts, outputs, diffs, and test results.
3. User inspects follow-up tasks generated by agents.
4. User resumes, retries, or pauses workflows if needed.

## 9. Functional Requirements

## 9.1 Product Management

The product hierarchy is not only a planning structure. It is also the canonical human-readable representation of the product.

### Product

- The system shall allow the user to create, edit, archive, and browse products.
- The system shall store product metadata including name, description, vision, goals, tags, and status.
- The system shall allow one or more repositories to be attached to a product.
- The system shall allow one default repository/workspace to be defined at product level.

### Module

- The system shall allow modules to be created under a product.
- The system shall store module metadata including name, description, purpose, and dependencies.
- The system shall allow a module-level repository/workspace override.

### Feature

- The system shall support features under modules.
- The system shall support nested features.
- The maximum supported hierarchy depth shall remain open for larger products and shall not be prematurely constrained to a shallow fixed model if deeper structure improves readability and system comprehension.
- The system shall store feature acceptance criteria, priority, risk, and technical notes.
- The system shall allow features to be linked to tasks.

### Task

- The system shall support tasks and nested subtasks.
- The system shall allow tasks to be created under product, module, and feature contexts.
- The system shall support task metadata:
  - title
  - problem statement
  - description
  - acceptance criteria
  - constraints
  - priority
  - complexity
  - task type
- The system shall support task types including feature, bug, refactor, test, review, security, and performance.
- The system shall allow task-level repository/workspace override.
- The system shall maintain task status and approval state.

### Product Structure as Documentation

- The system shall treat the product tree as a first-class documentation surface, not only as planning metadata.
- The system shall allow the hierarchy to be read in sequence like a technical manual or book.
- The system shall support export of the current hierarchy into a readable book/document format.
- The system shall support progressive editing, where local changes can be made while preserving visibility into the surrounding system context.
- The system shall avoid devolving into a Jira-style hierarchy manager that optimizes for ticket nesting over human understanding.

### Hierarchy Semantics

The hierarchy shall be treated as the canonical readable model of the product.

It serves two purposes at the same time:

- a human-readable documentation surface for understanding the system
- a structural source from which rollouts and work items are derived

The hierarchy shall optimize for:

- readability in sequence
- navigability by topic or system area
- local editing without losing global context
- derivation of execution work from the documented structure

Each level in the hierarchy shall have a distinct semantic meaning. A node shall exist only when it improves human understanding of the wider system, not merely to satisfy a fixed planning template.

Recommended semantic node kinds include:

- Product: the whole system or application
- Domain or Area: a major functional area of the system
- Subdomain or Subsystem: a meaningful subdivision inside a domain when scale requires it
- Capability or Feature Set: a user-visible or system-visible ability the product must provide
- Rollout: a concrete delivery slice that evolves, implements, or ships part of a capability
- Work Item: an implementation task derived from a rollout or directly from a documented node when necessary

The system shall support semantic hierarchies such as:

- Product -> Domain -> Capability -> Rollout -> Work Item
- Product -> Domain -> Subdomain -> Capability -> Rollout -> Work Item
- Product -> System -> Subsystem -> Feature Set -> Capability -> Rollout -> Work Item

The system shall not prematurely constrain the hierarchy to a shallow fixed number of levels if deeper structure improves comprehension for large products such as simulation software, complex creative tools, or broad platform products.

Execution must remain subordinate to the hierarchy:

- rollouts and work items shall be derived from the documented structure
- execution state shall not become a disconnected second planning system
- exported book/document views shall read like a technical manual or architecture book, not a backlog dump

## 9.2 Approval Workflow

- The system shall enforce human approval at these stages:
  - task approval
  - plan approval
  - test result review
- The system shall not require human approval for code-writing steps unless the task is escalated or blocked.
- The system shall preserve an approval history with timestamp and notes.

## 9.3 Agent Orchestration

- The system shall provide configurable agent roles.
- The system shall allow each agent to have:
  - a name
  - a role
  - a prompt template
  - allowed tools
  - capability tags
  - defined boundaries
- The system shall coordinate agents through a deterministic workflow engine.
- The workflow engine shall support:
  - stage transitions
  - retries
  - pauses
  - aborts
  - human takeover
  - resumable runs
- The system shall support iterative rework loops until the workflow reaches a terminal state.

## 9.4 Agent Roles (Initial)

- Requirement Analysis Agent
- Planning Agent
- Coding Agent
- Unit Test Agent
- Integration Test Agent
- UI Test Planning Agent
- QA Validation Agent
- Security Review Agent
- Performance Review Agent

Each role shall produce structured outputs that are persisted as artifacts or state transitions.

## 9.5 IDE and Workspace

- The system shall provide a VS Code-style desktop interface.
- The system shall include:
  - file explorer
  - Monaco-based editor
  - tabbed editing
  - search
  - terminal panel
  - source-control panel
  - diff viewer
  - task/agent/activity panels
- The system shall support multiple repositories attached across the product hierarchy.
- The system shall resolve the active execution workspace using inheritance rules.

## 9.6 Repository and Git Operations

- The system shall register repositories with local path, remote URL, and default branch.
- The system shall support repository resolution order:
  - task override
  - module override
  - product default
- The system shall support:
  - repository access
  - branch creation
  - file changes
  - diff capture
  - commit preparation
  - push to remote

## 9.7 Autonomous Coding

- The system shall create an isolated per-task working directory for code execution.
- The coding agent shall be able to modify files in the isolated task workspace.
- The system shall support autonomous iteration based on validation feedback.
- The system shall maintain a full change history for each task run.

## 9.8 Testing and Validation

- The system shall generate unit tests for implementation tasks where applicable.
- The system shall generate integration tests where applicable.
- The system shall generate UI test plans as structured artifacts.
- The system shall execute tests in Docker containers.
- The system shall collect and store:
  - logs
  - exit status
  - test summaries
  - structured results
- The system shall present summarized test results for human review.

## 9.9 Security and Performance Review

- The system shall run agent-based security review on generated changes.
- The system shall run agent-based performance review on generated changes.
- The system shall convert material findings into follow-up tasks or reopen the current task.

## 9.10 Model Integration

- The system shall support model providers via configurable API endpoints.
- The initial release shall support LM Studio/OpenAI-compatible endpoints.
- The system shall support model definitions independent of agent roles.
- The system shall allow any model to be assigned to any agent.
- The system shall allow future introduction of built-in local model lifecycle management without major architectural changes.

## 9.11 Observability and Audit

- The system shall log all workflow stage transitions.
- The system shall store prompt snapshots used in each agent run.
- The system shall store model outputs, retries, durations, and failure reasons.
- The system shall support replay and inspection of prior runs.
- The system shall store artifacts such as plans, diffs, test reports, QA reports, and findings.

## 10. Workflow Definition

### 10.1 Primary Task Flow

1. Task created
2. Human task approval
3. Requirement analysis
4. Planning
5. Human plan approval
6. Coding execution
7. Unit test generation
8. Integration test generation
9. UI test planning
10. Docker test execution
11. QA validation
12. Security review
13. Performance review
14. Human test-result review
15. Push preparation
16. Git push
17. Task completion or rework

### 10.2 Rework Conditions

- Test execution failure
- QA validation failure
- Blocking security finding
- Blocking performance finding
- Git push failure

### 10.3 Terminal States

- Done
- Blocked
- Failed
- Cancelled

## 11. Definition of Done

A task is considered complete when:

- the task has been approved,
- the execution plan has been approved,
- coding artifacts have been produced,
- required test stages have completed,
- test results have been reviewed by the user,
- no blocking validation failures remain,
- and the code has been successfully pushed to the target repository.

Agents may recommend completion, but the system shall enforce the completion checklist.

## 12. Non-Functional Requirements

### 12.1 Platform

- The application shall run on macOS as the initial supported platform.
- The application shall function as a local desktop app with network access only where configured for model providers, git remotes, and other explicit integrations.

### 12.2 Performance

- The UI shall remain responsive during long-running agent operations.
- Long-running workflows shall run asynchronously.
- The system shall support progress updates and stage timing visibility.

### 12.3 Reliability

- Workflow state shall persist across app restarts.
- Interrupted workflows shall be resumable where safe.
- Failures shall be stored with diagnostic context.

### 12.4 Observability

- Every material action shall be traceable.
- Every agent run shall be attributable to a specific prompt snapshot, model, and task.

### 12.5 Security (Initial Product Scope)

- Secrets shall remain local to the machine.
- Test execution shall occur in Docker containers.
- Task workspaces shall be isolated from the primary repository root.

### 12.6 Extensibility

- New agents, models, and workflow stages shall be addable without redesigning the entire application.
- Future local model management shall fit into the existing model abstraction.

## 13. UX Requirements

- The application shall provide a clear hierarchy browser for products, modules, features, and tasks.
- The hierarchy browser shall support reading and orientation, not only CRUD operations.
- The user shall be able to understand the wider product context while focused on a local part of the tree.
- The application shall provide a book/document style representation of the product structure suitable for export and human review.
- The application shall provide an IDE-like central workspace.
- The application shall provide a dedicated task detail surface showing:
  - status
  - approvals
  - active stage
  - artifacts
  - logs
  - findings
- The application shall make approval actions explicit and easy to execute.
- The application shall prioritize traceability over UI minimalism.

## 14. Success Metrics

- Time from task creation to approved plan
- Time from approved plan to code diff
- Time from approved plan to successful push
- Percentage of tasks completed with no manual code edits
- Percentage of tasks requiring human takeover
- Test pass rate after autonomous iterations
- Number of follow-up tasks generated per completed task
- Frequency of blocked workflows

## 15. Release Strategy

### MVP

The MVP must prove the core loop:

- create and manage product hierarchy,
- create tasks,
- approve task and plan,
- run autonomous coding in an isolated workspace,
- generate and run tests,
- review test results,
- and push code successfully to the repository.

### Post-MVP

- Advanced multi-model routing
- Built-in local model lifecycle management
- Richer GitHub automation (PR creation, CI integration)
- More advanced UI test automation execution
- Expanded security and compliance controls

## 16. Risks and Mitigations

### Risk: Scope overload

Mitigation:
- Deliver the product in phases.
- Keep MVP focused on the end-to-end task loop.

### Risk: Unreliable autonomous coding

Mitigation:
- Enforce approvals.
- Add strong observability.
- Limit workflow transitions to deterministic rules.

### Risk: Docker and sandbox complexity

Mitigation:
- Use isolated task workspaces for coding.
- Restrict Docker to test execution first.

### Risk: Local model inconsistency

Mitigation:
- Keep model assignment configurable.
- Persist prompt and output history for tuning.

## 17. Open Decisions for Execution

- Concrete Docker image strategy per language/runtime
- Prompt template versioning policy and storage format
- Exact retry limits and failure thresholds
- Push flow details for GitHub authentication
- Criteria for auto-generated follow-up task priority

## 18. Acceptance Criteria Summary

The product will be considered aligned with this PRD when:

- a solo developer can model product scope in the hierarchy,
- repositories can be attached and resolved correctly,
- the agent workflow executes from approved task to pushed code,
- coding occurs autonomously after plan approval,
- tests run in Docker and are reviewed by the user,
- and every action is traceable through logs, prompts, outputs, and artifacts.
