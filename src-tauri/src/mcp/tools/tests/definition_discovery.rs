use super::*;

#[test]
fn definitions_include_first_class_tools_after_legacy_tools() {
    let definitions = definitions();
    let legacy_index = definitions
        .iter()
        .position(|tool| tool.name == "aruvi_catalog")
        .expect("legacy aruvi_catalog tool");
    let first_class_index = definitions
        .iter()
        .position(|tool| tool.name == "catalog.products.get_tree")
        .expect("catalog.products.get_tree");
    let first_class_tool = &definitions[first_class_index];

    assert!(legacy_index < first_class_index);
    assert_eq!(first_class_tool.title.as_deref(), Some("Get Product Tree"));
    assert_eq!(
        first_class_tool
            .input_schema
            .get("additionalProperties")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn discovery_exposes_node_kind_constraints_in_catalog_tool_schemas() {
    let definitions = definitions();
    let product_area_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.product_areas.create")
        .expect("catalog.product_areas.create");
    let capability_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.capabilities.create")
        .expect("catalog.capabilities.create");

    let root_kind_enum = product_area_create_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("nodeKind"))
        .and_then(Value::as_object)
        .and_then(|node_kind| node_kind.get("enum"))
        .and_then(Value::as_array)
        .expect("product area nodeKind enum");

    assert_eq!(root_kind_enum, &vec![json!("product_area")]);
    assert!(product_area_create_tool
        .description
        .contains("aruvi://catalog/node-kind-constraints"));
    assert!(capability_create_tool
        .description
        .contains("Feature is the product-management leaf"));
}

#[test]
fn discovery_exposes_catalog_reference_tools() {
    let definitions = definitions();
    let reference_create_tool = definitions
        .iter()
        .find(|tool| tool.name == "catalog.references.create")
        .expect("catalog.references.create");

    let scope_type_enum = reference_create_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("scopeType"))
        .and_then(Value::as_object)
        .and_then(|scope_type| scope_type.get("enum"))
        .and_then(Value::as_array)
        .expect("reference scopeType enum");

    assert!(scope_type_enum.contains(&json!("product_area")));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.references.list"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.references.delete"));
}

#[test]
fn discovery_exposes_bulk_import_tools() {
    let definitions = definitions();
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.schema"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.submit"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.get_status"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "catalog.bulk_import.list_jobs"));
}

#[test]
fn discovery_exposes_agent_work_coordination_tools() {
    let definitions = definitions();
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "aruvi_agent_work"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.claim_next"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.heartbeat"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.items.requeue_expired"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.dependencies.upsert"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.evidence.append"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.commits.link"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.materialize_catalog"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.link_catalog_work_items"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.runs.summary"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.runs.health"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.context.get_feature"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "agent_work.context.export_feature"));
    assert!(definitions
        .iter()
        .any(|tool| tool.name == "repositories.git.status"));
}

#[test]
fn discovery_exposes_work_item_pagination_metadata_option() {
    let definitions = definitions();
    let list_tool = definitions
        .iter()
        .find(|tool| tool.name == "work_items.list")
        .expect("work_items.list tool");

    assert!(list_tool.description.contains("includePagination=true"));
    let properties = list_tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("properties object");
    assert!(properties.contains_key("includePagination"));
}
