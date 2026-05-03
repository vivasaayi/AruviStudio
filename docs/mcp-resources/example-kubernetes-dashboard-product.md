# Gold-Standard Example Product: Kubernetes Dashboard

## Why This Example Exists

This example shows how Aruvi should describe an operational system with dense data, drill-down behavior, and safety-sensitive actions.

## Product Framing

Name: Kubernetes Dashboard

Intent:

- help operators understand cluster health quickly
- support safe drill-down into workloads and incidents
- keep operational actions auditable and hard to misuse

## Recommended Semantic Tree

### System 1: Cluster Topology

Purpose:
Represent clusters, namespaces, nodes, and resource relationships in a readable operational model.

Children:

- Subsystem 1.1: Namespace Views
- Subsystem 1.2: Node Health
- Subsystem 1.3: Resource Utilization

### System 2: Workload Operations

Purpose:
Help operators inspect workloads, pods, deployments, jobs, and rollout states without losing context.

Children:

- Capability 2.1: Deployment Detail
- Capability 2.2: Pod Explorer
- Capability 2.3: Rollout Status and Revision History

### System 3: Observability

Purpose:
Expose logs, events, alerts, and timeline context needed to explain current cluster behavior.

Children:

- Capability 3.1: Pod Logs
- Capability 3.2: Event Timeline
- Capability 3.3: Alert Surface

### System 4: Safe Actions

Purpose:
Support operator actions such as restart, scale, or delete with clear guardrails and auditability.

Children:

- Capability 4.1: Action Preconditions
- Capability 4.2: Confirmation and Audit Trail
- Capability 4.3: Action Result Feedback

## Example Of Good Technical Depth

### Capability 2.2: Pod Explorer

Responsibility:
Let an operator inspect pods and related state for a workload or namespace.

Boundary:
Owns pod list presentation, pod detail drill-down, filtering, and state summaries. Does not own raw log streaming.

Inputs:

- cluster and namespace selection
- workload selection
- filter and search expressions

Outputs:

- pod list
- lifecycle state summaries
- direct links into logs, events, and owning workload

Invariants:

- list refresh must preserve operator context
- filters must not silently drop selected namespace scope
- workload-to-pod relationships must remain traceable

Failure modes:

- stale watch data
- partial cluster connectivity
- empty namespace states
- very large pod lists

Rollouts:

- Rollout 2.2.1: Namespace-Scoped Pod Listing
- Rollout 2.2.2: Pod Detail Drawer
- Rollout 2.2.3: Filter Persistence and Refresh Safety

## Delivery Derivation

Strong derived work items look like:

- `Render pod list with namespace-safe filtering`
- `Preserve selected pod during incremental refresh`
- `Attach event timeline links from pod detail`

Weak derived work items would be:

- `Make dashboard better`
- `Improve ops UX`

## Why This Is Gold-Standard

This example:

- describes the system as an operator-facing runtime model
- captures safety and observability concerns
- makes rollout boundaries obvious
- prevents the product tree from collapsing into generic UI labels
