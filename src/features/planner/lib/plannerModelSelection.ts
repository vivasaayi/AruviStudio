import type { ModelDefinition, ModelProvider } from "../../../lib/types";

export type PlannerModelPickerOption = {
  value: string;
  label: string;
};

export type PlannerSpeechModelSelection = {
  providerId: string;
  modelName: string;
  source: "settings" | "planner" | "auto" | "fallback";
};

function looksLikeSpeechModel(model: ModelDefinition) {
  return model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
    || /whisper|transcrib/i.test(model.name);
}

export function buildPlannerModelPickerOptions(
  models: ModelDefinition[],
  providers: ModelProvider[],
): PlannerModelPickerOption[] {
  return models
    .filter((model) => model.enabled)
    .map((model) => {
      const provider = providers.find((entry) => entry.id === model.provider_id);
      return {
        value: `${model.provider_id}::${model.name}`,
        label: `${provider?.name ?? "Unknown Provider"} / ${model.name}`,
      };
    });
}

export function resolvePlannerSpeechModelSelection({
  models,
  providerId,
  speechProviderSetting,
  speechModelSetting,
}: {
  models: ModelDefinition[];
  providerId: string;
  speechProviderSetting: string;
  speechModelSetting: string;
}): PlannerSpeechModelSelection | null {
  if (speechProviderSetting || speechModelSetting) {
    if (speechProviderSetting && speechModelSetting) {
      return { providerId: speechProviderSetting, modelName: speechModelSetting, source: "settings" };
    }
    if (speechProviderSetting) {
      const providerSpeechModel = models.find((model) => model.enabled && model.provider_id === speechProviderSetting && looksLikeSpeechModel(model));
      return {
        providerId: speechProviderSetting,
        modelName: providerSpeechModel?.name ?? speechModelSetting ?? "whisper-1",
        source: "settings",
      };
    }
    const namedSpeechModel = models.find((model) => model.enabled && model.name === speechModelSetting);
    if (namedSpeechModel) {
      return { providerId: namedSpeechModel.provider_id, modelName: namedSpeechModel.name, source: "settings" };
    }
  }

  const sameProvider = models.find((model) => model.enabled && model.provider_id === providerId && looksLikeSpeechModel(model));
  if (sameProvider) {
    return { providerId: sameProvider.provider_id, modelName: sameProvider.name, source: "planner" };
  }

  const anySpeechModel = models.find((model) => model.enabled && looksLikeSpeechModel(model));
  if (anySpeechModel) {
    return { providerId: anySpeechModel.provider_id, modelName: anySpeechModel.name, source: "auto" };
  }

  if (providerId) {
    return { providerId, modelName: "whisper-1", source: "fallback" };
  }
  return null;
}
