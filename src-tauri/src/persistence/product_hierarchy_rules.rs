use crate::domain::product::HierarchyNodeKind;
use crate::error::AppError;

fn normalize_node_kind_value(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

pub(crate) fn parse_root_node_kind(value: &str) -> Result<HierarchyNodeKind, AppError> {
    match normalize_node_kind_value(value).as_str() {
        "product_area" => Ok(HierarchyNodeKind::ProductArea),
        value => HierarchyNodeKind::parse(value).ok_or_else(|| {
            AppError::Validation(format!(
                "Unsupported product hierarchy node kind '{value}'. Use product_area, capability, or feature."
            ))
        }),
    }
}

pub(crate) fn parse_capability_node_kind(value: &str) -> Result<HierarchyNodeKind, AppError> {
    match normalize_node_kind_value(value).as_str() {
        "capability" => Ok(HierarchyNodeKind::Capability),
        "feature" => Ok(HierarchyNodeKind::Feature),
        value => HierarchyNodeKind::parse(value).ok_or_else(|| {
            AppError::Validation(format!(
                "Unsupported product hierarchy node kind '{value}'. Use product_area, capability, or feature."
            ))
        }),
    }
}

pub(crate) fn resolve_root_node_kind(
    node_kind: Option<&str>,
) -> Result<HierarchyNodeKind, AppError> {
    let kind = node_kind
        .map(parse_root_node_kind)
        .transpose()?
        .unwrap_or_else(HierarchyNodeKind::default_root);
    if !kind.is_root_kind() {
        return Err(AppError::Validation(
            "Root product areas must use product_area.".to_string(),
        ));
    }
    Ok(kind)
}

pub(crate) fn resolve_child_node_kind(
    parent_kind: HierarchyNodeKind,
    node_kind: Option<&str>,
) -> Result<HierarchyNodeKind, AppError> {
    if !parent_kind.can_have_children() {
        return Err(AppError::Validation(format!(
            "{} nodes cannot contain structural children.",
            parent_kind
        )));
    }
    let child_kind = node_kind
        .map(parse_capability_node_kind)
        .transpose()?
        .unwrap_or_else(|| HierarchyNodeKind::default_child(&parent_kind));
    if !parent_kind.supports_child_kind(&child_kind) {
        return Err(AppError::Validation(format!(
            "{} cannot contain {}.",
            parent_kind, child_kind
        )));
    }
    Ok(child_kind)
}
