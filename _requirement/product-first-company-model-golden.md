# Product-First Company Model Golden File

## Decision

AruviStudio is a product-oriented company operating system, not a project tracker and not a single universal hierarchy.

The previous all-in-one semantic tree mixed too many concerns:

- company strategy
- executive/head ownership
- product catalog
- architecture decomposition
- capability design
- delivery execution
- reference documentation

That made the system expressive, but ambiguous. The new model separates slow-moving company/product structure from fast-moving product design and delivery.

## Core Principle

Keep `Product` first-class.

A product is not just a node in a strategy tree. A product has lifecycle, health, roadmap, dependencies, evidence, delivery, documentation, and agent work. Products are the center of AruviStudio.

Strategy nodes organize products. They do not replace products.

## One-Person Company Model

AruviStudio should support a solo founder who switches hats:

- CEO hat: decide where to invest.
- Head hat: own a durable domain or product family.
- Product Owner hat: shape a product and its capabilities.
- Builder hat: break capabilities into executable delivery.
- Agent surface: execute only scoped product/design/delivery work.

These are persona lenses over one product truth, not separate org charts.

## New Canonical Surfaces

### Strategy Map

Slow-moving, human-created, not agent-mutated by default.

```text
Company
  Strategic Area
    Domain
      Subdomain
```

Purpose:

- CEO/head thinking
- investment themes
- incubation comparison
- durable market/problem organization
- product portfolio rollups

Examples:

- Strategic Area: Connected Devices
- Domain: Wearable Computing
- Subdomain: Health Monitoring

### Product Catalog

First-class products, linked to strategy.

```text
Product
  primary_strategy_node_id
  secondary_strategy_node_ids
  lifecycle
  health
  owner/lens metadata
  dependencies
```

Purpose:

- product-oriented thinking
- product comparison
- sellable/adoptable offering ownership
- success and incubation tracking

Examples:

- Product: Smart Watch
- Product: WiFi Platform
- Product: Aruvi Studio

### Product Design

Moderately changing. Human-owned, agent-assisted.

```text
Product
  Product Area
    Capability
      Feature
```

Purpose:

- product owner thinking
- product design
- roadmap framing
- agent context boundary

Examples:

- Product Area: Network Sync
- Capability: Sync health data over network
- Feature: Retry sync after transient WiFi loss

### Delivery

Fast-changing. Agent-visible and agent-executable.

```text
Feature
  Story
    Task
```

Purpose:

- implementation
- test work
- review work
- workflow execution
- evidence capture

Examples:

- Delivery Item: Implement retry backoff policy
- Delivery Item: Add network-state integration tests

### References

References are attached context, not structural nodes.

References can attach to:

- Strategy Area
- Domain
- Subdomain
- Product
- Product Area
- Capability
- Feature
- Story
- Task

Purpose:

- standards
- external docs
- architecture notes
- customer evidence
- regulatory constraints
- design packets

## Retired Concepts

Legacy architecture/shipment aliases should not remain as primary structural node kinds.

Replacement:

- company strategy grouping belongs in Portfolio
- product structure uses Product Area, Capability, and Feature
- execution is captured as stories and tasks
- references are attached context

## Persona Lenses

### CEO Lens

Shows:

- Strategic Areas
- Domains/Subdomains
- Products under each strategy node
- product lifecycle
- product health
- investment status
- cross-product comparison

Hides:

- features by default unless product detail is expanded
- stories and tasks by default
- agent implementation detail

### Head Lens

Shows:

- owned domains/subdomains
- products in that domain
- product dependencies
- capability health rollups
- risk/bottlenecks

Hides:

- low-level delivery unless expanded

### Product Owner Lens

Shows:

- selected product
- product areas
- capabilities
- features
- references
- roadmap/status
- dependencies on other products/capabilities

Hides:

- company-wide strategy editing by default
- raw agent execution logs by default

### Builder Lens

Shows:

- selected product
- features
- stories
- tasks
- repositories
- implementation artifacts
- tests/review evidence

Hides:

- strategy hierarchy editing

### Agent Lens

Agents can see:

- Product
- Product Area
- Capability
- Feature
- Story
- Task
- References attached to those scopes

Agents should not mutate by default:

- Strategic Area
- Domain
- Subdomain
- Product identity/lifecycle

## Cross-Product Dependencies

Cross-cutting platform work should be modeled as product dependencies, not nested architecture branches.

Example:

```text
Strategic Area: Connected Devices
  Domain: Wearable Computing
    Subdomain: Health Monitoring
      Product: Smart Watch
        Capability: Sync health data over network
          depends on: WiFi Platform / Secure device pairing

Strategic Area: Shared Platforms
  Domain: Device Connectivity
    Product: WiFi Platform
      Capability: Secure device pairing
      Capability: Reliable background sync
```

CEO sees Smart Watch as one product with health/status.

Head/Product Owner sees that Smart Watch depends on WiFi Platform capabilities.

Builder/Agent sees the scoped feature, story, and task.

## Implementation Direction

### Greenfield Structural Direction

Because AruviStudio is early-stage, prioritize structural clarity over backward compatibility.

Do not preserve the old mixed taxonomy in the main product design UI.

### Target Data Model

Introduce or converge toward:

- `strategy_nodes`
  - `strategic_area`
  - `domain`
  - `subdomain`
- `product_strategy_links`
- `product_dependencies`
- `product_areas` through module storage
- `capabilities`
- `features` through capability storage
- `work_items` as stories and tasks
- `references`

Keep `products` first-class.

### Target UI Navigation

Replace the single cluttered product tree with separate surfaces:

- Portfolio / CEO
- Strategy Map / Head
- Products
- Product Design
- Delivery / Builder
- Agents

### Product Page Tabs

Products page should become:

- Product List
- Product Status
- Product Design
- Dependencies

Product Design should show only:

- Product
- Product Areas
- Capabilities
- Features
- attached references

Delivery should show stories and tasks in the Work Items / Builder surface.

### Planner Direction

Planner should become product-first:

1. Select product.
2. Generate a design/review packet.
3. Show architecture/product-design proposal.
4. Generate HTML review artifact.
5. Show diffs.
6. Apply after human confirmation.

Planner should not freely create strategic hierarchy unless the user is explicitly in Strategy Map mode.

## Non-Goals

- Do not build a generic project management hierarchy.
- Do not expose every structural layer on one screen.
- Do not make agents edit company strategy by default.
- Do not model cross-cutting technology as nested children of every consuming product.
- Do not make `Product` a mere child node under strategy.

## Acceptance Criteria

The new model is working when:

- CEO/head views can compare products without delivery clutter.
- Product Owner view can refine capabilities without strategy clutter.
- Builder view can execute delivery without portfolio clutter.
- Agents receive smaller, safer context.
- Cross-product dependencies are visible without duplicating platform structure.
- A solo founder can switch hats without changing the underlying product truth.
