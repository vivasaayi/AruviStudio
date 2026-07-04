import { useState } from "react";
import { PlannerMobileClient } from "../api/client";
import { buildModelCallSessions } from "../lib/mobileFormatters";
import type { ModelCall } from "../types";

type MobileModelCallsControllerInput = {
  mobileClient: PlannerMobileClient;
  token: string;
  describeError: (error: unknown) => string;
};

export function useMobileModelCallsController({
  mobileClient,
  token,
  describeError,
}: MobileModelCallsControllerInput) {
  const [modelCalls, setModelCalls] = useState<ModelCall[]>([]);
  const [selectedModelCallSessionKey, setSelectedModelCallSessionKey] = useState<string | null>(null);
  const [selectedModelCall, setSelectedModelCall] = useState<ModelCall | null>(null);
  const [isModelCallsLoading, setIsModelCallsLoading] = useState(false);
  const [modelCallsError, setModelCallsError] = useState<string | null>(null);

  const loadModelCalls = async () => {
    if (!token.trim()) {
      setModelCallsError("Save a mobile API token before loading calls.");
      return;
    }
    try {
      setIsModelCallsLoading(true);
      setModelCallsError(null);
      const calls = await mobileClient.listModelCalls(100);
      const sessions = buildModelCallSessions(calls);
      setModelCalls(calls);
      setSelectedModelCallSessionKey((current) => {
        const nextSessionKey =
          current && sessions.some((session) => session.key === current)
            ? current
            : sessions[0]?.key ?? null;
        const nextSession = sessions.find((session) => session.key === nextSessionKey) ?? null;
        setSelectedModelCall((selectedCall) => {
          if (!nextSession) return null;
          return (
            nextSession.calls.find((call) => call.id === selectedCall?.id)
            ?? nextSession.calls[nextSession.calls.length - 1]
            ?? null
          );
        });
        return nextSessionKey;
      });
    } catch (error) {
      setModelCallsError(describeError(error));
    } finally {
      setIsModelCallsLoading(false);
    }
  };

  return {
    modelCalls,
    selectedModelCallSessionKey,
    selectedModelCall,
    isModelCallsLoading,
    modelCallsError,
    loadModelCalls,
    setSelectedModelCallSessionKey,
    setSelectedModelCall,
  };
}
