use crate::domain::product::NodeKindConversionResult;
use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo, work_item_repo};
use crate::services::product_example_catalog::{
    example_product_specs, ExampleCapabilitySpec, ExampleProductAreaSpec, ExampleProductSpec,
};
pub use crate::services::product_semantic_template::apply_semantic_template;
pub(crate) use crate::services::product_semantic_template::ApplySemanticTemplateInput;
use sqlx::SqlitePool;
use tracing::info;

pub const HIDE_EXAMPLE_PRODUCTS_KEY: &str = "catalog.hide_example_products";

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

pub async fn initialize_example_catalog(pool: &SqlitePool) -> Result<(), AppError> {
    if settings_repo::get_setting(pool, HIDE_EXAMPLE_PRODUCTS_KEY)
        .await?
        .is_none()
    {
        settings_repo::set_setting(pool, HIDE_EXAMPLE_PRODUCTS_KEY, "true").await?;
    }

    for product in example_product_specs().iter() {
        seed_example_product(pool, product).await?;
    }

    Ok(())
}

pub async fn convert_capability_kind(
    pool: &SqlitePool,
    capability_id: &str,
    node_kind: &str,
    child_strategy: Option<&str>,
) -> Result<NodeKindConversionResult, AppError> {
    product_repo::convert_capability_node_kind(pool, capability_id, node_kind, child_strategy).await
}

async fn seed_example_product(
    pool: &SqlitePool,
    product: &ExampleProductSpec,
) -> Result<(), AppError> {
    if !record_exists(pool, "products", product.id).await? {
        info!(product_id = %product.id, product_name = %product.name, "Seeding example product");
        let goals = serde_json::to_string(product.goals).unwrap_or_else(|_| "[]".to_string());
        let tags = serde_json::to_string(&build_product_tags(product.tags))
            .unwrap_or_else(|_| "[]".to_string());
        product_repo::create_product(
            pool,
            product_repo::CreateProductInput {
                id: product.id,
                name: product.name,
                description: product.description,
                vision: product.vision,
                goals: &goals,
                tags: &tags,
                lifecycle: Some("active"),
                health: Some("healthy"),
                owner_label: Some("Founder"),
                investment_status: Some("maintain"),
                roadmap: None,
                evidence: None,
            },
        )
        .await?;
    }

    let product_area = &product.product_area;
    if !record_exists(pool, "product_areas", product_area.id).await? {
        product_repo::create_product_area(
            pool,
            product_repo::CreateProductAreaInput {
                id: product_area.id,
                product_id: product.id,
                name: product_area.name,
                description: product_area.description,
                purpose: product_area.purpose,
                node_kind: None,
                explanation: "",
                examples: "",
                implementation_notes: "",
                test_guidance: "",
            },
        )
        .await?;
    }

    let bootstrap_work_item_id = format!("{}-bootstrap-local-repo", product.id);
    if !record_exists(pool, "work_items", &bootstrap_work_item_id).await? {
        let bootstrap_problem = format!(
            "{} needs a local repository, git history, and starter test structure before delivery work should begin.",
            product.name
        );
        work_item_repo::create_work_item(
            pool,
            work_item_repo::CreateWorkItemInput {
                id: &bootstrap_work_item_id,
                product_id: product.id,
                product_area_id: Some(product_area.id),
                capability_id: None,
                source_node_id: None,
                source_node_type: None,
                parent_work_item_id: None,
                title: "Initialize local repository and test scaffold",
                problem_statement: &bootstrap_problem,
                description: "Create or attach the local repository for this seeded example, initialize git if needed, add a minimal README, .gitignore, and tests folder, then attach that repository to product or product area scope so downstream work items inherit it.",
                acceptance_criteria: "A local repository is attached to the seeded product, git is initialized, the default branch exists, a starter tests folder is present, and downstream work items can resolve the repository automatically.",
                constraints: "Do not implement feature outcomes in this bootstrap work item. This step is only for repository and test scaffold readiness.",
                work_item_type: "setup",
                priority: "high",
                complexity: "low",
            },
        )
        .await?;
    }

    for capability in product_area.capabilities {
        seed_example_capability(pool, product, product_area, capability).await?;
    }

    Ok(())
}

async fn seed_example_capability(
    pool: &SqlitePool,
    product: &ExampleProductSpec,
    product_area: &ExampleProductAreaSpec,
    capability: &ExampleCapabilitySpec,
) -> Result<(), AppError> {
    if !record_exists(pool, "capabilities", capability.id).await? {
        let description = format!("{} capability for {}.", capability.name, product.name);
        let acceptance_criteria = format!(
            "{} ships these outcomes end-to-end: {}.",
            capability.name,
            capability.outcomes.join(", ")
        );
        product_repo::create_capability(
            pool,
            create_capability_input(CapabilityInputSpec {
                id: capability.id,
                product_area_id: product_area.id,
                parent_capability_id: None,
                name: capability.name,
                description: &description,
                acceptance_criteria: &acceptance_criteria,
                priority: capability.priority,
                risk: capability.risk,
                technical_notes: capability.technical_notes,
                node_kind: None,
            }),
        )
        .await?;
    }

    for outcome_name in capability.outcomes {
        let outcome_id = format!("{}-{}", capability.id, slugify(outcome_name));
        let work_item_id = format!("{}-ship", outcome_id);

        if !record_exists(pool, "capabilities", &outcome_id).await? {
            let description = format!(
                "Deliver the {} outcome under {} for {}.",
                outcome_name, capability.name, product.name
            );
            let acceptance_criteria = format!(
                "{} behaves correctly, keeps UI state coherent, and is ready for validation.",
                outcome_name
            );
            let technical_notes = format!(
                "Outcome belongs to the {} example product seed and should be delivered incrementally.",
                product.name
            );
            product_repo::create_capability(
                pool,
                create_capability_input(CapabilityInputSpec {
                    id: &outcome_id,
                    product_area_id: product_area.id,
                    parent_capability_id: Some(capability.id),
                    name: outcome_name,
                    description: &description,
                    acceptance_criteria: &acceptance_criteria,
                    priority: capability.priority,
                    risk: capability.risk,
                    technical_notes: &technical_notes,
                    node_kind: None,
                }),
            )
            .await?;
        }

        if !record_exists(pool, "work_items", &work_item_id).await? {
            let title = format!("Ship {}", outcome_name);
            let problem_statement = format!(
                "{} is defined in the {} product but not yet implemented end-to-end.",
                outcome_name, product.name
            );
            let description = format!(
                "Implement the {} outcome for {} inside the {} capability.",
                outcome_name, product.name, capability.name
            );
            work_item_repo::create_work_item(
                pool,
                work_item_repo::CreateWorkItemInput {
                    id: &work_item_id,
                    product_id: product.id,
                    product_area_id: Some(product_area.id),
                    capability_id: Some(&outcome_id),
                    source_node_id: None,
                    source_node_type: None,
                    parent_work_item_id: None,
                    title: &title,
                    problem_statement: &problem_statement,
                    description: &description,
                    acceptance_criteria: "Implementation, unit tests, integration tests, and UI validation all pass. The user can inspect the change in the IDE and workflow artifacts.",
                    constraints: "Keep the implementation scoped to the seeded example product. Preserve existing behavior and leave artifacts ready for human review.",
                    work_item_type: "story",
                    priority: capability.priority,
                    complexity: "medium",
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn record_exists(pool: &SqlitePool, table: &str, id: &str) -> Result<bool, AppError> {
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?)");
    let exists: i64 = sqlx::query_scalar(&query).bind(id).fetch_one(pool).await?;
    Ok(exists != 0)
}

fn build_product_tags(tags: &[&str]) -> Vec<String> {
    let mut all_tags = vec!["example_product".to_string(), "seeded_catalog".to_string()];
    all_tags.extend(tags.iter().map(|tag| tag.to_string()));
    all_tags
}

fn slugify(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace('&', "and")
        .replace(['/', ' '], "-")
}
