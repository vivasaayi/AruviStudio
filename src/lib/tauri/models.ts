import { invoke, toJsonStringArray } from "./core";
import type {
  ChatCompletionResponse,
  ChatMessagePayload,
  LocalModelRegistrationResult,
  ModelCall,
  ModelDefinition,
  ModelProvider,
} from "../types";

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
    request: {
      id: data.id,
      provider_id: data.providerId,
      name: data.name,
      context_window: data.contextWindow ?? null,
      capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
      notes: data.notes ?? null,
      enabled: data.enabled,
    },
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
    request: {
      provider_name: data.providerName,
      model_name: data.modelName,
      model_path: data.modelPath,
      capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
      notes: data.notes ?? null,
      context_window: data.contextWindow ?? null,
    },
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
    request: {
      provider_name: data.providerName,
      model_name: data.modelName,
      download_url: data.downloadUrl,
      file_name: data.fileName,
      capability_tags: data.capabilityTags ? toJsonStringArray(data.capabilityTags) : null,
      notes: data.notes ?? null,
      context_window: data.contextWindow ?? null,
    },
  });
export const runModelChatCompletion = (data: {
  providerId: string;
  model: string;
  messages: ChatMessagePayload[];
  temperature?: number;
  maxTokens?: number;
  sourceKind?: string;
  sourceId?: string;
  sourceLabel?: string;
}) =>
  invoke<ChatCompletionResponse>("run_model_chat_completion", {
    request: {
      provider_id: data.providerId,
      model: data.model,
      messages: data.messages,
      temperature: data.temperature ?? null,
      max_tokens: data.maxTokens ?? null,
      source_kind: data.sourceKind ?? null,
      source_id: data.sourceId ?? null,
      source_label: data.sourceLabel ?? null,
    },
  });
export const startModelChatStream = (data: {
  providerId: string;
  model: string;
  messages: ChatMessagePayload[];
  temperature?: number;
  maxTokens?: number;
  sourceKind?: string;
  sourceId?: string;
  sourceLabel?: string;
}) =>
  invoke<string>("start_model_chat_stream", {
    request: {
      provider_id: data.providerId,
      model: data.model,
      messages: data.messages,
      temperature: data.temperature ?? null,
      max_tokens: data.maxTokens ?? null,
      source_kind: data.sourceKind ?? null,
      source_id: data.sourceId ?? null,
      source_label: data.sourceLabel ?? null,
    },
  });
export const listModelCalls = (limit = 200) => invoke<ModelCall[]>("list_model_calls", { limit });
export const getModelCall = (id: string) => invoke<ModelCall>("get_model_call", { id });
export const readModelCallSnapshot = (id: string, kind: "request" | "response") =>
  invoke<string>("read_model_call_snapshot", { id, kind });
