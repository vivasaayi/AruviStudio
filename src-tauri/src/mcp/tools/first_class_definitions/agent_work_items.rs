use super::super::definitions::{
    enum_property, first_class_tool, integer_property, json_object_property, object_schema,
    string_array_property, string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
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
    ]
}
