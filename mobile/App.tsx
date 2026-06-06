import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Keyboard,
  Linking,
  Platform,
  Pressable,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  TurboModuleRegistry,
  View,
} from "react-native";
import {
  AudioModule,
  AudioQuality,
  IOSOutputFormat,
  RecordingPresets,
  setAudioModeAsync,
  type RecordingOptions,
  useAudioRecorder,
  useAudioRecorderState,
} from "expo-audio";
import * as FileSystem from "expo-file-system";
import * as Speech from "expo-speech";
import * as SecureStore from "expo-secure-store";
import { WebView } from "react-native-webview";
import { initWhisper, type WhisperContext } from "whisper.rn";
import { PlannerMobileClient } from "./src/api/client";

type ActiveTab = "planner" | "products" | "voice" | "models" | "activity";
type VoiceMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
};

const TABS: Array<{ id: ActiveTab; label: string }> = [
  { id: "planner", label: "Planner" },
  { id: "products", label: "Products" },
  { id: "voice", label: "Voice" },
  { id: "models", label: "Models" },
  { id: "activity", label: "Activity" },
];

const STORAGE_KEYS = {
  baseUrl: "aruvi.mobile.base_url",
  token: "aruvi.mobile.token",
  providerId: "aruvi.mobile.provider_id",
  modelName: "aruvi.mobile.model_name",
  locale: "aruvi.mobile.locale",
  activeTab: "aruvi.mobile.active_tab",
  readReplies: "aruvi.mobile.read_replies",
  selectedWhisperModelId: "aruvi.mobile.selected_whisper_model_id",
  installedWhisperModels: "aruvi.mobile.installed_whisper_models",
};

type WhisperModelOption = {
  id: string;
  label: string;
  fileName: string;
  sizeLabel: string;
  description: string;
  url: string;
};

type InstalledWhisperModel = {
  id: string;
  uri: string;
  fileName: string;
  installedAt: string;
  sizeBytes?: number;
};

const WHISPER_MODELS: WhisperModelOption[] = [
  {
    id: "tiny-en-q5_1",
    label: "Whisper tiny.en Q5",
    fileName: "ggml-tiny.en-q5_1.bin",
    sizeLabel: "31 MB",
    description: "Fastest install and best first test for phone voice chat.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin",
  },
  {
    id: "tiny-en",
    label: "Whisper tiny.en",
    fileName: "ggml-tiny.en.bin",
    sizeLabel: "75 MB",
    description: "Small English model with better quality than the quantized tiny file.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
  },
  {
    id: "base-en-q5_1",
    label: "Whisper base.en Q5",
    fileName: "ggml-base.en-q5_1.bin",
    sizeLabel: "57 MB",
    description: "Better accuracy while staying reasonable for mobile storage.",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin",
  },
];

const VOICE_RECORDING_OPTIONS: RecordingOptions = Platform.OS === "ios"
  ? {
      extension: ".wav",
      sampleRate: 16000,
      numberOfChannels: 1,
      bitRate: 256000,
      android: RecordingPresets.HIGH_QUALITY.android,
      ios: {
        extension: ".wav",
        outputFormat: IOSOutputFormat.LINEARPCM,
        audioQuality: AudioQuality.MAX,
        sampleRate: 16000,
        linearPCMBitDepth: 16,
        linearPCMIsBigEndian: false,
        linearPCMIsFloat: false,
      },
      web: {
        mimeType: "audio/wav",
      },
    }
  : RecordingPresets.HIGH_QUALITY;

function formatBytes(bytes?: number) {
  if (!bytes || bytes <= 0) return "";
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

function parseInstalledWhisperModels(raw: string | null): Record<string, InstalledWhisperModel> {
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as Record<string, InstalledWhisperModel>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function normalizeWhisperLanguage(locale: string) {
  const normalized = locale.trim().toLowerCase();
  if (!normalized) return "auto";
  return normalized.split(/[-_]/)[0] || "auto";
}

function whisperModelDirectory() {
  if (!FileSystem.documentDirectory) {
    throw new Error("App document storage is unavailable on this device.");
  }
  return `${FileSystem.documentDirectory}models/whisper/`;
}

function describeError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object") {
    const errorRecord = error as Record<string, unknown>;
    const directMessage = errorRecord.message ?? errorRecord.error ?? errorRecord.description ?? errorRecord.reason;
    if (typeof directMessage === "string" && directMessage.trim()) {
      return directMessage;
    }
    try {
      return JSON.stringify(errorRecord);
    } catch {
      return Object.prototype.toString.call(error);
    }
  }
  return String(error);
}

function assertWhisperNativeModuleAvailable() {
  if (!TurboModuleRegistry.get("RNWhisper")) {
    throw new Error(
      "On-device Whisper is not available in this app build. Rebuild and reinstall the Expo dev app after installing whisper.rn.",
    );
  }
}

type ConnectionValues = {
  baseUrl?: string;
  token?: string;
  providerId?: string;
  modelName?: string;
  locale?: string;
};

function parseConnectionUrl(url: string): ConnectionValues | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "aruvi-planner-mobile:") {
      return null;
    }
    const params = parsed.searchParams;
    const values: ConnectionValues = {
      baseUrl: params.get("baseUrl") || params.get("base_url") || undefined,
      token: params.get("token") || undefined,
      providerId: params.get("providerId") || params.get("provider_id") || undefined,
      modelName: params.get("modelName") || params.get("model_name") || undefined,
      locale: params.get("locale") || undefined,
    };
    return Object.values(values).some(Boolean) ? values : null;
  } catch {
    return null;
  }
}

function buildRemoteScript(payload: {
  token: string;
  provider: string;
  model: string;
  locale: string;
  activeTab: ActiveTab;
}) {
  return `
    (function () {
      try {
        var config = ${JSON.stringify(payload)};
        if (config.token) window.localStorage.setItem("aruvi.remote.token", config.token);
        else window.localStorage.removeItem("aruvi.remote.token");
        if (config.provider) window.localStorage.setItem("aruvi.remote.provider", config.provider);
        else window.localStorage.removeItem("aruvi.remote.provider");
        if (config.model) window.localStorage.setItem("aruvi.remote.model", config.model);
        else window.localStorage.removeItem("aruvi.remote.model");
        if (config.locale) window.localStorage.setItem("aruvi.remote.locale", config.locale);
        else window.localStorage.removeItem("aruvi.remote.locale");
        window.localStorage.setItem("aruvi.remote.active_tab", config.activeTab);

        var styleId = "aruvi-native-shell-style";
        var style = document.getElementById(styleId);
        if (!style) {
          style = document.createElement("style");
          style.id = styleId;
          style.textContent = [
            ".topbar,.tabbar{display:none!important}",
            ".shell{min-height:100vh!important;display:block!important}",
            ".main{padding:10px 10px 14px 10px!important}",
            ".tab-panel.active{display:block!important}",
            "body{background:#101214!important}"
          ].join("");
          document.head.appendChild(style);
        }

        var activate = function () {
          var button = document.querySelector('.tab-button[data-tab="' + config.activeTab + '"]');
          if (button) button.click();
        };
        activate();
        window.setTimeout(activate, 150);
        window.setTimeout(activate, 500);
      } catch (error) {}
    })();
    true;
  `;
}

function buildRemoteVoiceSubmitScript(transcript: string) {
  return `
    (function () {
      try {
        var transcript = ${JSON.stringify(transcript)};
        var voiceTab = document.querySelector('.tab-button[data-tab="voice"]');
        if (voiceTab) voiceTab.click();
        var composer = document.getElementById("voiceComposer");
        if (composer) {
          composer.value = transcript;
          composer.dispatchEvent(new Event("input", { bubbles: true }));
        }
        var sendButton = document.getElementById("voiceSendButton");
        if (sendButton) sendButton.click();
      } catch (error) {}
    })();
    true;
  `;
}

function createId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.round(Math.random() * 100000)}`;
}

export default function App() {
  const webViewRef = useRef<WebView>(null);
  const whisperContextRef = useRef<{ modelId: string; uri: string; context: WhisperContext } | null>(null);
  const audioRecorder = useAudioRecorder(VOICE_RECORDING_OPTIONS);
  const recorderState = useAudioRecorderState(audioRecorder, 250);
  const [baseUrl, setBaseUrl] = useState("http://100.66.32.111:8787");
  const [token, setToken] = useState("");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [locale, setLocale] = useState("en-US");
  const [activeTab, setActiveTab] = useState<ActiveTab>("planner");
  const [readReplies, setReadReplies] = useState(true);
  const [selectedWhisperModelId, setSelectedWhisperModelId] = useState(WHISPER_MODELS[0].id);
  const [installedWhisperModels, setInstalledWhisperModels] = useState<Record<string, InstalledWhisperModel>>({});
  const [modelInstallStatus, setModelInstallStatus] = useState("No on-device model installed yet.");
  const [modelInstallProgress, setModelInstallProgress] = useState<number | null>(null);
  const [modelInstallBusyId, setModelInstallBusyId] = useState<string | null>(null);
  const [isSetupOpen, setIsSetupOpen] = useState(false);
  const [webReloadKey, setWebReloadKey] = useState(0);
  const [isSaving, setIsSaving] = useState(false);
  const [isVoiceBusy, setIsVoiceBusy] = useState(false);
  const [nativeVoiceStatus, setNativeVoiceStatus] = useState("Ready");
  const [lastVoiceTranscript, setLastVoiceTranscript] = useState("");
  const [voiceDraft, setVoiceDraft] = useState("");
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const [voiceMessages, setVoiceMessages] = useState<VoiceMessage[]>([
    {
      id: "assistant-welcome",
      role: "assistant",
      content: "Ready when you are. Tap the mic and speak naturally.",
    },
  ]);

  const remoteUrl = useMemo(() => {
    const trimmed = baseUrl.trim().replace(/\/+$/, "");
    return trimmed ? `${trimmed}/remote` : "about:blank";
  }, [baseUrl]);

  const mobileClient = useMemo(() => {
    return new PlannerMobileClient(baseUrl.trim(), token.trim());
  }, [baseUrl, token]);

  const selectedWhisperModel = useMemo(() => {
    return WHISPER_MODELS.find((model) => model.id === selectedWhisperModelId) ?? WHISPER_MODELS[0];
  }, [selectedWhisperModelId]);

  const installedSelectedWhisperModel = installedWhisperModels[selectedWhisperModel.id];
  const firstInstalledWhisperModel = Object.values(installedWhisperModels)[0];
  const activeLocalWhisperModel = installedSelectedWhisperModel ?? firstInstalledWhisperModel;
  const canUseLocalSpeech = Boolean(activeLocalWhisperModel?.uri);

  const remoteBootstrapScript = useMemo(() => {
    return buildRemoteScript({
      token: token.trim(),
      provider: providerId.trim(),
      model: modelName.trim(),
      locale: locale.trim(),
      activeTab,
    });
  }, [activeTab, locale, modelName, providerId, token]);

  useEffect(() => {
    void setAudioModeAsync({
      allowsRecording: true,
      playsInSilentMode: true,
    });

    return () => {
      const currentContext = whisperContextRef.current;
      whisperContextRef.current = null;
      void currentContext?.context.release();
      void Speech.stop();
    };
  }, []);

  useEffect(() => {
    const showEvent = Platform.OS === "ios" ? "keyboardWillShow" : "keyboardDidShow";
    const hideEvent = Platform.OS === "ios" ? "keyboardWillHide" : "keyboardDidHide";
    const showSubscription = Keyboard.addListener(showEvent, (event) => {
      setKeyboardHeight(event.endCoordinates.height);
    });
    const hideSubscription = Keyboard.addListener(hideEvent, () => {
      setKeyboardHeight(0);
    });

    return () => {
      showSubscription.remove();
      hideSubscription.remove();
    };
  }, []);

  const applyConnectionValues = async (values: ConnectionValues) => {
    const next = {
      baseUrl: values.baseUrl?.trim(),
      token: values.token?.trim(),
      providerId: values.providerId?.trim(),
      modelName: values.modelName?.trim(),
      locale: values.locale?.trim(),
    };
    if (next.baseUrl) setBaseUrl(next.baseUrl);
    if (next.token) setToken(next.token);
    if (next.providerId !== undefined) setProviderId(next.providerId);
    if (next.modelName !== undefined) setModelName(next.modelName);
    if (next.locale) setLocale(next.locale);
    await Promise.all([
      next.baseUrl ? SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, next.baseUrl) : Promise.resolve(),
      next.token ? SecureStore.setItemAsync(STORAGE_KEYS.token, next.token) : Promise.resolve(),
      next.providerId !== undefined
        ? SecureStore.setItemAsync(STORAGE_KEYS.providerId, next.providerId)
        : Promise.resolve(),
      next.modelName !== undefined
        ? SecureStore.setItemAsync(STORAGE_KEYS.modelName, next.modelName)
        : Promise.resolve(),
      next.locale ? SecureStore.setItemAsync(STORAGE_KEYS.locale, next.locale) : Promise.resolve(),
    ]);
    setWebReloadKey((current) => current + 1);
  };

  useEffect(() => {
    let disposed = false;

    const loadSavedConnection = async () => {
      const [
        savedBaseUrl,
        savedToken,
        savedProviderId,
        savedModelName,
        savedLocale,
        savedActiveTab,
        savedReadReplies,
        savedSelectedWhisperModelId,
        savedInstalledWhisperModels,
      ] = await Promise.all([
        SecureStore.getItemAsync(STORAGE_KEYS.baseUrl),
        SecureStore.getItemAsync(STORAGE_KEYS.token),
        SecureStore.getItemAsync(STORAGE_KEYS.providerId),
        SecureStore.getItemAsync(STORAGE_KEYS.modelName),
        SecureStore.getItemAsync(STORAGE_KEYS.locale),
        SecureStore.getItemAsync(STORAGE_KEYS.activeTab),
        SecureStore.getItemAsync(STORAGE_KEYS.readReplies),
        SecureStore.getItemAsync(STORAGE_KEYS.selectedWhisperModelId),
        SecureStore.getItemAsync(STORAGE_KEYS.installedWhisperModels),
      ]);
      if (disposed) return;
      if (savedBaseUrl) setBaseUrl(savedBaseUrl);
      if (savedToken) setToken(savedToken);
      if (savedProviderId) setProviderId(savedProviderId);
      if (savedModelName) setModelName(savedModelName);
      if (savedLocale) setLocale(savedLocale);
      if (TABS.some((tab) => tab.id === savedActiveTab)) {
        setActiveTab(savedActiveTab as ActiveTab);
      } else if (savedActiveTab === "chat") {
        setActiveTab("voice");
        void SecureStore.setItemAsync(STORAGE_KEYS.activeTab, "voice");
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
      setInstalledWhisperModels(verifiedInstalledModels);
      setModelInstallStatus(
        Object.keys(verifiedInstalledModels).length
          ? "On-device Whisper model is available."
          : "No on-device model installed yet.",
      );

      const initialUrl = await Linking.getInitialURL();
      if (disposed || !initialUrl) return;
      const connectionValues = parseConnectionUrl(initialUrl);
      if (connectionValues) {
        await applyConnectionValues(connectionValues);
      }
    };

    const subscription = Linking.addEventListener("url", ({ url }) => {
      const connectionValues = parseConnectionUrl(url);
      if (connectionValues) {
        void applyConnectionValues(connectionValues);
      }
    });

    void loadSavedConnection();
    return () => {
      disposed = true;
      subscription.remove();
    };
  }, []);

  const saveConnection = async () => {
    try {
      setIsSaving(true);
      await Promise.all([
        SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, baseUrl.trim()),
        SecureStore.setItemAsync(STORAGE_KEYS.token, token.trim()),
        SecureStore.setItemAsync(STORAGE_KEYS.providerId, providerId.trim()),
        SecureStore.setItemAsync(STORAGE_KEYS.modelName, modelName.trim()),
        SecureStore.setItemAsync(STORAGE_KEYS.locale, locale.trim()),
      ]);
      setWebReloadKey((current) => current + 1);
      setIsSetupOpen(false);
    } catch (error) {
      Alert.alert("Save failed", describeError(error));
    } finally {
      setIsSaving(false);
    }
  };

  const switchTab = (nextTab: ActiveTab) => {
    setActiveTab(nextTab);
    void SecureStore.setItemAsync(STORAGE_KEYS.activeTab, nextTab);
    webViewRef.current?.injectJavaScript(
      buildRemoteScript({
        token: token.trim(),
        provider: providerId.trim(),
        model: modelName.trim(),
        locale: locale.trim(),
        activeTab: nextTab,
      }),
    );
  };

  const setReadRepliesPreference = async (nextValue: boolean) => {
    setReadReplies(nextValue);
    await SecureStore.setItemAsync(STORAGE_KEYS.readReplies, nextValue ? "true" : "false");
    if (!nextValue) {
      void Speech.stop();
      setNativeVoiceStatus("Reply reading off");
    }
  };

  const speakAssistantReply = (text: string) => {
    if (!readReplies || !text.trim()) return;
    const language = normalizeWhisperLanguage(locale);
    void Speech.stop();
    setNativeVoiceStatus("Reading reply...");
    Speech.speak(text, {
      language: language === "auto" ? undefined : language,
      rate: 0.95,
      onDone: () => setNativeVoiceStatus("Reply ready"),
      onStopped: () => setNativeVoiceStatus("Reply ready"),
      onError: () => setNativeVoiceStatus("Reply ready"),
    });
  };

  const submitVoicePrompt = async (prompt: string) => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using voice chat.");
      return;
    }

    const userMessage: VoiceMessage = {
      id: createId("voice-user"),
      role: "user",
      content: trimmed,
    };
    const history = voiceMessages
      .filter((message) => message.id !== "assistant-welcome")
      .slice(-18);
    setVoiceMessages((current) => [...current.filter((message) => message.id !== "assistant-welcome"), userMessage]);
    setVoiceDraft("");
    setNativeVoiceStatus("Thinking...");
    setIsVoiceBusy(true);

    try {
      const response = await mobileClient.runChatCompletion({
        provider_id: providerId.trim() || undefined,
        model_name: modelName.trim() || undefined,
        messages: [
          {
            role: "system",
            content: "You are Aruvi Studio's mobile voice assistant. Reply conversationally in one or two short sentences for spoken playback.",
          },
          ...history.map((message) => ({
            role: message.role,
            content: message.content,
          })),
          {
            role: "user",
            content: trimmed,
          },
        ],
        temperature: 0.7,
        max_tokens: 4096,
      });
      const assistantText = response.content.trim() || "(empty response)";
      const assistantMessage: VoiceMessage = {
        id: createId("voice-assistant"),
        role: "assistant",
        content: assistantText,
      };
      setVoiceMessages((current) => [...current, assistantMessage].slice(-24));
      if (readReplies) {
        speakAssistantReply(assistantText);
      } else {
        setNativeVoiceStatus("Reply ready");
      }
    } catch (error) {
      const message = describeError(error);
      setNativeVoiceStatus(message);
      Alert.alert("Voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const persistInstalledWhisperModels = async (nextModels: Record<string, InstalledWhisperModel>) => {
    setInstalledWhisperModels(nextModels);
    await SecureStore.setItemAsync(STORAGE_KEYS.installedWhisperModels, JSON.stringify(nextModels));
  };

  const selectWhisperModel = async (modelId: string) => {
    setSelectedWhisperModelId(modelId);
    await SecureStore.setItemAsync(STORAGE_KEYS.selectedWhisperModelId, modelId);
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
    setNativeVoiceStatus("Loading local Whisper model...");
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
    setNativeVoiceStatus("Transcribing on device...");
    const { promise } = context.transcribe(audioUri, {
      language: normalizeWhisperLanguage(locale),
      maxThreads: 4,
      onProgress: (progress: number) => setNativeVoiceStatus(`Transcribing on device ${Math.round(progress)}%`),
    });
    const result = await promise;
    return result.result.trim();
  };

  const transcribeNativeRecording = async (uri: string) => {
    return await transcribeWithLocalWhisper(uri);
  };

  const stopNativeVoiceRecording = async () => {
    try {
      setIsVoiceBusy(true);
      setNativeVoiceStatus("Stopping...");
      await audioRecorder.stop();
      const recordingUri = audioRecorder.uri ?? audioRecorder.getStatus().url;
      if (!recordingUri) {
        throw new Error("Recording did not produce an audio file.");
      }
      setNativeVoiceStatus("Transcribing...");
      const transcript = await transcribeNativeRecording(recordingUri);
      if (!transcript) {
        setLastVoiceTranscript("");
        setNativeVoiceStatus("No speech detected");
        return;
      }
      setLastVoiceTranscript(transcript);
      setVoiceDraft(transcript);
      setNativeVoiceStatus("Sending...");
      await submitVoicePrompt(transcript);
    } catch (error) {
      const message = describeError(error);
      setNativeVoiceStatus(message);
      Alert.alert("Voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const startNativeVoiceRecording = async () => {
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using voice chat.");
      return;
    }
    if (!canUseLocalSpeech) {
      Alert.alert("Install model first", "Install an on-device Whisper model before using voice recording.");
      switchTab("models");
      return;
    }
    try {
      setIsVoiceBusy(true);
      void Speech.stop();
      const permission = await AudioModule.requestRecordingPermissionsAsync();
      if (!permission.granted) {
        throw new Error("Microphone permission was denied.");
      }
      await setAudioModeAsync({
        allowsRecording: true,
        playsInSilentMode: true,
      });
      await audioRecorder.prepareToRecordAsync();
      audioRecorder.record();
      setLastVoiceTranscript("");
      setNativeVoiceStatus("Listening...");
    } catch (error) {
      const message = describeError(error);
      setNativeVoiceStatus(message);
      Alert.alert("Voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const toggleNativeVoiceRecording = async () => {
    if (recorderState.isRecording) {
      await stopNativeVoiceRecording();
    } else {
      await startNativeVoiceRecording();
    }
  };

  const shouldShowSetup = isSetupOpen || !token.trim();
  const nativeVoiceButtonDisabled = isVoiceBusy || !token.trim() || !canUseLocalSpeech;
  const speechModelDescription = canUseLocalSpeech
    ? `Using ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModel?.id)?.label ?? "Whisper"} on this phone for speech-to-text.`
    : "Install a Whisper model to enable on-device speech-to-text.";
  const speechModelLabel = canUseLocalSpeech
    ? `On-device ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModel?.id)?.label ?? "Whisper"}`
    : "Install Whisper";
  const isVoiceKeyboardOpen = activeTab === "voice" && keyboardHeight > 0;

  const renderVoiceScreen = () => (
    <View style={styles.voiceScreen}>
      <View style={styles.voiceTopBand}>
        <View style={styles.voiceTopCopy}>
          <Text style={styles.voiceTitle}>Voice</Text>
          <Text style={styles.voiceSubtitle} numberOfLines={1}>{nativeVoiceStatus}</Text>
        </View>
        <Pressable style={styles.runtimeChip} onPress={() => switchTab("models")}>
          <Text style={styles.runtimeChipText} numberOfLines={1}>{speechModelLabel}</Text>
        </Pressable>
      </View>

      <ScrollView
        style={styles.voiceConversation}
        contentContainerStyle={styles.voiceConversationContent}
        keyboardShouldPersistTaps="handled"
      >
        {voiceMessages.map((message) => (
          <View
            key={message.id}
            style={[
              styles.voiceBubble,
              message.role === "user" ? styles.voiceBubbleUser : styles.voiceBubbleAssistant,
            ]}
          >
            <Text
              style={[
                styles.voiceBubbleText,
                message.role === "user" ? styles.voiceBubbleTextUser : styles.voiceBubbleTextAssistant,
              ]}
            >
              {message.content}
            </Text>
          </View>
        ))}
      </ScrollView>

      <View
        style={[
          styles.voiceComposerPanel,
          isVoiceKeyboardOpen && { marginBottom: keyboardHeight + 8 },
        ]}
      >
        <View style={styles.voiceComposerHeader}>
          <Text style={styles.voiceComposerLabel} numberOfLines={1}>
            {recorderState.isRecording ? "Listening" : isVoiceBusy ? nativeVoiceStatus : "Voice transcript"}
          </Text>
          <Text style={styles.voiceComposerStatus} numberOfLines={1}>
            {token.trim() ? speechModelLabel : "Setup required"}
          </Text>
        </View>
        <TextInput
          style={styles.voiceComposerInput}
          value={voiceDraft}
          onChangeText={setVoiceDraft}
          placeholder="Speak or type a message"
          placeholderTextColor="#7f8a9c"
          multiline
          textAlignVertical="top"
        />
        <View style={styles.voiceComposerActions}>
          <Pressable
            style={[styles.voiceClearButton, (!voiceDraft.trim() || isVoiceBusy || recorderState.isRecording) && styles.buttonDisabled]}
            onPress={() => setVoiceDraft("")}
            disabled={!voiceDraft.trim() || isVoiceBusy || recorderState.isRecording}
          >
            <Text style={styles.voiceClearButtonText}>Clear</Text>
          </Pressable>
          <View style={styles.voiceComposerSpacer} />
          <Pressable
            style={[
              styles.voiceMicButton,
              recorderState.isRecording && styles.voiceMicButtonRecording,
              nativeVoiceButtonDisabled && !recorderState.isRecording && styles.buttonDisabled,
            ]}
            onPress={() => void toggleNativeVoiceRecording()}
            disabled={nativeVoiceButtonDisabled && !recorderState.isRecording}
          >
            <Text style={styles.voiceMicButtonText}>{recorderState.isRecording ? "Stop" : "Mic"}</Text>
          </Pressable>
          <Pressable
            style={[
              styles.voiceSendButton,
              (!voiceDraft.trim() || isVoiceBusy || recorderState.isRecording) && styles.buttonDisabled,
            ]}
            onPress={() => void submitVoicePrompt(voiceDraft)}
            disabled={!voiceDraft.trim() || isVoiceBusy || recorderState.isRecording}
          >
            <Text style={styles.voiceSendButtonText}>↑</Text>
          </Pressable>
        </View>
        <Text style={styles.voiceControlHint} numberOfLines={1}>
          {token.trim() ? speechModelDescription : "Save setup first."}
        </Text>
      </View>
    </View>
  );

  const renderModelManager = () => (
    <ScrollView style={styles.modelPage} contentContainerStyle={styles.modelPageContent}>
      <View style={styles.modelHeader}>
        <Text style={styles.sectionTitle}>Speech Model</Text>
        <Text style={styles.sectionText}>
          Install a Whisper model on this phone for private speech-to-text. Voice recording uses the selected local model.
        </Text>
      </View>

      <View style={styles.runtimePanel}>
        <Text style={styles.panelLabel}>On-device transcription</Text>
        <Text style={styles.modelStatusText}>{speechModelDescription}</Text>
      </View>

      <View style={styles.modelStatusPanel}>
        <View style={styles.modelStatusRow}>
          <Text style={styles.modelStatusText}>{modelInstallStatus}</Text>
          {modelInstallProgress !== null ? (
            <Text style={styles.progressText}>{modelInstallProgress}%</Text>
          ) : null}
        </View>
        {modelInstallProgress !== null ? (
          <View style={styles.progressTrack}>
            <View style={[styles.progressFill, { width: `${Math.min(100, Math.max(0, modelInstallProgress))}%` }]} />
          </View>
        ) : null}
      </View>

      <View style={styles.modelList}>
        {WHISPER_MODELS.map((model) => {
          const installedModel = installedWhisperModels[model.id];
          const isSelected = selectedWhisperModel.id === model.id;
          const isBusy = modelInstallBusyId === model.id;
          return (
            <Pressable
              key={model.id}
              style={[styles.modelCard, isSelected && styles.modelCardSelected]}
              onPress={() => void selectWhisperModel(model.id)}
            >
              <View style={styles.modelCardHeader}>
                <View style={styles.modelTitleBlock}>
                  <Text style={styles.modelTitle}>{model.label}</Text>
                  <Text style={styles.modelMeta}>
                    {model.sizeLabel}
                    {installedModel?.sizeBytes ? ` installed ${formatBytes(installedModel.sizeBytes)}` : ""}
                  </Text>
                </View>
                <Text style={[styles.installBadge, installedModel && styles.installBadgeActive]}>
                  {installedModel ? "Installed" : "Not installed"}
                </Text>
              </View>
              <Text style={styles.modelDescription}>{model.description}</Text>
              <View style={styles.modelActions}>
                <Pressable
                  style={[styles.smallButton, isSelected && styles.smallButtonActive]}
                  onPress={() => void selectWhisperModel(model.id)}
                >
                  <Text style={[styles.smallButtonText, isSelected && styles.smallButtonTextActive]}>
                    {isSelected ? "Selected" : "Select"}
                  </Text>
                </Pressable>
                {installedModel ? (
                  <Pressable style={styles.smallButton} onPress={() => void removeWhisperModel(model)}>
                    <Text style={styles.smallButtonText}>Remove</Text>
                  </Pressable>
                ) : (
                  <Pressable
                    style={[styles.smallButton, styles.smallButtonPrimary, isBusy && styles.buttonDisabled]}
                    onPress={() => void installWhisperModel(model)}
                    disabled={Boolean(modelInstallBusyId)}
                  >
                    <Text style={styles.smallButtonPrimaryText}>{isBusy ? "Installing" : "Install"}</Text>
                  </Pressable>
                )}
              </View>
            </Pressable>
          );
        })}
      </View>

      <Pressable style={styles.sourceButton} onPress={() => void Linking.openURL(selectedWhisperModel.url)}>
        <Text style={styles.sourceButtonText}>Open selected model source</Text>
      </Pressable>
    </ScrollView>
  );

  return (
    <SafeAreaView style={styles.safeArea}>
      <View style={styles.shell}>
        <View style={styles.header}>
          <View style={styles.titleRow}>
            <View style={styles.titleBlock}>
              <Text style={styles.title}>Aruvi Studio</Text>
              <View style={styles.connectionRow}>
                <View style={[styles.connectionDot, token.trim() ? styles.connectionDotReady : styles.connectionDotMissing]} />
                <Text style={styles.connectionText} numberOfLines={1}>
                  {token.trim() ? "Connected" : "Setup required"}
                </Text>
              </View>
            </View>
            <Pressable style={styles.headerButton} onPress={() => setIsSetupOpen((current) => !current)}>
              <Text style={styles.buttonText}>Settings</Text>
            </Pressable>
            <Pressable style={styles.headerButton} onPress={() => setWebReloadKey((current) => current + 1)}>
              <Text style={styles.buttonText}>Refresh</Text>
            </Pressable>
          </View>

          {shouldShowSetup ? (
            <View style={styles.setupPanel}>
              <Text style={styles.setupCaption} numberOfLines={1}>{remoteUrl}</Text>
              <TextInput
                style={styles.input}
                value={baseUrl}
                onChangeText={setBaseUrl}
                placeholder="http://mac-tailnet-ip:8787"
                placeholderTextColor="#7d8898"
                autoCapitalize="none"
                autoCorrect={false}
                keyboardType="url"
              />
              <TextInput
                style={styles.input}
                value={token}
                onChangeText={setToken}
                placeholder="mobile.api_token"
                placeholderTextColor="#7d8898"
                secureTextEntry
                autoCapitalize="none"
                autoCorrect={false}
              />
              <View style={styles.optionalGrid}>
                <TextInput
                  style={[styles.input, styles.flexInput]}
                  value={providerId}
                  onChangeText={setProviderId}
                  placeholder="provider id"
                  placeholderTextColor="#7d8898"
                  autoCapitalize="none"
                  autoCorrect={false}
                />
                <TextInput
                  style={[styles.input, styles.flexInput]}
                  value={modelName}
                  onChangeText={setModelName}
                  placeholder="model"
                  placeholderTextColor="#7d8898"
                  autoCapitalize="none"
                  autoCorrect={false}
                />
              </View>
              <View style={styles.settingsRow}>
                <View style={styles.settingsCopy}>
                  <Text style={styles.settingsLabel}>Read replies</Text>
                  <Text style={styles.settingsDescription}>Speak assistant replies after each voice message.</Text>
                </View>
                <Switch
                  value={readReplies}
                  onValueChange={(nextValue) => void setReadRepliesPreference(nextValue)}
                  trackColor={{ false: "#2a3442", true: "#1d6f9d" }}
                  thumbColor={readReplies ? "#f4f8ff" : "#8b98aa"}
                  ios_backgroundColor="#2a3442"
                />
              </View>
              <View style={styles.actionRow}>
                <Pressable style={styles.primaryButton} onPress={() => void saveConnection()} disabled={isSaving}>
                  <Text style={styles.primaryButtonText}>{isSaving ? "Saving..." : "Save + Load"}</Text>
                </Pressable>
              </View>
            </View>
          ) : null}
        </View>

        <View style={styles.content}>
          {activeTab === "voice" ? (
            renderVoiceScreen()
          ) : activeTab === "models" ? (
            renderModelManager()
          ) : (
            <WebView
              ref={webViewRef}
              key={`${remoteUrl}-${webReloadKey}`}
              source={{ uri: remoteUrl }}
              style={styles.webView}
              injectedJavaScriptBeforeContentLoaded={remoteBootstrapScript}
              onLoadEnd={() => webViewRef.current?.injectJavaScript(remoteBootstrapScript)}
              javaScriptEnabled
              domStorageEnabled
              allowsInlineMediaPlayback
              mediaCapturePermissionGrantType="grantIfSameHostElsePrompt"
              startInLoadingState
              renderLoading={() => (
                <View style={styles.loading}>
                  <ActivityIndicator color="#7bc8ff" />
                </View>
              )}
              renderError={(_, __, description) => (
                <View style={styles.errorPanel}>
                  <Text style={styles.errorTitle}>Remote UI unavailable</Text>
                  <Text style={styles.errorText}>{description}</Text>
                </View>
              )}
            />
          )}
        </View>

        {!isVoiceKeyboardOpen ? (
          <View style={styles.bottomTabs}>
            {TABS.map((tab) => (
              <Pressable
                key={tab.id}
                style={[styles.tabItem, activeTab === tab.id && styles.tabItemActive]}
                onPress={() => switchTab(tab.id)}
              >
                <View style={[styles.tabIndicator, activeTab === tab.id && styles.tabIndicatorActive]} />
                <Text style={[styles.tabText, activeTab === tab.id && styles.tabTextActive]} numberOfLines={1}>
                  {tab.label}
                </Text>
              </Pressable>
            ))}
          </View>
        ) : null}
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safeArea: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  shell: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  header: {
    borderBottomWidth: 1,
    borderBottomColor: "#242b35",
    paddingHorizontal: 16,
    paddingTop: 8,
    paddingBottom: 10,
    gap: 8,
    backgroundColor: "#10151d",
  },
  content: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  voiceScreen: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  voiceTopBand: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 12,
    paddingHorizontal: 16,
    paddingTop: 14,
    paddingBottom: 10,
  },
  voiceTopCopy: {
    flex: 1,
    minWidth: 0,
    gap: 2,
  },
  voiceTitle: {
    color: "#f4f8ff",
    fontSize: 22,
    fontWeight: "900",
  },
  voiceSubtitle: {
    color: "#9ba8ba",
    fontSize: 12,
    fontWeight: "700",
  },
  runtimeChip: {
    maxWidth: 190,
    minHeight: 34,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#2f8fc8",
    backgroundColor: "#10293a",
    justifyContent: "center",
    paddingHorizontal: 10,
  },
  runtimeChipText: {
    color: "#b9e4ff",
    fontSize: 11,
    fontWeight: "900",
  },
  voiceConversation: {
    flex: 1,
  },
  voiceConversationContent: {
    paddingHorizontal: 16,
    paddingTop: 8,
    paddingBottom: 12,
    gap: 12,
  },
  voiceBubble: {
    maxWidth: "86%",
    borderRadius: 8,
    paddingHorizontal: 14,
    paddingVertical: 12,
  },
  voiceBubbleAssistant: {
    alignSelf: "flex-start",
    backgroundColor: "#202733",
    borderTopLeftRadius: 3,
  },
  voiceBubbleUser: {
    alignSelf: "flex-end",
    backgroundColor: "#0e639c",
    borderTopRightRadius: 3,
  },
  voiceBubbleText: {
    fontSize: 15,
    lineHeight: 22,
  },
  voiceBubbleTextAssistant: {
    color: "#eef4fb",
  },
  voiceBubbleTextUser: {
    color: "#ffffff",
  },
  voiceComposerPanel: {
    marginHorizontal: 12,
    marginBottom: 10,
    borderWidth: 1,
    borderColor: "#2f3948",
    borderRadius: 24,
    backgroundColor: "#111820",
    paddingHorizontal: 12,
    paddingTop: 10,
    paddingBottom: 10,
    gap: 8,
  },
  voiceComposerHeader: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 10,
    paddingHorizontal: 4,
  },
  voiceComposerLabel: {
    flex: 1,
    color: "#e8eef7",
    fontSize: 13,
    fontWeight: "900",
  },
  voiceComposerStatus: {
    maxWidth: 178,
    color: "#8fa0b6",
    fontSize: 11,
    fontWeight: "800",
  },
  voiceComposerInput: {
    minHeight: 50,
    maxHeight: 112,
    color: "#f4f8ff",
    fontSize: 16,
    lineHeight: 22,
    paddingHorizontal: 4,
    paddingVertical: 2,
  },
  voiceComposerActions: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    minHeight: 42,
  },
  voiceClearButton: {
    minWidth: 68,
    height: 38,
    borderRadius: 19,
    borderWidth: 1,
    borderColor: "#344052",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#151c26",
    paddingHorizontal: 12,
  },
  voiceClearButtonText: {
    color: "#c5d0df",
    fontSize: 13,
    fontWeight: "900",
  },
  voiceComposerSpacer: {
    flex: 1,
  },
  voiceMicButton: {
    minWidth: 58,
    height: 38,
    borderRadius: 19,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#1a2330",
    borderWidth: 1,
    borderColor: "#39485c",
  },
  voiceMicButtonRecording: {
    backgroundColor: "#b33939",
    borderColor: "#d65f5f",
  },
  voiceMicButtonText: {
    color: "#eaf2fb",
    fontSize: 13,
    fontWeight: "900",
  },
  voiceSendButton: {
    width: 42,
    height: 42,
    borderRadius: 21,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#ffffff",
  },
  voiceSendButtonText: {
    color: "#0d1015",
    fontSize: 25,
    lineHeight: 28,
    fontWeight: "900",
  },
  voiceControlHint: {
    paddingHorizontal: 4,
    color: "#9ba8ba",
    fontSize: 11,
    lineHeight: 15,
    fontWeight: "700",
  },
  modelPage: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  modelPageContent: {
    padding: 16,
    gap: 12,
  },
  modelHeader: {
    gap: 6,
  },
  sectionTitle: {
    color: "#f4f8ff",
    fontSize: 22,
    fontWeight: "900",
  },
  sectionText: {
    color: "#a8b3c4",
    fontSize: 13,
    lineHeight: 19,
  },
  runtimePanel: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  panelLabel: {
    color: "#e7edf7",
    fontSize: 13,
    fontWeight: "800",
  },
  segmentedControl: {
    flexDirection: "row",
    borderWidth: 1,
    borderColor: "#354253",
    borderRadius: 8,
    overflow: "hidden",
    minHeight: 42,
  },
  segmentButton: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#12161c",
    paddingHorizontal: 8,
  },
  segmentButtonActive: {
    backgroundColor: "#0e639c",
  },
  segmentButtonText: {
    color: "#a8b3c4",
    fontSize: 13,
    fontWeight: "800",
  },
  segmentButtonTextActive: {
    color: "#ffffff",
  },
  modelStatusPanel: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  modelStatusRow: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 10,
  },
  modelStatusText: {
    flex: 1,
    color: "#a8b3c4",
    fontSize: 12,
    lineHeight: 17,
  },
  progressText: {
    color: "#f4f8ff",
    fontSize: 12,
    fontWeight: "800",
  },
  progressTrack: {
    height: 8,
    borderRadius: 4,
    backgroundColor: "#252d38",
    overflow: "hidden",
  },
  progressFill: {
    height: 8,
    borderRadius: 4,
    backgroundColor: "#56b6c2",
  },
  modelList: {
    gap: 10,
  },
  modelCard: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  modelCardSelected: {
    borderColor: "#7bc8ff",
  },
  modelCardHeader: {
    flexDirection: "row",
    alignItems: "flex-start",
    justifyContent: "space-between",
    gap: 10,
  },
  modelTitleBlock: {
    flex: 1,
    minWidth: 0,
    gap: 3,
  },
  modelTitle: {
    color: "#f4f8ff",
    fontSize: 15,
    fontWeight: "800",
  },
  modelMeta: {
    color: "#90a0b8",
    fontSize: 12,
  },
  installBadge: {
    color: "#9aa8bd",
    borderWidth: 1,
    borderColor: "#425066",
    borderRadius: 8,
    paddingHorizontal: 8,
    paddingVertical: 5,
    fontSize: 11,
    fontWeight: "800",
    overflow: "hidden",
  },
  installBadgeActive: {
    color: "#dafbe1",
    borderColor: "#3b7f55",
    backgroundColor: "#193323",
  },
  modelDescription: {
    color: "#a8b3c4",
    fontSize: 12,
    lineHeight: 17,
  },
  modelActions: {
    flexDirection: "row",
    gap: 8,
  },
  smallButton: {
    minHeight: 38,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#425066",
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 12,
    backgroundColor: "#18202a",
  },
  smallButtonActive: {
    borderColor: "#7bc8ff",
    backgroundColor: "#203348",
  },
  smallButtonPrimary: {
    backgroundColor: "#0e639c",
    borderColor: "#0e639c",
  },
  smallButtonText: {
    color: "#d9e4f2",
    fontSize: 13,
    fontWeight: "800",
  },
  smallButtonTextActive: {
    color: "#ffffff",
  },
  smallButtonPrimaryText: {
    color: "#ffffff",
    fontSize: 13,
    fontWeight: "800",
  },
  sourceButton: {
    minHeight: 42,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#425066",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#151922",
  },
  sourceButtonText: {
    color: "#d9e4f2",
    fontSize: 13,
    fontWeight: "800",
  },
  nativeVoiceBar: {
    flexDirection: "row",
    alignItems: "center",
    gap: 10,
    borderBottomWidth: 1,
    borderBottomColor: "#2f3642",
    backgroundColor: "#151922",
    paddingHorizontal: 12,
    paddingVertical: 10,
  },
  voiceButton: {
    minWidth: 86,
    minHeight: 42,
    borderRadius: 8,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#0e639c",
    paddingHorizontal: 14,
  },
  voiceButtonRecording: {
    backgroundColor: "#b33939",
  },
  buttonDisabled: {
    opacity: 0.55,
  },
  voiceButtonText: {
    color: "#ffffff",
    fontSize: 14,
    fontWeight: "800",
  },
  voiceStatusBlock: {
    flex: 1,
    minWidth: 0,
    gap: 3,
  },
  voiceStatus: {
    color: "#f4f8ff",
    fontSize: 13,
    fontWeight: "800",
  },
  voiceTranscript: {
    color: "#9aa8bd",
    fontSize: 12,
  },
  titleRow: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center",
    gap: 10,
  },
  titleBlock: {
    flex: 1,
    minWidth: 0,
    gap: 4,
  },
  title: {
    color: "#f4f8ff",
    fontSize: 23,
    fontWeight: "900",
  },
  url: {
    color: "#9aa8bd",
    fontSize: 12,
    marginTop: 2,
  },
  connectionRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: 7,
  },
  connectionDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
  },
  connectionDotReady: {
    backgroundColor: "#49c27c",
  },
  connectionDotMissing: {
    backgroundColor: "#d99b35",
  },
  connectionText: {
    color: "#96a4b8",
    fontSize: 12,
    fontWeight: "800",
  },
  setupPanel: {
    gap: 8,
  },
  setupCaption: {
    color: "#8390a3",
    fontSize: 11,
    fontWeight: "700",
  },
  settingsRow: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 12,
    borderWidth: 1,
    borderColor: "#2f3948",
    borderRadius: 8,
    backgroundColor: "#121820",
    paddingHorizontal: 12,
    paddingVertical: 10,
  },
  settingsCopy: {
    flex: 1,
    minWidth: 0,
    gap: 3,
  },
  settingsLabel: {
    color: "#f4f8ff",
    fontSize: 13,
    fontWeight: "900",
  },
  settingsDescription: {
    color: "#95a3b7",
    fontSize: 11,
    lineHeight: 15,
    fontWeight: "700",
  },
  input: {
    backgroundColor: "#12161c",
    borderColor: "#364152",
    borderWidth: 1,
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 10,
    color: "#f4f8ff",
  },
  flexInput: {
    flex: 1,
    minWidth: 0,
  },
  optionalGrid: {
    flexDirection: "row",
    gap: 8,
  },
  actionRow: {
    flexDirection: "row",
    gap: 8,
  },
  button: {
    backgroundColor: "#223040",
    borderRadius: 8,
    paddingVertical: 10,
    paddingHorizontal: 14,
  },
  headerButton: {
    minHeight: 40,
    borderRadius: 8,
    justifyContent: "center",
    backgroundColor: "#1d2a38",
    paddingHorizontal: 13,
  },
  buttonText: {
    color: "#edf3ff",
    fontWeight: "900",
  },
  primaryButton: {
    flex: 1,
    backgroundColor: "#0e639c",
    borderRadius: 8,
    paddingVertical: 12,
    alignItems: "center",
  },
  primaryButtonText: {
    color: "#ffffff",
    fontWeight: "700",
    fontSize: 15,
  },
  webView: {
    flex: 1,
    backgroundColor: "#101317",
  },
  bottomTabs: {
    flexDirection: "row",
    borderTopWidth: 1,
    borderTopColor: "#242b35",
    backgroundColor: "#10151d",
    paddingHorizontal: 6,
    paddingTop: 8,
    paddingBottom: 8,
    gap: 4,
  },
  tabItem: {
    flex: 1,
    minHeight: 50,
    borderRadius: 8,
    alignItems: "center",
    justifyContent: "center",
    gap: 4,
  },
  tabItemActive: {
    backgroundColor: "#1d2a38",
  },
  tabIndicator: {
    width: 20,
    height: 3,
    borderRadius: 2,
    backgroundColor: "transparent",
  },
  tabIndicatorActive: {
    backgroundColor: "#69c7ff",
  },
  tabText: {
    color: "#94a1b4",
    fontSize: 11,
    fontWeight: "900",
  },
  tabTextActive: {
    color: "#f4f8ff",
  },
  loading: {
    position: "absolute",
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#101317",
  },
  errorPanel: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    padding: 18,
    gap: 10,
    backgroundColor: "#111317",
  },
  errorTitle: {
    color: "#f4f8ff",
    fontSize: 16,
    fontWeight: "700",
  },
  errorText: {
    color: "#9aa8bd",
    fontSize: 13,
    lineHeight: 18,
    textAlign: "center",
  },
});
