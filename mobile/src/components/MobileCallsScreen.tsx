import React from "react";
import { Pressable, ScrollView, Text, View } from "react-native";
import type { ModelCall } from "../types";
import {
  buildModelCallSessions,
  formatDurationMs,
  formatInteger,
  formatSourceKind,
  type ModelCallSessionSummary,
} from "../lib/mobileFormatters";
import { styles } from "../styles/appStyles";

type MobileCallsScreenProps = {
  modelCalls: ModelCall[];
  selectedModelCallSessionKey: string | null;
  selectedModelCall: ModelCall | null;
  isModelCallsLoading: boolean;
  modelCallsError: string | null;
  onLoadModelCalls: () => Promise<void>;
  onSelectedModelCallSessionKeyChange: (key: string) => void;
  onSelectedModelCallChange: (call: ModelCall | null) => void;
};

export function MobileCallsScreen({
  modelCalls,
  selectedModelCallSessionKey,
  selectedModelCall,
  isModelCallsLoading,
  modelCallsError,
  onLoadModelCalls,
  onSelectedModelCallSessionKeyChange,
  onSelectedModelCallChange,
}: MobileCallsScreenProps) {
  const callSessions = buildModelCallSessions(modelCalls);
  const selectedSession = callSessions.find((session) => session.key === selectedModelCallSessionKey)
    ?? callSessions[0]
    ?? null;
  const selected = selectedSession?.calls.find((call) => call.id === selectedModelCall?.id)
    ?? selectedSession?.calls[selectedSession.calls.length - 1]
    ?? null;
  const selectedDetails = selected
    ? [
        ["Source", selected.source_label || formatSourceKind(selected.source_kind)],
        ["Status", selected.status],
        ["Provider", selected.provider_name || selected.provider_id],
        ["Model", selected.model_name],
        ["Input Tokens", formatInteger(selected.token_count_input)],
        ["Output Tokens", formatInteger(selected.token_count_output)],
        ["Prompt Chars", formatInteger(selected.prompt_chars)],
        ["Response Chars", formatInteger(selected.response_chars)],
        ["Messages", formatInteger(selected.request_message_count)],
        ["Duration", formatDurationMs(selected.duration_ms)],
        ["Max Tokens", formatInteger(selected.max_tokens)],
        ["Temperature", selected.temperature === null ? "n/a" : String(selected.temperature)],
        ["Created", selected.created_at],
        ["Workflow", selected.workflow_run_id ?? "n/a"],
        ["Agent Run", selected.agent_run_id ?? "n/a"],
        ["Session", selected.session_id ?? "n/a"],
        ["Product", selected.product_id ?? "n/a"],
      ]
    : [];

  const selectCallSession = (session: ModelCallSessionSummary) => {
    onSelectedModelCallSessionKeyChange(session.key);
    onSelectedModelCallChange(session.calls[session.calls.length - 1] ?? null);
  };

  return (
    <View style={styles.callsScreen}>
      <View style={styles.callsHeader}>
        <View>
          <Text style={styles.callsTitle}>Calls</Text>
          <Text style={styles.callsSubtitle}>
            {modelCalls.length ? `${callSessions.length} sessions · ${modelCalls.length} calls` : "No calls loaded"}
          </Text>
        </View>
        <Pressable
          style={[styles.callsRefreshButton, isModelCallsLoading && styles.buttonDisabled]}
          onPress={() => void onLoadModelCalls()}
          disabled={isModelCallsLoading}
        >
          <Text style={styles.callsRefreshText}>{isModelCallsLoading ? "Loading" : "Refresh"}</Text>
        </Pressable>
      </View>
      {modelCallsError ? <Text style={styles.callsError}>{modelCallsError}</Text> : null}
      <ScrollView style={styles.callsBody} contentContainerStyle={styles.callsBodyContent}>
        {callSessions.length ? (
          callSessions.map((session) => {
            const isSelected = selectedSession?.key === session.key;
            return (
              <Pressable
                key={session.key}
                style={[styles.callRow, isSelected && styles.callRowActive]}
                onPress={() => selectCallSession(session)}
              >
                <View style={styles.callRowTop}>
                  <Text style={styles.callSource} numberOfLines={1}>
                    {session.label}
                  </Text>
                  <Text style={[styles.callStatus, session.status === "failed" && styles.callStatusFailed]}>
                    {session.status}
                  </Text>
                </View>
                <Text style={styles.callMeta} numberOfLines={1}>
                  {session.callCount} calls · {session.providerLine} / {session.modelLine}
                </Text>
                <Text style={styles.callMeta} numberOfLines={1}>
                  input {formatInteger(session.tokenCountInput)} · output {formatInteger(session.tokenCountOutput)} · {formatDurationMs(session.durationMs)}
                </Text>
                <Text style={styles.callTime} numberOfLines={1}>{session.endedAt}</Text>
              </Pressable>
            );
          })
        ) : (
          <View style={styles.callsEmpty}>
            <Text style={styles.callsEmptyTitle}>No calls yet</Text>
            <Text style={styles.callsEmptyText}>Run a planner, chat, voice, or workflow request, then refresh.</Text>
          </View>
        )}
        {selectedSession && selected ? (
          <View style={styles.callDetailPanel}>
            <Text style={styles.callDetailTitle}>Session Details</Text>
            <Text style={styles.callDetailValue} numberOfLines={2}>
              {selectedSession.sessionId ?? selectedSession.sourceId ?? selectedSession.key}
            </Text>
            <View style={styles.callSessionStats}>
              <View style={styles.callSessionStat}>
                <Text style={styles.callSessionStatValue}>{formatInteger(selectedSession.callCount)}</Text>
                <Text style={styles.callSessionStatLabel}>Calls</Text>
              </View>
              <View style={styles.callSessionStat}>
                <Text style={styles.callSessionStatValue}>{formatInteger(selectedSession.tokenCountInput)}</Text>
                <Text style={styles.callSessionStatLabel}>Input</Text>
              </View>
              <View style={styles.callSessionStat}>
                <Text style={styles.callSessionStatValue}>{formatInteger(selectedSession.tokenCountOutput)}</Text>
                <Text style={styles.callSessionStatLabel}>Output</Text>
              </View>
            </View>
            <Text style={styles.callSessionCallsTitle}>Calls in session</Text>
            {selectedSession.calls.map((call) => {
              const isSelectedCall = selected.id === call.id;
              return (
                <Pressable
                  key={call.id}
                  style={[styles.callChildRow, isSelectedCall && styles.callChildRowActive]}
                  onPress={() => onSelectedModelCallChange(call)}
                >
                  <View style={styles.callRowTop}>
                    <Text style={styles.callSource}>Call #{formatInteger(call.call_index)}</Text>
                    <Text style={[styles.callStatus, call.status === "failed" && styles.callStatusFailed]}>
                      {call.status}
                    </Text>
                  </View>
                  <Text style={styles.callMeta} numberOfLines={1}>
                    input {formatInteger(call.token_count_input)} · output {formatInteger(call.token_count_output)} · {formatDurationMs(call.duration_ms)}
                  </Text>
                  <Text style={styles.callTime} numberOfLines={1}>{call.created_at}</Text>
                </Pressable>
              );
            })}
            <Text style={styles.callSessionCallsTitle}>Selected call details</Text>
            {selectedDetails.map(([label, value]) => (
              <View key={label} style={styles.callDetailRow}>
                <Text style={styles.callDetailLabel}>{label}</Text>
                <Text style={styles.callDetailValue}>{value}</Text>
              </View>
            ))}
            {selected.error_message ? (
              <View style={styles.callErrorPanel}>
                <Text style={styles.callDetailLabel}>Error</Text>
                <Text style={styles.callErrorText}>{selected.error_message}</Text>
              </View>
            ) : null}
          </View>
        ) : null}
      </ScrollView>
    </View>
  );
}
