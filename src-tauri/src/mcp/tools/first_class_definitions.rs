use super::definitions::ToolDefinition;

mod agent_work;
mod agent_work_catalog;
mod agent_work_items;
mod catalog;
mod repositories;
mod work_items;

pub(super) fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = Vec::new();
    definitions.extend(catalog::definitions());
    definitions.extend(work_items::definitions());
    definitions.extend(agent_work::definitions());
    definitions.extend(agent_work_catalog::definitions());
    definitions.extend(repositories::definitions());
    definitions
}
