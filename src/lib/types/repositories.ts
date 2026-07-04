export interface Repository {
  id: string;
  name: string;
  local_path: string;
  remote_url: string;
  default_branch: string;
  auth_profile: string | null;
  created_at: string;
  updated_at: string;
}

export interface RepositoryTreeNode {
  name: string;
  relative_path: string;
  node_type: "file" | "directory";
  size_bytes: number | null;
  children: RepositoryTreeNode[];
}

export interface RepositoryAttachment {
  id: string;
  scope_type: "product" | "product_area";
  scope_id: string;
  repository_id: string;
  is_default: boolean;
  created_at: string;
}

export interface WorkspaceProvisionResult {
  repository: Repository;
  created_path: string;
  attached_scope_type: "product" | "product_area";
  attached_scope_id: string;
}
