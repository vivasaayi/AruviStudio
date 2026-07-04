use crate::domain::product::{
    HierarchyNodeType, SemanticTemplateApplicationResult, SemanticTemplateKind,
};
use crate::error::AppError;
use crate::persistence::{product_repo, work_item_repo};
use sqlx::SqlitePool;

pub(crate) struct ApplySemanticTemplateInput<'a> {
    pub(crate) product_area_id: &'a str,
    pub(crate) parent_capability_id: Option<&'a str>,
    pub(crate) template_kind: &'a str,
    pub(crate) name: &'a str,
    pub(crate) description: &'a str,
    pub(crate) priority: Option<&'a str>,
    pub(crate) risk: Option<&'a str>,
    pub(crate) explanation: &'a str,
    pub(crate) examples: &'a str,
    pub(crate) implementation_notes: &'a str,
    pub(crate) test_guidance: &'a str,
}

struct CapabilityInputSpec<'a> {
    id: &'a str,
    product_area_id: &'a str,
    parent_capability_id: Option<&'a str>,
    name: &'a str,
    description: &'a str,
    acceptance_criteria: &'a str,
    priority: &'a str,
    risk: &'a str,
    technical_notes: &'a str,
    node_kind: Option<&'a str>,
}

fn create_capability_input(
    spec: CapabilityInputSpec<'_>,
) -> product_repo::CreateCapabilityInput<'_> {
    product_repo::CreateCapabilityInput {
        id: spec.id,
        product_area_id: spec.product_area_id,
        parent_capability_id: spec.parent_capability_id,
        name: spec.name,
        description: spec.description,
        acceptance_criteria: spec.acceptance_criteria,
        priority: spec.priority,
        risk: spec.risk,
        technical_notes: spec.technical_notes,
        node_kind: spec.node_kind,
        explanation: "",
        examples: "",
        implementation_notes: "",
        test_guidance: "",
    }
}

pub async fn apply_semantic_template(
    pool: &SqlitePool,
    input: ApplySemanticTemplateInput<'_>,
) -> Result<SemanticTemplateApplicationResult, AppError> {
    let template_kind = SemanticTemplateKind::parse(input.template_kind).ok_or_else(|| {
        AppError::Validation(
            "Unsupported template_kind. Use operator_chapter or technical_topic_book.".to_string(),
        )
    })?;
    let trimmed_name = input.name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::Validation(
            "Template topic name cannot be empty.".to_string(),
        ));
    }

    let priority = input.priority.unwrap_or("medium");
    let risk = input.risk.unwrap_or("medium");
    let chapter_description = if input.description.trim().is_empty() {
        format!("{trimmed_name} book section.")
    } else {
        input.description.trim().to_string()
    };
    let topic_id = uuid::Uuid::new_v4().to_string();
    let topic_acceptance = format!(
        "{} has definition, examples, implementation guidance, and test guidance captured.",
        trimmed_name
    );
    let topic_node = product_repo::create_capability(
        pool,
        create_capability_input(CapabilityInputSpec {
            id: &topic_id,
            product_area_id: input.product_area_id,
            parent_capability_id: input.parent_capability_id,
            name: trimmed_name,
            description: &chapter_description,
            acceptance_criteria: &topic_acceptance,
            priority,
            risk,
            technical_notes: "Template-generated semantic chapter root.",
            node_kind: Some("capability"),
        }),
    )
    .await?;

    let (definition_label, examples_label, implementation_label, tests_label) = match template_kind
    {
        SemanticTemplateKind::OperatorChapter => (
            format!("{trimmed_name} Definition"),
            format!("{trimmed_name} Examples"),
            format!("{trimmed_name} Implementation"),
            format!("{trimmed_name} Tests"),
        ),
        SemanticTemplateKind::TechnicalTopicBook => (
            format!("{trimmed_name} Overview"),
            format!("{trimmed_name} Examples"),
            format!("{trimmed_name} Implementation"),
            format!("{trimmed_name} Tests"),
        ),
    };

    let definition_id = uuid::Uuid::new_v4().to_string();
    let definition_description =
        format!("Explain what {trimmed_name} is and when it should be used.");
    let definition_node = product_repo::create_capability(
        pool,
        product_repo::CreateCapabilityInput {
            explanation: input.explanation,
            ..create_capability_input(CapabilityInputSpec {
                id: &definition_id,
                product_area_id: input.product_area_id,
                parent_capability_id: Some(&topic_node.id),
                name: &definition_label,
                description: &definition_description,
                acceptance_criteria: "",
                priority,
                risk,
                technical_notes: "Feature chapter for explanation and conceptual boundaries.",
                node_kind: Some("feature"),
            })
        },
    )
    .await?;
    let examples_id = uuid::Uuid::new_v4().to_string();
    let examples_description =
        format!("Capture worked examples and expected behaviors for {trimmed_name}.");
    let examples_node = product_repo::create_capability(
        pool,
        product_repo::CreateCapabilityInput {
            examples: input.examples,
            ..create_capability_input(CapabilityInputSpec {
                id: &examples_id,
                product_area_id: input.product_area_id,
                parent_capability_id: Some(&topic_node.id),
                name: &examples_label,
                description: &examples_description,
                acceptance_criteria: "",
                priority,
                risk,
                technical_notes: "Feature chapter for examples and concrete edge cases.",
                node_kind: Some("feature"),
            })
        },
    )
    .await?;
    let implementation_id = uuid::Uuid::new_v4().to_string();
    let implementation_description = format!("Describe how {trimmed_name} should be implemented.");
    let implementation_node = product_repo::create_capability(
        pool,
        product_repo::CreateCapabilityInput {
            implementation_notes: input.implementation_notes,
            ..create_capability_input(CapabilityInputSpec {
                id: &implementation_id,
                product_area_id: input.product_area_id,
                parent_capability_id: Some(&topic_node.id),
                name: &implementation_label,
                description: &implementation_description,
                acceptance_criteria: "",
                priority,
                risk,
                technical_notes: "Feature execution notes for implementation stories.",
                node_kind: Some("feature"),
            })
        },
    )
    .await?;
    let tests_id = uuid::Uuid::new_v4().to_string();
    let tests_description = format!("Describe how {trimmed_name} should be validated.");
    let tests_node = product_repo::create_capability(
        pool,
        product_repo::CreateCapabilityInput {
            test_guidance: input.test_guidance,
            ..create_capability_input(CapabilityInputSpec {
                id: &tests_id,
                product_area_id: input.product_area_id,
                parent_capability_id: Some(&topic_node.id),
                name: &tests_label,
                description: &tests_description,
                acceptance_criteria: "",
                priority,
                risk,
                technical_notes: "Feature execution notes for test and verification stories.",
                node_kind: Some("feature"),
            })
        },
    )
    .await?;

    let product_id = resolve_product_id_for_product_area(pool, input.product_area_id).await?;
    let implementation_work_item_id = uuid::Uuid::new_v4().to_string();
    let implementation_title = format!("Implement {trimmed_name}");
    let implementation_problem =
        format!("{trimmed_name} needs implementation aligned to the authored chapter structure.");
    let implementation_acceptance = format!(
        "{trimmed_name} is implemented and matches the documented behavior, examples, and edge cases."
    );
    let implementation_work_item = work_item_repo::create_work_item(
        pool,
        work_item_repo::CreateWorkItemInput {
            id: &implementation_work_item_id,
            product_id: &product_id,
            product_area_id: Some(input.product_area_id),
            capability_id: Some(&implementation_node.id),
            source_node_id: Some(&implementation_node.id),
            source_node_type: Some("capability"),
            parent_work_item_id: None,
            title: &implementation_title,
            problem_statement: &implementation_problem,
            description: input.implementation_notes,
            acceptance_criteria: &implementation_acceptance,
            constraints:
                "Preserve the authored semantic structure and keep behavior deterministic.",
            work_item_type: "story",
            priority,
            complexity: "medium",
        },
    )
    .await?;
    let test_work_item_id = uuid::Uuid::new_v4().to_string();
    let test_title = format!("Write {trimmed_name} test cases");
    let test_problem = format!(
        "{trimmed_name} needs verification that matches the documented examples and risks."
    );
    let test_acceptance =
        format!("Coverage validates happy paths, edge cases, and regressions for {trimmed_name}.");
    let test_work_item = work_item_repo::create_work_item(
        pool,
        work_item_repo::CreateWorkItemInput {
            id: &test_work_item_id,
            product_id: &product_id,
            product_area_id: Some(input.product_area_id),
            capability_id: Some(&tests_node.id),
            source_node_id: Some(&tests_node.id),
            source_node_type: Some("capability"),
            parent_work_item_id: None,
            title: &test_title,
            problem_statement: &test_problem,
            description: input.test_guidance,
            acceptance_criteria: &test_acceptance,
            constraints: "Keep tests aligned with the authored examples and implementation notes.",
            work_item_type: "test",
            priority,
            complexity: "medium",
        },
    )
    .await?;

    Ok(SemanticTemplateApplicationResult {
        template_kind,
        parent_node_id: input
            .parent_capability_id
            .map(ToString::to_string)
            .unwrap_or_else(|| input.product_area_id.to_string()),
        parent_node_type: if input.parent_capability_id.is_some() {
            HierarchyNodeType::Capability
        } else {
            HierarchyNodeType::ProductArea
        },
        topic_node,
        created_nodes: vec![
            definition_node,
            examples_node,
            implementation_node,
            tests_node,
        ],
        created_work_items: vec![implementation_work_item, test_work_item],
    })
}

async fn resolve_product_id_for_product_area(
    pool: &SqlitePool,
    product_area_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar("SELECT product_id FROM product_areas WHERE id=?")
        .bind(product_area_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product Area {product_area_id} not found")))
}
