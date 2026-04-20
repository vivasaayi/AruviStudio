import type { PlannerSessionInfo, PlannerTurnResponse, SpeechToTextResponse } from "../types";

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

  transcribeSpeech(body: {
    provider_id?: string;
    model_name?: string;
    audio_bytes_base64: string;
    mime_type: string;
    locale?: string;
  }) {
    return this.request<SpeechToTextResponse>("/api/mobile/speech/transcribe", {
      method: "POST",
      body,
    });
  }
}
