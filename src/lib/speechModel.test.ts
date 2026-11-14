import { describe, expect, it } from "vitest";
import { isTranscriptionModel, transcriptionModelRank } from "./speechModel";
import type { ModelDefinition } from "./types";

const model = (name: string, capability_tags: string[], enabled = true): ModelDefinition => ({
  id: "fixture", provider_id: "provider", name, capability_tags, enabled,
  context_window: null, notes: "", created_at: "", updated_at: "",
});

describe("transcription model selection", () => {
  it("matches the backend ranking for explicit capabilities and synthesis exclusions", () => {
    for (const [entry, expected] of [
      [model("audio-tts", ["audio"]), null],
      [model("speech-model", ["text_to_speech"]), null],
      [model("audio-tts", ["audio", "speech-to-text"]), 0],
      [model("custom", [" ASR "]), 0],
      [model("custom", ["stt"]), 0],
      [model("speech-model", []), 1],
      [model("whisper", [], false), null],
      [model("deepseek-voice-synthesis", ["audio"]), null],
      [model(" DeepSeek-Voice-Synthesis ", ["speech_to_text"]), null],
      [model("deepseek-voice-transcription", []), 1],
    ] as const) {
      expect(transcriptionModelRank(entry)).toBe(expected);
      expect(isTranscriptionModel(entry)).toBe(expected !== null);
    }
  });
});
