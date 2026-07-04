use super::super::definitions::{
    boolean_property, enum_property, first_class_tool, integer_property, object_schema,
    string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "agent_work.import_legacy_checkpoint",
            "Import Legacy Agent Checkpoint",
            "Import an AGBot-style .codex/.claude checkpoint.sqlite ledger into the Aruvi MCP agent-work tables.",
            object_schema(
                vec![
                    (
                        "checkpointPath",
                        string_property("Absolute path to checkpoint.sqlite."),
                    ),
                    ("runId", string_property("Optional target run id.")),
                    ("sourceLabel", string_property("Optional source label.")),
                ],
                &["checkpointPath"],
            ),
        ),
        first_class_tool(
            "agent_work.materialize_catalog",
            "Materialize Agent Work Catalog",
            "Bulk-create or update catalog product areas, capabilities, features, visible work items, and ledger work-item links from an agent-work run.",
            object_schema(
                vec![
                    ("runId", string_property("Agent-work run id.")),
                    (
                        "productId",
                        string_property(
                            "Optional product id. Required when the run is not attached to a product.",
                        ),
                    ),
                    (
                        "createWorkItems",
                        boolean_property(
                            "Whether to create visible delivery work items and link ledger rows. Defaults to true.",
                        ),
                    ),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.link_catalog_work_items",
            "Link Catalog Work Items",
            "Link existing catalog work items to agent-work rows by matching stable feature ids.",
            object_schema(
                vec![
                    ("runId", string_property("Agent-work run id.")),
                    (
                        "productId",
                        string_property(
                            "Optional product id. Required when the run is not attached to a product.",
                        ),
                    ),
                    (
                        "syncStatuses",
                        boolean_property(
                            "Whether to sync agent-work status onto matched work items and catalog features. Defaults to false.",
                        ),
                    ),
                ],
                &["runId"],
            ),
        ),
        first_class_tool(
            "agent_work.context.get_feature",
            "Get Feature Implementation Context",
            "Get 360-degree product, feature, story, parent, sibling, reference, dependency, evidence, and agent-work context for implementation.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    (
                        "featureId",
                        string_property("Optional product feature/capability id."),
                    ),
                    ("workItemId", string_property("Optional story/task id.")),
                    ("runId", string_property("Optional agent-work run id.")),
                    (
                        "includeProductTree",
                        boolean_property("Include full product tree."),
                    ),
                    (
                        "siblingLimit",
                        integer_property("Maximum siblings to include."),
                    ),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "agent_work.context.export_feature",
            "Export Feature Implementation Context",
            "Write feature implementation context to a JSON or Markdown file for agent handoff.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    (
                        "featureId",
                        string_property("Optional product feature/capability id."),
                    ),
                    ("workItemId", string_property("Optional story/task id.")),
                    ("runId", string_property("Optional agent-work run id.")),
                    (
                        "includeProductTree",
                        boolean_property("Include full product tree."),
                    ),
                    (
                        "siblingLimit",
                        integer_property("Maximum siblings to include."),
                    ),
                    ("outputPath", string_property("File path to write.")),
                    ("format", enum_property("Export format.", &["json", "markdown"])),
                ],
                &["outputPath"],
            ),
        ),
    ]
}
