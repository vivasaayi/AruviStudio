use super::super::definitions::{
    enum_property, first_class_tool, integer_property, json_object_property, object_schema,
    string_array_property, string_property, ToolDefinition,
};
use super::agent_work_items;

pub(super) fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = vec![
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
                    (
                        "nextAction",
                        string_property("Exact next action for resume."),
                    ),
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
    ];
    definitions.extend(agent_work_items::definitions());
    definitions.extend(vec![
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
    ]);
    definitions
}
