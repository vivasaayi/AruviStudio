import { invoke } from "./core";
import type {
  PlannerContactResult,
  PlannerDraftChildType,
  PlannerSessionInfo,
  PlannerTurnResponse,
  SpeechToTextResponse,
} from "../types";

// Planner commands
export const createPlannerSession = (data?: { providerId?: string; modelName?: string }) =>
  invoke<PlannerSessionInfo>("create_planner_session_command", {
    providerId: data?.providerId ?? null,
    provider_id: data?.providerId ?? null,
    modelName: data?.modelName ?? null,
    model_name: data?.modelName ?? null,
  });

export const updatePlannerSession = (data: { sessionId: string; providerId?: string; modelName?: string }) =>
  invoke<PlannerSessionInfo>("update_planner_session_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    providerId: data.providerId ?? null,
    provider_id: data.providerId ?? null,
    modelName: data.modelName ?? null,
    model_name: data.modelName ?? null,
  });

export const clearPlannerPending = (sessionId: string) =>
  invoke<PlannerSessionInfo>("clear_planner_pending_command", {
    sessionId,
    session_id: sessionId,
  });

export const submitPlannerTurn = (data: { sessionId: string; userInput: string; selectedDraftNodeId?: string | null; productId?: string | null }) =>
  invoke<PlannerTurnResponse>("submit_planner_turn_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    userInput: data.userInput,
    user_input: data.userInput,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
    productId: data.productId ?? null,
    product_id: data.productId ?? null,
  });

export const submitPlannerVoiceTurn = (data: { sessionId: string; transcript: string; selectedDraftNodeId?: string | null; productId?: string | null }) =>
  invoke<PlannerTurnResponse>("submit_planner_voice_turn_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    transcript: data.transcript,
    userInput: data.transcript,
    user_input: data.transcript,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
    productId: data.productId ?? null,
    product_id: data.productId ?? null,
  });

export const confirmPlannerPlan = (sessionId: string) =>
  invoke<PlannerTurnResponse>("confirm_planner_plan_command", {
    sessionId,
    session_id: sessionId,
  });

export const renamePlannerDraftNode = (data: {
  sessionId: string;
  nodeId: string;
  name: string;
}) =>
  invoke<PlannerTurnResponse>("rename_planner_draft_node_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    nodeId: data.nodeId,
    node_id: data.nodeId,
    name: data.name,
  });

export const addPlannerDraftChild = (data: {
  sessionId: string;
  parentNodeId: string;
  childType: PlannerDraftChildType;
  name: string;
  summary?: string;
}) =>
  invoke<PlannerTurnResponse>("add_planner_draft_child_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    parentNodeId: data.parentNodeId,
    parent_node_id: data.parentNodeId,
    childType: data.childType,
    child_type: data.childType,
    name: data.name,
    summary: data.summary ?? null,
  });

export const deletePlannerDraftNode = (data: {
  sessionId: string;
  nodeId: string;
}) =>
  invoke<PlannerTurnResponse>("delete_planner_draft_node_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    nodeId: data.nodeId,
    node_id: data.nodeId,
  });

export const analyzeRepositoryForPlanner = (data: {
  sessionId: string;
  repositoryId: string;
  selectedDraftNodeId?: string | null;
  productId?: string | null;
}) =>
  invoke<PlannerTurnResponse>("analyze_repository_for_planner_command", {
    sessionId: data.sessionId,
    session_id: data.sessionId,
    repositoryId: data.repositoryId,
    repository_id: data.repositoryId,
    selectedDraftNodeId: data.selectedDraftNodeId ?? null,
    selected_draft_node_id: data.selectedDraftNodeId ?? null,
    productId: data.productId ?? null,
    product_id: data.productId ?? null,
  });

export const transcribeAudio = (data: {
  providerId?: string;
  modelName?: string;
  audioBytesBase64: string;
  mimeType: string;
  locale?: string;
}) =>
  invoke<SpeechToTextResponse>("transcribe_audio_command", {
    providerId: data.providerId ?? null,
    provider_id: data.providerId ?? null,
    modelName: data.modelName ?? null,
    model_name: data.modelName ?? null,
    audioBytesBase64: data.audioBytesBase64,
    audio_bytes_base64: data.audioBytesBase64,
    mimeType: data.mimeType,
    mime_type: data.mimeType,
    locale: data.locale ?? null,
  });

export const speakTextNatively = (data: {
  text: string;
  voice?: string;
  locale?: string;
}) =>
  invoke<void>("speak_text_natively_command", {
    text: data.text,
    voice: data.voice ?? null,
    locale: data.locale ?? null,
  });

export const sendTwilioWhatsappMessage = (data: { to: string; content: string }) =>
  invoke<void>("send_twilio_whatsapp_message", {
    to: data.to,
    content: data.content,
  });

export const startTwilioVoiceCall = (data: { to: string; initialPrompt?: string }) =>
  invoke<void>("start_twilio_voice_call", {
    to: data.to,
    initial_prompt: data.initialPrompt ?? null,
  });

export const routePlannerContact = (data: {
  to: string;
  content: string;
  preferredChannel?: "whatsapp" | "voice";
  allowAfterHours?: boolean;
}) =>
  invoke<PlannerContactResult>("route_planner_contact_command", {
    to: data.to,
    content: data.content,
    preferred_channel: data.preferredChannel ?? null,
    allow_after_hours: data.allowAfterHours ?? null,
  });
