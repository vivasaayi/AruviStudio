export interface ModelProvider {
  id: string;
  name: string;
  provider_type: "openai_compatible" | "local_runtime";
  protocol?: "openai_compatible" | "anthropic";
  completion_profile?: "standard" | "openai_reasoning";
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
  metadata?: {
    response_id: string | null;
    model: string | null;
    finish_reason: string | null;
    usage: Record<string, unknown> | null;
  };
}
