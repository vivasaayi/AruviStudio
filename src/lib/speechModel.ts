import type { ModelDefinition } from "./types";

export function isVoiceOnlySelector(name: string): boolean {
  return ["deepseek-voice-transcription", "deepseek-voice-synthesis"].includes(name.trim().toLowerCase());
}

export function isTranscriptionModel(model: ModelDefinition): boolean {
  return model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
    || model.name.trim().toLowerCase() === "deepseek-voice-transcription"
    || /whisper|transcrib/i.test(model.name);
}
