import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  getSetting,
  listModelDefinitions,
  listProviders,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import type { VoiceChatModelOption } from "../components/VoiceChatHeader";
import {
  parseBooleanSetting,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
} from "../lib/voiceChatSettings";

const DEFAULT_SYSTEM_PROMPT =
  "You are a concise, capable voice assistant. Keep replies natural and easy to speak aloud.";

export function useVoiceChatConfiguration() {
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [systemPrompt, setSystemPrompt] = useState(DEFAULT_SYSTEM_PROMPT);
  const [speechProviderId, setSpeechProviderId] = useState("");
  const [speechModelName, setSpeechModelName] = useState("");
  const [speechLocale, setSpeechLocale] = useState("en-US");
  const [speechNativeVoice, setSpeechNativeVoice] = useState("");
  const [voiceEnabled, setVoiceEnabled] = useState(true);

  const { data: providers = [] } = useQuery<ModelProvider[]>({
    queryKey: ["voiceChatProviders"],
    queryFn: listProviders,
  });
  const { data: models = [] } = useQuery<ModelDefinition[]>({
    queryKey: ["voiceChatModels"],
    queryFn: listModelDefinitions,
  });

  useEffect(() => {
    void Promise.all([
      getSetting(SPEECH_PROVIDER_KEY),
      getSetting(SPEECH_MODEL_KEY),
      getSetting(SPEECH_LOCALE_KEY),
      getSetting(SPEECH_NATIVE_VOICE_KEY),
      getSetting(SPEECH_ENABLE_MIC_KEY),
    ]).then(([providerSetting, modelSetting, localeSetting, nativeVoiceSetting, micEnabledSetting]) => {
      if (providerSetting) setSpeechProviderId(providerSetting);
      if (modelSetting) setSpeechModelName(modelSetting);
      if (localeSetting) setSpeechLocale(localeSetting);
      if (nativeVoiceSetting) setSpeechNativeVoice(nativeVoiceSetting);
      setVoiceEnabled(parseBooleanSetting(micEnabledSetting, true));
    });
  }, []);

  const enabledProviders = useMemo(
    () => providers.filter((provider) => provider.enabled),
    [providers],
  );

  const enabledModels = useMemo(
    () => models.filter((model) => model.enabled),
    [models],
  );

  const combinedModelOptions = useMemo(
    () =>
      enabledModels
        .map((model) => {
          const provider = enabledProviders.find((entry) => entry.id === model.provider_id);
          if (!provider) {
            return null;
          }
          return {
            value: `${provider.id}::${model.name}`,
            label: `${provider.name} / ${model.name}`,
            providerId: provider.id,
            modelName: model.name,
          };
        })
        .filter((entry): entry is VoiceChatModelOption => Boolean(entry)),
    [enabledModels, enabledProviders],
  );

  useEffect(() => {
    if ((!providerId || !modelName) && combinedModelOptions.length > 0) {
      setProviderId(combinedModelOptions[0].providerId);
      setModelName(combinedModelOptions[0].modelName);
    }
  }, [providerId, modelName, combinedModelOptions]);

  const selectedModelValue = providerId && modelName ? `${providerId}::${modelName}` : "";

  const setSelectedModelValue = (value: string) => {
    const [nextProviderId, nextModelName] = value.split("::");
    setProviderId(nextProviderId ?? "");
    setModelName(nextModelName ?? "");
  };

  return {
    providerId,
    modelName,
    systemPrompt,
    setSystemPrompt,
    speechProviderId,
    speechModelName,
    speechLocale,
    speechNativeVoice,
    voiceEnabled,
    combinedModelOptions,
    selectedModelValue,
    setSelectedModelValue,
  };
}
