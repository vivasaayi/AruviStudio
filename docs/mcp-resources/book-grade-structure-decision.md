# Decision: Features Own Delivery Stories

Status: accepted

Date: 2026-06-14

## Decision

Aruvi will use one canonical product-management hierarchy:

Product -> Product Area -> Capability -> Feature

Delivery work starts below a feature:

Feature -> Story -> Task

Stories and tasks are work items. They are not product hierarchy nodes.

## Why

The product tree should explain what the product is and what it can do. Delivery work should explain what needs to be implemented, tested, reviewed, or shipped.

Keeping Feature as the product-management leaf gives each role a clean surface:

- CEO/Head: Product Areas and overall portfolio direction
- Product Owner: Capabilities and Features
- Deliverer: Stories and Tasks

## Book And Overview Rendering

Book and overview surfaces should render:

- Product Areas as major chapters
- Capabilities as sections
- Features as concrete subsections
- Stories and Tasks as delivery detail attached to features

If a topic needs richer explanation, add long-form fields, references, examples, acceptance criteria, stories, and tasks. Do not invent extra hierarchy levels.

## Unsupported

Legacy product hierarchy aliases and extra intermediate layers are no longer supported.

Strategy and portfolio grouping remain outside this product-management hierarchy.
