import { useState } from "react";
import { Alert } from "react-native";
import { AudioModule, setAudioModeAsync } from "expo-audio";
import * as Speech from "expo-speech";
import { getNodeSummary } from "../lib/productTree";
import type { HierarchyTreeNode, MobilePlannerToolTraceEntry, Product } from "../types";

type VoicePromptSource = "typed" | "recording";

type ProductPlannerAudioRecorder = {
  prepareToRecordAsync: () => Promise<void>;
  record: () => void;
  stop: () => Promise<void>;
  uri: string | null;
  getStatus: () => {
    url?: string | null;
  };
};

type SubmitPlannerPromptResult = {
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};

type MobileProductPlannerControllerInput = {
  token: string;
  readReplies: boolean;
  canUseLocalSpeech: boolean;
  selectedProduct: Product | null;
  selectedProductId: string | null;
  selectedProductNode: HierarchyTreeNode | null;
  selectedProductNodePath: HierarchyTreeNode[];
  selectedProductNodeId: string | null;
  audioRecorder: ProductPlannerAudioRecorder;
  isRecorderRecording: boolean;
  submitPlannerPrompt: (prompt: string) => Promise<SubmitPlannerPromptResult>;
  loadProducts: (preferredProductId?: string | null) => Promise<void>;
  setSelectedProductNodeId: (nodeId: string | null) => void;
  setVoiceMode: (mode: "assistant" | "planner") => void;
  setIsVoiceBusy: (isBusy: boolean) => void;
  switchTab: (nextTab: "models") => void;
  speakAssistantReply: (text: string) => void;
  transcribeRecording: (uri: string) => Promise<string>;
  describeError: (error: unknown) => string;
};

export function useMobileProductPlannerController({
  token,
  readReplies,
  canUseLocalSpeech,
  selectedProduct,
  selectedProductId,
  selectedProductNode,
  selectedProductNodePath,
  selectedProductNodeId,
  audioRecorder,
  isRecorderRecording,
  submitPlannerPrompt,
  loadProducts,
  setSelectedProductNodeId,
  setVoiceMode,
  setIsVoiceBusy,
  switchTab,
  speakAssistantReply,
  transcribeRecording,
  describeError,
}: MobileProductPlannerControllerInput) {
  const [productPlannerDraft, setProductPlannerDraft] = useState("");
  const [productPlannerStatus, setProductPlannerStatus] = useState("Planner ready");
  const [productPlannerReply, setProductPlannerReply] = useState("");
  const [productPlannerTrace, setProductPlannerTrace] = useState<MobilePlannerToolTraceEntry[]>([]);
  const [productPlannerRecording, setProductPlannerRecording] = useState(false);

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

  const submitProductPlannerPrompt = async (
    instruction: string,
    source: VoicePromptSource = "typed",
  ) => {
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
      const transcript = await transcribeRecording(recordingUri);
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
    if (productPlannerRecording || isRecorderRecording) {
      await stopProductPlannerRecording();
    } else {
      await startProductPlannerRecording();
    }
  };

  return {
    productPlannerDraft,
    productPlannerStatus,
    productPlannerReply,
    productPlannerTrace,
    productPlannerRecording,
    setProductPlannerDraft,
    submitProductPlannerPrompt,
    toggleProductPlannerRecording,
  };
}
