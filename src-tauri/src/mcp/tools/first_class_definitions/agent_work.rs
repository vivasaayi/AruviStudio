use super::super::definitions::{
    enum_property, first_class_tool, integer_property, json_object_property, object_schema,
    string_array_property, string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "agent_work.runs.upsert",
            "Upsert Agent Work Run",
            "Create or update a durable multi-agent roadmap run ledger.",
            object_schema(
                vec![
                    ("id", string_property("Run id, for example run-02-sim.")),
                    ("productId", string_property("Optional Aruvi product id.")),
                    ("repositoryId", string_property("Optional repository id.")),
                    ("roadmapHash", string_property("Hash of roadmap inputs.")),
                    (
                        "status",
                        enum_property(
                            "Run status.",
                            &["active", "paused", "completed", "blocked", "cancelled"],
                        ),
                    ),
                    (
                        "lastCommitSha",
                        string_property("Latest implementation commit SHA."),
                    ),
                    ("currentBatchId", string_property("Current batch id.")),
                    ("nextAction", string_property("Exact next action for resume.")),
                    ("metadata", json_object_property("Optional run metadata.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "agent_work.runs.get",
            "Get Agent Work Run",
            "Get a multi-agent run ledger.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.runs.list",
            "List Agent Work Runs",
            "List multi-agent run ledgers.",
            object_schema(
                vec![
                    ("status", string_property("Optional run status filter.")),
                    ("limit", integer_property("Maximum rows to return.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "agent_work.runs.summary",
            "Get Agent Work Run Summary",
            "Get run status counts, active lock count, and recent events.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("eventLimit", integer_property("Recent event limit.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.upsert",
            "Upsert Agent Work Item",
            "Create or update a feature row in a multi-agent run ledger.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("product_area", string_property("Roadmap product_area or domain id.")),
                    (
                        "serviceOrDomain",
                        string_property("Optional service/domain label."),
                    ),
                    ("priority", string_property("Priority, for example P0/P1/P2.")),
                    ("releasePhase", string_property("Release phase, for example M1.")),
                    ("title", string_property("Feature title.")),
                    ("description", string_property("Feature description.")),
                    (
                        "status",
                        enum_property(
                            "Feature status.",
                            &[
                                "pending",
                                "claimed",
                                "in_progress",
                                "implemented",
                                "tests_passed",
                                "committed",
                                "blocked",
                                "skipped",
                                "cancelled",
                            ],
                        ),
                    ),
                    ("batchId", string_property("Optional batch id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    (
                        "conflictZones",
                        string_array_property(
                            "Serialized resources that cannot be edited concurrently.",
                        ),
                    ),
                    ("metadata", json_object_property("Optional feature metadata.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.list",
            "List Agent Work Items",
            "List feature rows with server-side filters and pagination.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("status", string_property("Optional feature status filter.")),
                    ("agent", string_property("Optional agent filter.")),
                    ("limit", integer_property("Maximum rows.")),
                    ("offset", integer_property("Offset.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.claim_next",
            "Claim Next Agent Work Item",
            "Atomically claim the next pending feature row, create/update its batch, and acquire conflict-zone locks with a lease.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("agent", string_property("Claiming agent id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("selectionRule", string_property("Why this row was selected.")),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "agent"],
            ),
        ),
        first_class_tool(
            "agent_work.items.heartbeat",
            "Heartbeat Agent Work Item",
            "Extend a claimed feature row lease and its conflict locks.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    (
                        "claimToken",
                        string_property("Claim token returned by claim_next."),
                    ),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "featureId", "claimToken"],
            ),
        ),
        first_class_tool(
            "agent_work.items.update_status",
            "Update Agent Work Item Status",
            "Update feature row status and append a checkpoint event.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("status", string_property("New feature status.")),
                    ("agent", string_property("Agent id.")),
                    ("batchId", string_property("Batch id.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "featureId", "status"],
            ),
        ),
        first_class_tool(
            "agent_work.items.release_locks",
            "Release Agent Work Item Locks",
            "Release active conflict-zone locks for a feature row.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.requeue",
            "Requeue Agent Work Item",
            "Move a claimed, in-progress, blocked, or stale feature row back to pending and release its locks.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Feature row id.")),
                    ("agent", string_property("Agent or coordinator id.")),
                    ("details", string_property("Reason for requeue.")),
                ],
                &["runId", "featureId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.requeue_expired",
            "Requeue Expired Agent Work Items",
            "Release expired leases and move stale claimed/in-progress rows back to pending.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("agent", string_property("Agent or coordinator id.")),
                    ("details", string_property("Reason recorded on requeue events.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.items.list_ready",
            "List Ready Agent Work Items",
            "List pending feature rows whose dependencies are already satisfied.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("limit", integer_property("Maximum rows.")),
                    ("offset", integer_property("Offset.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.locks.list_active",
            "List Active Agent Work Locks",
            "List active conflict-zone locks for a run.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.conflict_zones.list",
            "List Agent Work Conflict Zones",
            "List active conflict-zone reservations grouped by zone.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.conflict_zones.inspect",
            "Inspect Agent Work Conflict Zone",
            "Inspect active locks for one conflict zone.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                ],
                &["runId", "zoneKey"],
            ),
        ),
        first_class_tool(
            "agent_work.conflict_zones.reserve",
            "Reserve Agent Work Conflict Zone",
            "Reserve a run-level conflict zone such as a shared build target, migration lane, or repo path.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                    ("agent", string_property("Agent id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("claimToken", string_property("Optional claim token.")),
                    ("leaseSeconds", integer_property("Lease duration in seconds.")),
                ],
                &["runId", "zoneKey", "agent"],
            ),
        ),
        first_class_tool(
            "agent_work.conflict_zones.release",
            "Release Agent Work Conflict Zone",
            "Release an active conflict-zone reservation.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("zoneKey", string_property("Conflict-zone key.")),
                    ("claimToken", string_property("Optional claim token guard.")),
                ],
                &["runId", "zoneKey"],
            ),
        ),
        first_class_tool(
            "agent_work.batches.complete",
            "Complete Agent Work Batch",
            "Mark a batch implemented, tests_passed, committed, blocked, skipped, or cancelled.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Batch id.")),
                    ("status", string_property("Batch status.")),
                    ("agent", string_property("Agent id.")),
                    ("commitSha", string_property("Optional commit SHA.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "batchId", "status"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.upsert",
            "Upsert Agent Work Dependency",
            "Record that one feature row depends on another feature row before it can be claimed.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Dependent feature id.")),
                    (
                        "dependsOnFeatureId",
                        string_property("Prerequisite feature id."),
                    ),
                    ("dependencyKind", string_property("Dependency kind.")),
                    ("metadata", json_object_property("Optional dependency metadata.")),
                ],
                &["runId", "featureId", "dependsOnFeatureId"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.delete",
            "Delete Agent Work Dependency",
            "Remove a dependency edge between two feature rows.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Dependent feature id.")),
                    (
                        "dependsOnFeatureId",
                        string_property("Prerequisite feature id."),
                    ),
                ],
                &["runId", "featureId", "dependsOnFeatureId"],
            ),
        ),
        first_class_tool(
            "agent_work.dependencies.list",
            "List Agent Work Dependencies",
            "List dependency edges for a run or one feature row.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Optional dependent feature id.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.evidence.append",
            "Append Agent Work Evidence",
            "Append structured evidence for commands, tests, changed files, artifacts, or validation.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("agent", string_property("Optional agent id.")),
                    (
                        "evidenceType",
                        string_property("Evidence type, such as test, diff, validation, review."),
                    ),
                    ("command", string_property("Command that produced the evidence.")),
                    ("exitCode", integer_property("Command exit code.")),
                    ("status", string_property("Evidence status.")),
                    ("summary", string_property("Short evidence summary.")),
                    ("details", string_property("Long evidence details.")),
                    ("changedFiles", string_array_property("Changed files.")),
                    ("artifactRefs", string_array_property("Artifact ids or URIs.")),
                    ("metadata", json_object_property("Optional evidence metadata.")),
                ],
                &["runId", "evidenceType"],
            ),
        ),
        first_class_tool(
            "agent_work.evidence.list",
            "List Agent Work Evidence",
            "List structured evidence for a run, feature, batch, or agent.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("limit", integer_property("Maximum rows.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.runs.health",
            "Get Agent Work Run Health",
            "Get summary counts, expired claims, ready items, active agents, active conflict zones, and latest evidence.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.agents.activity",
            "List Agent Work Agent Activity",
            "List agent activity counts and latest heartbeat/event times for a run.",
            object_schema(vec![("runId", string_property("Run id."))], &["runId"]),
        ),
        first_class_tool(
            "agent_work.events.append",
            "Append Agent Work Event",
            "Append a durable coordination/checkpoint event.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("eventType", string_property("Event type.")),
                    ("batchId", string_property("Optional batch id.")),
                    ("featureId", string_property("Optional feature row id.")),
                    ("workItemId", string_property("Optional Aruvi work item id.")),
                    ("agent", string_property("Optional agent id.")),
                    ("command", string_property("Command or action.")),
                    ("status", string_property("Event status.")),
                    ("details", string_property("Event details.")),
                    ("metadata", json_object_property("Optional event metadata.")),
                ],
                &["runId", "eventType"],
            ),
        ),
        first_class_tool(
            "agent_work.events.list",
            "List Agent Work Events",
            "List checkpoint events with cursor and feature filters.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("afterId", integer_property("Only events after this id.")),
                    ("featureId", string_property("Optional feature row id.")),
                    ("limit", integer_property("Maximum rows.")),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.commits.link",
            "Link Agent Work Commit",
            "Link a git commit to feature rows and a batch, mark them committed, release locks, and update the run checkpoint.",
            object_schema(
                vec![
                    ("runId", string_property("Run id.")),
                    ("batchId", string_property("Batch id.")),
                    (
                        "featureIds",
                        string_array_property("Feature row ids included in the commit."),
                    ),
                    ("commitSha", string_property("Commit SHA.")),
                    ("agent", string_property("Agent id.")),
                    ("details", string_property("Event details.")),
                ],
                &["runId", "batchId", "featureIds", "commitSha"],
            ),
        ),
    ]
}
