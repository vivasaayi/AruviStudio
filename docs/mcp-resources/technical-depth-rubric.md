# Aruvi Technical Depth Rubric

## What Technical Depth Means

Technical depth does not mean creating many hierarchy levels. It means that Product Areas, Capabilities, and Features expose the behavior and constraints that matter for real implementation.

A technically deep node helps an engineer answer:

- what this part of the product is responsible for
- what goes in and what comes out
- what state it owns or depends on
- what invariants must hold
- what can fail
- what is hard about delivering it correctly

## Minimum Depth Standard

For every meaningful Product Area, Capability, and Feature, capture most of the following:

1. Responsibility
   A precise description of the node's job.

2. Boundary
   What belongs inside the node and what belongs elsewhere.

3. Inputs and Outputs
   User inputs, machine inputs, emitted results, stored state, downstream effects.

4. State Model
   Important state transitions, persistence expectations, temporary state, derived state.

5. Invariants
   What must always remain true if the product is correct.

6. Failure Modes
   Invalid inputs, partial failure, timeout behavior, retry behavior, degraded modes.

7. Non-Functional Constraints
   Performance, determinism, observability, safety, auditability, reproducibility.

8. Delivery Shape
   What stories and tasks should exist under the feature, and how they can be validated independently.

## Signs A Product Is Too Shallow

- node names describe UI surfaces but not responsibilities
- capabilities have no acceptance criteria or technical notes
- features appear before the parent capability is understandable
- data flow and state are absent
- failure modes are not named
- complex behavior is reduced to labels like "support X"

## Relationship To Stories And Tasks

Depth should exist before stories and tasks are derived.

A story or task should feel like a direct consequence of the documented Product Area, Capability, and Feature, not a substitute for it.

If the only concrete statements live in stories or tasks, the product is under-specified.
