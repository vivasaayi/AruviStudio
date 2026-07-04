use crate::services::bulk_import_builder::prepare_json_import;
use crate::services::bulk_import_csv::{csv_field, parse_csv_records};
use serde_json::json;

#[test]
fn csv_parser_handles_quoted_commas() {
    let records = parse_csv_records(
        "recordType,id,name,description\nproduct,p1,\"Payments, Platform\",\"Line one\"\n",
    )
    .expect("csv parses");
    assert_eq!(records.len(), 1);
    assert_eq!(
        csv_field(&records[0], &["record_type"]).as_deref(),
        Some("product")
    );
    assert_eq!(
        csv_field(&records[0], &["name"]).as_deref(),
        Some("Payments, Platform")
    );
}

#[test]
fn json_import_prepares_nested_work_items() {
    let content = serde_json::to_string(&json!({
        "product": { "id": "p1", "name": "Product" },
        "productAreas": [{
            "id": "a1",
            "name": "Area",
            "capabilities": [{
                "id": "c1",
                "name": "Capability",
                "features": [{
                    "id": "f1",
                    "name": "Feature",
                    "workItems": [{
                        "id": "w1",
                        "title": "Story",
                        "tasks": [{ "id": "t1", "title": "Task" }]
                    }]
                }]
            }]
        }]
    }))
    .expect("json");
    let prepared = prepare_json_import(&content, None).expect("prepare import");
    assert_eq!(prepared.rows.product_areas.len(), 1);
    assert_eq!(
        prepared
            .rows
            .capabilities
            .iter()
            .filter(|row| row.node_kind == "capability")
            .count(),
        1
    );
    assert_eq!(
        prepared
            .rows
            .capabilities
            .iter()
            .filter(|row| row.node_kind == "feature")
            .count(),
        1
    );
    assert_eq!(prepared.rows.work_items.len(), 2);
    assert_eq!(
        prepared.rows.work_items[1].parent_work_item_id.as_deref(),
        Some("w1")
    );
}
