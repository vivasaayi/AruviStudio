import React, { useEffect, useMemo, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Linking,
  Pressable,
  SafeAreaView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import * as SecureStore from "expo-secure-store";
import { WebView } from "react-native-webview";

const STORAGE_KEYS = {
  baseUrl: "aruvi.mobile.base_url",
  token: "aruvi.mobile.token",
  providerId: "aruvi.mobile.provider_id",
  modelName: "aruvi.mobile.model_name",
  locale: "aruvi.mobile.locale",
};

type ConnectionValues = {
  baseUrl?: string;
  token?: string;
  providerId?: string;
  modelName?: string;
  locale?: string;
};

function parseConnectionUrl(url: string): ConnectionValues | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "aruvi-planner-mobile:") {
      return null;
    }
    const params = parsed.searchParams;
    const values: ConnectionValues = {
      baseUrl: params.get("baseUrl") || params.get("base_url") || undefined,
      token: params.get("token") || undefined,
      providerId: params.get("providerId") || params.get("provider_id") || undefined,
      modelName: params.get("modelName") || params.get("model_name") || undefined,
      locale: params.get("locale") || undefined,
    };
    return Object.values(values).some(Boolean) ? values : null;
  } catch {
    return null;
  }
}

export default function App() {
  const [baseUrl, setBaseUrl] = useState("http://100.66.32.111:8787");
  const [token, setToken] = useState("");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [locale, setLocale] = useState("en-US");
  const [webReloadKey, setWebReloadKey] = useState(0);
  const [isSaving, setIsSaving] = useState(false);

  const remoteUrl = useMemo(() => {
    const trimmed = baseUrl.trim().replace(/\/+$/, "");
    return trimmed ? `${trimmed}/remote` : "about:blank";
  }, [baseUrl]);

  const remoteBootstrapScript = useMemo(() => {
    const payload = {
      token: token.trim(),
      provider: providerId.trim(),
      model: modelName.trim(),
      locale: locale.trim(),
    };
    return `
      (function () {
        try {
          var config = ${JSON.stringify(payload)};
          if (config.token) window.localStorage.setItem("aruvi.remote.token", config.token);
          else window.localStorage.removeItem("aruvi.remote.token");
          if (config.provider) window.localStorage.setItem("aruvi.remote.provider", config.provider);
          else window.localStorage.removeItem("aruvi.remote.provider");
          if (config.model) window.localStorage.setItem("aruvi.remote.model", config.model);
          else window.localStorage.removeItem("aruvi.remote.model");
          if (config.locale) window.localStorage.setItem("aruvi.remote.locale", config.locale);
          else window.localStorage.removeItem("aruvi.remote.locale");
        } catch (error) {}
      })();
      true;
    `;
  }, [locale, modelName, providerId, token]);

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
      const [savedBaseUrl, savedToken, savedProviderId, savedModelName, savedLocale] = await Promise.all([
        SecureStore.getItemAsync(STORAGE_KEYS.baseUrl),
        SecureStore.getItemAsync(STORAGE_KEYS.token),
        SecureStore.getItemAsync(STORAGE_KEYS.providerId),
        SecureStore.getItemAsync(STORAGE_KEYS.modelName),
        SecureStore.getItemAsync(STORAGE_KEYS.locale),
      ]);
      if (disposed) return;
      if (savedBaseUrl) setBaseUrl(savedBaseUrl);
      if (savedToken) setToken(savedToken);
      if (savedProviderId) setProviderId(savedProviderId);
      if (savedModelName) setModelName(savedModelName);
      if (savedLocale) setLocale(savedLocale);

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
    } catch (error) {
      Alert.alert("Save failed", error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <SafeAreaView style={styles.safeArea}>
      <View style={styles.shell}>
        <View style={styles.header}>
          <View style={styles.titleRow}>
            <View style={styles.titleBlock}>
              <Text style={styles.title}>Aruvi Remote</Text>
              <Text style={styles.url} numberOfLines={1}>{remoteUrl}</Text>
            </View>
            <Pressable style={styles.button} onPress={() => setWebReloadKey((current) => current + 1)}>
              <Text style={styles.buttonText}>Reload</Text>
            </Pressable>
          </View>

          <TextInput
            style={styles.input}
            value={baseUrl}
            onChangeText={setBaseUrl}
            placeholder="http://mac-tailnet-ip:8787"
            placeholderTextColor="#7d8898"
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
          />
          <TextInput
            style={styles.input}
            value={token}
            onChangeText={setToken}
            placeholder="mobile.api_token"
            placeholderTextColor="#7d8898"
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
          />

          <View style={styles.optionalGrid}>
            <TextInput
              style={styles.input}
              value={providerId}
              onChangeText={setProviderId}
              placeholder="provider id"
              placeholderTextColor="#7d8898"
              autoCapitalize="none"
              autoCorrect={false}
            />
            <TextInput
              style={styles.input}
              value={modelName}
              onChangeText={setModelName}
              placeholder="model"
              placeholderTextColor="#7d8898"
              autoCapitalize="none"
              autoCorrect={false}
            />
          </View>

          <View style={styles.actionRow}>
            <Pressable style={styles.primaryButton} onPress={() => void saveConnection()} disabled={isSaving}>
              <Text style={styles.primaryButtonText}>{isSaving ? "Saving..." : "Save + Load"}</Text>
            </Pressable>
          </View>
        </View>

        <WebView
          key={`${remoteUrl}-${webReloadKey}`}
          source={{ uri: remoteUrl }}
          style={styles.webView}
          injectedJavaScriptBeforeContentLoaded={remoteBootstrapScript}
          javaScriptEnabled
          domStorageEnabled
          allowsInlineMediaPlayback
          startInLoadingState
          renderLoading={() => (
            <View style={styles.loading}>
              <ActivityIndicator color="#7bc8ff" />
            </View>
          )}
          renderError={(_, __, description) => (
            <View style={styles.errorPanel}>
              <Text style={styles.errorTitle}>Remote UI unavailable</Text>
              <Text style={styles.errorText}>{description}</Text>
            </View>
          )}
        />
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safeArea: {
    flex: 1,
    backgroundColor: "#111317",
  },
  shell: {
    flex: 1,
    backgroundColor: "#111317",
  },
  header: {
    borderBottomWidth: 1,
    borderBottomColor: "#2f3642",
    paddingHorizontal: 14,
    paddingTop: 10,
    paddingBottom: 12,
    gap: 8,
    backgroundColor: "#151922",
  },
  titleRow: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center",
    gap: 12,
  },
  titleBlock: {
    flex: 1,
    minWidth: 0,
  },
  title: {
    color: "#f4f8ff",
    fontSize: 24,
    fontWeight: "800",
  },
  url: {
    color: "#9aa8bd",
    fontSize: 12,
    marginTop: 2,
  },
  input: {
    backgroundColor: "#12161c",
    borderColor: "#364152",
    borderWidth: 1,
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 10,
    color: "#f4f8ff",
  },
  optionalGrid: {
    flexDirection: "row",
    gap: 8,
  },
  actionRow: {
    flexDirection: "row",
    gap: 8,
  },
  button: {
    backgroundColor: "#223040",
    borderRadius: 8,
    paddingVertical: 10,
    paddingHorizontal: 14,
  },
  buttonText: {
    color: "#edf3ff",
    fontWeight: "700",
  },
  primaryButton: {
    flex: 1,
    backgroundColor: "#0e639c",
    borderRadius: 8,
    paddingVertical: 12,
    alignItems: "center",
  },
  primaryButtonText: {
    color: "#ffffff",
    fontWeight: "700",
    fontSize: 15,
  },
  webView: {
    flex: 1,
    backgroundColor: "#101317",
  },
  loading: {
    position: "absolute",
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#101317",
  },
  errorPanel: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    padding: 18,
    gap: 10,
    backgroundColor: "#111317",
  },
  errorTitle: {
    color: "#f4f8ff",
    fontSize: 16,
    fontWeight: "700",
  },
  errorText: {
    color: "#9aa8bd",
    fontSize: 13,
    lineHeight: 18,
    textAlign: "center",
  },
});
