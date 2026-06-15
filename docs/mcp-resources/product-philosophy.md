# Aruvi Product Philosophy

## Core Idea

Aruvi does not treat a product tree as a thin planning container. The tree is the canonical readable model of the product.

The hierarchy exists to help a human understand:

- what the product is
- how it is divided
- what each part is responsible for
- where delivery work should come from

If a node does not improve understanding of the wider product, it should not exist.

## Canonical Model

Product management:

Product -> Product Area -> Capability -> Feature

Product delivery:

Feature -> Story -> Task

Product Areas, Capabilities, and Features describe the product. Stories and Tasks describe delivery.

## What Good Product Authoring Looks Like

A strong Aruvi product reads more like a technical manual or architecture book than a backlog.

Good product structure:

- explains the product in sequence
- preserves local detail without losing global context
- gives each node a real semantic job
- keeps execution work derived from the documented structure
- makes state, boundaries, and failure modes legible

Poor product structure:

- mirrors a shallow PM template
- creates nodes only to satisfy a fixed depth
- collapses technical behavior into vague labels like "Core" or "Platform"
- optimizes for ticket grouping instead of product understanding
- treats stories and tasks as the real product model

## Execution Is Subordinate To Structure

The hierarchy should answer:

- what exists in the product
- why it exists
- how it interacts with neighboring parts
- what must be true for it to be considered correct

Stories and tasks answer:

- what delivery outcome should happen next
- how execution should be staged
- what can be validated independently

Execution must not become a disconnected second planning system.

## Practical Authoring Standard

When in doubt:

1. Start from product comprehension, not implementation tasks.
2. Use Product Area, Capability, and Feature deliberately.
3. Capture boundaries, invariants, and technical responsibilities early.
4. Derive stories and tasks from features.
5. Prefer fewer meaningful nodes over many vague ones.
