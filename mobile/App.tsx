import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  SafeAreaView,
  View,
} from "react-native";
import {
  AudioModule,
  setAudioModeAsync,
  useAudioRecorder,
  useAudioRecorderState,
} from "expo-audio";
import * as Speech from "expo-speech";
import * as SecureStore from "expo-secure-store";
import { WebView } from "react-native-webview";
import { PlannerMobileClient } from "./src/api/client";
import { MobileAppHeader } from "./src/components/MobileAppHeader";
import { MobileBottomTabs, type MobileTabId } from "./src/components/MobileBottomTabs";
import { MobileCallsScreen } from "./src/components/MobileCallsScreen";
import { MobileModelManager } from "./src/components/MobileModelManager";
import { MobileProductExplorer } from "./src/components/MobileProductExplorer";
import { MobileRemoteWebView } from "./src/components/MobileRemoteWebView";
import { MobileVoiceScreen } from "./src/components/MobileVoiceScreen";
import { useKeyboardHeight } from "./src/hooks/useKeyboardHeight";
import { useMobileAppPreferences } from "./src/hooks/useMobileAppPreferences";
import { useMobileConnectionController } from "./src/hooks/useMobileConnectionController";
import { useMobileModelCallsController } from "./src/hooks/useMobileModelCallsController";
import { useMobilePlannerChatController } from "./src/hooks/useMobilePlannerChatController";
import { useMobileProductPlannerController } from "./src/hooks/useMobileProductPlannerController";
import { useMobileProductsController } from "./src/hooks/useMobileProductsController";
import { useMobileWhisperController } from "./src/hooks/useMobileWhisperController";
import type { MobilePlannerToolTraceEntry } from "./src/types";
import {
  buildRemoteScript,
  buildRemoteVoiceSubmitScript,
  getLoopbackFallbackBaseUrl,
  isNetworkRequestFailure,
  normalizeBaseUrlForDisplay,
} from "./src/lib/mobileConnection";
import { describeError } from "./src/lib/mobileFormatters";
import { MOBILE_STORAGE_KEYS } from "./src/lib/mobileStorageKeys";
import {
  normalizeWhisperLanguage,
  WHISPER_MODELS,
  VOICE_RECORDING_OPTIONS,
} from "./src/lib/mobileVoice";
import { styles } from "./src/styles/appStyles";

type ActiveTab = MobileTabId;
type VoiceMode = "assistant" | "planner";
type VoiceMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};
type VoicePromptSource = "typed" | "recording";

function createId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.round(Math.random() * 100000)}`;
}

export default function App() {
  const webViewRef = useRef<WebView>(null);
  const audioRecorder = useAudioRecorder(VOICE_RECORDING_OPTIONS);
  const recorderState = useAudioRecorderState(audioRecorder, 250);
  const [activeTab, setActiveTab] = useState<ActiveTab>("planner");
  const [voiceMode, setVoiceMode] = useState<VoiceMode>("assistant");
  const [readReplies, setReadReplies] = useState(true);
  const [isSetupOpen, setIsSetupOpen] = useState(false);
  const [isVoiceBusy, setIsVoiceBusy] = useState(false);
  const [nativeVoiceStatus, setNativeVoiceStatus] = useState("Ready");
  const [lastVoiceTranscript, setLastVoiceTranscript] = useState("");
  const [voiceDraft, setVoiceDraft] = useState("");
  const keyboardHeight = useKeyboardHeight();
  const [voiceMessages, setVoiceMessages] = useState<VoiceMessage[]>([
    {
      id: "assistant-welcome",
      role: "assistant",
      content: "Ready when you are. Tap the mic and speak naturally.",
    },
  ]);
  const {
    baseUrl,
    setBaseUrl,
    token,
    setToken,
    providerId,
    setProviderId,
    modelName,
    setModelName,
    locale,
    webReloadKey,
    connectionStatus,
    setConnectionStatus,
    isSaving,
    remoteUrl,
    mobileClient,
    reloadRemote,
    refreshConnection,
    saveConnection,
    applyFallbackBaseUrl,
  } = useMobileConnectionController();

  type ChatCompletionBody = Parameters<typeof mobileClient.runChatCompletion>[0];

  const productsController = useMobileProductsController({
    mobileClient,
    token,
    describeError,
  });

  const {
    selectedProduct,
    selectedProductNode,
    selectedProductNodePath,
    productStats,
    visibleProductChildren,
    filteredProductNodes,
    products,
    productSummary,
    productTree,
    selectedProductId,
    selectedProductNodeId,
    productExploreTab,
    productSearchQuery,
    isProductLoading,
    isProductTreeLoading,
    productError,
    isProductPickerOpen,
    loadProducts,
    ensureProductTree,
    setProductError,
    openProductNode,
    setSelectedProductNodeId,
    switchProductExploreTab,
    setProductSearchQuery,
    setIsProductPickerOpen,
  } = productsController;

  const {
    modelCalls,
    selectedModelCallSessionKey,
    selectedModelCall,
    isModelCallsLoading,
    modelCallsError,
    loadModelCalls,
    setSelectedModelCallSessionKey,
    setSelectedModelCall,
  } = useMobileModelCallsController({
    mobileClient,
    token,
    describeError,
  });

  const {
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
  } = useMobileWhisperController({
    locale,
    installedModelsStorageKey: MOBILE_STORAGE_KEYS.installedWhisperModels,
    selectedModelStorageKey: MOBILE_STORAGE_KEYS.selectedWhisperModelId,
    onStatusChange: setNativeVoiceStatus,
  });

  useMobileAppPreferences({
    setActiveTab,
    setVoiceMode,
    setReadReplies,
    setSelectedWhisperModelId,
    setVerifiedInstalledModels,
  });

  const {
    plannerContextProductName,
    submitPlannerPrompt,
  } = useMobilePlannerChatController({
    mobileClient,
    baseUrl,
    token,
    providerId,
    modelName,
    selectedProductId,
    onConnected: () => setConnectionStatus("connected"),
    onFallbackBaseUrl: applyFallbackBaseUrl,
  });

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
      void Speech.stop();
    };
  }, []);

  useEffect(() => {
    if (activeTab === "products" && token.trim() && !products.length && !isProductLoading) {
      void loadProducts(selectedProductId);
    }
  }, [activeTab, isProductLoading, products.length, selectedProductId, token]);

  useEffect(() => {
    if (activeTab === "calls" && token.trim() && !modelCalls.length && !isModelCallsLoading) {
      void loadModelCalls();
    }
  }, [activeTab, isModelCallsLoading, modelCalls.length, token]);

  const switchTab = (nextTab: ActiveTab) => {
    setActiveTab(nextTab);
    void SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.activeTab, nextTab);
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

  const switchVoiceMode = (nextMode: VoiceMode) => {
    setVoiceMode(nextMode);
    void SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.voiceMode, nextMode);
    setNativeVoiceStatus(nextMode === "planner" ? "Planner chat ready" : "Ready");
  };

  const setReadRepliesPreference = async (nextValue: boolean) => {
    setReadReplies(nextValue);
    await SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.readReplies, nextValue ? "true" : "false");
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

  const {
    productPlannerDraft,
    productPlannerStatus,
    productPlannerReply,
    productPlannerTrace,
    productPlannerRecording,
    setProductPlannerDraft,
    submitProductPlannerPrompt,
    toggleProductPlannerRecording,
  } = useMobileProductPlannerController({
    token,
    readReplies,
    canUseLocalSpeech,
    selectedProduct,
    selectedProductId,
    selectedProductNode,
    selectedProductNodePath,
    selectedProductNodeId,
    audioRecorder,
    isRecorderRecording: recorderState.isRecording,
    submitPlannerPrompt,
    loadProducts,
    setSelectedProductNodeId,
    setVoiceMode,
    setIsVoiceBusy,
    switchTab: (nextTab) => switchTab(nextTab),
    speakAssistantReply,
    transcribeRecording: transcribeWithLocalWhisper,
    describeError,
  });

  const runChatCompletionWithFallback = async (body: ChatCompletionBody) => {
    try {
      const response = await mobileClient.runChatCompletion(body);
      setConnectionStatus("connected");
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim()).runChatCompletion(body);
          await applyFallbackBaseUrl(fallbackBaseUrl);
          return response;
        } catch {
          throw new Error(
            `Cannot reach Aruvi at ${normalizeBaseUrlForDisplay(baseUrl)} or ${fallbackBaseUrl}. Check Settings base URL and that the desktop bridge is running.`,
          );
        }
      }
      throw error;
    }
  };

  const submitVoicePrompt = async (prompt: string, source: VoicePromptSource = "typed") => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using chat.");
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
    setNativeVoiceStatus(
      voiceMode === "planner"
        ? "Planning with MCP..."
        : source === "recording"
          ? "Sending voice prompt..."
          : "Thinking...",
    );
    setIsVoiceBusy(true);

    try {
      const assistantResult = voiceMode === "planner"
        ? await submitPlannerPrompt(trimmed)
        : await runChatCompletionWithFallback({
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
          }).then((response) => ({
            content: response.content.trim() || "(empty response)",
            toolTrace: undefined,
          }));
      const assistantText = assistantResult.content;
      const assistantMessage: VoiceMessage = {
        id: createId("voice-assistant"),
        role: "assistant",
        content: assistantText,
        toolTrace: assistantResult.toolTrace,
      };
      setVoiceMessages((current) => [...current, assistantMessage].slice(-24));
      if (readReplies) {
        speakAssistantReply(assistantText);
      } else {
        setNativeVoiceStatus("Reply ready");
      }
    } catch (error) {
      const message = describeError(error);
      const title = source === "recording" ? "Voice failed" : "Chat failed";
      setNativeVoiceStatus(`${title}: ${message}`);
      Alert.alert(title, message);
    } finally {
      setIsVoiceBusy(false);
    }
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
      await submitVoicePrompt(transcript, "recording");
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
  const nativeVoiceButtonDisabled = isVoiceBusy || !token.trim();
  const speechModelDescription = canUseLocalSpeech
    ? `Using ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModel?.id)?.label ?? "Whisper"} on this phone for speech-to-text.`
    : "Type a message, or install Whisper to use the mic.";
  const speechModelLabel = canUseLocalSpeech
    ? `On-device ${WHISPER_MODELS.find((model) => model.id === activeLocalWhisperModel?.id)?.label ?? "Whisper"}`
    : "Install Whisper";
  const plannerRuntimeLabel = modelName.trim() || providerId.trim() || "Planner model";
  const plannerContextLabel = plannerContextProductName ? `Context: ${plannerContextProductName}` : "Context: not selected";
  const voiceComposerStatus = token.trim()
    ? canUseLocalSpeech
      ? speechModelLabel
      : "Text chat ready"
    : "Setup required";
  const connectionText = !token.trim()
    ? "Setup required"
    : connectionStatus === "connected"
      ? "Connected"
      : connectionStatus === "checking"
        ? "Checking..."
        : connectionStatus === "offline"
          ? "Backend offline"
          : "Not checked";
  const isVoiceKeyboardOpen = activeTab === "voice" && keyboardHeight > 0;

  return (
    <SafeAreaView style={styles.safeArea}>
      <View style={styles.shell}>
        <MobileAppHeader
          remoteUrl={remoteUrl}
          baseUrl={baseUrl}
          token={token}
          providerId={providerId}
          modelName={modelName}
          readReplies={readReplies}
          connectionText={connectionText}
          connectionStatus={connectionStatus}
          shouldShowSetup={shouldShowSetup}
          isSaving={isSaving}
          onToggleSetup={() => setIsSetupOpen((current) => !current)}
          onRefresh={() => {
            if (activeTab === "products") {
              void loadProducts(selectedProductId);
            } else {
              reloadRemote();
            }
            refreshConnection();
          }}
          onBaseUrlChange={setBaseUrl}
          onTokenChange={setToken}
          onProviderIdChange={setProviderId}
          onModelNameChange={setModelName}
          onReadRepliesChange={setReadRepliesPreference}
          onSaveConnection={async () => {
            const didSave = await saveConnection();
            if (didSave) {
              setIsSetupOpen(false);
            }
          }}
        />

        <View style={styles.content}>
          {activeTab === "voice" ? (
            <MobileVoiceScreen
              voiceMode={voiceMode}
              nativeVoiceStatus={nativeVoiceStatus}
              plannerContextLabel={plannerContextLabel}
              plannerRuntimeLabel={plannerRuntimeLabel}
              speechModelLabel={speechModelLabel}
              voiceMessages={voiceMessages}
              isVoiceKeyboardOpen={isVoiceKeyboardOpen}
              keyboardHeight={keyboardHeight}
              isVoiceBusy={isVoiceBusy}
              isRecording={recorderState.isRecording}
              voiceComposerStatus={voiceComposerStatus}
              voiceDraft={voiceDraft}
              nativeVoiceButtonDisabled={nativeVoiceButtonDisabled}
              canUseLocalSpeech={canUseLocalSpeech}
              token={token}
              speechModelDescription={speechModelDescription}
              onOpenModels={() => switchTab("models")}
              onSwitchVoiceMode={switchVoiceMode}
              onVoiceDraftChange={setVoiceDraft}
              onClearVoiceDraft={() => setVoiceDraft("")}
              onToggleNativeVoiceRecording={toggleNativeVoiceRecording}
              onSubmitVoicePrompt={(prompt) => submitVoicePrompt(prompt, "typed")}
            />
          ) : activeTab === "models" ? (
            <MobileModelManager
              speechModelDescription={speechModelDescription}
              modelInstallStatus={modelInstallStatus}
              modelInstallProgress={modelInstallProgress}
              installedWhisperModels={installedWhisperModels}
              selectedWhisperModel={selectedWhisperModel}
              modelInstallBusyId={modelInstallBusyId}
              onSelectWhisperModel={selectWhisperModel}
              onInstallWhisperModel={installWhisperModel}
              onRemoveWhisperModel={removeWhisperModel}
            />
          ) : activeTab === "calls" ? (
            <MobileCallsScreen
              modelCalls={modelCalls}
              selectedModelCallSessionKey={selectedModelCallSessionKey}
              selectedModelCall={selectedModelCall}
              isModelCallsLoading={isModelCallsLoading}
              modelCallsError={modelCallsError}
              onLoadModelCalls={loadModelCalls}
              onSelectedModelCallSessionKeyChange={setSelectedModelCallSessionKey}
              onSelectedModelCallChange={setSelectedModelCall}
            />
          ) : activeTab === "products" ? (
            <MobileProductExplorer
              selectedProduct={selectedProduct}
              selectedProductId={selectedProductId}
              selectedProductNode={selectedProductNode}
              selectedProductNodePath={selectedProductNodePath}
              productSummary={productSummary}
              productTree={productTree}
              productStats={productStats}
              visibleProductChildren={visibleProductChildren}
              filteredProductNodes={filteredProductNodes}
              productExploreTab={productExploreTab}
              productSearchQuery={productSearchQuery}
              productError={productError}
              isProductTreeLoading={isProductTreeLoading}
              isProductPickerOpen={isProductPickerOpen}
              products={products}
              productPlannerRecording={productPlannerRecording}
              isRecorderRecording={recorderState.isRecording}
              isVoiceBusy={isVoiceBusy}
              productPlannerStatus={productPlannerStatus}
              productPlannerReply={productPlannerReply}
              productPlannerDraft={productPlannerDraft}
              productPlannerTrace={productPlannerTrace}
              onLoadProducts={loadProducts}
              onEnsureProductTree={ensureProductTree}
              onProductError={setProductError}
              onOpenNode={openProductNode}
              onSelectParentNode={setSelectedProductNodeId}
              onSwitchExploreTab={switchProductExploreTab}
              onProductSearchQueryChange={setProductSearchQuery}
              onProductPickerOpenChange={setIsProductPickerOpen}
              onSpeakPlannerReply={speakAssistantReply}
              onPlannerDraftChange={setProductPlannerDraft}
              onTogglePlannerRecording={toggleProductPlannerRecording}
              onSubmitPlannerPrompt={(prompt) => submitProductPlannerPrompt(prompt, "typed")}
              describeError={describeError}
            />
          ) : (
            <MobileRemoteWebView
              webViewRef={webViewRef}
              remoteUrl={remoteUrl}
              reloadKey={webReloadKey}
              bootstrapScript={remoteBootstrapScript}
            />
          )}
        </View>

        <MobileBottomTabs activeTab={activeTab} isHidden={isVoiceKeyboardOpen} onSwitchTab={switchTab} />
      </View>
    </SafeAreaView>
  );
}
