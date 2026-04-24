# Aruvi Semantic Tree Rules

## Purpose

The semantic tree is the canonical readable representation of the product. It is not only a planning taxonomy.

## Node Kinds

### Root Kinds

Only these node kinds should be used as top-level roots under a product:

- `area`
- `domain`
- `system`

### Full Kind Set

- `area`: a major functional or conceptual area
- `domain`: a coherent domain of responsibility
- `subdomain`: a meaningful subdivision inside a domain
- `system`: a concrete technical system or major runtime surface
- `subsystem`: a subdivision inside a system
- `feature_set`: a grouped set of related abilities
- `capability`: a stable ability the product must provide
- `rollout`: a delivery slice that evolves or ships part of a capability
- `reference`: supporting context, external dependency, or explanatory attachment

## Structural Rules

### Kinds That Can Be Roots

- `area`
- `domain`
- `system`

### Kinds That Cannot Have Structural Children

- `rollout`
- `reference`

### Allowed Child Kinds

#### `area`

- `area`
- `domain`
- `system`
- `subsystem`
- `feature_set`
- `capability`
- `reference`

#### `domain`

- `subdomain`
- `system`
- `subsystem`
- `feature_set`
- `capability`
- `reference`

#### `subdomain`

- `subdomain`
- `feature_set`
- `capability`
- `reference`

#### `system`

- `subsystem`
- `feature_set`
- `capability`
- `reference`

#### `subsystem`

- `subsystem`
- `feature_set`
- `capability`
- `reference`

#### `feature_set`

- `feature_set`
- `capability`
- `rollout`
- `reference`

#### `capability`

- `feature_set`
- `capability`
- `rollout`
- `reference`

#### `rollout`

- no structural children

#### `reference`

- no structural children

## Usage Guidance

Use `area`, `domain`, and `system` to make the product legible at the top level.

Use `feature_set` when you need a grouping layer but a node is not yet a single stable ability.

Use `capability` when the node represents a real product or system ability with clear ownership and correctness conditions.

Use `rollout` only when you are describing a delivery slice, staged implementation path, or guarded release boundary.

Use `reference` when the node exists to attach durable context rather than to describe a part of the product itself.

## Example Valid Chains

- Product -> Area -> Capability -> Rollout
- Product -> Domain -> Subdomain -> Capability -> Rollout
- Product -> System -> Subsystem -> Feature Set -> Capability -> Rollout
- Product -> Area -> Reference

## Example Invalid Patterns

- Product -> Rollout as a root
- Rollout -> Capability
- Reference -> Rollout
- Deep chains created only to mimic an org chart

## Book-Grade Detail Pattern

`rollout` is an execution leaf. It is not the right place for deeper structural chapters such as:

- what this topic is
- examples
- implementation notes
- test strategy

If you need book-grade detail under a technical topic, use a parent that can own structure:

- `feature_set` when grouping a family of related concepts
- `capability` when the node represents a stable technical ability
- `reference` when the child exists to carry durable explanatory context

Supported pattern:

- parent `feature_set` or `capability` for the operator family or technical chapter
- child `capability` or `reference` nodes for definition, examples, constraints, or variants
- child `rollout` nodes for implementation and shipping slices
- child work items for executable delivery under the rollout or directly under the documented node when necessary

Unsupported pattern:

- `rollout` -> `capability`
- `rollout` -> `reference`
- using a rollout page as if it were the summary container for the full chapter tree

When an LLM wants to create a "book" under a rollout, that structure should move one level up.

## Authoring Standard

Choose the shallowest tree that still preserves understanding.

Add depth only when it clarifies:

- architectural boundaries
- major subsystems
- delivery sequencing
- technical ownership

Do not add depth only because the UI can render it.
