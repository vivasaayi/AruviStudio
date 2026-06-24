import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Keyboard,
  Linking,
  Platform,
  SafeAreaView,
  View,
} from "react-native";
import {
  AudioModule,
  setAudioModeAsync,
  useAudioRecorder,
  useAudioRecorderState,
} from "expo-audio";
import * as FileSystem from "expo-file-system";
import * as Speech from "expo-speech";
import * as SecureStore from "expo-secure-store";
import { WebView } from "react-native-webview";
import { initWhisper, type WhisperContext } from "whisper.rn";
import { PlannerMobileClient } from "./src/api/client";
import { MobileAppHeader } from "./src/components/MobileAppHeader";
import { MobileBottomTabs, MOBILE_TABS, type MobileTabId } from "./src/components/MobileBottomTabs";
import { MobileCallsScreen } from "./src/components/MobileCallsScreen";
import { MobileModelManager } from "./src/components/MobileModelManager";
import { MobileProductExplorer, type ProductExploreTab } from "./src/components/MobileProductExplorer";
import { MobileRemoteWebView } from "./src/components/MobileRemoteWebView";
import { MobileVoiceScreen } from "./src/components/MobileVoiceScreen";
import type { MobilePlannerToolTraceEntry, ModelCall, Product, ProductTree, ProductTreeSummary } from "./src/types";
import {
  buildRemoteScript,
  buildRemoteVoiceSubmitScript,
  getLoopbackFallbackBaseUrl,
  isNetworkRequestFailure,
  normalizeBaseUrlForDisplay,
  parseConnectionUrl,
  type ConnectionValues,
} from "./src/lib/mobileConnection";
import {
  buildModelCallSessions,
  describeError,
} from "./src/lib/mobileFormatters";
import {
  assertWhisperNativeModuleAvailable,
  normalizeWhisperLanguage,
  parseInstalledWhisperModels,
  whisperModelDirectory,
  WHISPER_MODELS,
  VOICE_RECORDING_OPTIONS,
  type InstalledWhisperModel,
  type WhisperModelOption,
} from "./src/lib/mobileVoice";
import {
  countLeafNodes,
  countTreeNodes,
  findProductNode,
  findProductNodePath,
  flattenProductNodes,
  getNodeSummary,
  productViewNeedsTree,
} from "./src/lib/productTree";
import { styles } from "./src/styles/appStyles";

type ActiveTab = MobileTabId;
type ConnectionStatus = "unchecked" | "checking" | "connected" | "offline";
type VoiceMode = "assistant" | "planner";
type VoiceMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};
type VoicePromptSource = "typed" | "recording";
type ChatCompletionBody = Parameters<PlannerMobileClient["runChatCompletion"]>[0];
type PlannerChatTurnBody = Parameters<PlannerMobileClient["submitMobilePlannerChatTurn"]>[1];

const STORAGE_KEYS = {
  baseUrl: "aruvi.mobile.base_url",
  token: "aruvi.mobile.token",
  providerId: "aruvi.mobile.provider_id",
  modelName: "aruvi.mobile.model_name",
  locale: "aruvi.mobile.locale",
  activeTab: "aruvi.mobile.active_tab",
  voiceMode: "aruvi.mobile.voice_mode",
  readReplies: "aruvi.mobile.read_replies",
  selectedWhisperModelId: "aruvi.mobile.selected_whisper_model_id",
  installedWhisperModels: "aruvi.mobile.installed_whisper_models",
};

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
  const [voiceMode, setVoiceMode] = useState<VoiceMode>("assistant");
  const [plannerChatSessionId, setPlannerChatSessionId] = useState<string | null>(null);
  const [plannerContextProductName, setPlannerContextProductName] = useState<string | null>(null);
  const [readReplies, setReadReplies] = useState(true);
  const [selectedWhisperModelId, setSelectedWhisperModelId] = useState(WHISPER_MODELS[0].id);
  const [installedWhisperModels, setInstalledWhisperModels] = useState<Record<string, InstalledWhisperModel>>({});
  const [modelInstallStatus, setModelInstallStatus] = useState("No on-device model installed yet.");
  const [modelInstallProgress, setModelInstallProgress] = useState<number | null>(null);
  const [modelInstallBusyId, setModelInstallBusyId] = useState<string | null>(null);
  const [isSetupOpen, setIsSetupOpen] = useState(false);
  const [webReloadKey, setWebReloadKey] = useState(0);
  const [connectionCheckKey, setConnectionCheckKey] = useState(0);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>("unchecked");
  const [isSaving, setIsSaving] = useState(false);
  const [isVoiceBusy, setIsVoiceBusy] = useState(false);
  const [nativeVoiceStatus, setNativeVoiceStatus] = useState("Ready");
  const [lastVoiceTranscript, setLastVoiceTranscript] = useState("");
  const [voiceDraft, setVoiceDraft] = useState("");
  const [productPlannerDraft, setProductPlannerDraft] = useState("");
  const [productPlannerStatus, setProductPlannerStatus] = useState("Planner ready");
  const [productPlannerReply, setProductPlannerReply] = useState("");
  const [productPlannerTrace, setProductPlannerTrace] = useState<MobilePlannerToolTraceEntry[]>([]);
  const [productPlannerRecording, setProductPlannerRecording] = useState(false);
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const [voiceMessages, setVoiceMessages] = useState<VoiceMessage[]>([
    {
      id: "assistant-welcome",
      role: "assistant",
      content: "Ready when you are. Tap the mic and speak naturally.",
    },
  ]);
  const [products, setProducts] = useState<Product[]>([]);
  const [productSummary, setProductSummary] = useState<ProductTreeSummary | null>(null);
  const [productTree, setProductTree] = useState<ProductTree | null>(null);
  const [selectedProductId, setSelectedProductId] = useState<string | null>(null);
  const [selectedProductNodeId, setSelectedProductNodeId] = useState<string | null>(null);
  const [productExploreTab, setProductExploreTab] = useState<ProductExploreTab>("overview");
  const [productSearchQuery, setProductSearchQuery] = useState("");
  const [isProductLoading, setIsProductLoading] = useState(false);
  const [isProductTreeLoading, setIsProductTreeLoading] = useState(false);
  const [productError, setProductError] = useState<string | null>(null);
  const [isProductPickerOpen, setIsProductPickerOpen] = useState(false);
  const [modelCalls, setModelCalls] = useState<ModelCall[]>([]);
  const [selectedModelCallSessionKey, setSelectedModelCallSessionKey] = useState<string | null>(null);
  const [selectedModelCall, setSelectedModelCall] = useState<ModelCall | null>(null);
  const [isModelCallsLoading, setIsModelCallsLoading] = useState(false);
  const [modelCallsError, setModelCallsError] = useState<string | null>(null);

  const remoteUrl = useMemo(() => {
    const trimmed = normalizeBaseUrlForDisplay(baseUrl);
    return trimmed ? `${trimmed}/remote` : "about:blank";
  }, [baseUrl]);

  const mobileClient = useMemo(() => {
    return new PlannerMobileClient(baseUrl.trim(), token.trim());
  }, [baseUrl, token]);

  const selectedProduct = useMemo(() => {
    return products.find((product) => product.id === selectedProductId) ?? productTree?.product ?? products[0] ?? null;
  }, [productTree?.product, products, selectedProductId]);

  const selectedProductNode = useMemo(() => {
    return findProductNode(productTree?.roots ?? [], selectedProductNodeId);
  }, [productTree?.roots, selectedProductNodeId]);

  const selectedProductNodePath = useMemo(() => {
    return findProductNodePath(productTree?.roots ?? [], selectedProductNodeId);
  }, [productTree?.roots, selectedProductNodeId]);

  const productFlatNodes = useMemo(() => {
    return flattenProductNodes(productTree?.roots ?? []);
  }, [productTree?.roots]);

  const productStats = useMemo(() => {
    const roots = productTree?.roots ?? [];
    return {
      productAreas: productSummary?.product_area_count ?? productTree?.product_areas.length ?? 0,
      capabilities: productSummary?.capability_count ?? Math.max(countTreeNodes(roots) - (productTree?.product_areas.length ?? 0), 0),
      totalNodes: productSummary?.total_node_count ?? countTreeNodes(roots),
      leafNodes: productSummary?.leaf_node_count ?? countLeafNodes(roots),
    };
  }, [productSummary, productTree?.product_areas.length, productTree?.roots]);

  const visibleProductChildren = selectedProductNode?.children ?? productTree?.roots ?? [];

  const filteredProductNodes = useMemo(() => {
    const query = productSearchQuery.trim().toLowerCase();
    if (!query) return productFlatNodes;
    return productFlatNodes.filter(({ node, pathLabel }) => {
      return [
        node.name,
        node.summary,
        node.description,
        node.node_kind,
        node.node_type,
        pathLabel,
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(query));
    });
  }, [productFlatNodes, productSearchQuery]);

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

  useEffect(() => {
    let disposed = false;

    const checkConnection = async () => {
      if (!token.trim() || !baseUrl.trim()) {
        setConnectionStatus("unchecked");
        return;
      }
      setConnectionStatus("checking");
      try {
        await mobileClient.health();
        if (!disposed) {
          setConnectionStatus("connected");
        }
      } catch (error) {
        const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
        if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
          try {
            await new PlannerMobileClient(fallbackBaseUrl, token.trim()).health();
            if (!disposed) {
              setBaseUrl(fallbackBaseUrl);
              await SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, fallbackBaseUrl);
              setConnectionStatus("connected");
            }
            return;
          } catch {
            // Keep the original configured URL visible when loopback cannot reach the backend either.
          }
        }
        if (!disposed) {
          setConnectionStatus("offline");
        }
      }
    };

    void checkConnection();
    return () => {
      disposed = true;
    };
  }, [baseUrl, connectionCheckKey, mobileClient, token]);

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
        savedVoiceMode,
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
        SecureStore.getItemAsync(STORAGE_KEYS.voiceMode),
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
      if (MOBILE_TABS.some((tab) => tab.id === savedActiveTab)) {
        setActiveTab(savedActiveTab as ActiveTab);
      } else if (savedActiveTab === "chat") {
        setActiveTab("voice");
        void SecureStore.setItemAsync(STORAGE_KEYS.activeTab, "voice");
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

  const loadProductSummary = async (productId: string) => {
    const summary = await mobileClient.getProductSummary(productId);
    setProductSummary(summary);
    setSelectedProductId(productId);
    if (productTree?.product.id !== productId || !productViewNeedsTree(productExploreTab)) {
      setProductTree(null);
      setSelectedProductNodeId(null);
    }
  };

  const ensureProductTree = async (productId: string, force = false) => {
    if (!force && productTree?.product.id === productId) {
      return productTree;
    }
    try {
      setIsProductTreeLoading(true);
      setProductError(null);
      const previousProductId = selectedProductId;
      const tree = await mobileClient.getProductTree(productId);
      setProductTree(tree);
      setSelectedProductId(productId);
      if (previousProductId !== productId) {
        setSelectedProductNodeId(null);
      }
      return tree;
    } finally {
      setIsProductTreeLoading(false);
    }
  };

  const switchProductExploreTab = async (mode: ProductExploreTab) => {
    setProductExploreTab(mode);
    if (!productViewNeedsTree(mode) || !selectedProductId) {
      return;
    }
    try {
      await ensureProductTree(selectedProductId);
    } catch (error) {
      setProductError(describeError(error));
    }
  };

  const loadProducts = async (preferredProductId?: string | null) => {
    if (!token.trim()) {
      setProductError("Save a mobile API token before loading products.");
      return;
    }
    try {
      setIsProductLoading(true);
      setProductError(null);
      const loadedProducts = await mobileClient.listProducts();
      setProducts(loadedProducts);
      const nextProductId =
        preferredProductId && loadedProducts.some((product) => product.id === preferredProductId)
          ? preferredProductId
          : selectedProductId && loadedProducts.some((product) => product.id === selectedProductId)
            ? selectedProductId
            : loadedProducts[0]?.id ?? null;
      if (nextProductId) {
        await loadProductSummary(nextProductId);
        if (productViewNeedsTree(productExploreTab)) {
          await ensureProductTree(nextProductId, true);
        }
      } else {
        setProductSummary(null);
        setProductTree(null);
        setSelectedProductId(null);
        setSelectedProductNodeId(null);
      }
    } catch (error) {
      setProductError(describeError(error));
    } finally {
      setIsProductLoading(false);
    }
  };

  const loadModelCalls = async () => {
    if (!token.trim()) {
      setModelCallsError("Save a mobile API token before loading calls.");
      return;
    }
    try {
      setIsModelCallsLoading(true);
      setModelCallsError(null);
      const calls = await mobileClient.listModelCalls(100);
      const sessions = buildModelCallSessions(calls);
      setModelCalls(calls);
      setSelectedModelCallSessionKey((current) => {
        const nextSessionKey = current && sessions.some((session) => session.key === current)
          ? current
          : sessions[0]?.key ?? null;
        const nextSession = sessions.find((session) => session.key === nextSessionKey) ?? null;
        setSelectedModelCall((selectedCall) => {
          if (!nextSession) return null;
          return nextSession.calls.find((call) => call.id === selectedCall?.id)
            ?? nextSession.calls[nextSession.calls.length - 1]
            ?? null;
        });
        return nextSessionKey;
      });
    } catch (error) {
      setModelCallsError(describeError(error));
    } finally {
      setIsModelCallsLoading(false);
    }
  };

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

  const switchVoiceMode = (nextMode: VoiceMode) => {
    setVoiceMode(nextMode);
    void SecureStore.setItemAsync(STORAGE_KEYS.voiceMode, nextMode);
    setNativeVoiceStatus(nextMode === "planner" ? "Planner chat ready" : "Ready");
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
          setBaseUrl(fallbackBaseUrl);
          await SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, fallbackBaseUrl);
          setConnectionStatus("connected");
          setWebReloadKey((current) => current + 1);
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

  const createPlannerChatSessionWithFallback = async () => {
    const body = {
      provider_id: providerId.trim() || undefined,
      model_name: modelName.trim() || undefined,
      product_id: selectedProductId ?? undefined,
    };
    try {
      const response = await mobileClient.createMobilePlannerChatSession(body);
      setConnectionStatus("connected");
      setPlannerContextProductName(response.product_name ?? null);
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim()).createMobilePlannerChatSession(body);
          setBaseUrl(fallbackBaseUrl);
          await SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, fallbackBaseUrl);
          setConnectionStatus("connected");
          setPlannerContextProductName(response.product_name ?? null);
          setWebReloadKey((current) => current + 1);
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

  const runPlannerChatWithFallback = async (sessionId: string, body: PlannerChatTurnBody) => {
    try {
      const response = await mobileClient.submitMobilePlannerChatTurn(sessionId, body);
      setConnectionStatus("connected");
      setPlannerContextProductName(response.product_name ?? null);
      return response;
    } catch (error) {
      const fallbackBaseUrl = getLoopbackFallbackBaseUrl(baseUrl);
      if (isNetworkRequestFailure(error) && fallbackBaseUrl) {
        try {
          const response = await new PlannerMobileClient(fallbackBaseUrl, token.trim()).submitMobilePlannerChatTurn(sessionId, body);
          setBaseUrl(fallbackBaseUrl);
          await SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, fallbackBaseUrl);
          setConnectionStatus("connected");
          setPlannerContextProductName(response.product_name ?? null);
          setWebReloadKey((current) => current + 1);
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

  const submitPlannerPrompt = async (trimmed: string) => {
    const activeSessionId = plannerChatSessionId ?? (await createPlannerChatSessionWithFallback()).session_id;
    if (!plannerChatSessionId) {
      setPlannerChatSessionId(activeSessionId);
    }
    const response = await runPlannerChatWithFallback(activeSessionId, {
      provider_id: providerId.trim() || undefined,
      model_name: modelName.trim() || undefined,
      product_id: selectedProductId ?? undefined,
      messages: [
        {
          role: "user",
          content: trimmed,
        },
      ],
      max_tool_steps: 4,
    });
    const assistantText = response.assistant_message.trim() || "(empty planner response)";
    return {
      content: assistantText,
      toolTrace: response.tool_trace,
    };
  };

  const buildProductPlannerPrompt = (instruction: string) => {
    const pathLabel = selectedProductNodePath.map((node) => node.name).join(" / ");
    const contextLines = [
      "Current mobile Products screen context:",
      selectedProduct ? `Product: ${selectedProduct.name} (${selectedProduct.id})` : "Product: none selected",
      selectedProductNode
        ? `Selected node: ${selectedProductNode.name} (${selectedProductNode.id})`
        : "Selected node: product root",
      selectedProductNode ? `Node type: ${selectedProductNode.node_type}` : null,
      selectedProductNode ? `Node kind: ${selectedProductNode.node_kind}` : null,
      selectedProductNode?.product_area_id ? `Product Area id: ${selectedProductNode.product_area_id}` : null,
      selectedProductNode?.capability_id ? `Capability id: ${selectedProductNode.capability_id}` : null,
      pathLabel ? `Path: ${pathLabel}` : null,
      selectedProductNode ? `Node summary: ${getNodeSummary(selectedProductNode)}` : null,
      "",
      "User instruction:",
      instruction,
      "",
      "Use the selected node as the working context. If the user asks to add children, sub-items, revise, split, or expand, call the appropriate MCP catalog/work item tools against this product/node. After changes, summarize exactly what changed.",
    ].filter((line): line is string => Boolean(line));
    return contextLines.join("\n");
  };

  const submitProductPlannerPrompt = async (instruction: string, source: VoicePromptSource = "typed") => {
    const trimmed = instruction.trim();
    if (!trimmed) return;
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using the planner.");
      return;
    }
    try {
      setVoiceMode("planner");
      setProductPlannerStatus(source === "recording" ? "Processing voice instruction..." : "Planning...");
      setIsVoiceBusy(true);
      const prompt = buildProductPlannerPrompt(trimmed);
      const nodeIdToRestore = selectedProductNodeId;
      const result = await submitPlannerPrompt(prompt);
      setProductPlannerReply(result.content);
      setProductPlannerTrace(result.toolTrace ?? []);
      setProductPlannerDraft("");
      setProductPlannerStatus("Ready for follow-up");
      if (readReplies) {
        speakAssistantReply(result.content);
      }
      if (selectedProductId) {
        await loadProducts(selectedProductId);
        setSelectedProductNodeId(nodeIdToRestore);
      }
    } catch (error) {
      const message = describeError(error);
      setProductPlannerStatus(message);
      Alert.alert("Planner failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const startProductPlannerRecording = async () => {
    if (!token.trim()) {
      Alert.alert("Setup required", "Save a mobile API token before using the planner mic.");
      return;
    }
    if (!canUseLocalSpeech) {
      Alert.alert("Install model first", "Install an on-device Whisper model before using voice recording.");
      switchTab("models");
      return;
    }
    try {
      setIsVoiceBusy(true);
      setProductPlannerRecording(true);
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
      setProductPlannerStatus("Listening...");
    } catch (error) {
      const message = describeError(error);
      setProductPlannerRecording(false);
      setProductPlannerStatus(message);
      Alert.alert("Planner voice failed", message);
    } finally {
      setIsVoiceBusy(false);
    }
  };

  const stopProductPlannerRecording = async () => {
    try {
      setIsVoiceBusy(true);
      setProductPlannerStatus("Stopping...");
      await audioRecorder.stop();
      const recordingUri = audioRecorder.uri ?? audioRecorder.getStatus().url;
      if (!recordingUri) {
        throw new Error("Recording did not produce an audio file.");
      }
      setProductPlannerStatus("Transcribing...");
      const transcript = await transcribeNativeRecording(recordingUri);
      setProductPlannerDraft(transcript);
      if (!transcript.trim()) {
        setProductPlannerStatus("No speech detected");
        return;
      }
      setProductPlannerStatus("Transcript ready");
    } catch (error) {
      const message = describeError(error);
      setProductPlannerStatus(message);
      Alert.alert("Planner voice failed", message);
    } finally {
      setProductPlannerRecording(false);
      setIsVoiceBusy(false);
    }
  };

  const toggleProductPlannerRecording = async () => {
    if (productPlannerRecording || recorderState.isRecording) {
      await stopProductPlannerRecording();
    } else {
      await startProductPlannerRecording();
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

  const openProductNode = (nodeId: string) => {
    setSelectedProductNodeId(nodeId);
    setProductExploreTab("map");
  };

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
              setWebReloadKey((current) => current + 1);
            }
            setConnectionCheckKey((current) => current + 1);
          }}
          onBaseUrlChange={setBaseUrl}
          onTokenChange={setToken}
          onProviderIdChange={setProviderId}
          onModelNameChange={setModelName}
          onReadRepliesChange={setReadRepliesPreference}
          onSaveConnection={saveConnection}
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
