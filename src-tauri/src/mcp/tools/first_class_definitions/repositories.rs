use super::super::definitions::{
    boolean_property, empty_object_schema, enum_property, first_class_tool, integer_property,
    object_schema, string_property, ToolDefinition,
};

pub(super) fn definitions() -> Vec<ToolDefinition> {
    vec![
        first_class_tool(
            "repositories.list",
            "List Repositories",
            "List registered repositories.",
            empty_object_schema(),
        ),
        first_class_tool(
            "repositories.attachments.list_all",
            "List All Repository Attachments",
            "List repository attachments across product and product area scopes.",
            empty_object_schema(),
        ),
        first_class_tool(
            "repositories.register",
            "Register Repository",
            "Register a repository with Aruvi.",
            object_schema(
                vec![
                    ("name", string_property("Repository display name.")),
                    ("localPath", string_property("Absolute local repository path.")),
                    ("remoteUrl", string_property("Optional remote url.")),
                    ("defaultBranch", string_property("Default branch name.")),
                ],
                &["name", "localPath"],
            ),
        ),
        first_class_tool(
            "repositories.delete",
            "Delete Repository",
            "Delete a registered repository.",
            object_schema(vec![("id", string_property("The repository id."))], &["id"]),
        ),
        first_class_tool(
            "repositories.attachments.create",
            "Attach Repository",
            "Attach a repository to a product or product area scope.",
            object_schema(
                vec![
                    (
                        "scopeType",
                        enum_property("Attachment scope type.", &["product", "product_area"]),
                    ),
                    ("scopeId", string_property("Scope id to attach to.")),
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "isDefault",
                        boolean_property("Whether the attachment is the default."),
                    ),
                ],
                &["scopeType", "scopeId", "repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_work_item",
            "Resolve Repository For Work Item",
            "Resolve the repository associated with a work item.",
            object_schema(
                vec![("workItemId", string_property("The work item id."))],
                &["workItemId"],
            ),
        ),
        first_class_tool(
            "repositories.resolution.for_scope",
            "Resolve Repository For Scope",
            "Resolve the repository associated with a product or product area scope.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.workspaces.create_for_scope",
            "Create Local Workspace",
            "Create a new local workspace for a product, product area, or work item scope. Existing destinations are never overwritten; register an existing repository instead.",
            object_schema(
                vec![
                    ("productId", string_property("Optional product id.")),
                    ("productAreaId", string_property("Optional product area id.")),
                    ("workItemId", string_property("Optional work item id.")),
                    (
                        "preferredPath",
                        string_property("Optional preferred workspace path."),
                    ),
                ],
                &[],
            ),
        ),
        first_class_tool(
            "repositories.trees.list",
            "List Repository Tree",
            "List the file tree for a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("includeHidden", boolean_property("Whether to include hidden files.")),
                    ("maxDepth", integer_property("Optional maximum traversal depth.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.files.read",
            "Read Repository File",
            "Read a file from a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                    ("expectedRoot", string_property("Optional workspace root captured earlier; reject if it changed.")),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.read_binary",
            "Read Binary Repository File",
            "Read a saved file up to 8 MiB as base64; optionally reject a changed workspace root.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                    ("expectedRoot", string_property("Optional workspace root captured when browsing.")),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.write",
            "Write Repository File",
            "Write a file in a repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                    ("content", string_property("New file content.")),
                    ("expectedRoot", string_property("Optional workspace root captured earlier; reject if it changed.")),
                ],
                &["repositoryId", "relativePath", "content"],
            ),
        ),
        first_class_tool(
            "repositories.files.get_sha256",
            "Get Repository File SHA256",
            "Get the SHA256 of a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                ],
                &["repositoryId", "relativePath"],
            ),
        ),
        first_class_tool(
            "repositories.files.apply_patch",
            "Apply Repository Patch",
            "Apply a patch to a repository file.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "relativePath",
                        string_property("Repository-relative file path."),
                    ),
                    ("patch", string_property("Unified patch text to apply.")),
                    (
                        "baseSha256",
                        string_property("Optional expected base SHA256 for optimistic locking."),
                    ),
                    ("expectedRoot", string_property("Optional workspace root captured earlier; reject if it changed.")),
                ],
                &["repositoryId", "relativePath", "patch"],
            ),
        ),
        first_class_tool(
            "repositories.files.preview_patch",
            "Preview Repository Patch",
            "Preview a patch against a captured workspace root without writing. Retain the returned workspace_root, base_sha256 and exists values for reviewed application.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                    ("patch", string_property("Unified patch text to preview.")),
                    ("baseSha256", string_property("Optional expected base SHA256.")),
                    ("expectedRoot", string_property("Workspace root captured when reading the file.")),
                ],
                &["repositoryId", "relativePath", "patch", "expectedRoot"],
            ),
        ),
        first_class_tool(
            "repositories.files.apply_reviewed_patch",
            "Apply Reviewed Repository Patch",
            "Apply a previously previewed patch only if its workspace root and file baseline still match.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("relativePath", string_property("Repository-relative file path.")),
                    ("patch", string_property("The reviewed unified patch.")),
                    ("baseSha256", string_property("Base SHA256 returned by preview.")),
                    ("expectedExists", boolean_property("File existence returned by preview.")),
                    ("expectedRoot", string_property("Workspace root returned by preview.")),
                ],
                &["repositoryId", "relativePath", "patch", "baseSha256", "expectedExists", "expectedRoot"],
            ),
        ),
        first_class_tool(
            "repositories.git.status",
            "Get Repository Git Status",
            "Get branch, head SHA, dirty flag, and changed file status for a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    (
                        "includeIgnored",
                        boolean_property("Whether to include ignored files."),
                    ),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.diff",
            "Get Repository Git Diff",
            "Get a patch diff for working tree changes in a registered repository.",
            object_schema(
                vec![
                    ("repositoryId", string_property("The repository id.")),
                    ("maxBytes", integer_property("Maximum diff bytes returned.")),
                ],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.changed_files",
            "List Repository Git Changed Files",
            "List changed files in a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
        first_class_tool(
            "repositories.git.current_branch",
            "Get Repository Git Current Branch",
            "Get the current branch for a registered repository.",
            object_schema(
                vec![("repositoryId", string_property("The repository id."))],
                &["repositoryId"],
            ),
        ),
    ]
}
