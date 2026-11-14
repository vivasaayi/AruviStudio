import type { ModelDefinition } from "./types";

export function isTranscriptionModel(model: ModelDefinition): boolean {
  return model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
    || model.name.trim().toLowerCase() === "deepseek-voice-transcription"
    || /whisper|transcrib/i.test(model.name);
}
