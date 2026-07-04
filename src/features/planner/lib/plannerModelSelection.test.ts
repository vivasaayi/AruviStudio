import { describe, expect, it } from "vitest";

import {
  buildPlannerModelPickerOptions,
  resolvePlannerSpeechModelSelection,
} from "./plannerModelSelection";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";

function provider(id: string, name: string): ModelProvider {
  return {
    id,
    name,
    provider_type: "openai_compatible",
    base_url: "",
    auth_secret_ref: null,
    enabled: true,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

function model(
  id: string,
  providerId: string,
  name: string,
  capabilityTags: string[] = [],
  enabled = true,
): ModelDefinition {
  return {
    id,
    provider_id: providerId,
    name,
    context_window: null,
    capability_tags: capabilityTags,
    notes: "",
    enabled,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

describe("plannerModelSelection", () => {
  it("builds picker options for enabled models with provider labels", () => {
    expect(buildPlannerModelPickerOptions([
      model("model-1", "provider-1", "planner"),
      model("model-2", "missing", "orphan"),
      model("model-3", "provider-1", "disabled", [], false),
    ], [provider("provider-1", "OpenAI")])).toEqual([
      { value: "provider-1::planner", label: "OpenAI / planner" },
      { value: "missing::orphan", label: "Unknown Provider / orphan" },
    ]);
  });

  it("resolves explicit speech provider and model settings first", () => {
    expect(resolvePlannerSpeechModelSelection({
      models: [model("model-1", "provider-1", "whisper-large", ["speech_to_text"])],
      providerId: "provider-2",
      speechProviderSetting: "provider-1",
      speechModelSetting: "custom-transcriber",
    })).toEqual({
      providerId: "provider-1",
      modelName: "custom-transcriber",
      source: "settings",
    });
  });

  it("auto-detects speech models by current provider, then any provider, then fallback", () => {
    const models = [
      model("model-1", "provider-1", "planner"),
      model("model-2", "provider-1", "whisper-small"),
      model("model-3", "provider-2", "audio-transcriber", ["audio"]),
    ];

    expect(resolvePlannerSpeechModelSelection({
      models,
      providerId: "provider-1",
      speechProviderSetting: "",
      speechModelSetting: "",
    })).toEqual({
      providerId: "provider-1",
      modelName: "whisper-small",
      source: "planner",
    });

    expect(resolvePlannerSpeechModelSelection({
      models: [models[0], models[2]],
      providerId: "provider-1",
      speechProviderSetting: "",
      speechModelSetting: "",
    })).toEqual({
      providerId: "provider-2",
      modelName: "audio-transcriber",
      source: "auto",
    });

    expect(resolvePlannerSpeechModelSelection({
      models: [models[0]],
      providerId: "provider-1",
      speechProviderSetting: "",
      speechModelSetting: "",
    })).toEqual({
      providerId: "provider-1",
      modelName: "whisper-1",
      source: "fallback",
    });
  });

  it("returns null when no provider or speech model can be resolved", () => {
    expect(resolvePlannerSpeechModelSelection({
      models: [],
      providerId: "",
      speechProviderSetting: "",
      speechModelSetting: "",
    })).toBeNull();
  });
});
