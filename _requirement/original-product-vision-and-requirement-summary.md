# Original Product Vision and Requirement Summary

## 1. Original Product Vision

Build an AI-native coding platform for a solo developer that behaves like a virtual software company inside one desktop application.

The goal is to let the developer define what needs to be built at the product level, then have an army of AI agents pick up the work, refine it, plan it, code it, test it, review it, and move it toward completion with minimal manual implementation effort from the human.

The human should primarily:

- define products, features, and tasks,
- read and navigate the evolving product structure as documentation,
- review plans,
- review test outcomes,
- and approve final progress.

The agents should primarily:

- analyze requirements,
- break work into steps,
- collaborate like real developers,
- write code,
- write tests,
- review quality,
- and keep iterating until the work is done.

This is intended to be:

- a VS Code-style AI-first IDE,
- a readable product architecture and rollout planning platform,
- and an autonomous multi-agent SDLC engine

all within one system.

The product structure should not behave like a Jira-style hierarchy manager. It should behave more like a navigable technical book or systems manual, where the user can move through the structure, understand the wider system, and make progressive changes without losing context.

## 2. Original Product Requirement Summary

The original requirement describes a desktop application built using:

- Rust
- Tauri
- React for the frontend

The product should provide a VS Code-like coding experience, but it is not meant to be just a normal IDE. Its main purpose is AI-driven code generation and delivery.

The product should also include a full embedded product management system where a solo developer can organize work across multiple products and drive implementation from product-level planning down to task execution.

## 3. Original Core Product Concepts

### 3.1 AI-First Coding Assistant

The product is centered around AI-assisted coding. The system should allow the developer to define work and let AI agents handle most of the implementation lifecycle.

### 3.2 Product Structure and Documentation Built In

The app should have an integrated product management structure:

- A developer can manage multiple products.
- A product can contain multiple modules.
- A module can contain multiple top-level features.
- A feature can have nested subfeatures when the product demands it.
- Each feature can have multiple tasks.
- Each task can have nested subtasks.

This hierarchy should be native to the product and directly connected to the agent workflow.

The hierarchy should also serve as product documentation:

- the product tree should be readable as a coherent system description,
- the exported book should be a first-class human review surface,
- the user should be able to read the structure the way they would read a large product manual or architecture document,
- and edits should be progressive, with local changes remaining understandable in the wider system context.

Hierarchy depth should remain flexible. For very large products, such as a flight simulator or other systems with large operational surface area, the structure may need more than a fixed shallow model.

### 3.2.1 Hierarchy Semantics

The hierarchy should be treated as the canonical readable model of the product.

It should work like a navigable technical book:

- broad at the top
- progressively more specific as the user drills in
- clear enough that a human can enter from many points and still build a mental map

The hierarchy serves two purposes at once:

- documentation for understanding the product
- structure for deriving rollouts and work items

Semantic levels should be preferred over rigid fixed levels. Useful node kinds may include:

- Product
- Domain or Area
- Subdomain or Subsystem
- Capability or Feature Set
- Rollout
- Work Item

Each parent should explain the grouping logic of its children. A node should only exist if it improves readability, navigation, or execution clarity.

Execution should be derived from the hierarchy, not managed in a second disconnected planning structure. The exported book should read like a technical manual, not like a backlog dump.

### 3.3 Agent Army

The system should include multiple specialized AI agents that work together like a real engineering organization.

These agents should:

- review requirements,
- improve and clarify them,
- break them into proper steps,
- pass work to reviewer agents,
- wait for human approval where required,
- and then continue into execution.

### 3.4 Autonomous SDLC Flow

For each task, the system should trigger a chain of activities across multiple agents, including:

- requirement review,
- requirement enhancement,
- planning,
- review of the plan,
- human approval,
- coding,
- unit test creation,
- integration test creation,
- UI test planning,
- QA validation,
- security review,
- performance review,
- and creation of follow-up tasks where issues are found.

The system should keep cycling work among agents until the task reaches completion.

### 3.5 Human Role

The human should not be doing most of the coding.

The human should mainly:

- create the product structure,
- read and refine the product book/document,
- define tasks,
- approve tasks,
- review plans,
- review test outputs,
- and inspect results.

The product should reduce the human role from implementer to reviewer/operator without destroying human comprehension. The system should reduce implementation burden while preserving the user’s cognitive map of the product.

### 3.6 Sandbox Execution

All code generation and execution should happen in isolated environments.

The original requirement suggested:

- sandboxed execution,
- likely using Docker containers,
- so agents can operate safely while coding and running validation workflows.

### 3.7 Model Support

The initial model direction is:

- start with DeepSeek as the coding model,
- connect through LM Studio first,
- then evolve toward broader support for local LLMs,
- and eventually provide built-in model management similar to LM Studio.

## 4. Original Product Goals

The original goals can be summarized as:

- Let a solo developer manage many products in one place.
- Turn product planning into executable engineering work.
- Use AI agents to perform most of the SDLC.
- Keep the human focused on approvals and review.
- Provide visibility into status, task breakdowns, and progress.
- Enable end-to-end software delivery from task definition to coded output.

## 5. Original Intended User Experience

From the user’s point of view, the ideal experience is:

- Define the product, module, feature, and task hierarchy.
- Read the hierarchy as a structured book that explains the product.
- Export that structure into a readable document when needed.
- Review one area, chapter, or section at a time and make progressive improvements.
- Approve the task and plan.
- Watch agents break work down and execute it.
- See status updates, agent activity, and task progress through the UI.
- Review test outcomes and final changes.
- Let the system handle the majority of technical execution behind the scenes.

The user should experience the platform as a product-thinking cockpit and navigable product book, not just a coding tool.

## 6. Original Scope in Plain Terms

At its most direct, the original vision was:

Create a Mac desktop application that combines product management, AI coding, and autonomous engineering workflows so a solo developer can define what should be built and let specialized AI agents carry out the software delivery lifecycle with the human acting mainly as an approver and reviewer.

## 7. Original Vision Checkpoint

Any implementation should still align with these original intentions:

- Is the system reducing hands-on coding work for the human?
- Is the product hierarchy directly driving execution?
- Is the product hierarchy readable as documentation and exportable as a book?
- Can the user make progressive changes without losing the wider system context?
- Are agents behaving like specialized team members?
- Is the UI centered on task status, progress, and review?
- Is the system moving toward local model support and stronger autonomy?

If the answer to those is yes, the product is still aligned with the original vision.
