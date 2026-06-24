use serde_json::{json, Value};

pub fn bulk_import_schema() -> Value {
    json!({
        "formats": ["json", "csv"],
        "json": {
            "description": "Canonical nested import format. Product Areas contain capabilities, capabilities contain features, and workItems contain stories/tasks.",
            "requiredTopLevel": ["product or submit productId", "productAreas"],
            "shape": {
                "product": {
                    "id": "optional stable product id",
                    "name": "required when creating a product",
                    "description": "optional",
                    "vision": "optional",
                    "goals": ["optional goal"],
                    "tags": ["optional tag"],
                    "lifecycle": "idea|incubating|active|maturing|sunsetting|retired",
                    "health": "unknown|healthy|watch|at_risk|blocked",
                    "ownerLabel": "optional",
                    "investmentStatus": "evaluate|invest|maintain|pause|retire",
                    "roadmap": "optional",
                    "evidence": "optional"
                },
                "productAreas": [{
                    "id": "optional stable area id",
                    "name": "required",
                    "description": "optional",
                    "purpose": "optional",
                    "capabilities": [{
                        "id": "optional stable capability id",
                        "name": "required",
                        "description": "optional",
                        "acceptanceCriteria": "optional",
                        "priority": "critical|high|medium|low",
                        "risk": "high|medium|low",
                        "features": [{
                            "id": "optional stable feature id",
                            "name": "required",
                            "description": "optional",
                            "workItems": [{
                                "id": "optional stable work item id",
                                "title": "required",
                                "workItemType": "story|task|setup|bug|refactor|test|review|security_fix|performance_improvement",
                                "priority": "critical|high|medium|low",
                                "complexity": "trivial|low|medium|high|very_high",
                                "tasks": [{
                                    "title": "required child task title",
                                    "workItemType": "task"
                                }]
                            }]
                        }]
                    }]
                }]
            },
            "example": {
                "product": {
                    "id": "payments-platform",
                    "name": "Payments Platform",
                    "goals": ["Reduce payment failure rate"]
                },
                "productAreas": [{
                    "id": "payments-checkout",
                    "name": "Checkout",
                    "capabilities": [{
                        "id": "cap-card-payments",
                        "name": "Card Payments",
                        "features": [{
                            "id": "feat-3ds",
                            "name": "3DS Challenge Flow",
                            "workItems": [{
                                "id": "story-3ds-browser",
                                "title": "Implement browser challenge handoff",
                                "workItemType": "story",
                                "tasks": [{
                                    "id": "task-3ds-tests",
                                    "title": "Add challenge handoff tests",
                                    "workItemType": "task"
                                }]
                            }]
                        }]
                    }]
                }]
            }
        },
        "csv": {
            "description": "Flat CSV format. Parent records must appear before child records. Use stable ids for parent references.",
            "columns": [
                "record_type",
                "id",
                "parent_id",
                "product_id",
                "product_area_id",
                "capability_id",
                "feature_id",
                "parent_work_item_id",
                "name",
                "title",
                "description",
                "problem_statement",
                "acceptance_criteria",
                "constraints",
                "priority",
                "risk",
                "complexity",
                "work_item_type",
                "status"
            ],
            "recordTypes": ["product", "product_area", "capability", "feature", "work_item", "story", "task"],
            "exampleRows": [
                "record_type,id,parent_id,product_id,product_area_id,capability_id,feature_id,parent_work_item_id,name,title,description,problem_statement,acceptance_criteria,constraints,priority,risk,complexity,work_item_type,status",
                "product,payments-platform,,,,,,,Payments Platform,,,,,,,,,",
                "product_area,payments-checkout,,payments-platform,,,,,Checkout,,,,,,,,,",
                "capability,cap-card-payments,payments-checkout,payments-platform,payments-checkout,,,,Card Payments,,,,,high,medium,,,",
                "feature,feat-3ds,cap-card-payments,payments-platform,payments-checkout,cap-card-payments,,,3DS Challenge Flow,,,,,high,medium,,,",
                "story,story-3ds-browser,,payments-platform,payments-checkout,,feat-3ds,,,Implement browser challenge handoff,,,,high,,medium,story,draft",
                "task,task-3ds-tests,story-3ds-browser,payments-platform,,,,story-3ds-browser,,Add challenge handoff tests,,,,medium,,low,task,draft"
            ]
        },
        "submitTool": {
            "name": "catalog.bulk_import.submit",
            "arguments": {
                "filePath": "absolute or process-relative JSON/CSV file path",
                "format": "optional json|csv, inferred from extension when omitted",
                "productId": "optional existing product id when the file does not define product"
            }
        },
        "statusTool": {
            "name": "catalog.bulk_import.get_status",
            "arguments": {
                "jobId": "bulk import job id returned by submit"
            }
        }
    })
}
