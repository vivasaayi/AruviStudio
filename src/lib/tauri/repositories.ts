import { invoke } from "./core";
import type {
  Repository,
  RepositoryTreeNode,
  WorkspaceProvisionResult,
} from "../types";

// Repository commands
export const registerRepository = (data: { name: string; localPath: string; remoteUrl: string; defaultBranch: string }) =>
  invoke<Repository>("register_repository", {
    name: data.name,
    localPath: data.localPath,
    remoteUrl: data.remoteUrl,
    defaultBranch: data.defaultBranch,
    local_path: data.localPath,
    remote_url: data.remoteUrl,
    default_branch: data.defaultBranch,
  });
export const updateRepository = (data: { id: string; name: string; localPath: string; remoteUrl: string; defaultBranch: string }) =>
  invoke<Repository>("update_repository", {
    id: data.id,
    name: data.name,
    localPath: data.localPath,
    remoteUrl: data.remoteUrl,
    defaultBranch: data.defaultBranch,
    local_path: data.localPath,
    remote_url: data.remoteUrl,
    default_branch: data.defaultBranch,
  });
export const listRepositories = () => invoke<Repository[]>("list_repositories");
export const deleteRepository = (id: string) => invoke("delete_repository", { id });
export const browseForRepositoryPath = () => invoke<string | null>("browse_for_repository_path");
export const revealInFinder = (path: string) => invoke<void>("reveal_in_finder", { path });
export const exportProductOverviewHtml = (data: { fileName: string; html: string }) =>
  invoke<string>("export_product_overview_html", {
    fileName: data.fileName,
    file_name: data.fileName,
    html: data.html,
  });
export const exportProductOverviewEpub = (data: {
  fileName: string;
  title: string;
  html: string;
  tocItems: { id: string; title: string; level: number }[];
  author?: string;
  language?: string;
}) =>
  invoke<string>("export_product_overview_epub", {
    fileName: data.fileName,
    file_name: data.fileName,
    title: data.title,
    html: data.html,
    tocItems: data.tocItems,
    toc_items: data.tocItems,
    author: data.author ?? null,
    language: data.language ?? "en",
  });
export const exportProductOverviewPdf = (data: {
  fileName: string;
  html: string;
  pageWidth: string;
  pageHeight: string;
  marginTop: string;
  marginRight: string;
  marginBottom: string;
  marginLeft: string;
  headerTitle: string;
  headerRight?: string;
}) =>
  invoke<string>("export_product_overview_pdf", {
    request: {
      file_name: data.fileName,
      html: data.html,
      page_width: data.pageWidth,
      page_height: data.pageHeight,
      margin_top: data.marginTop,
      margin_right: data.marginRight,
      margin_bottom: data.marginBottom,
      margin_left: data.marginLeft,
      header_title: data.headerTitle,
      header_right: data.headerRight ?? null,
    },
  });
export const attachRepository = (data: { scopeType: "product" | "product_area"; scopeId: string; repositoryId: string; isDefault: boolean }) =>
  invoke("attach_repository", {
    scope_type: data.scopeType,
    scope_id: data.scopeId,
    repository_id: data.repositoryId,
    is_default: data.isDefault,
  });
export const resolveRepositoryForWorkItem = (workItemId: string) => invoke<Repository | null>("resolve_repository_for_work_item", { work_item_id: workItemId });
export const resolveRepositoryForScope = (data: { productId?: string | null; productAreaId?: string | null }) =>
  invoke<Repository | null>("resolve_repository_for_scope", {
    product_id: data.productId ?? null,
    product_area_id: data.productAreaId ?? null,
  });
export const createLocalWorkspace = (data: {
  productId?: string | null;
  productAreaId?: string | null;
  workItemId?: string | null;
  preferredPath?: string | null;
}) =>
  invoke<WorkspaceProvisionResult>("create_local_workspace", {
    productId: data.productId ?? null,
    product_id: data.productId ?? null,
    productAreaId: data.productAreaId ?? null,
    product_area_id: data.productAreaId ?? null,
    workItemId: data.workItemId ?? null,
    work_item_id: data.workItemId ?? null,
    preferredPath: data.preferredPath ?? null,
    preferred_path: data.preferredPath ?? null,
  });
export const listRepositoryTree = (data: { repositoryId: string; includeHidden?: boolean; maxDepth?: number }) =>
  invoke<RepositoryTreeNode[]>("list_repository_tree", {
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    includeHidden: data.includeHidden ?? false,
    include_hidden: data.includeHidden ?? false,
    maxDepth: data.maxDepth ?? null,
    max_depth: data.maxDepth ?? null,
  });
export const readRepositoryFile = (data: { repositoryId: string; relativePath: string }) =>
  invoke<string>("read_repository_file", {
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    relativePath: data.relativePath,
    relative_path: data.relativePath,
  });
export const writeRepositoryFile = (data: { repositoryId: string; relativePath: string; content: string }) =>
  invoke<void>("write_repository_file", {
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    relativePath: data.relativePath,
    relative_path: data.relativePath,
    content: data.content,
  });
export const getRepositoryFileSha256 = (data: { repositoryId: string; relativePath: string }) =>
  invoke<string>("get_repository_file_sha256", {
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    relativePath: data.relativePath,
    relative_path: data.relativePath,
  });
export const applyRepositoryPatch = (data: {
  repositoryId: string;
  relativePath: string;
  patch: string;
  baseSha256?: string;
}) =>
  invoke<string>("apply_repository_patch", {
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    relativePath: data.relativePath,
    relative_path: data.relativePath,
    patch: data.patch,
    baseSha256: data.baseSha256 ?? null,
    base_sha256: data.baseSha256 ?? null,
  });
