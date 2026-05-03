# Aruvi Technical Depth Rubric

## What "Technical Depth" Means

In Aruvi, technical depth does not mean writing a large number of nodes. It means that the product description exposes the behavior and constraints that matter for real implementation.

A technically deep node should help an engineer answer:

- what this part of the system is responsible for
- what goes in and what comes out
- what state it owns or depends on
- what invariants must hold
- what can fail
- what is hard about delivering it correctly

## Minimum Depth Standard For Important Nodes

For every meaningful `capability`, `feature_set`, `system`, or `subsystem`, capture most of the following:

1. Responsibility
   A precise description of the node's job.

2. Boundary
   What belongs inside the node and what belongs elsewhere.

3. Inputs and Outputs
   User inputs, machine inputs, emitted results, stored state, downstream effects.

4. State Model
   Important state transitions, persistence expectations, temporary state, derived state.

5. Invariants
   What must always remain true if the system is correct.

6. Failure Modes
   Invalid inputs, partial failure, timeout behavior, retry behavior, degraded modes.

7. Non-Functional Constraints
   Performance, determinism, observability, safety, auditability, reproducibility.

8. Delivery Shape
   What rollouts make sense under this node and how delivery can be staged safely.

## Signs A Product Is Too Shallow

- node names describe UI surfaces but not system responsibilities
- capabilities have no acceptance criteria or technical notes
- rollouts appear before the parent node is understandable
- data flow and state are absent
- failure modes are not named
- complex behavior is reduced to labels like "support X"

## Rewrite Pattern

Weak:

- `Scientific Functions`

Better:

- `Expression Engine`
- `Arithmetic Evaluation`
- `Scientific Evaluation`
- `Angle Mode State`
- `Error and Validation Surfaces`

The better version explains the system as a set of technical responsibilities, not just a menu of buttons.

## What To Capture For A Strong Capability

For a capability like `Arithmetic Evaluation`, strong depth usually includes:

- supported operator classes
- precedence and grouping rules
- calculator state transitions
- formatting and display behavior
- error handling for malformed expressions
- repeated-equals behavior
- determinism and test strategy

## Relationship To Work Items

Depth should exist before work items are derived.

A rollout or work item should feel like a direct consequence of the documented model, not a substitute for it.

If the only concrete statements live in work items, the product is under-specified.

## Rollouts Are Not Chapter Containers

In Aruvi, `rollout` nodes are execution leaves. They may carry rollout-level intent, but they should not be used as the parent container for the full explanatory structure of a topic.

If you want a topic to read like a chapter in the book, put the detailed explanation one level above the rollout:

- parent `feature_set` or `capability` for the topic itself
- child `reference` or `capability` nodes for "what it is", examples, constraints, or variants
- child `rollout` nodes for implementation and shipping slices
- child work items for executable delivery steps

If the model tries to put "Examples", "Implementation", and "Tests" under a rollout, that is a signal that the documented topic is missing a parent structural node.
