import React from "react";
import { Linking, Pressable, ScrollView, Text, View } from "react-native";
import { formatBytes } from "../lib/mobileFormatters";
import {
  WHISPER_MODELS,
  type InstalledWhisperModel,
  type WhisperModelOption,
} from "../lib/mobileVoice";
import { styles } from "../styles/appStyles";

type MobileModelManagerProps = {
  speechModelDescription: string;
  modelInstallStatus: string;
  modelInstallProgress: number | null;
  installedWhisperModels: Record<string, InstalledWhisperModel>;
  selectedWhisperModel: WhisperModelOption;
  modelInstallBusyId: string | null;
  onSelectWhisperModel: (modelId: string) => Promise<void>;
  onInstallWhisperModel: (model: WhisperModelOption) => Promise<void>;
  onRemoveWhisperModel: (model: WhisperModelOption) => Promise<void>;
};

export function MobileModelManager({
  speechModelDescription,
  modelInstallStatus,
  modelInstallProgress,
  installedWhisperModels,
  selectedWhisperModel,
  modelInstallBusyId,
  onSelectWhisperModel,
  onInstallWhisperModel,
  onRemoveWhisperModel,
}: MobileModelManagerProps) {
  return (
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
              onPress={() => void onSelectWhisperModel(model.id)}
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
                  onPress={() => void onSelectWhisperModel(model.id)}
                >
                  <Text style={[styles.smallButtonText, isSelected && styles.smallButtonTextActive]}>
                    {isSelected ? "Selected" : "Select"}
                  </Text>
                </Pressable>
                {installedModel ? (
                  <Pressable style={styles.smallButton} onPress={() => void onRemoveWhisperModel(model)}>
                    <Text style={styles.smallButtonText}>Remove</Text>
                  </Pressable>
                ) : (
                  <Pressable
                    style={[styles.smallButton, styles.smallButtonPrimary, isBusy && styles.buttonDisabled]}
                    onPress={() => void onInstallWhisperModel(model)}
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
}
