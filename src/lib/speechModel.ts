import type { ModelDefinition } from "./types";

export function isVoiceOnlySelector(name: string): boolean {
  return ["deepseek-voice-transcription", "deepseek-voice-synthesis"].includes(name.trim().toLowerCase());
}

export function transcriptionModelRank(model: ModelDefinition): number | null {
  if (!model.enabled) return null;
  if (model.name.trim().toLowerCase() === "deepseek-voice-synthesis") return null;
  const tags = model.capability_tags.map((tag) => tag.trim().toLowerCase().replace(/-/g, "_"));
  if (tags.some((tag) => ["speech_to_text", "transcription", "stt", "asr"].includes(tag))) return 0;
  const name = model.name.toLowerCase();
  const synthesis = name.split(/[^a-z0-9]+/).some((part) => ["tts", "synthesis", "synthesizer"].includes(part))
    || name.includes("text-to-speech") || name.includes("text_to_speech")
    || tags.some((tag) => ["tts", "text_to_speech", "speech_synthesis", "synthesis"].includes(tag));
  if (synthesis) return null;
  return tags.includes("audio") || name.trim() === "deepseek-voice-transcription" || /whisper|transcrib|speech|stt|audio/i.test(name) ? 1 : null;
}

export function isTranscriptionModel(model: ModelDefinition): boolean {
  return transcriptionModelRank(model) !== null;
}
