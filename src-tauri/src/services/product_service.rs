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
    module: ExampleModuleSpec,
}

struct ExampleModuleSpec {
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

async fn seed_example_product(
    pool: &SqlitePool,
    product: &ExampleProductSpec,
) -> Result<(), AppError> {
    if !record_exists(pool, "products", product.id).await? {
        info!(product_id = %product.id, product_name = %product.name, "Seeding example product");
        product_repo::create_product(
            pool,
            product.id,
            product.name,
            product.description,
            product.vision,
            &serde_json::to_string(product.goals).unwrap_or_else(|_| "[]".to_string()),
            &serde_json::to_string(&build_product_tags(product.tags))
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .await?;
    }

    let module = &product.module;
    if !record_exists(pool, "modules", module.id).await? {
        product_repo::create_module(
            pool,
            module.id,
            product.id,
            module.name,
            module.description,
            module.purpose,
        )
        .await?;
    }

    let bootstrap_work_item_id = format!("{}-bootstrap-local-repo", product.id);
    if !record_exists(pool, "work_items", &bootstrap_work_item_id).await? {
        work_item_repo::create_work_item(
            pool,
            &bootstrap_work_item_id,
            product.id,
            Some(module.id),
            None,
            None,
            "Initialize local repository and test scaffold",
            &format!(
                "{} needs a local repository, git history, and starter test structure before delivery work should begin.",
                product.name
            ),
            "Create or attach the local repository for this seeded example, initialize git if needed, add a minimal README, .gitignore, and tests folder, then attach that repository to product or module scope so downstream work items inherit it.",
            "A local repository is attached to the seeded product, git is initialized, the default branch exists, a starter tests folder is present, and downstream work items can resolve the repository automatically.",
            "Do not implement feature outcomes in this bootstrap work item. This step is only for repository and test scaffold readiness.",
            "setup",
            "high",
            "low",
        )
        .await?;
    }

    for capability in module.capabilities {
        seed_example_capability(pool, product, module, capability).await?;
    }

    Ok(())
}

async fn seed_example_capability(
    pool: &SqlitePool,
    product: &ExampleProductSpec,
    module: &ExampleModuleSpec,
    capability: &ExampleCapabilitySpec,
) -> Result<(), AppError> {
    if !record_exists(pool, "capabilities", capability.id).await? {
        product_repo::create_capability(
            pool,
            capability.id,
            module.id,
            None,
            capability.name,
            &format!("{} capability for {}.", capability.name, product.name),
            &format!(
                "{} ships these outcomes end-to-end: {}.",
                capability.name,
                capability.outcomes.join(", ")
            ),
            capability.priority,
            capability.risk,
            capability.technical_notes,
        )
        .await?;
    }

    for outcome_name in capability.outcomes {
        let outcome_id = format!("{}-{}", capability.id, slugify(outcome_name));
        let work_item_id = format!("{}-ship", outcome_id);

        if !record_exists(pool, "capabilities", &outcome_id).await? {
            product_repo::create_capability(
                pool,
                &outcome_id,
                module.id,
                Some(capability.id),
                outcome_name,
                &format!(
                    "Deliver the {} outcome under {} for {}.",
                    outcome_name, capability.name, product.name
                ),
                &format!(
                    "{} behaves correctly, keeps UI state coherent, and is ready for validation.",
                    outcome_name
                ),
                capability.priority,
                capability.risk,
                &format!(
                    "Outcome belongs to the {} example product seed and should be delivered incrementally.",
                    product.name
                ),
            )
            .await?;
        }

        if !record_exists(pool, "work_items", &work_item_id).await? {
            work_item_repo::create_work_item(
                pool,
                &work_item_id,
                product.id,
                Some(module.id),
                Some(&outcome_id),
                None,
                &format!("Ship {}", outcome_name),
                &format!(
                    "{} is defined in the {} product but not yet implemented end-to-end.",
                    outcome_name, product.name
                ),
                &format!(
                    "Implement the {} outcome for {} inside the {} capability.",
                    outcome_name, product.name, capability.name
                ),
                "Implementation, unit tests, integration tests, and UI validation all pass. The user can inspect the change in the IDE and workflow artifacts.",
                "Keep the implementation scoped to the seeded example product. Preserve existing behavior and leave artifacts ready for human review.",
                "feature",
                capability.priority,
                "medium",
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
        .replace('/', "-")
        .replace(' ', "-")
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
            module: ExampleModuleSpec {
                id: "example-module-calculator-core",
                name: "Calculator Core",
                description: "Outcome-driven delivery module for calculator behavior and test coverage.",
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
            module: ExampleModuleSpec {
                id: "example-module-budgeting-core",
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
            module: ExampleModuleSpec {
                id: "example-module-ai-book-reader",
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
            module: ExampleModuleSpec {
                id: "example-module-kubernetes-dashboard",
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
                        outcomes: &["Deployment Detail", "Pod Explorer", "ReplicaSet Rollout Status"],
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
            module: ExampleModuleSpec {
                id: "example-module-email-client",
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
            module: ExampleModuleSpec {
                id: "example-module-kanban-board",
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
            module: ExampleModuleSpec {
                id: "example-module-recipe-planner",
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
            module: ExampleModuleSpec {
                id: "example-module-habit-tracker",
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
            module: ExampleModuleSpec {
                id: "example-module-doc-portal",
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
            module: ExampleModuleSpec {
                id: "example-module-incident-center",
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
