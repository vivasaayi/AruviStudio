import { useEffect, useMemo, useRef, useState } from "react";
import { Alert } from "react-native";
import * as FileSystem from "expo-file-system";
import * as SecureStore from "expo-secure-store";
import { initWhisper, type WhisperContext } from "whisper.rn";
import { describeError } from "../lib/mobileFormatters";
import {
  assertWhisperNativeModuleAvailable,
  normalizeWhisperLanguage,
  whisperModelDirectory,
  WHISPER_MODELS,
  type InstalledWhisperModel,
  type WhisperModelOption,
} from "../lib/mobileVoice";

type MobileWhisperControllerInput = {
  locale: string;
  installedModelsStorageKey: string;
  selectedModelStorageKey: string;
  onStatusChange: (status: string) => void;
};

export function useMobileWhisperController({
  locale,
  installedModelsStorageKey,
  selectedModelStorageKey,
  onStatusChange,
}: MobileWhisperControllerInput) {
  const whisperContextRef = useRef<{ modelId: string; uri: string; context: WhisperContext } | null>(null);
  const [selectedWhisperModelId, setSelectedWhisperModelId] = useState(WHISPER_MODELS[0].id);
  const [installedWhisperModels, setInstalledWhisperModels] = useState<Record<string, InstalledWhisperModel>>({});
  const [modelInstallStatus, setModelInstallStatus] = useState("No on-device model installed yet.");
  const [modelInstallProgress, setModelInstallProgress] = useState<number | null>(null);
  const [modelInstallBusyId, setModelInstallBusyId] = useState<string | null>(null);

  const selectedWhisperModel = useMemo(() => {
    return WHISPER_MODELS.find((model) => model.id === selectedWhisperModelId) ?? WHISPER_MODELS[0];
  }, [selectedWhisperModelId]);

  const installedSelectedWhisperModel = installedWhisperModels[selectedWhisperModel.id];
  const firstInstalledWhisperModel = Object.values(installedWhisperModels)[0];
  const activeLocalWhisperModel = installedSelectedWhisperModel ?? firstInstalledWhisperModel;
  const canUseLocalSpeech = Boolean(activeLocalWhisperModel?.uri);

  useEffect(() => {
    return () => {
      const currentContext = whisperContextRef.current;
      whisperContextRef.current = null;
      void currentContext?.context.release();
    };
  }, []);

  const setVerifiedInstalledModels = (nextModels: Record<string, InstalledWhisperModel>) => {
    setInstalledWhisperModels(nextModels);
    setModelInstallStatus(
      Object.keys(nextModels).length
        ? "On-device Whisper model is available."
        : "No on-device model installed yet.",
    );
  };

  const persistInstalledWhisperModels = async (nextModels: Record<string, InstalledWhisperModel>) => {
    setInstalledWhisperModels(nextModels);
    await SecureStore.setItemAsync(installedModelsStorageKey, JSON.stringify(nextModels));
  };

  const selectWhisperModel = async (modelId: string) => {
    setSelectedWhisperModelId(modelId);
    await SecureStore.setItemAsync(selectedModelStorageKey, modelId);
  };

  const installWhisperModel = async (model: WhisperModelOption) => {
    try {
      setModelInstallBusyId(model.id);
      setModelInstallProgress(0);
      setModelInstallStatus(`Preparing ${model.label}...`);
      const directory = whisperModelDirectory();
      await FileSystem.makeDirectoryAsync(directory, { intermediates: true });
      const destinationUri = `${directory}${model.fileName}`;
      const existingInfo = await FileSystem.getInfoAsync(destinationUri);

      if (existingInfo.exists) {
        const nextModels = {
          ...installedWhisperModels,
          [model.id]: {
            id: model.id,
            uri: destinationUri,
            fileName: model.fileName,
            installedAt: new Date().toISOString(),
            sizeBytes: "size" in existingInfo ? existingInfo.size : undefined,
          },
        };
        await persistInstalledWhisperModels(nextModels);
        await selectWhisperModel(model.id);
        setModelInstallProgress(100);
        setModelInstallStatus(`${model.label} installed and selected for voice chat.`);
        return;
      }

      const download = FileSystem.createDownloadResumable(
        model.url,
        destinationUri,
        {},
        ({ totalBytesWritten, totalBytesExpectedToWrite }) => {
          if (totalBytesExpectedToWrite > 0) {
            setModelInstallProgress(Math.round((totalBytesWritten / totalBytesExpectedToWrite) * 100));
          }
        },
      );
      const result = await download.downloadAsync();
      if (!result?.uri) {
        throw new Error("Model download did not produce a local file.");
      }
      const downloadedInfo = await FileSystem.getInfoAsync(result.uri);
      const nextModels = {
        ...installedWhisperModels,
        [model.id]: {
          id: model.id,
          uri: result.uri,
          fileName: model.fileName,
          installedAt: new Date().toISOString(),
          sizeBytes: downloadedInfo.exists && "size" in downloadedInfo ? downloadedInfo.size : undefined,
        },
      };
      await persistInstalledWhisperModels(nextModels);
      await selectWhisperModel(model.id);
      setModelInstallProgress(100);
      setModelInstallStatus(`${model.label} installed and selected for voice chat.`);
    } catch (error) {
      const message = describeError(error);
      setModelInstallStatus(message);
      Alert.alert("Model install failed", message);
    } finally {
      setModelInstallBusyId(null);
    }
  };

  const removeWhisperModel = async (model: WhisperModelOption) => {
    try {
      const installedModel = installedWhisperModels[model.id];
      if (installedModel?.uri) {
        const info = await FileSystem.getInfoAsync(installedModel.uri);
        if (info.exists) {
          await FileSystem.deleteAsync(installedModel.uri, { idempotent: true });
        }
      }
      const nextModels = { ...installedWhisperModels };
      delete nextModels[model.id];
      await persistInstalledWhisperModels(nextModels);
      const currentContext = whisperContextRef.current;
      if (currentContext?.modelId === model.id) {
        whisperContextRef.current = null;
        await currentContext.context.release();
      }
      setModelInstallProgress(null);
      setModelInstallStatus(`${model.label} removed.`);
    } catch (error) {
      const message = describeError(error);
      setModelInstallStatus(message);
      Alert.alert("Remove failed", message);
    }
  };

  const getWhisperContext = async (modelId: string, modelUri: string) => {
    const currentContext = whisperContextRef.current;
    if (currentContext?.modelId === modelId && currentContext.uri === modelUri) {
      return currentContext.context;
    }
    if (currentContext) {
      await currentContext.context.release();
    }
    onStatusChange("Loading local Whisper model...");
    assertWhisperNativeModuleAvailable();
    const context = await initWhisper({
      filePath: modelUri,
      useGpu: true,
      useCoreMLIos: false,
    });
    whisperContextRef.current = { modelId, uri: modelUri, context };
    return context;
  };

  const transcribeWithLocalWhisper = async (audioUri: string) => {
    const installedModel = activeLocalWhisperModel;
    if (!installedModel?.uri) {
      throw new Error("Install the selected Whisper model before using on-device transcription.");
    }
    const context = await getWhisperContext(installedModel.id, installedModel.uri);
    onStatusChange("Transcribing on device...");
    const { promise } = context.transcribe(audioUri, {
      language: normalizeWhisperLanguage(locale),
      maxThreads: 4,
      onProgress: (progress: number) => onStatusChange(`Transcribing on device ${Math.round(progress)}%`),
    });
    const result = await promise;
    return result.result.trim();
  };

  return {
    selectedWhisperModel,
    selectedWhisperModelId,
    setSelectedWhisperModelId,
    installedWhisperModels,
    setVerifiedInstalledModels,
    modelInstallStatus,
    modelInstallProgress,
    modelInstallBusyId,
    activeLocalWhisperModel,
    canUseLocalSpeech,
    selectWhisperModel,
    installWhisperModel,
    removeWhisperModel,
    transcribeWithLocalWhisper,
  };
}
