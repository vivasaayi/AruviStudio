use super::super::definitions::{
    boolean_property, empty_object_schema, enum_property, first_class_tool, integer_property,
    object_schema, string_array_property, string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "work_items.list",
            "List Work Items",
            "List delivery stories and tasks filtered by product, product area, feature, source scope, or status. Results are server-side paginated; set includePagination=true to receive workItems plus pagination metadata.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("sourceNodeId", string_property("Optional source node id.")),
                    (
                        "sourceNodeType",
                        enum_property(
                            "Optional source node type.",
                            &["product_area", "capability", "feature"],
                        ),
                    ),
                    ("status", string_property("Optional work item status filter.")),
                    (
                        "limit",
                        integer_property(
                            "Maximum rows to return. Defaults to 500 and is capped at 2000.",
                        ),
                    ),
                    ("offset", integer_property("Pagination offset.")),
                    (
                        "includePagination",
                        boolean_property(
                            "When true, return { workItems, pagination } instead of the legacy raw array result.",
                        ),
                    ),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "work_items.get",
            "Get Work Item",
            "Get a work item by id.",
            object_schema(vec![("id", string_property("The work item id."))], &["id"]),
        ),
        first_class_tool(
            "work_items.create",
            "Create Work Item",
            "Create a delivery story or task attached to a product and optional source scope. Prefer work_items.stories.create for feature-attached stories and work_items.tasks.create for tasks under stories.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("featureId", string_property("Optional feature id.")),
                    ("sourceNodeId", string_property("Optional source node id.")),
                    (
                        "sourceNodeType",
                        enum_property(
                            "Optional source node type.",
                            &["product_area", "capability", "feature"],
                        ),
                    ),
                    (
                        "parentWorkItemId",
                        string_property("Optional parent work item id."),
                    ),
                    ("title", string_property("The work item title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short work item description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the work item."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "workItemType",
                        enum_property(
                            "Delivery work item type. story/task are accepted MCP aliases and persist on the legacy delivery type.",
                            &[
                                "story",
                                "task",
                                "setup",
                                "bug",
                                "refactor",
                                "test",
                                "review",
                                "security_fix",
                                "performance_improvement",
                            ],
                        ),
                    ),
                    (
                        "priority",
                        enum_property("Priority level.", &["critical", "high", "medium", "low"]),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["productId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.stories.create",
            "Create Story",
            "Create a delivery story attached directly to a product feature.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    (
                        "featureId",
                        string_property("The feature id that owns this story."),
                    ),
                    ("title", string_property("The story title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short story description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the story."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "priority",
                        enum_property("Priority level.", &["critical", "high", "medium", "low"]),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["productId", "featureId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.tasks.create",
            "Create Task",
            "Create an implementation, test, review, documentation, or release task under a delivery story.",
            object_schema(
                vec![
                    ("storyId", string_property("The parent story work item id.")),
                    (
                        "productId",
                        string_property(
                            "Optional product id. If omitted, it is inherited from the story.",
                        ),
                    ),
                    ("title", string_property("The task title.")),
                    ("problemStatement", string_property("Problem statement.")),
                    ("description", string_property("Short task description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the task."),
                    ),
                    ("constraints", string_property("Execution constraints.")),
                    (
                        "priority",
                        enum_property("Priority level.", &["critical", "high", "medium", "low"]),
                    ),
                    (
                        "complexity",
                        enum_property(
                            "Complexity level.",
                            &["trivial", "low", "medium", "high", "very_high"],
                        ),
                    ),
                ],
                &["storyId", "title"],
            ),
        ),
        first_class_tool(
            "work_items.update",
            "Update Work Item",
            "Update mutable fields on a work item.",
            object_schema(
                vec![
                    ("id", string_property("The work item id.")),
                    ("title", string_property("Updated title.")),
                    ("description", string_property("Updated description.")),
                    ("status", string_property("Updated status.")),
                    (
                        "problemStatement",
                        string_property("Updated problem statement."),
                    ),
                    (
                        "acceptanceCriteria",
                        string_property("Updated acceptance criteria."),
                    ),
                    ("constraints", string_property("Updated constraints.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "work_items.delete",
            "Delete Work Item",
            "Delete a work item.",
            object_schema(vec![("id", string_property("The work item id."))], &["id"]),
        ),
        first_class_tool(
            "work_items.list_children",
            "List Child Work Items",
            "List direct child work items for a parent work item.",
            object_schema(
                vec![
                    ("workItemId", string_property("The parent work item id.")),
                    ("limit", integer_property("Maximum rows to return.")),
                    ("offset", integer_property("Pagination offset.")),
                ],
                &["workItemId"],
            ),
        ),
        first_class_tool(
            "work_items.reorder",
            "Reorder Work Items",
            "Reorder work items by supplying the desired ordered ids.",
            object_schema(
                vec![(
                    "orderedIds",
                    string_array_property("Work item ids in the desired order."),
                )],
                &["orderedIds"],
            ),
        ),
        first_class_tool(
            "work_items.summarize_by_product",
            "Summarize Work Items By Product",
            "Summarize work item counts grouped by product.",
            empty_object_schema(),
        ),
    ]
}
