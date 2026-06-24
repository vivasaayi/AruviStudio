use super::super::super::definitions::{
    enum_property, first_class_tool, object_schema, string_array_property, string_property,
    ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
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
