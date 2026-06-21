use super::definitions::{
    boolean_property, empty_object_schema, enum_property, first_class_tool, integer_property,
    json_object_property, object_schema, string_array_property, string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = Vec::new();
    definitions.extend(catalog_definitions());
    definitions.extend(work_item_definitions());
    definitions.extend(agent_work_definitions());
    definitions.extend(repository_definitions());
    definitions
}

fn catalog_definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "catalog.products.list",
            "List Products",
            "List visible products in the Aruvi catalog.",
            empty_object_schema(),
        ),
        first_class_tool(
            "catalog.products.get",
            "Get Product",
            "Get a single product by id.",
            object_schema(vec![("id", string_property("The product id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.products.create",
            "Create Product",
            "Create a new product.",
            object_schema(
                vec![
                    ("name", string_property("The product name.")),
                    ("description", string_property("Short product description.")),
                    ("vision", string_property("Longer product vision statement.")),
                    ("goals", string_array_property("Ordered list of product goals.")),
                    ("tags", string_array_property("Optional product tags.")),
                ],
                &["name"],
            ),
        ),
        first_class_tool(
            "catalog.products.update",
            "Update Product",
            "Update mutable fields on an existing product.",
            object_schema(
                vec![
                    ("id", string_property("The product id.")),
                    ("name", string_property("Updated product name.")),
                    ("description", string_property("Updated product description.")),
                    ("vision", string_property("Updated product vision.")),
                    ("goals", string_array_property("Updated list of product goals.")),
                    ("tags", string_array_property("Updated product tags.")),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.products.archive",
            "Archive Product",
            "Archive a product.",
            object_schema(vec![("id", string_property("The product id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.products.get_tree",
            "Get Product Tree",
            "Get the full semantic product tree for a product.",
            object_schema(
                vec![("productId", string_property("The product id."))],
                &["productId"],
            ),
        ),
        first_class_tool(
            "catalog.references.list",
            "List Catalog References",
            "List scoped notes, external docs, evidence, architecture references, standards, and design packets attached to product book scopes.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property(
                            "Optional scope type filter.",
                            &[
                                "strategy_node",
                                "product",
                                "product_area",
                                "capability",
                                "feature",
                                "delivery_item",
                            ],
                        ),
                    ),
                    ("scopeId", string_property("Optional scope id filter.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "catalog.references.create",
            "Create Catalog Reference",
            "Attach a scoped reference to a product, product area, capability, feature, or delivery item.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property(
                            "Reference scope type.",
                            &[
                                "strategy_node",
                                "product",
                                "product_area",
                                "capability",
                                "feature",
                                "delivery_item",
                            ],
                        ),
                    ),
                    ("scopeId", string_property("Reference scope id.")),
                    ("title", string_property("Reference title.")),
                    (
                        "referenceKind",
                        enum_property(
                            "Reference kind.",
                            &[
                                "note",
                                "external_doc",
                                "architecture",
                                "customer_evidence",
                                "regulatory",
                                "design_packet",
                                "standard",
                                "other",
                            ],
                        ),
                    ),
                    ("uri", string_property("Optional URI or file path.")),
                    (
                        "content",
                        string_property("Optional reference summary or pasted note."),
                    ),
                ],
                &["scopeType", "scopeId", "title"],
            ),
        ),
        first_class_tool(
            "catalog.references.delete",
            "Delete Catalog Reference",
            "Delete a scoped catalog reference.",
            object_schema(vec![("id", string_property("The reference id."))], &["id"]),
        ),
        first_class_tool(
            "catalog.bulk_import.schema",
            "Get Bulk Import Schema",
            "Return the expected JSON and CSV structures for bulk importing products, product areas, capabilities, features, work items, and tasks.",
            empty_object_schema(),
        ),
        first_class_tool(
            "catalog.bulk_import.submit",
            "Submit Bulk Import",
            "Start an asynchronous JSON or CSV bulk import job from a local file path.",
            object_schema(
                vec![
                    ("filePath", string_property("JSON or CSV file path to import.")),
                    (
                        "format",
                        enum_property(
                            "Optional file format; inferred from extension when omitted.",
                            &["json", "csv"],
                        ),
                    ),
                    (
                        "productId",
                        string_property(
                            "Optional existing product id used when the file does not define a product.",
                        ),
                    ),
                ],
                &["filePath"],
            ),
        ),
        first_class_tool(
            "catalog.bulk_import.get_status",
            "Get Bulk Import Status",
            "Get durable status, progress counts, and recent errors for a bulk import job.",
            object_schema(
                vec![("jobId", string_property("Bulk import job id."))],
                &["jobId"],
            ),
        ),
        first_class_tool(
            "catalog.bulk_import.list_jobs",
            "List Bulk Import Jobs",
            "List recent bulk import jobs.",
            object_schema(
                vec![(
                    "limit",
                    integer_property("Maximum jobs to return, capped at 100."),
                )],
                &[],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.list",
            "List Product Areas",
            "List product areas for a product. Product areas are the top-level product-management boundaries in the canonical Product Area > Capability > Feature hierarchy.",
            object_schema(
                vec![("productId", string_property("The product id."))],
                &["productId"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.create",
            "Create Product Area",
            "Create a top-level product area. Product areas must use nodeKind=product_area; see aruvi://catalog/node-kind-constraints.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    ("name", string_property("The product area name.")),
                    ("description", string_property("Short product area description.")),
                    ("purpose", string_property("Product area purpose or summary.")),
                    ("explanation", string_property("Long-form area explanation.")),
                    ("examples", string_property("Worked examples or concrete scenarios.")),
                    (
                        "implementationNotes",
                        string_property("Implementation-oriented notes for the product area."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test guidance or validation notes for the product area."),
                    ),
                    (
                        "nodeKind",
                        enum_property(
                            "Canonical node kind for the product area.",
                            &["product_area"],
                        ),
                    ),
                ],
                &["productId", "name"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.update",
            "Update Product Area",
            "Update an existing top-level product area.",
            object_schema(
                vec![
                    ("id", string_property("The product area id.")),
                    ("name", string_property("Updated product area name.")),
                    ("description", string_property("Updated description.")),
                    ("purpose", string_property("Updated purpose.")),
                    ("explanation", string_property("Updated area explanation.")),
                    ("examples", string_property("Updated worked examples.")),
                    (
                        "implementationNotes",
                        string_property("Updated implementation-oriented notes."),
                    ),
                    ("testGuidance", string_property("Updated test guidance.")),
                    (
                        "nodeKind",
                        enum_property(
                            "Updated storage node kind for the product area.",
                            &["product_area"],
                        ),
                    ),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.delete",
            "Delete Product Area",
            "Delete a product area.",
            object_schema(
                vec![("id", string_property("The product area id."))],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.product_areas.reorder",
            "Reorder Product Areas",
            "Reorder product areas within a product.",
            object_schema(
                vec![
                    ("productId", string_property("The product id.")),
                    (
                        "orderedIds",
                        string_array_property("Product area ids in the desired order."),
                    ),
                ],
                &["productId", "orderedIds"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.list",
            "List Capabilities",
            "List capabilities and features for a product area.",
            object_schema(
                vec![("productAreaId", string_property("The product area id."))],
                &["productAreaId"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.create",
            "Create Capability",
            "Create a capability or feature inside the product management hierarchy. Feature is the product-management leaf; stories and tasks live in work items.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    (
                        "parentCapabilityId",
                        string_property("Optional parent capability id."),
                    ),
                    ("name", string_property("The child node name.")),
                    ("description", string_property("Short node description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Acceptance criteria for the node."),
                    ),
                    (
                        "explanation",
                        string_property("Long-form explanation for the node."),
                    ),
                    ("examples", string_property("Worked examples for the node.")),
                    (
                        "priority",
                        enum_property("Priority level.", &["critical", "high", "medium", "low"]),
                    ),
                    ("risk", enum_property("Risk level.", &["high", "medium", "low"])),
                    ("technicalNotes", string_property("Technical notes for the node.")),
                    (
                        "implementationNotes",
                        string_property("Implementation plan or engineering notes."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test strategy or verification notes."),
                    ),
                    (
                        "nodeKind",
                        enum_property("Semantic node kind.", &["capability", "feature"]),
                    ),
                ],
                &["productAreaId", "name"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.update",
            "Update Capability",
            "Update a capability or feature.",
            object_schema(
                vec![
                    ("id", string_property("The capability id.")),
                    ("name", string_property("Updated node name.")),
                    ("description", string_property("Updated description.")),
                    (
                        "acceptanceCriteria",
                        string_property("Updated acceptance criteria."),
                    ),
                    (
                        "explanation",
                        string_property("Updated long-form explanation."),
                    ),
                    ("examples", string_property("Updated worked examples.")),
                    (
                        "priority",
                        enum_property(
                            "Updated priority level.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    (
                        "risk",
                        enum_property("Updated risk level.", &["high", "medium", "low"]),
                    ),
                    (
                        "technicalNotes",
                        string_property("Updated technical notes."),
                    ),
                    (
                        "implementationNotes",
                        string_property("Updated implementation plan or engineering notes."),
                    ),
                    (
                        "testGuidance",
                        string_property("Updated test strategy or verification notes."),
                    ),
                    (
                        "nodeKind",
                        enum_property("Updated semantic node kind.", &["capability", "feature"]),
                    ),
                ],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.delete",
            "Delete Capability",
            "Delete a product design child node.",
            object_schema(
                vec![("id", string_property("The capability id."))],
                &["id"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.reorder",
            "Reorder Capabilities",
            "Reorder capabilities or features under a product design scope.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    (
                        "parentCapabilityId",
                        string_property("Optional parent capability id."),
                    ),
                    (
                        "orderedIds",
                        string_array_property("Child capability ids in the desired order."),
                    ),
                ],
                &["productAreaId", "orderedIds"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.apply_template",
            "Apply Capability Template",
            "Create a supported book-shaped subtree under a product area or capability. Use this for topics such as operator chapters with definition, examples, implementation, and tests.",
            object_schema(
                vec![
                    ("productAreaId", string_property("The product area id.")),
                    (
                        "parentCapabilityId",
                        string_property("Optional parent capability id."),
                    ),
                    (
                        "templateKind",
                        enum_property(
                            "Template kind to apply.",
                            &["operator_chapter", "technical_topic_book"],
                        ),
                    ),
                    ("name", string_property("Topic name for the generated subtree.")),
                    ("description", string_property("Optional chapter description.")),
                    ("explanation", string_property("Long-form explanation content.")),
                    ("examples", string_property("Worked examples content.")),
                    (
                        "implementationNotes",
                        string_property("Implementation guidance for the generated subtree."),
                    ),
                    (
                        "testGuidance",
                        string_property("Test guidance for the generated subtree."),
                    ),
                    (
                        "priority",
                        enum_property(
                            "Priority level applied to generated nodes and work items.",
                            &["critical", "high", "medium", "low"],
                        ),
                    ),
                    ("risk", enum_property("Risk level.", &["high", "medium", "low"])),
                ],
                &["productAreaId", "templateKind", "name"],
            ),
        ),
        first_class_tool(
            "catalog.capabilities.convert_kind",
            "Convert Capability Kind",
            "Safely convert a product design node between capability and feature. Use childStrategy=reparent_to_parent when converting a structural node into a feature while preserving children.",
            object_schema(
                vec![
                    ("id", string_property("The capability id.")),
                    (
                        "nodeKind",
                        enum_property("Target semantic node kind.", &["capability", "feature"]),
                    ),
                    (
                        "childStrategy",
                        enum_property(
                            "How to handle existing structural children during conversion.",
                            &["reject", "reparent_to_parent"],
                        ),
                    ),
                ],
                &["id", "nodeKind"],
            ),
        ),
    ]
}

fn work_item_definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "work_items.list",
            "List Work Items",
            "List delivery stories and tasks filtered by product, product area, feature, source scope, or status.",
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
                        integer_property("Maximum rows to return, capped at 2000."),
                    ),
                    ("offset", integer_property("Pagination offset.")),
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

fn agent_work_definitions() -> Vec<ToolDefinition> {
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

fn repository_definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "repositories.list",
            "List Repositories",
            "List registered repositories.",
            empty_object_schema(),
        ),
        first_class_tool(
            "repositories.register",
            "Register Repository",
            "Register a repository with Aruvi.",
            object_schema(
                vec![
                    ("name", string_property("Repository display name.")),
                    ("localPath", string_property("Absolute local repository path.")),
                    ("remoteUrl", string_property("Optional remote url.")),
                    ("defaultBranch", string_property("Default branch name.")),
                ],
                &["name", "localPath"],
            ),
        ),
        first_class_tool(
            "repositories.delete",
            "Delete Repository",
            "Delete a registered repository.",
            object_schema(vec![("id", string_property("The repository id."))], &["id"]),
        ),
        first_class_tool(
            "repositories.attachments.create",
            "Attach Repository",
            "Attach a repository to a product or product area scope.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property("Attachment scope type.", &["product", "product_area"]),
                    ),
                    ("scopeId", string_property("Scope id to attach to.")),
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "isDefault",
                        boolean_property("Whether the attachment is the default."),
                    ),
                ],
                &["scopeType", "scopeId", "repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_work_item",
            "Resolve Repository For Work Item",
            "Resolve the repository associated with a work item.",
            object_schema(
                vec![("workItemId", string_property("The work item id."))],
                &["workItemId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_scope",
            "Resolve Repository For Scope",
            "Resolve the repository associated with a product or product area scope.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.workspaces.create_for_scope",
            "Create Local Workspace",
            "Create a local workspace for a product, product area, or work item scope.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("workItemId", string_property("Optional work item id.")),
                    (
                        "preferredPath",
                        string_property("Optional preferred workspace path."),
                    ),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.trees.list",
            "List Repository Tree",
            "List the file tree for a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("includeHidden", boolean_property("Whether to include hidden files.")),
                    ("maxDepth", integer_property("Optional maximum traversal depth.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.files.read",
            "Read Repository File",
            "Read a file from a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.write",
            "Write Repository File",
            "Write a file in a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                    ("content", string_property("New file content.")),
                ],
                &["repositoryId", "relativePath", "content"],
            ),
        ),
        first_class_tool(
            "repositories.files.get_sha256",
            "Get Repository File SHA256",
            "Get the SHA256 of a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.apply_patch",
            "Apply Repository Patch",
            "Apply a patch to a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                    ("patch", string_property("Unified patch text to apply.")),
                    (
                        "baseSha256",
                        string_property("Optional expected base SHA256 for optimistic locking."),
                    ),
                ],
                &["repositoryId", "relativePath", "patch"],
            ),
        ),
        first_class_tool(
            "repositories.git.status",
            "Get Repository Git Status",
            "Get branch, head SHA, dirty flag, and changed file status for a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "includeIgnored",
                        boolean_property("Whether to include ignored files."),
                    ),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.diff",
            "Get Repository Git Diff",
            "Get a patch diff for working tree changes in a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("maxBytes", integer_property("Maximum diff bytes returned.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.changed_files",
            "List Repository Git Changed Files",
            "List changed files in a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.current_branch",
            "Get Repository Git Current Branch",
            "Get the current branch for a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
    ]
}
