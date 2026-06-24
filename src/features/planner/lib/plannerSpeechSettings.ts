import { getSetting } from "../../../lib/tauri";
import {
  SPEECH_AUTO_SPEAK_REPLIES_KEY,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
  SPEECH_REVIEW_BEFORE_SEND_KEY,
} from "./plannerPageModel";

export interface PlannerSpeechSettings {
  provider?: string;
  model?: string;
  locale?: string;
  nativeVoice?: string;
  micEnabled?: boolean;
  autoSpeak?: boolean;
  reviewBeforeSend?: boolean;
}

export async function loadPlannerSpeechSettings(): Promise<PlannerSpeechSettings> {
  const [
    provider,
    model,
    locale,
    nativeVoice,
    micEnabled,
    autoSpeak,
    reviewBeforeSend,
  ] = await Promise.all([
    getSetting(SPEECH_PROVIDER_KEY),
    getSetting(SPEECH_MODEL_KEY),
    getSetting(SPEECH_LOCALE_KEY),
    getSetting(SPEECH_NATIVE_VOICE_KEY),
    getSetting(SPEECH_ENABLE_MIC_KEY),
    getSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY),
    getSetting(SPEECH_REVIEW_BEFORE_SEND_KEY),
  ]);

  return {
    provider: provider || undefined,
    model: model || undefined,
    locale: locale || undefined,
    nativeVoice: nativeVoice || undefined,
    micEnabled: parseOptionalEnabledFlag(micEnabled),
    autoSpeak: parseOptionalBoolean(autoSpeak),
    reviewBeforeSend: parseOptionalBoolean(reviewBeforeSend),
  };
}

function parseOptionalBoolean(value: string | null): boolean | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  return value.trim().toLowerCase() === "true";
}

function parseOptionalEnabledFlag(value: string | null): boolean | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  return value.trim().toLowerCase() !== "false";
}
