import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { transcriptionModelRank, isVoiceOnlySelector } from "../../../lib/speechModel";

function preferredTranscriptionModel(models: ModelDefinition[], providerId?: string): ModelDefinition | undefined {
  const eligible = models.filter((model) => !providerId || model.provider_id === providerId);
  return eligible.find((model) => transcriptionModelRank(model) === 0)
    ?? eligible.find((model) => transcriptionModelRank(model) === 1);
}

export type PlannerModelPickerOption = {
  value: string;
  label: string;
};

export type PlannerSpeechModelSelection = {
  providerId: string;
  modelName?: string;
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
      const providerSpeechModel = preferredTranscriptionModel(models, speechProviderSetting);
      return {
        providerId: speechProviderSetting,
        modelName: providerSpeechModel?.name || undefined,
        source: "settings",
      };
    }
    const namedSpeechModel = models.find((model) => model.enabled && model.name === speechModelSetting);
    if (namedSpeechModel) {
      return { providerId: namedSpeechModel.provider_id, modelName: namedSpeechModel.name, source: "settings" };
    }
  }

  const sameProvider = providerId ? preferredTranscriptionModel(models, providerId) : undefined;
  if (sameProvider) {
    return { providerId: sameProvider.provider_id, modelName: sameProvider.name, source: "planner" };
  }

  const anySpeechModel = preferredTranscriptionModel(models);
  if (anySpeechModel) {
    return { providerId: anySpeechModel.provider_id, modelName: anySpeechModel.name, source: "auto" };
  }

  if (providerId) {
    return { providerId, modelName: undefined, source: "fallback" };
  }
  return null;
}
