import React from "react";
import { Pressable, ScrollView, Text, TextInput, View } from "react-native";
import type { MobilePlannerToolTraceEntry } from "../types";
import { compactJson, formatPlannerToolTrace } from "../lib/mobileFormatters";
import { styles } from "../styles/appStyles";

type VoiceMode = "assistant" | "planner";

type VoiceMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolTrace?: MobilePlannerToolTraceEntry[];
};

type MobileVoiceScreenProps = {
  voiceMode: VoiceMode;
  nativeVoiceStatus: string;
  plannerContextLabel: string;
  plannerRuntimeLabel: string;
  speechModelLabel: string;
  voiceMessages: VoiceMessage[];
  isVoiceKeyboardOpen: boolean;
  keyboardHeight: number;
  isVoiceBusy: boolean;
  isRecording: boolean;
  voiceComposerStatus: string;
  voiceDraft: string;
  nativeVoiceButtonDisabled: boolean;
  canUseLocalSpeech: boolean;
  token: string;
  speechModelDescription: string;
  onOpenModels: () => void;
  onSwitchVoiceMode: (nextMode: VoiceMode) => void;
  onVoiceDraftChange: (nextDraft: string) => void;
  onClearVoiceDraft: () => void;
  onToggleNativeVoiceRecording: () => Promise<void>;
  onSubmitVoicePrompt: (prompt: string) => Promise<void>;
};

export function MobileVoiceScreen({
  voiceMode,
  nativeVoiceStatus,
  plannerContextLabel,
  plannerRuntimeLabel,
  speechModelLabel,
  voiceMessages,
  isVoiceKeyboardOpen,
  keyboardHeight,
  isVoiceBusy,
  isRecording,
  voiceComposerStatus,
  voiceDraft,
  nativeVoiceButtonDisabled,
  canUseLocalSpeech,
  token,
  speechModelDescription,
  onOpenModels,
  onSwitchVoiceMode,
  onVoiceDraftChange,
  onClearVoiceDraft,
  onToggleNativeVoiceRecording,
  onSubmitVoicePrompt,
}: MobileVoiceScreenProps) {
  return (
    <View style={styles.voiceScreen}>
      <View style={styles.voiceTopBand}>
        <View style={styles.voiceTopCopy}>
          <Text style={styles.voiceTitle}>{voiceMode === "planner" ? "Planner Chat" : "Voice"}</Text>
          <Text style={styles.voiceSubtitle} numberOfLines={1}>
            {voiceMode === "planner" ? `${nativeVoiceStatus} · ${plannerContextLabel}` : nativeVoiceStatus}
          </Text>
        </View>
        <Pressable style={styles.runtimeChip} onPress={onOpenModels}>
          <Text style={styles.runtimeChipText} numberOfLines={1}>
            {voiceMode === "planner" ? plannerRuntimeLabel : speechModelLabel}
          </Text>
        </Pressable>
      </View>
      <View style={styles.voiceModeRow}>
        <Pressable
          style={[styles.voiceModeButton, voiceMode === "assistant" && styles.voiceModeButtonActive]}
          onPress={() => onSwitchVoiceMode("assistant")}
        >
          <Text style={[styles.voiceModeText, voiceMode === "assistant" && styles.voiceModeTextActive]}>Assistant</Text>
        </Pressable>
        <Pressable
          style={[styles.voiceModeButton, voiceMode === "planner" && styles.voiceModeButtonActive]}
          onPress={() => onSwitchVoiceMode("planner")}
        >
          <Text style={[styles.voiceModeText, voiceMode === "planner" && styles.voiceModeTextActive]}>Planner</Text>
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
            {message.toolTrace?.length ? (
              <View style={styles.plannerTraceList}>
                {message.toolTrace.map((entry) => (
                  <View key={`${message.id}-${entry.step}-${entry.tool_name}`} style={styles.plannerTraceCard}>
                    <View style={styles.plannerTraceHeader}>
                      <Text style={styles.plannerTraceTitle} numberOfLines={1}>{formatPlannerToolTrace(entry)}</Text>
                      <Text style={[styles.plannerTraceStatus, entry.error && styles.plannerTraceStatusError]}>
                        {entry.error ? "Error" : "OK"}
                      </Text>
                    </View>
                    <Text style={styles.plannerTraceMeta} numberOfLines={2}>
                      {entry.error ? entry.error : compactJson(entry.arguments)}
                    </Text>
                  </View>
                ))}
              </View>
            ) : null}
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
            {isRecording ? "Listening" : isVoiceBusy ? nativeVoiceStatus : "Voice transcript"}
          </Text>
          <Text style={styles.voiceComposerStatus} numberOfLines={1}>
            {voiceComposerStatus}
          </Text>
        </View>
        <TextInput
          style={styles.voiceComposerInput}
          value={voiceDraft}
          onChangeText={onVoiceDraftChange}
          placeholder={voiceMode === "planner" ? "Ask the planner to inspect or update the product" : "Speak or type a message"}
          placeholderTextColor="#7f8a9c"
          multiline
          textAlignVertical="top"
        />
        <View style={styles.voiceComposerActions}>
          <Pressable
            style={[styles.voiceClearButton, (!voiceDraft.trim() || isVoiceBusy || isRecording) && styles.buttonDisabled]}
            onPress={onClearVoiceDraft}
            disabled={!voiceDraft.trim() || isVoiceBusy || isRecording}
          >
            <Text style={styles.voiceClearButtonText}>Clear</Text>
          </Pressable>
          <View style={styles.voiceComposerSpacer} />
          <Pressable
            style={[
              styles.voiceMicButton,
              isRecording && styles.voiceMicButtonRecording,
              nativeVoiceButtonDisabled && !isRecording && styles.buttonDisabled,
            ]}
            onPress={() => void onToggleNativeVoiceRecording()}
            disabled={nativeVoiceButtonDisabled && !isRecording}
          >
            <Text style={styles.voiceMicButtonText}>
              {isRecording ? "Stop" : canUseLocalSpeech ? "Mic" : "Install"}
            </Text>
          </Pressable>
          <Pressable
            style={[
              styles.voiceSendButton,
              (!voiceDraft.trim() || isVoiceBusy || isRecording) && styles.buttonDisabled,
            ]}
            onPress={() => void onSubmitVoicePrompt(voiceDraft)}
            disabled={!voiceDraft.trim() || isVoiceBusy || isRecording}
          >
            <Text style={styles.voiceSendButtonText}>↑</Text>
          </Pressable>
        </View>
        <Text style={styles.voiceControlHint} numberOfLines={1}>
          {token.trim()
            ? voiceMode === "planner"
              ? "Planner mode uses backend MCP tools and your selected model."
              : speechModelDescription
            : "Save setup first."}
        </Text>
      </View>
    </View>
  );
}
