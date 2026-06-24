import type { PlannerPlan } from "./plannerPageTypes";

export const DEFAULT_ASSISTANT_OPENING =
  "Select a product first, then describe the product area, capability, feature, story, task, or design change you want to explore. Planning stays inside the selected product until you switch products.";

export const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
export const SPEECH_MODEL_KEY = "speech.transcription_model_name";
export const SPEECH_LOCALE_KEY = "speech.locale";
export const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
export const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";
export const SPEECH_AUTO_SPEAK_REPLIES_KEY = "speech.auto_speak_replies";
export const SPEECH_REVIEW_BEFORE_SEND_KEY = "speech.review_before_send";

export function normalize(value?: string | null) {
  return (value ?? "").trim().toLowerCase();
}

export function makeId() {
  return crypto.randomUUID();
}

export function isInformationalOnly(plan: PlannerPlan) {
  return plan.actions.length > 0 && plan.actions.every((action) => action.type === "report_status" || action.type === "report_tree");
}

export function formatElapsedMs(value: number) {
  const totalSeconds = Math.max(0, Math.round(value / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}:${String(seconds).padStart(2, "0")}`;
  }
  return `${seconds}s`;
}
