# Product and Task UX Redesign Specification

## 1. Purpose

This document defines the first major UX redesign for the two primary operator workflows in AruviStudio:

- Product Management
- Task Management

These two areas are the main day-to-day working surfaces for the solo developer. The redesign shifts them from generic CRUD pages into a hierarchy-driven product operations workspace.

## 2. Problem Summary

The current UI is functional but not aligned with the user’s actual workflow.

Current issues:

- too much empty space and low information density
- product screens are passive and display-oriented
- task creation is form-heavy and not context-aware
- product selection is hidden in the left sidebar and creates friction
- the product hierarchy is not the main navigation model
- the right sidebar carries very little value
- products and tasks feel like separate admin pages rather than connected workspaces

This creates unnecessary navigation, context loss, and friction during refinement.

## 3. UX Design Principles

The redesign should follow these principles:

- Hierarchy first: the product structure must be the main organizational model.
- Context stays visible: the user should see where they are in the product tree while editing.
- Actions should be local: create/edit actions should be available where the user is already working.
- Density over decoration: this is an operational workspace, not a showcase dashboard.
- Backlog-first task work: task management should prioritize review and refinement, not large blank forms.
- Zero hidden dependencies: required context such as selected product must be explicit and visible.

## 4. Product Management Redesign

## 4.1 Primary Goal

Turn product management into a persistent workspace for refining product structure.

## 4.2 New Interaction Model

The product management screen should operate as a split-pane workspace:

- Left pane: product list and hierarchy navigator
- Center pane: selected product overview and editable structure
- Right pane: context, quick actions, and summary metrics

## 4.3 Product Workspace Layout

### Left Column

- Compact list of products
- Active product clearly highlighted
- Quick search/filter for products
- `New Product` action
- Product counts where useful

### Center Column

When a product is selected:

- product header
- editable summary
- goals/tags/status snapshot
- module list
- feature hierarchy grouped under each module
- active tasks summary for the selected product

### Right Column

- selected product context
- quick actions:
  - add module
  - add top-level feature
  - create task
  - attach repository
- structure summary:
  - module count
  - feature count
  - task count

## 4.4 Product Management Behaviors

- selecting a product should immediately update the workspace
- creating a product should not navigate away from the workspace
- if no product exists, the empty state should prompt creation directly
- empty module state should offer `Create your first module`
- feature creation should happen in context under the selected module

## 4.5 Product Hierarchy UX

The user should be able to see and understand:

- which modules belong to the product
- which features belong to each module
- which features are nested
- where tasks will attach

The hierarchy should use:

- indentation
- compact labels
- status badges
- nested grouping
- local create actions

## 4.6 Product Empty States

Empty states must be actionable, not passive.

Examples:

- No products: `Create your first product`
- No modules: `Add the first module for this product`
- No features: `Start by defining a top-level feature`

## 5. Task Management Redesign

## 5.1 Primary Goal

Turn task management into a backlog and refinement workspace instead of a form-first page.

## 5.2 New Interaction Model

The task management screen should operate as a master-detail workspace:

- Left pane: backlog list
- Center pane: selected task detail
- Right pane: creation/edit drawer and quick context

This reduces form fatigue and keeps existing work visible while refining tasks.

## 5.3 Task Workspace Layout

### Top Bar

- current product context
- status filter
- quick `New Task` action
- visible product scope indicator

### Left Pane: Backlog

- compact task list
- each task row shows:
  - title
  - status
  - priority
  - task type
  - optional module/feature context

The list should emphasize scannability and rapid selection.

### Center Pane: Task Detail

When a task is selected:

- title and description
- problem statement
- acceptance criteria
- constraints
- status, type, priority, complexity
- subtasks
- approvals
- artifacts
- findings
- workflow actions

### Right Pane: Create / Edit

The create form should live in a side panel, not as the default full-page state.

The panel should:

- open only when needed
- inherit product context automatically
- make product context visible
- allow creation without requiring hidden sidebar state

## 5.4 Task Creation Rules

- If a product is already selected, default to that product.
- If no product is selected, the user must choose a product in the form.
- If launched from a module or feature in the future, inherit that context automatically.

The user must never be blocked by a hidden “select product in sidebar first” dependency.

## 5.5 Task Detail UX

The selected task must become the anchor for refinement.

The task detail view should:

- prioritize content over chrome
- keep metadata compact
- keep approvals, artifacts, and findings visible but secondary
- support starting workflow and approval actions without leaving the screen

## 5.6 Task Empty States

- No tasks in product: `Create the first task for this product`
- No task selected: `Select a task to inspect details`

## 6. Shared Shell and Navigation Changes

## 6.1 Left Sidebar

The left sidebar should better support hierarchy-driven work.

It should:

- list products more clearly
- show active selection
- allow quick switching between product contexts
- reinforce that products are the primary workspace anchor

Later phases should expand this into a collapsible tree of product -> module -> feature -> task.

## 6.2 Right Sidebar

The right sidebar should provide operational context instead of a placeholder.

It should summarize:

- selected product
- selected task
- current workflow anchor
- what the user is currently working on

## 7. First Redesign Pass Scope

The first redesign pass should include:

- replace product card grid with a split-pane product workspace
- add contextual module and feature creation within the product workspace
- replace task form-first page with backlog + detail + create panel
- make task creation explicitly aware of selected product context
- improve left sidebar product context
- improve right sidebar summary context

This first pass does not require:

- inline tree editing for every node
- drag and drop ordering
- kanban board task view
- repository attachment UX
- advanced task editing forms

## 8. Success Criteria

The redesign is successful if:

- the user can manage products without feeling forced through card/list CRUD flows
- the user can refine product structure from one screen
- the user can browse and edit tasks without losing backlog context
- the UI clearly shows current product and task context
- the product and task areas feel like workspaces instead of admin pages

## 9. Future UX Extensions

After this redesign, later improvements should include:

- collapsible hierarchy tree in the global sidebar
- inline renaming for modules/features/tasks
- drag-and-drop task ordering
- feature-scoped task views
- richer task board views
- repository attachment management in context
- task templates and bulk creation
