import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  SafeAreaView,
  View,
} from "react-native";
import {
  useAudioRecorder,
  useAudioRecorderState,
} from "expo-audio";
import * as Speech from "expo-speech";
import * as SecureStore from "expo-secure-store";
import { WebView } from "react-native-webview";
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
import { useMobileVoiceController } from "./src/hooks/useMobileVoiceController";
import { useMobileWhisperController } from "./src/hooks/useMobileWhisperController";
import {
  buildRemoteScript,
} from "./src/lib/mobileConnection";
import { describeError } from "./src/lib/mobileFormatters";
import { MOBILE_STORAGE_KEYS } from "./src/lib/mobileStorageKeys";
import {
  VOICE_RECORDING_OPTIONS,
} from "./src/lib/mobileVoice";
import { styles } from "./src/styles/appStyles";

type ActiveTab = MobileTabId;

export default function App() {
  const webViewRef = useRef<WebView>(null);
  const audioRecorder = useAudioRecorder(VOICE_RECORDING_OPTIONS);
  const recorderState = useAudioRecorderState(audioRecorder, 250);
  const [activeTab, setActiveTab] = useState<ActiveTab>("planner");
  const [readReplies, setReadReplies] = useState(true);
  const [isSetupOpen, setIsSetupOpen] = useState(false);
  const [nativeVoiceStatus, setNativeVoiceStatus] = useState("Ready");
  const keyboardHeight = useKeyboardHeight();
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

  const setReadRepliesPreference = async (nextValue: boolean) => {
    setReadReplies(nextValue);
    await SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.readReplies, nextValue ? "true" : "false");
    if (!nextValue) {
      void Speech.stop();
      setNativeVoiceStatus("Reply reading off");
    }
  };

  const {
    voiceMode,
    setVoiceMode,
    isVoiceBusy,
    setIsVoiceBusy,
    voiceDraft,
    setVoiceDraft,
    voiceMessages,
    speechModelDescription,
    speechModelLabel,
    voiceComposerStatus,
    switchVoiceMode,
    speakAssistantReply,
    submitVoicePrompt,
    toggleNativeVoiceRecording,
  } = useMobileVoiceController({
    mobileClient,
    baseUrl,
    token,
    providerId,
    modelName,
    locale,
    readReplies,
    canUseLocalSpeech,
    activeLocalWhisperModelId: activeLocalWhisperModel?.id ?? null,
    audioRecorder,
    isRecorderRecording: recorderState.isRecording,
    setNativeVoiceStatus,
    submitPlannerPrompt,
    transcribeRecording: transcribeWithLocalWhisper,
    setConnectionStatus,
    applyFallbackBaseUrl,
    switchTab: (nextTab) => switchTab(nextTab),
    describeError,
  });

  useMobileAppPreferences({
    setActiveTab,
    setVoiceMode,
    setReadReplies,
    setSelectedWhisperModelId,
    setVerifiedInstalledModels,
  });

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

  const shouldShowSetup = isSetupOpen || !token.trim();
  const nativeVoiceButtonDisabled = isVoiceBusy || !token.trim();
  const plannerRuntimeLabel = modelName.trim() || providerId.trim() || "Planner model";
  const plannerContextLabel = plannerContextProductName ? `Context: ${plannerContextProductName}` : "Context: not selected";
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
