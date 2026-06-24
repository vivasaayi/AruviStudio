use super::definitions::ToolDefinition;

mod agent_work;
mod catalog;
mod repositories;
mod work_items;

pub(super) fn definitions() -> Vec<ToolDefinition> {
    let mut definitions = Vec::new();
    definitions.extend(catalog::definitions());
    definitions.extend(work_items::definitions());
    definitions.extend(agent_work::definitions());
    definitions.extend(repositories::definitions());
    definitions
}
