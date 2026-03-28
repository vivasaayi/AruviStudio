import React, { useEffect, useMemo, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Pressable,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  View,
} from "react-native";
import { Audio } from "expo-av";
import * as FileSystem from "expo-file-system";
import * as SecureStore from "expo-secure-store";
import * as Speech from "expo-speech";
import { PlannerMobileClient } from "./src/api/client";
import type { PlannerTreeNode, PlannerTurnResponse } from "./src/types";

const STORAGE_KEYS = {
  baseUrl: "aruvi.mobile.base_url",
  token: "aruvi.mobile.token",
  providerId: "aruvi.mobile.provider_id",
  modelName: "aruvi.mobile.model_name",
  locale: "aruvi.mobile.locale",
};

type PlannerMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
};

function makeId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function DraftTree({
  nodes,
  selectedNodeId,
  onSelect,
  depth = 0,
}: {
  nodes: PlannerTreeNode[];
  selectedNodeId: string | null;
  onSelect: (nodeId: string) => void;
  depth?: number;
}) {
  return (
    <View style={styles.treeGroup}>
      {nodes.map((node) => {
        const selected = node.id === selectedNodeId;
        return (
          <View key={node.id}>
            <Pressable
              style={[styles.treeNode, selected ? styles.treeNodeSelected : null, { marginLeft: depth * 12 }]}
              onPress={() => onSelect(node.id)}
            >
              <Text style={styles.treeNodeTitle}>{node.label}</Text>
              {node.meta ? <Text style={styles.treeNodeMeta}>{node.meta}</Text> : null}
              {node.summary ? <Text style={styles.treeNodeSummary}>{node.summary}</Text> : null}
            </Pressable>
            {node.children.length > 0 ? (
              <DraftTree
                nodes={node.children}
                selectedNodeId={selectedNodeId}
                onSelect={onSelect}
                depth={depth + 1}
              />
            ) : null}
          </View>
        );
      })}
    </View>
  );
}

export default function App() {
  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:8787");
  const [token, setToken] = useState("");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [locale, setLocale] = useState("en-US");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [composer, setComposer] = useState("");
  const [selectedDraftNodeId, setSelectedDraftNodeId] = useState<string | null>(null);
  const [draftTreeNodes, setDraftTreeNodes] = useState<PlannerTreeNode[]>([]);
  const [messages, setMessages] = useState<PlannerMessage[]>([
    {
      id: makeId(),
      role: "assistant",
      content: "Connect to your desktop planner bridge, then talk to the same staged planner session from iPhone.",
    },
  ]);
  const [recording, setRecording] = useState<Audio.Recording | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [autoSpeak, setAutoSpeak] = useState(false);

  const client = useMemo(() => new PlannerMobileClient(baseUrl.trim(), token.trim()), [baseUrl, token]);

  useEffect(() => {
    void (async () => {
      const [savedBaseUrl, savedToken, savedProviderId, savedModelName, savedLocale] = await Promise.all([
        SecureStore.getItemAsync(STORAGE_KEYS.baseUrl),
        SecureStore.getItemAsync(STORAGE_KEYS.token),
        SecureStore.getItemAsync(STORAGE_KEYS.providerId),
        SecureStore.getItemAsync(STORAGE_KEYS.modelName),
        SecureStore.getItemAsync(STORAGE_KEYS.locale),
      ]);
      if (savedBaseUrl) setBaseUrl(savedBaseUrl);
      if (savedToken) setToken(savedToken);
      if (savedProviderId) setProviderId(savedProviderId);
      if (savedModelName) setModelName(savedModelName);
      if (savedLocale) setLocale(savedLocale);
    })();
  }, []);

  const appendAssistantReply = (response: PlannerTurnResponse) => {
    const content = [
      response.assistant_message,
      ...response.execution_lines,
      ...(response.execution_errors.length > 0 ? [`Errors: ${response.execution_errors.join(" | ")}`] : []),
    ]
      .filter(Boolean)
      .join("\n");
    setMessages((current) => [...current, { id: makeId(), role: "assistant", content }]);
    setDraftTreeNodes(response.draft_tree_nodes ?? []);
    setSelectedDraftNodeId(response.selected_draft_node_id ?? null);
    if (autoSpeak && content.trim()) {
      Speech.stop();
      Speech.speak(content, { language: locale });
    }
  };

  const saveConnection = async () => {
    await Promise.all([
      SecureStore.setItemAsync(STORAGE_KEYS.baseUrl, baseUrl.trim()),
      SecureStore.setItemAsync(STORAGE_KEYS.token, token.trim()),
      SecureStore.setItemAsync(STORAGE_KEYS.providerId, providerId.trim()),
      SecureStore.setItemAsync(STORAGE_KEYS.modelName, modelName.trim()),
      SecureStore.setItemAsync(STORAGE_KEYS.locale, locale.trim()),
    ]);
    Alert.alert("Saved", "Planner mobile connection settings are stored on this device.");
  };

  const ensureSession = async () => {
    if (sessionId) {
      return sessionId;
    }
    const session = await client.createPlannerSession({
      provider_id: providerId.trim() || undefined,
      model_name: modelName.trim() || undefined,
    });
    setSessionId(session.session_id);
    return session.session_id;
  };

  const sendTurn = async () => {
    if (!composer.trim()) {
      return;
    }
    try {
      setIsBusy(true);
      const nextSessionId = await ensureSession();
      const prompt = composer.trim();
      setComposer("");
      setMessages((current) => [...current, { id: makeId(), role: "user", content: prompt }]);
      const response = await client.submitPlannerTurn(nextSessionId, {
        user_input: prompt,
        selected_draft_node_id: selectedDraftNodeId,
      });
      appendAssistantReply(response);
    } catch (error) {
      Alert.alert("Planner error", error instanceof Error ? error.message : String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const confirmDraft = async () => {
    if (!sessionId) {
      return;
    }
    try {
      setIsBusy(true);
      const response = await client.confirmPlannerDraft(sessionId);
      appendAssistantReply(response);
      if (!response.draft_tree_nodes) {
        setDraftTreeNodes([]);
        setSelectedDraftNodeId(null);
      }
    } catch (error) {
      Alert.alert("Commit failed", error instanceof Error ? error.message : String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const clearDraft = async () => {
    if (!sessionId) {
      return;
    }
    try {
      setIsBusy(true);
      await client.clearPlannerDraft(sessionId);
      setDraftTreeNodes([]);
      setSelectedDraftNodeId(null);
    } catch (error) {
      Alert.alert("Clear failed", error instanceof Error ? error.message : String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const startRecording = async () => {
    try {
      const permission = await Audio.requestPermissionsAsync();
      if (permission.status !== "granted") {
        Alert.alert("Microphone required", "Allow microphone access to use voice planning.");
        return;
      }
      await Audio.setAudioModeAsync({
        allowsRecordingIOS: true,
        playsInSilentModeIOS: true,
      });
      const nextRecording = new Audio.Recording();
      await nextRecording.prepareToRecordAsync(Audio.RecordingOptionsPresets.HIGH_QUALITY);
      await nextRecording.startAsync();
      setRecording(nextRecording);
    } catch (error) {
      Alert.alert("Voice failed", error instanceof Error ? error.message : String(error));
    }
  };

  const stopRecording = async () => {
    if (!recording) {
      return;
    }
    try {
      setIsBusy(true);
      await recording.stopAndUnloadAsync();
      const uri = recording.getURI();
      setRecording(null);
      if (!uri) {
        return;
      }
      const audioBase64 = await FileSystem.readAsStringAsync(uri, {
        encoding: FileSystem.EncodingType.Base64,
      });
      const transcription = await client.transcribeSpeech({
        provider_id: providerId.trim() || undefined,
        model_name: modelName.trim() || undefined,
        audio_bytes_base64: audioBase64,
        mime_type: "audio/m4a",
        locale,
      });
      setComposer((current) => (current ? `${current.trim()} ${transcription.transcript}` : transcription.transcript));
    } catch (error) {
      Alert.alert("Transcription failed", error instanceof Error ? error.message : String(error));
    } finally {
      setIsBusy(false);
    }
  };

  return (
    <SafeAreaView style={styles.safeArea}>
      <ScrollView contentContainerStyle={styles.container}>
        <Text style={styles.title}>Aruvi Planner Mobile</Text>
        <Text style={styles.subtitle}>
          Securely connect to the desktop planner bridge, stage draft structure, and keep iterating from iPhone against the same planner and speech APIs.
        </Text>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Connection</Text>
          <TextInput style={styles.input} value={baseUrl} onChangeText={setBaseUrl} placeholder="http://desktop-ip:8787" placeholderTextColor="#7d8898" />
          <TextInput style={styles.input} value={token} onChangeText={setToken} placeholder="mobile api token" placeholderTextColor="#7d8898" secureTextEntry />
          <TextInput style={styles.input} value={providerId} onChangeText={setProviderId} placeholder="optional planner provider id" placeholderTextColor="#7d8898" />
          <TextInput style={styles.input} value={modelName} onChangeText={setModelName} placeholder="optional planner model name" placeholderTextColor="#7d8898" />
          <TextInput style={styles.input} value={locale} onChangeText={setLocale} placeholder="en-US" placeholderTextColor="#7d8898" />
          <Pressable style={styles.primaryButton} onPress={() => void saveConnection()}>
            <Text style={styles.primaryButtonText}>Save Connection</Text>
          </Pressable>
        </View>

        <View style={styles.card}>
          <View style={styles.row}>
            <Text style={styles.sectionTitle}>Planner</Text>
            <View style={styles.switchRow}>
              <Text style={styles.helper}>Voice replies</Text>
              <Switch value={autoSpeak} onValueChange={setAutoSpeak} />
            </View>
          </View>
          <View style={styles.row}>
            <Pressable style={styles.secondaryButton} onPress={() => void startRecording()} disabled={Boolean(recording) || isBusy}>
              <Text style={styles.secondaryButtonText}>{recording ? "Recording..." : "Start Voice"}</Text>
            </Pressable>
            <Pressable style={styles.secondaryButton} onPress={() => void stopRecording()} disabled={!recording || isBusy}>
              <Text style={styles.secondaryButtonText}>Stop Voice</Text>
            </Pressable>
            <Pressable style={styles.secondaryButton} onPress={() => void confirmDraft()} disabled={!sessionId || draftTreeNodes.length === 0 || isBusy}>
              <Text style={styles.secondaryButtonText}>Commit</Text>
            </Pressable>
            <Pressable style={styles.dangerButton} onPress={() => void clearDraft()} disabled={!sessionId || isBusy}>
              <Text style={styles.secondaryButtonText}>Clear</Text>
            </Pressable>
          </View>
          <TextInput
            style={styles.textarea}
            value={composer}
            onChangeText={setComposer}
            multiline
            placeholder="Describe what to plan, or tap Start Voice and speak."
            placeholderTextColor="#7d8898"
          />
          <Pressable style={styles.primaryButton} onPress={() => void sendTurn()} disabled={isBusy}>
            <Text style={styles.primaryButtonText}>{isBusy ? "Working..." : "Send To Planner"}</Text>
          </Pressable>
          {isBusy ? <ActivityIndicator color="#7bc8ff" style={styles.spinner} /> : null}
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Conversation</Text>
          {messages.map((message) => (
            <View key={message.id} style={message.role === "user" ? styles.userBubble : styles.assistantBubble}>
              <Text style={styles.bubbleText}>{message.content}</Text>
            </View>
          ))}
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Draft Tree</Text>
          {draftTreeNodes.length > 0 ? (
            <DraftTree nodes={draftTreeNodes} selectedNodeId={selectedDraftNodeId} onSelect={setSelectedDraftNodeId} />
          ) : (
            <Text style={styles.helper}>No staged draft yet. Ask the planner to design or revise the product structure first.</Text>
          )}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safeArea: {
    flex: 1,
    backgroundColor: "#111317",
  },
  container: {
    padding: 18,
    gap: 16,
  },
  title: {
    color: "#f4f8ff",
    fontSize: 28,
    fontWeight: "800",
  },
  subtitle: {
    color: "#a8b2c4",
    fontSize: 14,
    lineHeight: 20,
  },
  card: {
    backgroundColor: "#181c22",
    borderRadius: 18,
    borderWidth: 1,
    borderColor: "#2f3642",
    padding: 14,
    gap: 12,
  },
  sectionTitle: {
    color: "#f4f8ff",
    fontSize: 16,
    fontWeight: "700",
  },
  helper: {
    color: "#9aa8bd",
    fontSize: 13,
    lineHeight: 18,
  },
  row: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center",
    gap: 10,
    flexWrap: "wrap",
  },
  switchRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
  },
  input: {
    backgroundColor: "#12161c",
    borderColor: "#364152",
    borderWidth: 1,
    borderRadius: 12,
    paddingHorizontal: 12,
    paddingVertical: 10,
    color: "#f4f8ff",
  },
  textarea: {
    backgroundColor: "#12161c",
    borderColor: "#364152",
    borderWidth: 1,
    borderRadius: 12,
    paddingHorizontal: 12,
    paddingVertical: 12,
    minHeight: 100,
    color: "#f4f8ff",
    textAlignVertical: "top",
  },
  primaryButton: {
    backgroundColor: "#0e639c",
    borderRadius: 12,
    paddingVertical: 12,
    alignItems: "center",
  },
  primaryButtonText: {
    color: "#ffffff",
    fontWeight: "700",
    fontSize: 15,
  },
  secondaryButton: {
    backgroundColor: "#223040",
    borderRadius: 12,
    paddingVertical: 10,
    paddingHorizontal: 14,
  },
  secondaryButtonText: {
    color: "#edf3ff",
    fontWeight: "700",
  },
  dangerButton: {
    backgroundColor: "#6c2020",
    borderRadius: 12,
    paddingVertical: 10,
    paddingHorizontal: 14,
  },
  spinner: {
    marginTop: 8,
  },
  userBubble: {
    alignSelf: "flex-end",
    backgroundColor: "#0e639c",
    borderRadius: 14,
    padding: 12,
    maxWidth: "88%",
  },
  assistantBubble: {
    alignSelf: "flex-start",
    backgroundColor: "#27303c",
    borderRadius: 14,
    padding: 12,
    maxWidth: "92%",
  },
  bubbleText: {
    color: "#f4f8ff",
    lineHeight: 20,
  },
  treeGroup: {
    gap: 8,
  },
  treeNode: {
    borderRadius: 12,
    borderWidth: 1,
    borderColor: "#364152",
    backgroundColor: "#111821",
    padding: 10,
    gap: 4,
  },
  treeNodeSelected: {
    borderColor: "#0e639c",
    backgroundColor: "#173450",
  },
  treeNodeTitle: {
    color: "#f4f8ff",
    fontWeight: "700",
    fontSize: 14,
  },
  treeNodeMeta: {
    color: "#9aa8bd",
    fontSize: 11,
    textTransform: "uppercase",
  },
  treeNodeSummary: {
    color: "#c4d1e4",
    fontSize: 12,
    lineHeight: 18,
  },
});
