export interface ModelProvider {
  id: string;
  name: string;
  provider_type: "openai_compatible" | "local_runtime";
  base_url: string;
  auth_secret_ref: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ModelDefinition {
  id: string;
  provider_id: string;
  name: string;
  context_window: number | null;
  capability_tags: string[];
  notes: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface LocalModelRegistrationResult {
  file_path: string;
  downloaded: boolean;
  provider: ModelProvider;
  model_definition: ModelDefinition;
}

export interface ChatMessagePayload {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatCompletionResponse {
  content: string;
  token_count_input: number | null;
  token_count_output: number | null;
}
