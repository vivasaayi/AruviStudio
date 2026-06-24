import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { listModelDefinitions, listProviders } from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";

type SettingsModelOptionsInput = {
  speechProviderId: string;
};

function looksLikeSpeechModel(model: ModelDefinition) {
  return (
    model.capability_tags.some((tag) =>
      ["speech_to_text", "transcription", "audio"].includes(tag),
    ) || /whisper|transcrib/i.test(model.name)
  );
}

export function useSettingsModelOptions({
  speechProviderId,
}: SettingsModelOptionsInput) {
  const { data: providers = [] } = useQuery<ModelProvider[]>({
    queryKey: ["settingsProviders"],
    queryFn: listProviders,
  });
  const { data: models = [] } = useQuery<ModelDefinition[]>({
    queryKey: ["settingsModels"],
    queryFn: listModelDefinitions,
  });

  const speechProviderOptions = useMemo(
    () => providers.filter((provider) => provider.enabled),
    [providers],
  );
  const speechModelOptions = useMemo(
    () =>
      models.filter(
        (model) =>
          model.enabled &&
          looksLikeSpeechModel(model) &&
          (!speechProviderId || model.provider_id === speechProviderId),
      ),
    [models, speechProviderId],
  );

  return {
    speechProviderOptions,
    speechModelOptions,
  };
}
