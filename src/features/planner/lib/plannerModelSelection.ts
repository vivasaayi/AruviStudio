import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { isTranscriptionModel, isVoiceOnlySelector } from "../../../lib/speechModel";

export type PlannerModelPickerOption = {
  value: string;
  label: string;
};

export type PlannerSpeechModelSelection = {
  providerId: string;
  modelName: string;
  source: "settings" | "planner" | "auto" | "fallback";
};

export function buildPlannerModelPickerOptions(
  models: ModelDefinition[],
  providers: ModelProvider[],
): PlannerModelPickerOption[] {
  return models
    .filter((model) => model.enabled && !isVoiceOnlySelector(model.name)
      && providers.find((provider) => provider.id === model.provider_id)?.provider_type !== "local_runtime")
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
      const providerSpeechModel = models.find((model) => model.enabled && model.provider_id === speechProviderSetting && isTranscriptionModel(model));
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

  const sameProvider = models.find((model) => model.enabled && model.provider_id === providerId && isTranscriptionModel(model));
  if (sameProvider) {
    return { providerId: sameProvider.provider_id, modelName: sameProvider.name, source: "planner" };
  }

  const anySpeechModel = models.find((model) => model.enabled && isTranscriptionModel(model));
  if (anySpeechModel) {
    return { providerId: anySpeechModel.provider_id, modelName: anySpeechModel.name, source: "auto" };
  }

  if (providerId) {
    return { providerId, modelName: "whisper-1", source: "fallback" };
  }
  return null;
}
