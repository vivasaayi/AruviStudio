import { useEffect } from "react";
import type { QueryClient } from "@tanstack/react-query";
import {
  clearDatabasePathOverride,
  getActiveDatabasePath,
  getDatabaseHealth,
  getDatabasePathOverride,
  getSetting,
  routePlannerContact,
  seedExampleProducts,
  sendTwilioWhatsappMessage,
  setDatabasePathOverride,
  setSetting,
  startTwilioVoiceCall,
} from "../../../lib/tauri";
import {
  AUTO_APPROVE_PLAN_KEY,
  AUTO_APPROVE_TEST_REVIEW_KEY,
  AUTO_START_AFTER_APPROVAL_KEY,
  HIDE_EXAMPLE_PRODUCTS_KEY,
  MCP_API_TOKEN_KEY,
  MOBILE_API_TOKEN_KEY,
  MOBILE_BIND_HOST_KEY,
  MOBILE_BIND_PORT_KEY,
  PLANNER_CALL_QUIET_HOURS_END_KEY,
  PLANNER_CALL_QUIET_HOURS_START_KEY,
  PLANNER_CHANNEL_PREFERENCE_KEY,
  PLANNER_CONTACT_OPENING_MESSAGE_KEY,
  PLANNER_CONTACT_TARGET_KEY,
  PLANNER_DEFAULT_MODEL_KEY,
  PLANNER_DEFAULT_PROVIDER_KEY,
  PLANNER_ESCALATE_TO_CALL_KEY,
  SPEECH_AUTO_SPEAK_REPLIES_KEY,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
  SPEECH_REVIEW_BEFORE_SEND_KEY,
  TWILIO_ACCOUNT_SID_KEY,
  TWILIO_AUTH_TOKEN_KEY,
  TWILIO_VOICE_FROM_KEY,
  TWILIO_WEBHOOK_BASE_URL_KEY,
  TWILIO_WHATSAPP_FROM_KEY,
  parseBooleanSetting,
} from "../lib/settingsKeys";
import type { useSettingsPageState } from "./useSettingsPageState";

type SettingsPageState = ReturnType<typeof useSettingsPageState>;

export function useSettingsLoader(settings: SettingsPageState) {
  const {
    setDockerHost,
    setMaxRetries,
    setAutoStartAfterApproval,
    setAutoApprovePlan,
    setAutoApproveTestReview,
    setHideExampleProducts,
    setDbHealth,
    setDbHealthError,
    setActiveDbPath,
    setDbPathOverrideInput,
    setDbPathOverrideError,
    setPlannerDefaultProviderId,
    setPlannerDefaultModelName,
    setPlannerChannelPreference,
    setPlannerEscalateToCall,
    setPlannerCallQuietHoursStart,
    setPlannerCallQuietHoursEnd,
    setSpeechProviderId,
    setSpeechModelName,
    setSpeechLocale,
    setSpeechNativeVoice,
    setSpeechEnableMic,
    setSpeechAutoSpeakReplies,
    setSpeechReviewBeforeSend,
    setMcpApiToken,
    setMobileApiToken,
    setMobileBindHost,
    setMobileBindPort,
    setTwilioAccountSid,
    setTwilioAuthToken,
    setTwilioWhatsappFrom,
    setTwilioVoiceFrom,
    setTwilioWebhookBaseUrl,
    setPlannerContactTarget,
    setPlannerContactOpeningMessage,
  } = settings;

  useEffect(() => {
    getSetting("docker_host").then((value) => {
      if (value) setDockerHost(value);
    });
    getSetting("max_workflow_retries").then((value) => {
      if (value) setMaxRetries(value);
    });
    getSetting(AUTO_START_AFTER_APPROVAL_KEY).then((value) =>
      setAutoStartAfterApproval(parseBooleanSetting(value, true)),
    );
    getSetting(AUTO_APPROVE_PLAN_KEY).then((value) =>
      setAutoApprovePlan(parseBooleanSetting(value, true)),
    );
    getSetting(AUTO_APPROVE_TEST_REVIEW_KEY).then((value) =>
      setAutoApproveTestReview(parseBooleanSetting(value, true)),
    );
    getSetting(HIDE_EXAMPLE_PRODUCTS_KEY).then((value) =>
      setHideExampleProducts(parseBooleanSetting(value, true)),
    );
    getSetting(PLANNER_DEFAULT_PROVIDER_KEY).then((value) => {
      if (value) setPlannerDefaultProviderId(value);
    });
    getSetting(PLANNER_DEFAULT_MODEL_KEY).then((value) => {
      if (value) setPlannerDefaultModelName(value);
    });
    getSetting(PLANNER_CHANNEL_PREFERENCE_KEY).then((value) => {
      if (value) setPlannerChannelPreference(value);
    });
    getSetting(PLANNER_ESCALATE_TO_CALL_KEY).then((value) =>
      setPlannerEscalateToCall(parseBooleanSetting(value, true)),
    );
    getSetting(PLANNER_CALL_QUIET_HOURS_START_KEY).then((value) => {
      if (value) setPlannerCallQuietHoursStart(value);
    });
    getSetting(PLANNER_CALL_QUIET_HOURS_END_KEY).then((value) => {
      if (value) setPlannerCallQuietHoursEnd(value);
    });
    getSetting(SPEECH_PROVIDER_KEY).then((value) => {
      if (value) setSpeechProviderId(value);
    });
    getSetting(SPEECH_MODEL_KEY).then((value) => {
      if (value) setSpeechModelName(value);
    });
    getSetting(SPEECH_LOCALE_KEY).then((value) => {
      if (value) setSpeechLocale(value);
    });
    getSetting(SPEECH_NATIVE_VOICE_KEY).then((value) => {
      if (value) setSpeechNativeVoice(value);
    });
    getSetting(SPEECH_ENABLE_MIC_KEY).then((value) =>
      setSpeechEnableMic(parseBooleanSetting(value, true)),
    );
    getSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY).then((value) =>
      setSpeechAutoSpeakReplies(parseBooleanSetting(value, false)),
    );
    getSetting(SPEECH_REVIEW_BEFORE_SEND_KEY).then((value) =>
      setSpeechReviewBeforeSend(parseBooleanSetting(value, false)),
    );
    getSetting(MCP_API_TOKEN_KEY).then((value) => {
      if (value) setMcpApiToken(value);
    });
    getSetting(MOBILE_API_TOKEN_KEY).then((value) => {
      if (value) setMobileApiToken(value);
    });
    getSetting(MOBILE_BIND_HOST_KEY).then((value) => {
      if (value) setMobileBindHost(value);
    });
    getSetting(MOBILE_BIND_PORT_KEY).then((value) => {
      if (value) setMobileBindPort(value);
    });
    getSetting(TWILIO_ACCOUNT_SID_KEY).then((value) => {
      if (value) setTwilioAccountSid(value);
    });
    getSetting(TWILIO_AUTH_TOKEN_KEY).then((value) => {
      if (value) setTwilioAuthToken(value);
    });
    getSetting(TWILIO_WHATSAPP_FROM_KEY).then((value) => {
      if (value) setTwilioWhatsappFrom(value);
    });
    getSetting(TWILIO_VOICE_FROM_KEY).then((value) => {
      if (value) setTwilioVoiceFrom(value);
    });
    getSetting(TWILIO_WEBHOOK_BASE_URL_KEY).then((value) => {
      if (value) setTwilioWebhookBaseUrl(value);
    });
    getSetting(PLANNER_CONTACT_TARGET_KEY).then((value) => {
      if (value) setPlannerContactTarget(value);
    });
    getSetting(PLANNER_CONTACT_OPENING_MESSAGE_KEY).then((value) => {
      if (value) setPlannerContactOpeningMessage(value);
    });
    getActiveDatabasePath()
      .then(setActiveDbPath)
      .catch((error) => setDbPathOverrideError(String(error)));
    getDatabasePathOverride().then((value) => {
      if (value) setDbPathOverrideInput(value);
    });
    getDatabaseHealth()
      .then((health) => {
        setDbHealth(health);
        setDbHealthError(null);
      })
      .catch((error) => {
        setDbHealthError(String(error));
      });
  }, [
    setActiveDbPath,
    setAutoApprovePlan,
    setAutoApproveTestReview,
    setAutoStartAfterApproval,
    setDbHealth,
    setDbHealthError,
    setDbPathOverrideError,
    setDbPathOverrideInput,
    setDockerHost,
    setHideExampleProducts,
    setMaxRetries,
    setMcpApiToken,
    setMobileApiToken,
    setMobileBindHost,
    setMobileBindPort,
    setPlannerCallQuietHoursEnd,
    setPlannerCallQuietHoursStart,
    setPlannerChannelPreference,
    setPlannerContactOpeningMessage,
    setPlannerContactTarget,
    setPlannerDefaultModelName,
    setPlannerDefaultProviderId,
    setPlannerEscalateToCall,
    setSpeechAutoSpeakReplies,
    setSpeechEnableMic,
    setSpeechLocale,
    setSpeechModelName,
    setSpeechNativeVoice,
    setSpeechProviderId,
    setSpeechReviewBeforeSend,
    setTwilioAccountSid,
    setTwilioAuthToken,
    setTwilioVoiceFrom,
    setTwilioWebhookBaseUrl,
    setTwilioWhatsappFrom,
  ]);
}

export function useSettingsPageActions({
  queryClient,
  settings,
}: {
  queryClient: QueryClient;
  settings: SettingsPageState;
}) {
  const {
    dbPathOverrideInput,
    plannerContactTarget,
    plannerContactOpeningMessage,
    setCatalogActionError,
    setCatalogActionMsg,
    setDbPathOverrideError,
    setDbPathOverrideInput,
    setDbPathOverrideSaved,
    setPlannerContactError,
    setPlannerContactMsg,
    setSavedMsg,
  } = settings;

  const invalidateProductCatalogQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["products"] }),
      queryClient.invalidateQueries({ queryKey: ["productTree"] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
      queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
    ]);
  };

  const saveSetting = async (key: string, value: string) => {
    await setSetting(key, value);
    await queryClient.invalidateQueries({ queryKey: ["setting"] });
    await queryClient.invalidateQueries({ queryKey: ["mcpBridgeStatus"] });
    await queryClient.invalidateQueries({ queryKey: ["mobileBridgeStatus"] });
    if (key === HIDE_EXAMPLE_PRODUCTS_KEY) {
      await invalidateProductCatalogQueries();
    }
    setSavedMsg(key);
    setTimeout(() => setSavedMsg(null), 2000);
  };

  const seedExampleCatalog = async () => {
    try {
      setCatalogActionError(null);
      await seedExampleProducts();
      await Promise.all([
        invalidateProductCatalogQueries(),
        queryClient.invalidateQueries({ queryKey: ["workItems"] }),
      ]);
      setCatalogActionMsg("Example catalog is present and up to date.");
      setTimeout(() => setCatalogActionMsg(null), 2500);
    } catch (error) {
      setCatalogActionError(String(error));
    }
  };

  const saveDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await setDatabasePathOverride(dbPathOverrideInput);
      setDbPathOverrideSaved("saved");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  const clearDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await clearDatabasePathOverride();
      setDbPathOverrideInput("");
      setDbPathOverrideSaved("cleared");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  const copyText = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setSavedMsg(`copied:${value}`);
      setTimeout(() => setSavedMsg(null), 2000);
    } catch {
      setSavedMsg(null);
    }
  };

  const autoRoutePlannerContact = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      const result = await routePlannerContact({
        to: plannerContactTarget.trim(),
        content: plannerContactOpeningMessage.trim(),
      });
      const channelLabel = result.channel === "voice" ? "voice call" : "WhatsApp";
      if (result.status === "blocked") {
        setPlannerContactError(`Auto-routing blocked: ${result.reason}`);
        return;
      }
      setPlannerContactMsg(`Auto-routed to ${channelLabel}. ${result.reason}`);
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  const sendPlannerWhatsapp = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      await sendTwilioWhatsappMessage({
        to: plannerContactTarget.trim(),
        content: plannerContactOpeningMessage.trim(),
      });
      setPlannerContactMsg("WhatsApp message queued through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  const startPlannerVoiceCall = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      await startTwilioVoiceCall({
        to: plannerContactTarget.trim(),
        initialPrompt: plannerContactOpeningMessage.trim() || undefined,
      });
      setPlannerContactMsg("Voice call requested through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  return {
    autoRoutePlannerContact,
    clearDbOverride,
    copyText,
    saveDbOverride,
    saveSetting,
    seedExampleCatalog,
    sendPlannerWhatsapp,
    startPlannerVoiceCall,
  };
}
