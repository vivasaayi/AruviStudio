import { useEffect, useMemo, useState } from "react";
import { Alert, Linking } from "react-native";
import * as SecureStore from "expo-secure-store";
import { PlannerMobileClient } from "../api/client";
import {
  getLoopbackFallbackBaseUrl,
  isNetworkRequestFailure,
  normalizeBaseUrlForDisplay,
  parseConnectionUrl,
  type ConnectionValues,
} from "../lib/mobileConnection";
import { describeError } from "../lib/mobileFormatters";
import { MOBILE_STORAGE_KEYS } from "../lib/mobileStorageKeys";

export type ConnectionStatus = "unchecked" | "checking" | "connected" | "offline";

export function useMobileConnectionController() {
  const [baseUrl, setBaseUrl] = useState("http://100.66.32.111:8787");
  const [token, setToken] = useState("");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [locale, setLocale] = useState("en-US");
  const [webReloadKey, setWebReloadKey] = useState(0);
  const [connectionCheckKey, setConnectionCheckKey] = useState(0);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>("unchecked");
  const [isSaving, setIsSaving] = useState(false);

  const remoteUrl = useMemo(() => {
    const trimmed = normalizeBaseUrlForDisplay(baseUrl);
    return trimmed ? `${trimmed}/remote` : "about:blank";
  }, [baseUrl]);

  const mobileClient = useMemo(() => {
    return new PlannerMobileClient(baseUrl.trim(), token.trim());
  }, [baseUrl, token]);

  const reloadRemote = () => {
    setWebReloadKey((current) => current + 1);
  };

  const refreshConnection = () => {
    setConnectionCheckKey((current) => current + 1);
  };

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
      next.baseUrl ? SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.baseUrl, next.baseUrl) : Promise.resolve(),
      next.token ? SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.token, next.token) : Promise.resolve(),
      next.providerId !== undefined
        ? SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.providerId, next.providerId)
        : Promise.resolve(),
      next.modelName !== undefined
        ? SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.modelName, next.modelName)
        : Promise.resolve(),
      next.locale ? SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.locale, next.locale) : Promise.resolve(),
    ]);
    reloadRemote();
  };

  const saveConnection = async () => {
    try {
      setIsSaving(true);
      await Promise.all([
        SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.baseUrl, baseUrl.trim()),
        SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.token, token.trim()),
        SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.providerId, providerId.trim()),
        SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.modelName, modelName.trim()),
        SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.locale, locale.trim()),
      ]);
      reloadRemote();
      return true;
    } catch (error) {
      Alert.alert("Save failed", describeError(error));
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const applyFallbackBaseUrl = async (fallbackBaseUrl: string) => {
    setBaseUrl(fallbackBaseUrl);
    await SecureStore.setItemAsync(MOBILE_STORAGE_KEYS.baseUrl, fallbackBaseUrl);
    setConnectionStatus("connected");
    reloadRemote();
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
      ] = await Promise.all([
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.baseUrl),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.token),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.providerId),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.modelName),
        SecureStore.getItemAsync(MOBILE_STORAGE_KEYS.locale),
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
              await applyFallbackBaseUrl(fallbackBaseUrl);
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

  return {
    baseUrl,
    setBaseUrl,
    token,
    setToken,
    providerId,
    setProviderId,
    modelName,
    setModelName,
    locale,
    setLocale,
    webReloadKey,
    connectionStatus,
    setConnectionStatus,
    isSaving,
    remoteUrl,
    mobileClient,
    reloadRemote,
    refreshConnection,
    saveConnection,
    applyFallbackBaseUrl,
  };
}
