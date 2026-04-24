# Aruvi Product Philosophy

## Core Idea

Aruvi does not treat a product tree as a thin planning container. The tree is the canonical readable model of the product.

The hierarchy exists to help a human understand:

- what the system is
- how it is divided
- what each part is responsible for
- where delivery work should come from

If a node does not improve understanding of the wider system, it should not exist.

## What Good Product Authoring Looks Like

A strong Aruvi product reads more like a technical manual or architecture book than a backlog.

Good product structure:

- explains the system in sequence
- preserves local detail without losing global context
- gives each node a real semantic job
- keeps execution work derived from the documented structure
- makes architecture, state, boundaries, and failure modes legible

Poor product structure:

- mirrors a shallow PM template
- creates nodes only to satisfy a fixed depth
- collapses technical behavior into vague labels like "Core" or "Platform"
- optimizes for ticket grouping instead of system understanding
- treats rollouts and work items as the real product model

## Execution Is Subordinate To Structure

In Aruvi, rollouts and work items are downstream of the product hierarchy.

The hierarchy should answer:

- what exists in the system
- why it exists
- how it interacts with neighboring parts
- what must be true for it to be considered correct

Then rollouts and work items answer:

- what delivery slice should happen next
- how execution should be staged
- what can be validated independently

Execution must not become a disconnected second planning system.

## Product Nodes Should Carry Technical Meaning

Each node should make a concrete claim about the system.

Examples of good node intent:

- "Expression Engine" owns parsing, state normalization, and deterministic evaluation
- "Cluster Observability" owns logs, events, alerts, and operator-facing incident context
- "Inbox Search" owns query semantics, filtering, result ranking, and empty-state behavior

Examples of weak node intent:

- "Stuff"
- "Core Features"
- "Future Work"
- "Frontend"

## Reader-First, Not Ticket-First

When authoring a product in Aruvi, prefer a structure that a new engineer could read top to bottom and come away with a mental model of:

- the major areas of the system
- the important subsystems and capabilities
- the technical constraints
- the rollout shape

If the structure would only make sense to someone already looking at tickets, it is too shallow.

## Practical Authoring Standard

When in doubt:

1. Start from system comprehension, not implementation tasks.
2. Use semantic node kinds deliberately.
3. Capture boundaries, invariants, and technical responsibilities early.
4. Derive rollouts only after the parent node is understandable.
5. Prefer fewer meaningful nodes over many vague ones.
