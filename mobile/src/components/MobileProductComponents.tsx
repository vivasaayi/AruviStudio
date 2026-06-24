import React from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import type { HierarchyTreeNode, MobilePlannerToolTraceEntry, ProductExploreTab } from "../types";
import { formatPlannerToolTrace } from "../lib/mobileFormatters";
import { formatNodeKind, getNodeSummary } from "../lib/productTree";
import { styles } from "../styles/appStyles";

type ProductModeButtonProps = {
  mode: ProductExploreTab;
  label: string;
  activeMode: ProductExploreTab;
  onPress: (mode: ProductExploreTab) => Promise<void>;
};

export function ProductModeButton({ mode, label, activeMode, onPress }: ProductModeButtonProps) {
  return (
    <Pressable
      key={mode}
      style={[styles.productModeButton, activeMode === mode && styles.productModeButtonActive]}
      onPress={() => void onPress(mode)}
    >
      <Text style={[styles.productModeText, activeMode === mode && styles.productModeTextActive]}>{label}</Text>
    </Pressable>
  );
}

type ProductNodeRowProps = {
  node: HierarchyTreeNode;
  pathLabel?: string;
  onOpenNode: (nodeId: string) => void;
};

export function ProductNodeRow({ node, pathLabel, onOpenNode }: ProductNodeRowProps) {
  const childCount = node.children?.length ?? 0;
  return (
    <Pressable
      key={node.id}
      style={styles.productNodeRow}
      onPress={() => onOpenNode(node.id)}
    >
      <View style={styles.productNodeMain}>
        <View style={styles.productNodeTitleRow}>
          <Text style={styles.productNodeTitle} numberOfLines={2}>{node.name}</Text>
          <Text style={styles.productNodeChevron}>{childCount ? "Open" : "View"}</Text>
        </View>
        {pathLabel ? <Text style={styles.productNodePath} numberOfLines={1}>{pathLabel}</Text> : null}
        <Text style={styles.productNodeSummary} numberOfLines={3}>{getNodeSummary(node)}</Text>
        <View style={styles.productNodeMetaRow}>
          <Text style={styles.productKindBadge}>{formatNodeKind(node.node_kind)}</Text>
          <Text style={styles.productNodeMeta}>{formatNodeKind(node.node_type)}</Text>
          <Text style={styles.productNodeMeta}>
            {childCount === 1 ? "1 child" : `${childCount} children`}
          </Text>
        </View>
      </View>
    </Pressable>
  );
}

type ProductPlannerPanelProps = {
  isRecording: boolean;
  isDisabled: boolean;
  status: string;
  reply: string;
  draft: string;
  trace: MobilePlannerToolTraceEntry[];
  onSpeakReply: (reply: string) => void;
  onDraftChange: (nextDraft: string) => void;
  onToggleRecording: () => Promise<void>;
  onSubmitPrompt: (prompt: string) => Promise<void>;
};

export function ProductPlannerPanel({
  isRecording,
  isDisabled,
  status,
  reply,
  draft,
  trace,
  onSpeakReply,
  onDraftChange,
  onToggleRecording,
  onSubmitPrompt,
}: ProductPlannerPanelProps) {
  return (
    <View style={styles.productPlannerPanel}>
      <View style={styles.productPlannerHeader}>
        <View style={styles.productPlannerCopy}>
          <Text style={styles.productPlannerTitle}>Planner</Text>
          <Text style={styles.productPlannerStatus} numberOfLines={1}>{status}</Text>
        </View>
        {reply.trim() ? (
          <Pressable style={styles.productPlannerIconButton} onPress={() => onSpeakReply(reply)}>
            <Text style={styles.productPlannerIconText}>Speak</Text>
          </Pressable>
        ) : null}
      </View>
      <TextInput
        style={styles.productPlannerInput}
        value={draft}
        onChangeText={onDraftChange}
        placeholder={reply.trim() ? "Ask a follow-up or revise what it just did" : "Say what to add, revise, split, or plan here"}
        placeholderTextColor="#7f8a9c"
        multiline
        textAlignVertical="top"
      />
      <View style={styles.productPlannerActions}>
        <Pressable
          style={[
            styles.productPlannerAction,
            isRecording && styles.productPlannerActionRecording,
            isDisabled && styles.buttonDisabled,
          ]}
          onPress={() => void onToggleRecording()}
          disabled={isDisabled}
        >
          <Text style={styles.productPlannerActionText}>{isRecording ? "Stop" : "Mic"}</Text>
        </Pressable>
        <Pressable
          style={[
            styles.productPlannerAction,
            styles.productPlannerActionPrimary,
            (!draft.trim() || isDisabled || isRecording) && styles.buttonDisabled,
          ]}
          onPress={() => void onSubmitPrompt(draft)}
          disabled={!draft.trim() || isDisabled || isRecording}
        >
          <Text style={styles.productPlannerPrimaryText}>Send</Text>
        </Pressable>
      </View>
      {reply.trim() ? (
        <Text style={styles.productPlannerReply} numberOfLines={7}>{reply}</Text>
      ) : null}
      {trace.length ? (
        <View style={styles.productPlannerTraceList}>
          {trace.slice(-3).map((entry) => (
            <Text key={`${entry.step}-${entry.tool_name}`} style={styles.productPlannerTraceItem} numberOfLines={1}>
              {formatPlannerToolTrace(entry)}
            </Text>
          ))}
        </View>
      ) : null}
    </View>
  );
}
