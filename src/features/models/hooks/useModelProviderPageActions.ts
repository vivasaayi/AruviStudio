import { useMutation, type QueryClient } from "@tanstack/react-query";
import type { Dispatch, SetStateAction } from "react";
import {
  browseForLocalModelFile,
  createModelDefinition,
  createProvider,
  deleteModelDefinition,
  deleteProvider,
  installManagedLocalModel,
  registerLocalRuntimeModel,
  setSetting,
  testProviderConnectivity,
  updateModelDefinition,
  updateProvider,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  LEGACY_WHISPER_PLACEHOLDER_PATH,
  MANAGED_LOCAL_MODELS,
  SPEECH_MODEL_KEY,
  SPEECH_PROVIDER_KEY,
} from "../lib/modelProviderConstants";
import { splitCommaSeparated } from "../lib/modelProviderFormUtils";

type ManagedLocalModelEntry = (typeof MANAGED_LOCAL_MODELS)[number];

export type ProviderFormState = {
  name: string;
  providerType: string;
  baseUrl: string;
  authSecretRef: string;
};

export type ModelFormState = {
  providerId: string;
  name: string;
  contextWindow: string;
  capabilityTags: string;
  notes: string;
};

export type ProviderEditFormState = ProviderFormState & {
  enabled: boolean;
};

export type ModelEditFormState = ModelFormState & {
  enabled: boolean;
};

export type ProviderTestResults = Record<string, { status: string; message: string }>;

type UseModelProviderPageActionsArgs = {
  queryClient: QueryClient;
  providers: ModelProvider[];
  form: ProviderFormState;
  setForm: Dispatch<SetStateAction<ProviderFormState>>;
  modelForm: ModelFormState;
  setModelForm: Dispatch<SetStateAction<ModelFormState>>;
  setShowForm: Dispatch<SetStateAction<boolean>>;
  setShowModelForm: Dispatch<SetStateAction<boolean>>;
  setTestResults: Dispatch<SetStateAction<ProviderTestResults>>;
  editingProvider: ModelProvider | null;
  setEditingProvider: Dispatch<SetStateAction<ModelProvider | null>>;
  providerEditForm: ProviderEditFormState;
  setProviderEditForm: Dispatch<SetStateAction<ProviderEditFormState>>;
  editingModel: ModelDefinition | null;
  setEditingModel: Dispatch<SetStateAction<ModelDefinition | null>>;
  modelEditForm: ModelEditFormState;
  setModelEditForm: Dispatch<SetStateAction<ModelEditFormState>>;
  setProviderError: Dispatch<SetStateAction<string | null>>;
  setProviderSuccess: Dispatch<SetStateAction<string | null>>;
  setModelError: Dispatch<SetStateAction<string | null>>;
  setModelSuccess: Dispatch<SetStateAction<string | null>>;
};

export function useModelProviderPageActions({
  queryClient,
  providers,
  form,
  setForm,
  modelForm,
  setModelForm,
  setShowForm,
  setShowModelForm,
  setTestResults,
  editingProvider,
  setEditingProvider,
  providerEditForm,
  setProviderEditForm,
  editingModel,
  setEditingModel,
  modelEditForm,
  setModelEditForm,
  setProviderError,
  setProviderSuccess,
  setModelError,
  setModelSuccess,
}: UseModelProviderPageActionsArgs) {
  const refreshModelQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["providers"] }),
      queryClient.invalidateQueries({ queryKey: ["model-definitions"] }),
    ]);
  };

  const applySpeechSettings = async (providerId: string, modelName: string) => {
    await Promise.all([
      setSetting(SPEECH_PROVIDER_KEY, providerId),
      setSetting(SPEECH_MODEL_KEY, modelName),
    ]);
    setProviderSuccess(`Speech settings now use ${modelName}.`);
    setProviderError(null);
  };

  const installLocalModelMutation = useMutation({
    mutationFn: (entry: ManagedLocalModelEntry) =>
      installManagedLocalModel({
        providerName: entry.providerName,
        modelName: entry.modelName,
        downloadUrl: entry.downloadUrl,
        fileName: entry.fileName,
        capabilityTags: ["speech_to_text", "transcription", "audio", "local_runtime"],
        notes: entry.notes,
      }),
    onSuccess: async (result, entry) => {
      await refreshModelQueries();
      await applySpeechSettings(result.provider.id, result.model_definition.name);
      setProviderSuccess(`${entry.displayName} is ready at ${result.file_path} and set as the active speech model.`);
      setProviderError(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const registerLocalModelMutation = useMutation({
    mutationFn: async (entry: ManagedLocalModelEntry) => {
      const selectedPath = await browseForLocalModelFile();
      if (!selectedPath) {
        throw new Error("Model registration cancelled.");
      }
      return registerLocalRuntimeModel({
        providerName: entry.providerName,
        modelName: entry.modelName,
        modelPath: selectedPath,
        capabilityTags: ["speech_to_text", "transcription", "audio", "local_runtime"],
        notes: entry.notes,
      });
    },
    onSuccess: async (result, entry) => {
      await refreshModelQueries();
      await applySpeechSettings(result.provider.id, result.model_definition.name);
      setProviderSuccess(`${entry.displayName} registered from ${result.file_path} and set as the active speech model.`);
      setProviderError(null);
    },
    onError: (error) => {
      if (String(error).includes("cancelled")) {
        return;
      }
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const createMutation = useMutation({
    mutationFn: () => createProvider(form),
    onSuccess: async (provider) => {
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      setProviderSuccess(`Provider "${provider.name}" added.`);
      setProviderError(null);
      setForm({
        name: "",
        providerType: "openai_compatible",
        baseUrl: "http://localhost:1234/v1",
        authSecretRef: "",
      });
      setShowForm(false);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const createModelMutation = useMutation({
    mutationFn: () =>
      createModelDefinition({
        providerId: modelForm.providerId,
        name: modelForm.name,
        contextWindow: modelForm.contextWindow ? Number(modelForm.contextWindow) : undefined,
        capabilityTags: splitCommaSeparated(modelForm.capabilityTags),
        notes: modelForm.notes.trim() || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess(`Model "${modelForm.name}" added.`);
      setModelError(null);
      setModelForm({ providerId: "", name: "", contextWindow: "", capabilityTags: "", notes: "" });
      setShowModelForm(false);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const updateProviderMutation = useMutation({
    mutationFn: () =>
      updateProvider({
        id: editingProvider!.id,
        name: providerEditForm.name,
        providerType: providerEditForm.providerType,
        baseUrl: providerEditForm.baseUrl,
        authSecretRef: providerEditForm.authSecretRef.trim()
          ? providerEditForm.authSecretRef.trim()
          : undefined,
        enabled: providerEditForm.enabled,
      }),
    onSuccess: async (provider) => {
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      setProviderSuccess(`Provider "${provider.name}" updated.`);
      setProviderError(null);
      setEditingProvider(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const deleteProviderMutation = useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: async () => {
      await refreshModelQueries();
      setProviderSuccess("Provider deleted.");
      setProviderError(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const updateModelMutation = useMutation({
    mutationFn: () =>
      updateModelDefinition({
        id: editingModel!.id,
        providerId: modelEditForm.providerId,
        name: modelEditForm.name,
        contextWindow: modelEditForm.contextWindow ? Number(modelEditForm.contextWindow) : undefined,
        capabilityTags: splitCommaSeparated(modelEditForm.capabilityTags),
        notes: modelEditForm.notes.trim() || undefined,
        enabled: modelEditForm.enabled,
      }),
    onSuccess: async (model) => {
      await queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess(`Model "${model.name}" updated.`);
      setModelError(null);
      setEditingModel(null);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const deleteModelMutation = useMutation({
    mutationFn: (id: string) => deleteModelDefinition(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess("Model deleted.");
      setModelError(null);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const testConnectivity = async (id: string) => {
    setTestResults((prev) => ({
      ...prev,
      [id]: { status: "testing", message: "Testing..." },
    }));
    try {
      const result = await testProviderConnectivity(id);
      setTestResults((prev) => ({
        ...prev,
        [id]: { status: "success", message: result },
      }));
    } catch (err: unknown) {
      setTestResults((prev) => ({
        ...prev,
        [id]: { status: "error", message: String(err) },
      }));
    }
  };

  const applyPreset = (preset: "deepseek" | "lm_studio") => {
    if (preset === "deepseek") {
      setForm({
        name: "DeepSeek (Hosted)",
        providerType: "openai_compatible",
        baseUrl: "https://api.deepseek.com/v1",
        authSecretRef: "",
      });
    } else {
      setForm({
        name: "LM Studio (Local)",
        providerType: "openai_compatible",
        baseUrl: "http://localhost:1234/v1",
        authSecretRef: "",
      });
    }
    setProviderError(null);
    setProviderSuccess(null);
    setShowForm(true);
  };

  const visibleProviders = providers.filter(
    (provider) =>
      !(
        provider.provider_type === "local_runtime" &&
        provider.base_url === LEGACY_WHISPER_PLACEHOLDER_PATH
      ),
  );

  const startEditingProvider = (provider: ModelProvider) => {
    setEditingProvider(provider);
    setProviderEditForm({
      name: provider.name,
      providerType: provider.provider_type,
      baseUrl: provider.base_url,
      authSecretRef: "",
      enabled: provider.enabled,
    });
  };

  const startEditingModel = (model: ModelDefinition) => {
    setEditingModel(model);
    setModelEditForm({
      providerId: model.provider_id,
      name: model.name,
      contextWindow: model.context_window ? String(model.context_window) : "",
      capabilityTags: model.capability_tags.join(", "),
      notes: model.notes,
      enabled: model.enabled,
    });
  };

  return {
    applyPreset,
    applySpeechSettings,
    createModelMutation,
    createMutation,
    deleteModelMutation,
    deleteProviderMutation,
    installLocalModelMutation,
    registerLocalModelMutation,
    startEditingModel,
    startEditingProvider,
    testConnectivity,
    updateModelMutation,
    updateProviderMutation,
    visibleProviders,
  };
}
