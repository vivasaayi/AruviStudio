import { invoke as tauriInvoke } from "@tauri-apps/api/core";

declare global {
  interface Window {
    __ARUVI_E2E__?: {
      invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T> | T;
      runPlannerVoiceTranscript?: (transcript: string) => Promise<void> | void;
    };
  }
}

export const invoke = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (typeof window !== "undefined") {
    const mockInvoke = window.__ARUVI_E2E__?.invoke;
    if (mockInvoke) {
      return await mockInvoke<T>(command, args);
    }
  }
  return tauriInvoke<T>(command, args);
};

export function toJsonArrayString(value: string | undefined): string | undefined {
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

export function toJsonStringArray(value: string[] | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return JSON.stringify(value.map((item) => item.trim()).filter(Boolean));
}

export function toJsonObjectString(value: string | undefined): string | undefined {
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
