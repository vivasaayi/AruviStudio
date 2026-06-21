use crate::domain::product::{
    HierarchyNodeType, NodeKindConversionResult, SemanticTemplateApplicationResult,
    SemanticTemplateKind,
};
use crate::error::AppError;
use crate::persistence::{product_repo, settings_repo, work_item_repo};
use sqlx::SqlitePool;
use tracing::info;

pub const HIDE_EXAMPLE_PRODUCTS_KEY: &str = "catalog.hide_example_products";

struct ExampleProductSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    vision: &'static str,
    goals: &'static [&'static str],
    tags: &'static [&'static str],
    product_area: ExampleProductAreaSpec,
}

struct ExampleProductAreaSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    purpose: &'static str,
    capabilities: &'static [ExampleCapabilitySpec],
}

struct ExampleCapabilitySpec {
    id: &'static str,
    name: &'static str,
    outcomes: &'static [&'static str],
    priority: &'static str,
    risk: &'static str,
    technical_notes: &'static str,
}

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

fn example_product_specs() -> Vec<ExampleProductSpec> {
    vec![
        ExampleProductSpec {
            id: "example-product-calculator",
            name: "Calculator",
            description: "A staged React calculator used to pressure-test implementation, unit testing, integration testing, and UI validation agents.",
            vision: "Ship calculator outcomes one by one and verify the full autonomous delivery loop.",
            goals: &["Validate coding agents against a familiar React app", "Exercise testing agents on incremental mathematical outcomes"],
            tags: &["react", "calculator", "testing_agents"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-calculator-core",
                name: "Calculator Core",
                description: "Outcome-driven delivery product_area for calculator behavior and test coverage.",
                purpose: "Stress the workflow by implementing one calculator outcome at a time.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-calculator-simple-math",
                        name: "Simple Math",
                        outcomes: &["Addition", "Subtraction", "Multiplication", "Division"],
                        priority: "high",
                        risk: "low",
                        technical_notes: "Keep state handling explicit and easy to validate through unit and UI tests.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-calculator-scientific",
                        name: "Scientific",
                        outcomes: &["Sin", "Cos", "Tan"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Scientific functions should keep formatting and angle handling consistent.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-calculator-exponents",
                        name: "Exponents",
                        outcomes: &["Square", "Cube", "Power of X"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Avoid regressions in button sequencing and numeric precision.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-calculator-roots",
                        name: "Roots",
                        outcomes: &["Square Root", "Cube Root"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Negative and invalid inputs should surface predictable validation behavior.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-calculator-programming",
                        name: "Programming",
                        outcomes: &["ASCII", "HEX"],
                        priority: "low",
                        risk: "medium",
                        technical_notes: "Conversion outcomes should be deterministic and easy to snapshot test.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-budgeting-tool",
            name: "Household Budgeting Tool",
            description: "A personal finance workspace covering bill intake, transaction tracking, and budget reporting.",
            vision: "Help a solo user manage household money through clear flows, ledgers, and forecast views.",
            goals: &["Test forms-heavy CRUD flows", "Exercise reconciliation, reporting, and dashboard agents"],
            tags: &["react", "finance", "dashboard"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-budgeting-core",
                name: "Budget Operations",
                description: "Core household finance workflows.",
                purpose: "Model recurring bills, day-to-day transactions, and budget health.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-budgeting-bill-tracker",
                        name: "Bill Tracker",
                        outcomes: &["Add Bill", "Mark Bill Paid", "Upcoming Bills View"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Recurring dates and overdue states should be explicit in tests.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-budgeting-home-transactions",
                        name: "Home Transactions",
                        outcomes: &["Capture Expense", "Capture Income", "Category Ledger"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Ledger ordering and balance math should be covered by integration tests.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-budgeting-budget-health",
                        name: "Budget Health",
                        outcomes: &["Monthly Summary", "Cash Flow Forecast", "Budget vs Actual"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Summary tiles and chart adapters should remain presentation-friendly.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-ai-book-reader",
            name: "AI Native Book Reader",
            description: "A dynamic reading product where titles and sections are defined by the user and chapter content is generated with LLM support.",
            vision: "Generate and present book content dynamically while keeping chapter structure and reading UX coherent.",
            goals: &["Test AI-assisted content generation flows", "Validate hierarchical content rendering and reader state"],
            tags: &["ai", "reader", "content_generation"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-ai-book-reader",
                name: "Book Experience",
                description: "Authoring, generation, and reading workflows for dynamic books.",
                purpose: "Let a user define a book outline and consume generated chapters cleanly.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-book-outline",
                        name: "Book Outline",
                        outcomes: &["Define Title", "Define Sections", "Reorder Chapter Outline"],
                        priority: "high",
                        risk: "low",
                        technical_notes: "Outline changes should preserve stable identifiers for generated content.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-book-generation",
                        name: "Content Generation",
                        outcomes: &["Generate Chapter Draft", "Regenerate Section", "Persist Generated Content"],
                        priority: "high",
                        risk: "high",
                        technical_notes: "Token budgets and prompt provenance matter for reproducibility.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-book-reader",
                        name: "Reader UX",
                        outcomes: &["Chapter Navigation", "Reading Progress", "Inline AI Notes"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Reader state should survive refresh and avoid losing scroll progress.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-kubernetes-dashboard",
            name: "Kubernetes Dashboard",
            description: "A full dashboard for cluster overview, workload inspection, logs, and operational actions.",
            vision: "Give operators a concise but powerful view into clusters, workloads, and incidents.",
            goals: &["Stress dense data tables and filters", "Exercise observability and action-oriented workflows"],
            tags: &["kubernetes", "dashboard", "operations"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-kubernetes-dashboard",
                name: "Cluster Operations",
                description: "Cluster monitoring and workload management.",
                purpose: "Render operational data and enable guided actions from the same console.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-k8s-cluster-overview",
                        name: "Cluster Overview",
                        outcomes: &["Namespace Summary", "Node Health", "Resource Utilization"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Tables should support incremental refresh without losing operator context.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-k8s-workloads",
                        name: "Workload Inspection",
                        outcomes: &["Deployment Detail", "Pod Explorer", "ReplicaSet Release Status"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Hierarchical drill-down should stay fast with large lists.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-k8s-observability",
                        name: "Observability",
                        outcomes: &["Pod Logs", "Event Timeline", "Alert Surface"],
                        priority: "medium",
                        risk: "high",
                        technical_notes: "Logs and event panes should remain stream-friendly and filterable.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-email-client",
            name: "Personal Email Client",
            description: "A lightweight mail product for inbox triage, compose, search, and local organization.",
            vision: "Help a solo user stay on top of mail without the weight of a full enterprise suite.",
            goals: &["Exercise message lists, thread views, and compose workflows", "Validate search and folder state"],
            tags: &["email", "productivity", "react"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-email-client",
                name: "Mailbox Experience",
                description: "Inbox, compose, and thread management.",
                purpose: "Model a practical communication workflow with rich list/detail patterns.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-email-inbox",
                        name: "Inbox",
                        outcomes: &["Thread List", "Unread Filters", "Pinned Conversations"],
                        priority: "high",
                        risk: "low",
                        technical_notes: "Message state transitions should be easy to assert in tests.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-email-compose",
                        name: "Compose",
                        outcomes: &["Compose Draft", "Attachment Stub", "Send Flow"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Form preservation and validation errors should be explicit.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-email-search",
                        name: "Search",
                        outcomes: &["Query Inbox", "Saved Search", "Filter by Sender"],
                        priority: "medium",
                        risk: "low",
                        technical_notes: "Search UI should tolerate empty and large-result states.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-kanban-board",
            name: "Kanban Delivery Board",
            description: "A delivery planning board with lists, cards, swimlanes, and lightweight reporting.",
            vision: "Track delivery work visually while keeping planning and throughput transparent.",
            goals: &["Test drag-and-drop list behavior", "Exercise reporting from board state"],
            tags: &["kanban", "planning", "workflow"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-kanban-board",
                name: "Board Flow",
                description: "Board interactions and throughput reporting.",
                purpose: "Provide a list-based planning surface that is fast to refine.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-kanban-board-core",
                        name: "Board Core",
                        outcomes: &["Create Card", "Move Card", "Swimlane View"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Movement rules should be deterministic and event-driven.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-kanban-reporting",
                        name: "Reporting",
                        outcomes: &["Cycle Time Summary", "WIP Limits", "Delivery Snapshot"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Reporting should not depend on hidden UI-only fields.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-recipe-planner",
            name: "Recipe Planner",
            description: "A meal and recipe planning app for storing recipes, weekly plans, and shopping lists.",
            vision: "Turn recipe management into a practical weekly planning experience.",
            goals: &["Exercise nested forms and detail views", "Validate derived shopping list flows"],
            tags: &["planner", "recipes", "household"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-recipe-planner",
                name: "Meal Planning",
                description: "Recipe storage and weekly planning.",
                purpose: "Translate saved recipes into a weekly plan and ingredient list.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-recipe-library",
                        name: "Recipe Library",
                        outcomes: &["Add Recipe", "Ingredient List", "Cooking Steps"],
                        priority: "medium",
                        risk: "low",
                        technical_notes: "Structured recipe content should be easy to edit incrementally.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-recipe-weekly-plan",
                        name: "Weekly Plan",
                        outcomes: &["Plan Meal", "Daily View", "Shopping List"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Derived list generation should remain predictable across edits.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-habit-tracker",
            name: "Habit Tracker",
            description: "A habit product covering streaks, daily check-ins, and progress summaries.",
            vision: "Help a user build consistency with lightweight daily feedback loops.",
            goals: &["Test time-based state and summaries", "Exercise compact mobile-friendly workflows"],
            tags: &["habits", "tracker", "personal"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-habit-tracker",
                name: "Habit Engine",
                description: "Daily check-in and progress workflows.",
                purpose: "Support habit creation, completion logging, and streak reporting.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-habit-setup",
                        name: "Habit Setup",
                        outcomes: &["Create Habit", "Target Frequency", "Habit Categories"],
                        priority: "medium",
                        risk: "low",
                        technical_notes: "Configuration should stay simple and highly testable.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-habit-progress",
                        name: "Progress Tracking",
                        outcomes: &["Daily Check-in", "Streak View", "Completion Calendar"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Date handling should be isolated from rendering concerns.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-doc-portal",
            name: "Documentation Portal",
            description: "A docs product with navigation, search, and embedded examples.",
            vision: "Present structured technical documentation clearly and keep examples easy to discover.",
            goals: &["Exercise content tree rendering", "Validate search and detail panes"],
            tags: &["documentation", "portal", "search"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-doc-portal",
                name: "Docs Experience",
                description: "Navigation, search, and content presentation.",
                purpose: "Render structured documentation with fast lookup and readable layouts.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-doc-navigation",
                        name: "Navigation",
                        outcomes: &["Sidebar Tree", "Breadcrumbs", "Section Anchor Links"],
                        priority: "medium",
                        risk: "low",
                        technical_notes: "Navigation should stay consistent across large trees.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-doc-search",
                        name: "Docs Search",
                        outcomes: &["Search Index", "Result Highlighting", "Recent Queries"],
                        priority: "medium",
                        risk: "medium",
                        technical_notes: "Search should degrade cleanly when index content is sparse.",
                    },
                ],
            },
        },
        ExampleProductSpec {
            id: "example-product-incident-center",
            name: "Incident Command Center",
            description: "An operational product for incident timelines, responders, and remediation tracking.",
            vision: "Make incident handling visible, auditable, and faster to coordinate.",
            goals: &["Exercise high-signal dashboards and logs", "Validate approval and review workflows"],
            tags: &["incident_response", "operations", "coordination"],
            product_area: ExampleProductAreaSpec {
                id: "example-product_area-incident-center",
                name: "Incident Response",
                description: "Incident lifecycle and responder coordination.",
                purpose: "Capture incidents, coordinate responders, and track remediation to closure.",
                capabilities: &[
                    ExampleCapabilitySpec {
                        id: "example-capability-incident-intake",
                        name: "Incident Intake",
                        outcomes: &["Declare Incident", "Severity Routing", "Responder Assignment"],
                        priority: "high",
                        risk: "medium",
                        technical_notes: "Routing and severity changes should be easy to audit.",
                    },
                    ExampleCapabilitySpec {
                        id: "example-capability-incident-execution",
                        name: "Execution",
                        outcomes: &["Timeline Log", "Action Checklist", "Resolution Summary"],
                        priority: "high",
                        risk: "high",
                        technical_notes: "Timeline fidelity matters for postmortem usefulness.",
                    },
                ],
            },
        },
    ]
}
