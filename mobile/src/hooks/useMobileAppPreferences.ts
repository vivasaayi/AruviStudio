import { useEffect } from "react";
import * as FileSystem from "expo-file-system";
import * as SecureStore from "expo-secure-store";
import { MOBILE_TABS, type MobileTabId } from "../components/MobileBottomTabs";
import { MOBILE_STORAGE_KEYS } from "../lib/mobileStorageKeys";
import {
  parseInstalledWhisperModels,
  WHISPER_MODELS,
  type InstalledWhisperModel,
} from "../lib/mobileVoice";

type MobileAppPreferencesInput = {
  setActiveTab: (tab: MobileTabId) => void;
  setVoiceMode: (mode: "assistant" | "planner") => void;
  setReadReplies: (value: boolean) => void;
  setSelectedWhisperModelId: (modelId: string) => void;
  setVerifiedInstalledModels: (models: Record<string, InstalledWhisperModel>) => void;
};

export function useMobileAppPreferences({
  setActiveTab,
  setVoiceMode,
  setReadReplies,
  setSelectedWhisperModelId,
  setVerifiedInstalledModels,
}: MobileAppPreferencesInput) {
  useEffect(() => {
    let disposed = false;

    const loadSavedPreferences = async () => {
      const [
        savedActiveTab,
        savedVoiceMode,
        savedReadReplies,
        savedSelectedWhisperModelId,
        savedInstalledWhisperModels,
      ] = await Promise.all([
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.activeTab),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.voiceMode),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.readReplies),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.selectedWhisperModelId),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.installedWhisperModels),
      ]);
      if (disposed) return;
      if (MOBILE_TABS.some((tab) => tab.id === savedActiveTab)) {
        setActiveTab(savedActiveTab as MobileTabId);
      } else if (savedActiveTab === "chat") {
        setActiveTab("voice");
        void SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.activeTab, "voice");
      }
      if (savedVoiceMode === "assistant" || savedVoiceMode === "planner") {
        setVoiceMode(savedVoiceMode);
      }
      if (savedReadReplies === "true" || savedReadReplies === "false") {
        setReadReplies(savedReadReplies === "true");
      }
      if (
        typeof savedSelectedWhisperModelId === "string"
        && WHISPER_MODELS.some((model) => model.id === savedSelectedWhisperModelId)
      ) {
        setSelectedWhisperModelId(savedSelectedWhisperModelId);
      }

      const parsedInstalledModels = parseInstalledWhisperModels(savedInstalledWhisperModels);
      const verifiedInstalledModels: Record<string, InstalledWhisperModel> = {};
      await Promise.all(
        Object.values(parsedInstalledModels).map(async (model) => {
          const info = await FileSystem.getInfoAsync(model.uri);
          if (info.exists) {
            verifiedInstalledModels[model.id] = {
              ...model,
              sizeBytes: "size" in info ? info.size : model.sizeBytes,
            };
          }
        }),
      );
      if (disposed) return;
      setVerifiedInstalledModels(verifiedInstalledModels);
    };

    void loadSavedPreferences();
    return () => {
      disposed = true;
    };
  }, [
    setActiveTab,
    setReadReplies,
    setSelectedWhisperModelId,
    setVerifiedInstalledModels,
    setVoiceMode,
  ]);
}
