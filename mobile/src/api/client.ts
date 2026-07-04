import type {
  ChatCompletionResponse,
  ChatMessagePayload,
  MobilePlannerChatSession,
  MobilePlannerChatTurnResponse,
  ModelCall,
  PlannerSessionInfo,
  PlannerTurnResponse,
  Product,
  ProductTree,
  ProductTreeSummary,
} from "../types";

type RequestOptions = {
  method?: "GET" | "POST";
  body?: unknown;
};

export class PlannerMobileClient {
  constructor(
    private readonly baseUrl: string,
    private readonly bearerToken: string,
  ) {}

  private buildUrl(path: string) {
    return `${this.baseUrl.replace(/\/+$/, "")}${path}`;
  }

  private async request<T>(path: string, options: RequestOptions = {}) {
    const response = await fetch(this.buildUrl(path), {
      method: options.method ?? "GET",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${this.bearerToken}`,
      },
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `Request failed with status ${response.status}`);
    }
    return await response.json() as T;
  }

  health() {
    return this.request<{ status: string }>("/api/mobile/health");
  }

  listModelCalls(limit = 100) {
    return this.request<ModelCall[]>(`/api/mobile/model-calls?limit=${encodeURIComponent(String(limit))}`);
  }

  getModelCall(id: string) {
    return this.request<ModelCall>(`/api/mobile/model-calls/${encodeURIComponent(id)}`);
  }

  listProducts() {
    return this.request<Product[]>("/api/mobile/products");
  }

  getProductTree(productId: string) {
    return this.request<ProductTree>(`/api/mobile/products/${encodeURIComponent(productId)}/tree`);
  }

  getProductSummary(productId: string) {
    return this.request<ProductTreeSummary>(`/api/mobile/products/${encodeURIComponent(productId)}/summary`);
  }

  createPlannerSession(body?: { provider_id?: string; model_name?: string }) {
    return this.request<PlannerSessionInfo>("/api/mobile/planner/sessions", {
      method: "POST",
      body,
    });
  }

  updatePlannerSession(sessionId: string, body: { provider_id?: string; model_name?: string }) {
    return this.request<PlannerSessionInfo>(`/api/mobile/planner/sessions/${sessionId}`, {
      method: "POST",
      body,
    });
  }

  submitPlannerTurn(sessionId: string, body: { user_input: string; selected_draft_node_id?: string | null }) {
    return this.request<PlannerTurnResponse>(`/api/mobile/planner/sessions/${sessionId}/turn`, {
      method: "POST",
      body,
    });
  }

  submitPlannerVoiceTurn(sessionId: string, body: { user_input: string; selected_draft_node_id?: string | null }) {
    return this.request<PlannerTurnResponse>(`/api/mobile/planner/sessions/${sessionId}/voice-turn`, {
      method: "POST",
      body,
    });
  }

  confirmPlannerDraft(sessionId: string) {
    return this.request<PlannerTurnResponse>(`/api/mobile/planner/sessions/${sessionId}/confirm`, {
      method: "POST",
    });
  }

  clearPlannerDraft(sessionId: string) {
    return this.request<PlannerSessionInfo>(`/api/mobile/planner/sessions/${sessionId}/clear`, {
      method: "POST",
    });
  }

  runChatCompletion(body: {
    provider_id?: string;
    model_name?: string;
    messages: ChatMessagePayload[];
    temperature?: number;
    max_tokens?: number;
  }) {
    return this.request<ChatCompletionResponse>("/api/mobile/chat/completions", {
      method: "POST",
      body,
    });
  }

  createMobilePlannerChatSession(body?: { provider_id?: string; model_name?: string; product_id?: string | null }) {
    return this.request<MobilePlannerChatSession>("/api/mobile/planner-chat/sessions", {
      method: "POST",
      body,
    });
  }

  submitMobilePlannerChatTurn(
    sessionId: string,
    body: {
      provider_id?: string;
      model_name?: string;
      product_id?: string | null;
      messages: ChatMessagePayload[];
      max_tool_steps?: number;
    },
  ) {
    return this.request<MobilePlannerChatTurnResponse>(
      `/api/mobile/planner-chat/sessions/${encodeURIComponent(sessionId)}/turn`,
      {
        method: "POST",
        body,
      },
    );
  }

}
