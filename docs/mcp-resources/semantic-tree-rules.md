# Aruvi Product Hierarchy Rules

## Purpose

The product hierarchy is the canonical readable representation of a product. It is not a Jira taxonomy and it is not a generic node-kind playground.

## Canonical Shape

Product management:

- Product
- Product Area
- Capability
- Feature

Product delivery:

- Feature
- Story
- Task

The full planning chain is:

Product -> Product Area -> Capability -> Feature -> Story -> Task

## Node Kinds

Only these product hierarchy node kinds are valid:

- `area`: a durable top-level product area
- `capability`: something the product must be able to do inside a product area
- `feature`: a product-visible feature under a capability

Stories and tasks are delivery work items, not hierarchy nodes.

## Structural Rules

Allowed product hierarchy children:

- `area` -> `capability`
- `capability` -> `feature`
- `feature` -> no product hierarchy children

Allowed delivery children:

- story -> task

## Authoring Guidance

Use Product Area when the product needs a stable management boundary.

Use Capability when the product must be able to do something durable and owned.

Use Feature when the capability needs a concrete user-visible or system-visible expression that can own stories and tasks.

Use Story for delivery intent attached to a feature.

Use Task for implementation, test, review, or documentation work under a story.

## Valid Examples

- Product -> Product Area -> Capability -> Feature
- Feature -> Story -> Task

## Invalid Examples

- Product -> Feature
- Product Area -> Feature
- Capability -> Story as a hierarchy child
- Story -> Feature
- Task -> Capability

## Book-Grade Detail

Book and overview exports should read the product hierarchy as Product Area chapters, Capability sections, and Feature subsections.

Do not create extra hierarchy levels just to make a document look deeper. Add richer text fields, references, stories, and tasks instead.
