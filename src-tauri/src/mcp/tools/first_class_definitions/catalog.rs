use super::super::definitions::{
    empty_object_schema, enum_property, first_class_tool, integer_property, object_schema,
    string_array_property, string_property, ToolDefinition,
};

mod product_design;

pub(super) fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = vec![
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
    ];
    definitions.extend(product_design::definitions());
    definitions
}
