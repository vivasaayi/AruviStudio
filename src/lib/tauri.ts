import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type {
  Product,
  Module,
  Capability,
  WorkItem,
  ProductWorkItemSummary,
  Repository,
  RepositoryTreeNode,
  Approval,
  AgentModelBinding,
  ModelProvider,
  ModelDefinition,
  LocalModelRegistrationResult,
  AgentDefinition,
  AgentTeam,
  AgentTeamMembership,
  TeamAssignment,
  Skill,
  AgentSkillLink,
  TeamSkillLink,
  WorkflowStagePolicy,
  AgentRun,
  WorkflowRun,
  WorkflowStageHistory,
  Artifact,
  Finding,
  ProductTree,
  DatabaseHealth,
  MobileBridgeStatus,
  McpBridgeStatus,
  ChatMessagePayload,
  ChatCompletionResponse,
  WorkspaceProvisionResult,
  PlannerContactResult,
    PlannerDraftChildType,
    PlannerSessionInfo,
    SpeechToTextResponse,
    PlannerTurnResponse,
    SemanticTemplateApplicationResult,
    NodeKindConversionResult,
  } from "./types";

declare global {
  interface Window {
    __ARUVI_E2E__?: {
      invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T> | T;
      runPlannerVoiceTranscript?: (transcript: string) => Promise<void> | void;
    };
  }
}

const invoke = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (typeof window !== "undefined") {
    const mockInvoke = window.__ARUVI_E2E__?.invoke;
    if (mockInvoke) {
      return await mockInvoke<T>(command, args);
    }
  }
  return tauriInvoke<T>(command, args);
};

// Product commands
function toJsonArrayString(value: string | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return JSON.stringify(
    value
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean),
  );
}

function toJsonStringArray(value: string[] | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return JSON.stringify(value.map((item) => item.trim()).filter(Boolean));
}

function toJsonObjectString(value: string | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return JSON.stringify({});
  }
  try {
    return JSON.stringify(JSON.parse(trimmed));
  } catch {
    return JSON.stringify({});
  }
}

export const createProduct = (data: { name: string; description: string; vision: string; goals: string; tags: string }) =>
  invoke<Product>("create_product", {
    ...data,
    goals: toJsonArrayString(data.goals),
    tags: toJsonArrayString(data.tags),
  });

export const getProduct = (id: string) => invoke<Product>("get_product", { id });
export const listProducts = () => invoke<Product[]>("list_products");
export const seedExampleProducts = () => invoke<void>("seed_example_products");
export const updateProduct = (data: { id: string; name?: string; description?: string; vision?: string; goals?: string; tags?: string }) =>
  invoke<Product>("update_product", {
    ...data,
    goals: toJsonArrayString(data.goals),
    tags: toJsonArrayString(data.tags),
  });
export const archiveProduct = (id: string) => invoke<Product>("archive_product", { id });

// Module commands
export const createModule = (data: {
  productId: string;
  name: string;
  description: string;
  purpose: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Module>("create_module", {
    productId: data.productId,
    product_id: data.productId,
    name: data.name,
    description: data.description,
    purpose: data.purpose,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    explanation: data.explanation,
    examples: data.examples,
    implementationNotes: data.implementationNotes,
    implementation_notes: data.implementationNotes,
    testGuidance: data.testGuidance,
    test_guidance: data.testGuidance,
  });
export const listModules = (productId: string) =>
  invoke<Module[]>("list_modules", { productId, product_id: productId });
export const updateModule = (data: {
  id: string;
  name?: string;
  description?: string;
  purpose?: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Module>("update_module", {
    id: data.id,
    name: data.name,
    description: data.description,
    purpose: data.purpose,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    explanation: data.explanation,
    examples: data.examples,
    implementationNotes: data.implementationNotes,
    implementation_notes: data.implementationNotes,
    testGuidance: data.testGuidance,
    test_guidance: data.testGuidance,
  });
export const deleteModule = (id: string) => invoke("delete_module", { id });
export const reorderModules = (productId: string, orderedIds: string[]) =>
  invoke("reorder_modules", {
    productId,
    product_id: productId,
    orderedIds,
    ordered_ids: orderedIds,
  });

// Capability commands
export const createCapability = (data: {
  moduleId: string;
  parentCapabilityId?: string;
  name: string;
  description: string;
  acceptanceCriteria: string;
  priority: string;
  risk: string;
  technicalNotes: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Capability>("create_capability", {
    moduleId: data.moduleId,
    module_id: data.moduleId,
    parentCapabilityId: data.parentCapabilityId,
    parent_capability_id: data.parentCapabilityId,
    name: data.name,
    description: data.description,
    acceptanceCriteria: data.acceptanceCriteria,
    acceptance_criteria: data.acceptanceCriteria,
    priority: data.priority,
    risk: data.risk,
    technicalNotes: data.technicalNotes,
    technical_notes: data.technicalNotes,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    explanation: data.explanation,
    examples: data.examples,
    implementationNotes: data.implementationNotes,
    implementation_notes: data.implementationNotes,
    testGuidance: data.testGuidance,
    test_guidance: data.testGuidance,
  });
export const listCapabilities = (moduleId: string) =>
  invoke<Capability[]>("list_capabilities", { moduleId, module_id: moduleId });
export const updateCapability = (data: {
  id: string;
  name?: string;
  description?: string;
  acceptanceCriteria?: string;
  priority?: string;
  risk?: string;
  technicalNotes?: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Capability>("update_capability", {
    id: data.id,
    name: data.name,
    description: data.description,
    acceptance_criteria: data.acceptanceCriteria,
    priority: data.priority,
    risk: data.risk,
    technical_notes: data.technicalNotes,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    explanation: data.explanation,
    examples: data.examples,
    implementationNotes: data.implementationNotes,
    implementation_notes: data.implementationNotes,
    testGuidance: data.testGuidance,
    test_guidance: data.testGuidance,
  });
export const deleteCapability = (id: string) => invoke("delete_capability", { id });
export const reorderCapabilities = (data: { moduleId: string; parentCapabilityId?: string; orderedIds: string[] }) =>
  invoke("reorder_capabilities", {
    module_id: data.moduleId,
    parent_capability_id: data.parentCapabilityId,
    ordered_ids: data.orderedIds,
  });
export const applySemanticTemplate = (data: {
  moduleId: string;
  parentCapabilityId?: string;
  templateKind: "operator_chapter" | "technical_topic_book";
  name: string;
  description?: string;
  priority?: string;
  risk?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<SemanticTemplateApplicationResult>("apply_semantic_template", {
    moduleId: data.moduleId,
    module_id: data.moduleId,
    parentCapabilityId: data.parentCapabilityId,
    parent_capability_id: data.parentCapabilityId,
    templateKind: data.templateKind,
    template_kind: data.templateKind,
    name: data.name,
    description: data.description,
    priority: data.priority,
    risk: data.risk,
    explanation: data.explanation,
    examples: data.examples,
    implementationNotes: data.implementationNotes,
    implementation_notes: data.implementationNotes,
    testGuidance: data.testGuidance,
    test_guidance: data.testGuidance,
  });
export const convertCapabilityKind = (data: {
  id: string;
  nodeKind: string;
  childStrategy?: "reject" | "reparent_to_parent";
}) =>
  invoke<NodeKindConversionResult>("convert_capability_kind", {
    id: data.id,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    childStrategy: data.childStrategy,
    child_strategy: data.childStrategy,
  });

// Product tree
export const getProductTree = (productId: string) =>
  invoke<ProductTree>("get_product_tree", { productId, product_id: productId });

// Work item commands
export const createWorkItem = (data: {
  productId: string; moduleId?: string; capabilityId?: string; sourceNodeId?: string; sourceNodeType?: string; parentWorkItemId?: string;
  title: string; problemStatement: string; description: string; acceptanceCriteria: string;
  constraints: string; workItemType: string; priority: string; complexity: string;
}) =>
  invoke<WorkItem>("create_work_item", {
    productId: data.productId,
    product_id: data.productId,
    moduleId: data.moduleId,
    module_id: data.moduleId,
    capabilityId: data.capabilityId,
    capability_id: data.capabilityId,
    sourceNodeId: data.sourceNodeId,
    source_node_id: data.sourceNodeId,
    sourceNodeType: data.sourceNodeType,
    source_node_type: data.sourceNodeType,
    parentWorkItemId: data.parentWorkItemId,
    parent_work_item_id: data.parentWorkItemId,
    title: data.title,
    problemStatement: data.problemStatement,
    problem_statement: data.problemStatement,
    description: data.description,
    acceptanceCriteria: data.acceptanceCriteria,
    acceptance_criteria: data.acceptanceCriteria,
    constraints: data.constraints,
    workItemType: data.workItemType,
    work_item_type: data.workItemType,
    priority: data.priority,
    complexity: data.complexity,
  });

export const getWorkItem = (id: string) => invoke<WorkItem>("get_work_item", { id });
export const listWorkItems = (filters?: { productId?: string; moduleId?: string; capabilityId?: string; sourceNodeId?: string; sourceNodeType?: string; status?: string }) =>
  invoke<WorkItem[]>("list_work_items", {
    product_id: filters?.productId, module_id: filters?.moduleId,
    capability_id: filters?.capabilityId,
    source_node_id: filters?.sourceNodeId,
    source_node_type: filters?.sourceNodeType,
    status: filters?.status,
  });
export const summarizeWorkItemsByProduct = () =>
  invoke<ProductWorkItemSummary[]>("summarize_work_items_by_product");
export const updateWorkItem = (data: {
  id: string;
  title?: string;
  description?: string;
  status?: string;
  problemStatement?: string;
  acceptanceCriteria?: string;
  constraints?: string;
}) =>
  invoke<WorkItem>("update_work_item", {
    id: data.id,
    title: data.title,
    description: data.description,
    status: data.status,
    problem_statement: data.problemStatement,
    acceptance_criteria: data.acceptanceCriteria,
    constraints: data.constraints,
  });
export const deleteWorkItem = (id: string) => invoke("delete_work_item", { id });
export const getSubWorkItems = (workItemId: string) => invoke<WorkItem[]>("get_sub_work_items", { work_item_id: workItemId });
export const reorderWorkItems = (orderedIds: string[]) => invoke("reorder_work_items", { ordered_ids: orderedIds });

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
export const attachRepository = (data: { scopeType: "product" | "module"; scopeId: string; repositoryId: string; isDefault: boolean }) =>
  invoke("attach_repository", {
    scope_type: data.scopeType,
    scope_id: data.scopeId,
    repository_id: data.repositoryId,
    is_default: data.isDefault,
  });
export const resolveRepositoryForWorkItem = (workItemId: string) => invoke<Repository | null>("resolve_repository_for_work_item", { work_item_id: workItemId });
export const resolveRepositoryForScope = (data: { productId?: string | null; moduleId?: string | null }) =>
  invoke<Repository | null>("resolve_repository_for_scope", {
    product_id: data.productId ?? null,
    module_id: data.moduleId ?? null,
  });
export const createLocalWorkspace = (data: {
  productId?: string | null;
  moduleId?: string | null;
  workItemId?: string | null;
  preferredPath?: string | null;
}) =>
  invoke<WorkspaceProvisionResult>("create_local_workspace", {
    productId: data.productId ?? null,
    product_id: data.productId ?? null,
    moduleId: data.moduleId ?? null,
    module_id: data.moduleId ?? null,
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

// Approval commands
export const approveWorkItem = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item", { workItemId, work_item_id: workItemId, notes });
export const rejectWorkItem = (workItemId: string, notes: string) =>
  invoke<Approval>("reject_work_item", { workItemId, work_item_id: workItemId, notes });
export const approveWorkItemPlan = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item_plan", { workItemId, work_item_id: workItemId, notes });
export const rejectWorkItemPlan = (workItemId: string, notes: string) =>
  invoke<Approval>("reject_work_item_plan", { workItemId, work_item_id: workItemId, notes });
export const approveWorkItemTestReview = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item_test_review", { workItemId, work_item_id: workItemId, notes });
export const getWorkItemApprovals = (workItemId: string) =>
  invoke<Approval[]>("get_work_item_approvals", { workItemId, work_item_id: workItemId });

// Model commands
export const createProvider = (data: { name: string; providerType: string; baseUrl: string; authSecretRef?: string }) =>
  invoke<ModelProvider>("create_provider", {
    name: data.name,
    providerType: data.providerType,
    baseUrl: data.baseUrl,
    authSecretRef: data.authSecretRef,
    // Backward-compatible payload keys for mixed command argument conventions.
    provider_type: data.providerType,
    base_url: data.baseUrl,
    auth_secret_ref: data.authSecretRef,
  });
export const listProviders = () => invoke<ModelProvider[]>("list_providers");
export const updateProvider = (data: {
  id: string;
  name?: string;
  providerType?: string;
  baseUrl?: string;
  authSecretRef?: string;
  enabled?: boolean;
}) =>
  invoke<ModelProvider>("update_provider", {
    id: data.id,
    name: data.name,
    providerType: data.providerType,
    provider_type: data.providerType,
    baseUrl: data.baseUrl,
    base_url: data.baseUrl,
    authSecretRef: data.authSecretRef,
    auth_secret_ref: data.authSecretRef,
    enabled: data.enabled,
  });
export const deleteProvider = (id: string) => invoke("delete_provider", { id });
export const createModelDefinition = (data: { providerId: string; name: string; contextWindow?: number; capabilityTags?: string[]; notes?: string }) =>
  invoke<ModelDefinition>("create_model_definition", {
    providerId: data.providerId,
    provider_id: data.providerId,
    name: data.name,
    contextWindow: data.contextWindow ?? null,
    context_window: data.contextWindow ?? null,
    capabilityTags: toJsonStringArray(data.capabilityTags) ?? "[]",
    capability_tags: toJsonStringArray(data.capabilityTags) ?? "[]",
    notes: data.notes ?? "",
  });
export const listModelDefinitions = () => invoke<ModelDefinition[]>("list_model_definitions");
export const updateModelDefinition = (data: {
  id: string;
  providerId?: string;
  name?: string;
  contextWindow?: number;
  capabilityTags?: string[];
  notes?: string;
  enabled?: boolean;
}) =>
  invoke<ModelDefinition>("update_model_definition", {
    id: data.id,
    providerId: data.providerId,
    provider_id: data.providerId,
    name: data.name,
    contextWindow: data.contextWindow ?? null,
    context_window: data.contextWindow ?? null,
    capabilityTags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    notes: data.notes ?? null,
    enabled: data.enabled,
  });
export const deleteModelDefinition = (id: string) => invoke("delete_model_definition", { id });
export const testProviderConnectivity = (id: string) => invoke<string>("test_provider_connectivity", { id });
export const browseForLocalModelFile = () =>
  invoke<string | null>("browse_for_local_model_file");
export const registerLocalRuntimeModel = (data: {
  providerName: string;
  modelName: string;
  modelPath: string;
  capabilityTags?: string[];
  notes?: string;
  contextWindow?: number;
}) =>
  invoke<LocalModelRegistrationResult>("register_local_runtime_model_command", {
    providerName: data.providerName,
    provider_name: data.providerName,
    modelName: data.modelName,
    model_name: data.modelName,
    modelPath: data.modelPath,
    model_path: data.modelPath,
    capabilityTags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    notes: data.notes ?? null,
    contextWindow: data.contextWindow ?? null,
    context_window: data.contextWindow ?? null,
  });
export const installManagedLocalModel = (data: {
  providerName: string;
  modelName: string;
  downloadUrl: string;
  fileName: string;
  capabilityTags?: string[];
  notes?: string;
  contextWindow?: number;
}) =>
  invoke<LocalModelRegistrationResult>("install_managed_local_model_command", {
    providerName: data.providerName,
    provider_name: data.providerName,
    modelName: data.modelName,
    model_name: data.modelName,
    downloadUrl: data.downloadUrl,
    download_url: data.downloadUrl,
    fileName: data.fileName,
    file_name: data.fileName,
    capabilityTags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
    notes: data.notes ?? null,
    contextWindow: data.contextWindow ?? null,
    context_window: data.contextWindow ?? null,
  });
export const runModelChatCompletion = (data: {
  providerId: string;
  model: string;
  messages: ChatMessagePayload[];
  temperature?: number;
  maxTokens?: number;
}) =>
  invoke<ChatCompletionResponse>("run_model_chat_completion", {
    providerId: data.providerId,
    provider_id: data.providerId,
    model: data.model,
    messages: data.messages,
    temperature: data.temperature ?? null,
    maxTokens: data.maxTokens ?? null,
    max_tokens: data.maxTokens ?? null,
  });
export const startModelChatStream = (data: {
  providerId: string;
  model: string;
  messages: ChatMessagePayload[];
  temperature?: number;
  maxTokens?: number;
}) =>
  invoke<string>("start_model_chat_stream", {
    providerId: data.providerId,
    provider_id: data.providerId,
    model: data.model,
    messages: data.messages,
    temperature: data.temperature ?? null,
    maxTokens: data.maxTokens ?? null,
    max_tokens: data.maxTokens ?? null,
  });

// Agent commands
export const listAgentDefinitions = () => invoke<AgentDefinition[]>("list_agent_definitions");
export const listAgentModelBindings = () => invoke<AgentModelBinding[]>("list_agent_model_bindings");
export const setPrimaryAgentModelBinding = (data: { agentId: string; modelId: string }) =>
  invoke<AgentModelBinding>("set_primary_agent_model_binding", {
    agentId: data.agentId,
    agent_id: data.agentId,
    modelId: data.modelId,
    model_id: data.modelId,
  });
export const createAgentDefinition = (data: {
  name: string;
  role: string;
  description: string;
  promptTemplateRef: string;
  allowedTools: string;
  skillTags: string;
  boundaries: string;
  enabled: boolean;
  employmentStatus: "active" | "inactive" | "terminated";
}) =>
  invoke<AgentDefinition>("create_agent_definition", {
    name: data.name,
    role: data.role,
    description: data.description,
    prompt_template_ref: data.promptTemplateRef,
    allowed_tools: toJsonArrayString(data.allowedTools) ?? "[]",
    skill_tags: toJsonArrayString(data.skillTags) ?? "[]",
    boundaries: toJsonObjectString(data.boundaries) ?? "{}",
    enabled: data.enabled,
    employment_status: data.employmentStatus,
  });
export const updateAgentDefinition = (data: {
  id: string;
  name?: string;
  role?: string;
  description?: string;
  promptTemplateRef?: string;
  allowedTools?: string;
  skillTags?: string;
  boundaries?: string;
  enabled?: boolean;
  employmentStatus?: "active" | "inactive" | "terminated";
}) =>
  invoke<AgentDefinition>("update_agent_definition", {
    id: data.id,
    name: data.name,
    role: data.role,
    description: data.description,
    prompt_template_ref: data.promptTemplateRef,
    allowed_tools: toJsonArrayString(data.allowedTools),
    skill_tags: toJsonArrayString(data.skillTags),
    boundaries: toJsonObjectString(data.boundaries),
    enabled: data.enabled,
    employment_status: data.employmentStatus,
  });
export const deleteAgentDefinition = (id: string) => invoke("delete_agent_definition", { id });
export const listAgentTeams = () => invoke<AgentTeam[]>("list_agent_teams");
export const createAgentTeam = (data: { name: string; department: string; description: string; enabled: boolean; maxConcurrentWorkflows: number }) =>
  invoke<AgentTeam>("create_agent_team", {
    name: data.name,
    department: data.department,
    description: data.description,
    enabled: data.enabled,
    maxConcurrentWorkflows: data.maxConcurrentWorkflows,
    max_concurrent_workflows: data.maxConcurrentWorkflows,
  });
export const updateAgentTeam = (data: {
  id: string;
  name?: string;
  department?: string;
  description?: string;
  enabled?: boolean;
  maxConcurrentWorkflows?: number;
}) => invoke<AgentTeam>("update_agent_team", {
  id: data.id,
  name: data.name,
  department: data.department,
  description: data.description,
  enabled: data.enabled,
  maxConcurrentWorkflows: data.maxConcurrentWorkflows,
  max_concurrent_workflows: data.maxConcurrentWorkflows,
});
export const deleteAgentTeam = (id: string) => invoke("delete_agent_team", { id });
export const listTeamMemberships = () => invoke<AgentTeamMembership[]>("list_team_memberships");
export const addTeamMember = (data: { teamId: string; agentId: string; title: string; isLead: boolean }) =>
  invoke<AgentTeamMembership>("add_team_member", {
    teamId: data.teamId,
    team_id: data.teamId,
    agentId: data.agentId,
    agent_id: data.agentId,
    title: data.title,
    isLead: data.isLead,
    is_lead: data.isLead,
  });
export const removeTeamMember = (id: string) => invoke("remove_team_member", { id });
export const listTeamAssignments = () => invoke<TeamAssignment[]>("list_team_assignments");
export const assignTeamScope = (data: { teamId: string; scopeType: "product" | "module" | "capability"; scopeId: string }) =>
  invoke<TeamAssignment>("assign_team_scope", {
    teamId: data.teamId,
    team_id: data.teamId,
    scopeType: data.scopeType,
    scope_type: data.scopeType,
    scopeId: data.scopeId,
    scope_id: data.scopeId,
  });
export const removeTeamAssignment = (id: string) => invoke("remove_team_assignment", { id });
export const listSkills = () => invoke<Skill[]>("list_skills");
export const createSkill = (data: { name: string; category: string; description: string; instructions: string; enabled: boolean }) =>
  invoke<Skill>("create_skill", data);
export const updateSkill = (data: {
  id: string;
  name?: string;
  category?: string;
  description?: string;
  instructions?: string;
  enabled?: boolean;
}) => invoke<Skill>("update_skill", data);
export const deleteSkill = (id: string) => invoke("delete_skill", { id });
export const listAgentSkillLinks = () => invoke<AgentSkillLink[]>("list_agent_skill_links");
export const linkSkillToAgent = (data: { agentId: string; skillId: string; proficiency: "learning" | "working" | "expert" }) =>
  invoke<AgentSkillLink>("link_skill_to_agent", {
    agent_id: data.agentId,
    skill_id: data.skillId,
    proficiency: data.proficiency,
  });
export const unlinkSkillFromAgent = (id: string) => invoke("unlink_skill_from_agent", { id });
export const listTeamSkillLinks = () => invoke<TeamSkillLink[]>("list_team_skill_links");
export const linkSkillToTeam = (data: { teamId: string; skillId: string }) =>
  invoke<TeamSkillLink>("link_skill_to_team", {
    team_id: data.teamId,
    skill_id: data.skillId,
  });
export const unlinkSkillFromTeam = (id: string) => invoke("unlink_skill_from_team", { id });
export const listWorkflowStagePolicies = () => invoke<WorkflowStagePolicy[]>("list_workflow_stage_policies");
export const upsertWorkflowStagePolicy = (data: {
  stageName: string;
  primaryRoles: string;
  fallbackRoles: string;
  coordinatorRequired: boolean;
}) =>
  invoke<WorkflowStagePolicy>("upsert_workflow_stage_policy", {
    stage_name: data.stageName,
    primary_roles: toJsonArrayString(data.primaryRoles) ?? "[]",
    fallback_roles: toJsonArrayString(data.fallbackRoles) ?? "[]",
    coordinator_required: data.coordinatorRequired,
  });
export const deleteWorkflowStagePolicy = (stageName: string) =>
  invoke("delete_workflow_stage_policy", { stage_name: stageName });

// Workflow commands
export const startWorkItemWorkflow = (workItemId: string) =>
  invoke<string>("start_work_item_workflow", { workItemId, work_item_id: workItemId });
export const getWorkflowRun = (workflowRunId: string) =>
  invoke<WorkflowRun>("get_workflow_run", { workflowRunId, workflow_run_id: workflowRunId });
export const getLatestWorkflowRunForWorkItem = (workItemId: string) =>
  invoke<WorkflowRun | null>("get_latest_workflow_run_for_work_item", { workItemId, work_item_id: workItemId });
export const getWorkflowHistory = (workflowRunId: string) =>
  invoke<WorkflowStageHistory[]>("get_workflow_history", { workflowRunId, workflow_run_id: workflowRunId });
export const handleWorkflowUserAction = (data: {
  workflowRunId: string;
  action: "approve" | "reject" | "pause" | "resume" | "cancel";
  notes?: string;
}) =>
  invoke<void>("handle_workflow_user_action", {
    workflowRunId: data.workflowRunId,
    workflow_run_id: data.workflowRunId,
    action: data.action,
    notes: data.notes ?? null,
  });
export const listAgentRunsForWorkflow = (workflowRunId: string) =>
  invoke<AgentRun[]>("list_agent_runs_for_workflow", {
    workflowRunId,
    workflow_run_id: workflowRunId,
  });
export const markWorkflowRunFailed = (workflowRunId: string, reason?: string) =>
  invoke<void>("mark_workflow_run_failed", {
    workflowRunId,
    workflow_run_id: workflowRunId,
    reason: reason ?? null,
  });
export const restartWorkflowRun = (workflowRunId: string) =>
  invoke<string>("restart_workflow_run", {
    workflowRunId,
    workflow_run_id: workflowRunId,
  });

// Settings commands
export const getSetting = (key: string) => invoke<string | null>("get_setting", { key });
export const setSetting = (key: string, value: string) => invoke("set_setting", { key, value });
export const getMobileBridgeStatus = () => invoke<MobileBridgeStatus>("get_mobile_bridge_status");
export const getMcpBridgeStatus = () => invoke<McpBridgeStatus>("get_mcp_bridge_status");
export const getDatabaseHealth = () => invoke<DatabaseHealth>("get_database_health");
export const getActiveDatabasePath = () => invoke<string>("get_active_database_path");
export const getDatabasePathOverride = () => invoke<string | null>("get_database_path_override");
export const setDatabasePathOverride = (dbPath: string) =>
  invoke<void>("set_database_path_override", { dbPath, db_path: dbPath });
export const clearDatabasePathOverride = () => invoke<void>("clear_database_path_override");

// Artifact commands
export const listWorkItemArtifacts = (workItemId: string) =>
  invoke<Artifact[]>("list_work_item_artifacts", { workItemId, work_item_id: workItemId });
export const readArtifactContent = (artifactId: string) =>
  invoke<string>("read_artifact_content", { artifactId, artifact_id: artifactId });

// Finding commands
export const listWorkItemFindings = (workItemId: string) => invoke<Finding[]>("list_work_item_findings", { work_item_id: workItemId });

// Planner commands
export const createPlannerSession = (data?: { providerId?: string; modelName?: string }) =>
  invoke<PlannerSessionInfo>("create_planner_session_command", {
    providerId: data?.providerId ?? null,
    provider_id: data?.providerId ?? null,
    modelName: data?.modelName ?? null,
    model_name: data?.modelName ?? null,
  });

export const updatePlannerSession = (data: { sessionId: string; providerId?: string; modelName?: string }) =>
  invoke<PlannerSessionInfo>("update_planner_session_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    providerId: data.providerId ?? null,
    provider_id: data.providerId ?? null,
    modelName: data.modelName ?? null,
    model_name: data.modelName ?? null,
  });

export const clearPlannerPending = (sessionId: string) =>
  invoke<PlannerSessionInfo>("clear_planner_pending_command", {
    sessionId,
    session_id: sessionId,
  });

export const submitPlannerTurn = (data: { sessionId: string; userInput: string; selectedDraftNodeId?: string | null }) =>
  invoke<PlannerTurnResponse>("submit_planner_turn_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    userInput: data.userInput,
    user_input: data.userInput,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
  });

export const submitPlannerVoiceTurn = (data: { sessionId: string; transcript: string; selectedDraftNodeId?: string | null }) =>
  invoke<PlannerTurnResponse>("submit_planner_voice_turn_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    transcript: data.transcript,
    userInput: data.transcript,
    user_input: data.transcript,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
  });

export const confirmPlannerPlan = (sessionId: string) =>
  invoke<PlannerTurnResponse>("confirm_planner_plan_command", {
    sessionId,
    session_id: sessionId,
  });

export const renamePlannerDraftNode = (data: {
  sessionId: string;
  nodeId: string;
  name: string;
}) =>
  invoke<PlannerTurnResponse>("rename_planner_draft_node_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    nodeId: data.nodeId,
    node_id: data.nodeId,
    name: data.name,
  });

export const addPlannerDraftChild = (data: {
  sessionId: string;
  parentNodeId: string;
  childType: PlannerDraftChildType;
  name: string;
  summary?: string;
}) =>
  invoke<PlannerTurnResponse>("add_planner_draft_child_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    parentNodeId: data.parentNodeId,
    parent_node_id: data.parentNodeId,
    childType: data.childType,
    child_type: data.childType,
    name: data.name,
    summary: data.summary ?? null,
  });

export const deletePlannerDraftNode = (data: {
  sessionId: string;
  nodeId: string;
}) =>
  invoke<PlannerTurnResponse>("delete_planner_draft_node_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    nodeId: data.nodeId,
    node_id: data.nodeId,
  });

export const analyzeRepositoryForPlanner = (data: {
  sessionId: string;
  repositoryId: string;
  selectedDraftNodeId?: string | null;
}) =>
  invoke<PlannerTurnResponse>("analyze_repository_for_planner_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
  });

export const transcribeAudio = (data: {
  providerId?: string;
  modelName?: string;
  audioBytesBase64: string;
  mimeType: string;
  locale?: string;
}) =>
  invoke<SpeechToTextResponse>("transcribe_audio_command", {
    providerId: data.providerId ?? null,
    provider_id: data.providerId ?? null,
    modelName: data.modelName ?? null,
    model_name: data.modelName ?? null,
    audioBytesBase64: data.audioBytesBase64,
    audio_bytes_base64: data.audioBytesBase64,
    mimeType: data.mimeType,
    mime_type: data.mimeType,
    locale: data.locale ?? null,
  });

export const speakTextNatively = (data: {
  text: string;
  voice?: string;
  locale?: string;
}) =>
  invoke<void>("speak_text_natively_command", {
    text: data.text,
    voice: data.voice ?? null,
    locale: data.locale ?? null,
  });

export const sendTwilioWhatsappMessage = (data: { to: string; content: string }) =>
  invoke<void>("send_twilio_whatsapp_message", {
    to: data.to,
    content: data.content,
  });

export const startTwilioVoiceCall = (data: { to: string; initialPrompt?: string }) =>
  invoke<void>("start_twilio_voice_call", {
    to: data.to,
    initial_prompt: data.initialPrompt ?? null,
  });

export const routePlannerContact = (data: {
  to: string;
  content: string;
  preferredChannel?: "whatsapp" | "voice";
  allowAfterHours?: boolean;
}) =>
  invoke<PlannerContactResult>("route_planner_contact_command", {
    to: data.to,
    content: data.content,
    preferred_channel: data.preferredChannel ?? null,
    allow_after_hours: data.allowAfterHours ?? null,
  });
