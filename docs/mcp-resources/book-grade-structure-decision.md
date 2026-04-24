# Decision: Keep Rollouts as Execution Leaves

Status: accepted

Date: 2026-04-22

## Decision

Aruvi will keep `rollout` nodes as execution leaves.

For book-grade detail, the supported structural pattern is:

- a semantic parent such as `feature_set`, `capability`, `system`, or `subsystem`
- child `capability` or `reference` nodes for explanation, examples, constraints, or variants
- child `rollout` nodes for implementation and shipping slices
- child work items for executable delivery

Aruvi will not allow structural children under `rollout` nodes.

Aruvi will also not introduce a generic `chapter` node kind at this stage.

## Why

### Why Keep Rollouts As Leaves

`rollout` has a clear semantic job: it describes a delivery slice.

If rollouts can own deeper structure, they stop being clean execution boundaries and become mixed documentation containers. That would blur:

- what is system structure
- what is explanatory context
- what is execution planning

Current Aruvi assumptions already treat rollouts as leaves:

- node-kind rules forbid structural children under rollouts
- UI behavior reads rollout pages as execution-oriented summaries
- work-item derivation is cleaner when rollout remains a terminal delivery concept

Keeping rollouts as leaves preserves a strong separation between readable structure and execution state.

### Why Not Add A Generic `chapter` Node Kind Yet

A generic `chapter` node kind is attractive from a book-rendering perspective, but it is weak at the semantic layer.

Problems with a `chapter` node kind:

- it is presentation-oriented rather than system-oriented
- it overlaps with existing kinds such as `feature_set`, `capability`, and `reference`
- it encourages users to model the document instead of the system
- it creates a second axis of meaning that clients would need to interpret

The semantic tree should describe the product, not just the eventual book layout.

## Supported Pattern

When a topic needs rich explanation, examples, implementation guidance, and tests:

1. Create a structural parent for the topic.
2. Put durable explanatory content in child `reference` or `capability` nodes.
3. Put implementation slices in child `rollout` nodes.
4. Put executable tasks in work items attached to those rollout or topic nodes.

Example:

- `Feature Set`: Powers and Roots Book
- `Capability`: Square Operator
- `Reference`: What It Is
- `Reference`: Worked Examples
- `Rollout`: Implementation
- `Rollout`: Tests and Validation

This keeps the topic readable while preserving the rollout as an execution leaf.

## Unsupported Pattern

This is explicitly unsupported:

- `Rollout`: Powers and Roots
  - `Reference`: What It Is
  - `Reference`: Examples
  - `Rollout`: Implementation

That structure fails because rollout nodes cannot contain structural children.

## Consequences

This decision implies the following follow-on work:

- expose node-kind constraints earlier in MCP discovery surfaces
- add semantic authoring templates for book-shaped topics
- add richer long-form fields so explanation does not require artificial trees
- support safer node-kind conversion when users modeled a topic as a rollout too early
- improve planner decomposition so clients propose the supported structure automatically

## Practical Guidance For Clients

If a model wants to create a book under a rollout, it should move that detail one level up.

The correct recovery is not "force deeper rollout nesting." The correct recovery is:

- create a structural parent for the topic
- move explanation into `reference` or `capability` children
- keep rollouts for execution
