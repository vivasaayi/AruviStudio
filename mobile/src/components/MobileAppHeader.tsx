import React from "react";
import { Pressable, Switch, Text, TextInput, View } from "react-native";
import { styles } from "../styles/appStyles";

type ConnectionStatus = "unchecked" | "checking" | "connected" | "offline";

type MobileAppHeaderProps = {
  remoteUrl: string;
  baseUrl: string;
  token: string;
  providerId: string;
  modelName: string;
  readReplies: boolean;
  connectionText: string;
  connectionStatus: ConnectionStatus;
  shouldShowSetup: boolean;
  isSaving: boolean;
  onToggleSetup: () => void;
  onRefresh: () => void;
  onBaseUrlChange: (nextValue: string) => void;
  onTokenChange: (nextValue: string) => void;
  onProviderIdChange: (nextValue: string) => void;
  onModelNameChange: (nextValue: string) => void;
  onReadRepliesChange: (nextValue: boolean) => Promise<void>;
  onSaveConnection: () => Promise<void>;
};

export function MobileAppHeader({
  remoteUrl,
  baseUrl,
  token,
  providerId,
  modelName,
  readReplies,
  connectionText,
  connectionStatus,
  shouldShowSetup,
  isSaving,
  onToggleSetup,
  onRefresh,
  onBaseUrlChange,
  onTokenChange,
  onProviderIdChange,
  onModelNameChange,
  onReadRepliesChange,
  onSaveConnection,
}: MobileAppHeaderProps) {
  return (
    <View style={styles.header}>
      <View style={styles.titleRow}>
        <View style={styles.titleBlock}>
          <Text style={styles.title}>Aruvi Studio</Text>
          <View style={styles.connectionRow}>
            <View
              style={[
                styles.connectionDot,
                connectionStatus === "connected" ? styles.connectionDotReady : styles.connectionDotMissing,
              ]}
            />
            <Text style={styles.connectionText} numberOfLines={1}>
              {connectionText}
            </Text>
          </View>
        </View>
        <Pressable style={styles.headerButton} onPress={onToggleSetup}>
          <Text style={styles.buttonText}>Settings</Text>
        </Pressable>
        <Pressable style={styles.headerButton} onPress={onRefresh}>
          <Text style={styles.buttonText}>Refresh</Text>
        </Pressable>
      </View>

      {shouldShowSetup ? (
        <View style={styles.setupPanel}>
          <Text style={styles.setupCaption} numberOfLines={1}>{remoteUrl}</Text>
          <TextInput
            style={styles.input}
            value={baseUrl}
            onChangeText={onBaseUrlChange}
            placeholder="http://mac-tailnet-ip:8787"
            placeholderTextColor="#7d8898"
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
          />
          <TextInput
            style={styles.input}
            value={token}
            onChangeText={onTokenChange}
            placeholder="mobile.api_token"
            placeholderTextColor="#7d8898"
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
          />
          <View style={styles.optionalGrid}>
            <TextInput
              style={[styles.input, styles.flexInput]}
              value={providerId}
              onChangeText={onProviderIdChange}
              placeholder="provider id"
              placeholderTextColor="#7d8898"
              autoCapitalize="none"
              autoCorrect={false}
            />
            <TextInput
              style={[styles.input, styles.flexInput]}
              value={modelName}
              onChangeText={onModelNameChange}
              placeholder="model"
              placeholderTextColor="#7d8898"
              autoCapitalize="none"
              autoCorrect={false}
            />
          </View>
          <View style={styles.settingsRow}>
            <View style={styles.settingsCopy}>
              <Text style={styles.settingsLabel}>Read replies</Text>
              <Text style={styles.settingsDescription}>Speak assistant replies after each voice message.</Text>
            </View>
            <Switch
              value={readReplies}
              onValueChange={(nextValue) => void onReadRepliesChange(nextValue)}
              trackColor={{ false: "#2a3442", true: "#1d6f9d" }}
              thumbColor={readReplies ? "#f4f8ff" : "#8b98aa"}
              ios_backgroundColor="#2a3442"
            />
          </View>
          <View style={styles.actionRow}>
            <Pressable style={styles.primaryButton} onPress={() => void onSaveConnection()} disabled={isSaving}>
              <Text style={styles.primaryButtonText}>{isSaving ? "Saving..." : "Save + Load"}</Text>
            </Pressable>
          </View>
        </View>
      ) : null}
    </View>
  );
}
