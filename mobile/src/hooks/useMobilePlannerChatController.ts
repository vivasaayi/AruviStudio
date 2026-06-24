import { useState } from "react";
import { PlannerMobileClient } from "../api/client";
import {
  getLoopbackFallbackBaseUrl,
  isNetworkRequestFailure,
  normalizeBaseUrlForDisplay,
} from "../lib/mobileConnection";

type MobilePlannerChatControllerInput = {
  mobileClient: PlannerMobileClient;
  baseUrl: string;
  token: string;
  providerId: string;
  modelName: string;
  selectedProductId: string | null;
  onConnected: () => void;
  onFallbackBaseUrl: (fallbackBaseUrl: string) => Promise<void>;
};

type PlannerChatTurnBody = Parameters<PlannerMobileClient["submitMobilePlannerChatTurn"]>[1];

export function useMobilePlannerChatController({
  mobileClient,
  baseUrl,
  token,
  providerId,
  modelName,
  selectedProductId,
  onConnected,
  onFallbackBaseUrl,
}: MobilePlannerChatControllerInput) {
  const [plannerChatSessionId, setPlannerChatSessionId] = useState<string | null>(null);
  const [plannerContextProductName, setPlannerContextProductName] = useState<string | null>(null);

  const createPlannerChatSessionWithFallback = async () => {
    const body = {
      provider_id: providerId.trim() || undefined,
      model_name: modelName.trim() || undefined,
      product_id: selectedProductId ?? undefined,
    };
    try {
      const response = await mobileClient.createMobilePlannerChatSession(body);
      onConnected();
      setPlannerContextProductName(response.product_name ?? null);
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim())
            .createMobilePlannerChatSession(body);
          setPlannerContextProductName(response.product_name ?? null);
          await onFallbackBaseUrl(fallbackBaseUrl);
          return response;
        } catch {
          throw new Error(
            `Cannot reach Aruvi at ${normalizeBaseUrlForDisplay(baseUrl)} or ${fallbackBaseUrl}. Check Settings base URL and that the desktop bridge is running.`,
          );
        }
      }
      throw error;
    }
  };

  const runPlannerChatWithFallback = async (sessionId: string, body: PlannerChatTurnBody) => {
    try {
      const response = await mobileClient.submitMobilePlannerChatTurn(sessionId, body);
      onConnected();
      setPlannerContextProductName(response.product_name ?? null);
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim())
            .submitMobilePlannerChatTurn(sessionId, body);
          setPlannerContextProductName(response.product_name ?? null);
          await onFallbackBaseUrl(fallbackBaseUrl);
          return response;
        } catch {
          throw new Error(
            `Cannot reach Aruvi at ${normalizeBaseUrlForDisplay(baseUrl)} or ${fallbackBaseUrl}. Check Settings base URL and that the desktop bridge is running.`,
          );
        }
      }
      throw error;
    }
  };

  const submitPlannerPrompt = async (trimmed: string) => {
    const activeSessionId = plannerChatSessionId ?? (await createPlannerChatSessionWithFallback()).session_id;
    if (!plannerChatSessionId) {
      setPlannerChatSessionId(activeSessionId);
    }
    const response = await runPlannerChatWithFallback(activeSessionId, {
      provider_id: providerId.trim() || undefined,
      model_name: modelName.trim() || undefined,
      product_id: selectedProductId ?? undefined,
      messages: [
        {
          role: "user",
          content: trimmed,
        },
      ],
      max_tool_steps: 4,
    });
    const assistantText = response.assistant_message.trim() || "(empty planner response)";
    return {
      content: assistantText,
      toolTrace: response.tool_trace,
    };
  };

  return {
    plannerContextProductName,
    submitPlannerPrompt,
  };
}
