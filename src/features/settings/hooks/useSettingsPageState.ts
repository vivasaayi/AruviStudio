import { useState } from "react";
import type { DatabaseHealth } from "../../../lib/types";

export function useSettingsPageState() {
  const [dockerHost, setDockerHost] = useState("");
  const [maxRetries, setMaxRetries] = useState("3");
  const [autoStartAfterApproval, setAutoStartAfterApproval] = useState(true);
  const [autoApprovePlan, setAutoApprovePlan] = useState(true);
  const [autoApproveTestReview, setAutoApproveTestReview] = useState(true);
  const [hideExampleProducts, setHideExampleProducts] = useState(true);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);
  const [dbHealth, setDbHealth] = useState<DatabaseHealth | null>(null);
  const [dbHealthError, setDbHealthError] = useState<string | null>(null);
  const [activeDbPath, setActiveDbPath] = useState("");
  const [dbPathOverrideInput, setDbPathOverrideInput] = useState("");
  const [dbPathOverrideSaved, setDbPathOverrideSaved] = useState<string | null>(null);
  const [dbPathOverrideError, setDbPathOverrideError] = useState<string | null>(null);
  const [catalogActionMsg, setCatalogActionMsg] = useState<string | null>(null);
  const [catalogActionError, setCatalogActionError] = useState<string | null>(null);
  const [plannerDefaultProviderId, setPlannerDefaultProviderId] = useState("");
  const [plannerDefaultModelName, setPlannerDefaultModelName] = useState("");
  const [plannerChannelPreference, setPlannerChannelPreference] = useState("hybrid");
  const [plannerEscalateToCall, setPlannerEscalateToCall] = useState(true);
  const [plannerCallQuietHoursStart, setPlannerCallQuietHoursStart] = useState("21:00");
  const [plannerCallQuietHoursEnd, setPlannerCallQuietHoursEnd] = useState("08:00");
  const [speechProviderId, setSpeechProviderId] = useState("");
  const [speechModelName, setSpeechModelName] = useState("");
  const [speechLocale, setSpeechLocale] = useState("en-US");
  const [speechNativeVoice, setSpeechNativeVoice] = useState("");
  const [speechEnableMic, setSpeechEnableMic] = useState(true);
  const [speechAutoSpeakReplies, setSpeechAutoSpeakReplies] = useState(false);
  const [speechReviewBeforeSend, setSpeechReviewBeforeSend] = useState(false);
  const [mcpApiToken, setMcpApiToken] = useState("");
  const [mobileApiToken, setMobileApiToken] = useState("");
  const [mobileBindHost, setMobileBindHost] = useState("127.0.0.1");
  const [mobileBindPort, setMobileBindPort] = useState("8787");
  const [twilioAccountSid, setTwilioAccountSid] = useState("");
  const [twilioAuthToken, setTwilioAuthToken] = useState("");
  const [twilioWhatsappFrom, setTwilioWhatsappFrom] = useState("");
  const [twilioVoiceFrom, setTwilioVoiceFrom] = useState("");
  const [twilioWebhookBaseUrl, setTwilioWebhookBaseUrl] = useState("");
  const [plannerContactTarget, setPlannerContactTarget] = useState("");
  const [plannerContactOpeningMessage, setPlannerContactOpeningMessage] = useState(
    "Call me and ask what work should be prioritized next.",
  );
  const [plannerContactMsg, setPlannerContactMsg] = useState<string | null>(null);
  const [plannerContactError, setPlannerContactError] = useState<string | null>(null);

  return {
    dockerHost, setDockerHost, maxRetries, setMaxRetries,
    autoStartAfterApproval, setAutoStartAfterApproval,
    autoApprovePlan, setAutoApprovePlan, autoApproveTestReview, setAutoApproveTestReview,
    hideExampleProducts, setHideExampleProducts, savedMsg, setSavedMsg,
    dbHealth, setDbHealth, dbHealthError, setDbHealthError, activeDbPath, setActiveDbPath,
    dbPathOverrideInput, setDbPathOverrideInput, dbPathOverrideSaved, setDbPathOverrideSaved,
    dbPathOverrideError, setDbPathOverrideError, catalogActionMsg, setCatalogActionMsg,
    catalogActionError, setCatalogActionError, plannerDefaultProviderId,
    setPlannerDefaultProviderId, plannerDefaultModelName, setPlannerDefaultModelName,
    plannerChannelPreference, setPlannerChannelPreference, plannerEscalateToCall,
    setPlannerEscalateToCall, plannerCallQuietHoursStart, setPlannerCallQuietHoursStart,
    plannerCallQuietHoursEnd, setPlannerCallQuietHoursEnd, speechProviderId,
    setSpeechProviderId, speechModelName, setSpeechModelName, speechLocale, setSpeechLocale,
    speechNativeVoice, setSpeechNativeVoice, speechEnableMic, setSpeechEnableMic,
    speechAutoSpeakReplies, setSpeechAutoSpeakReplies, speechReviewBeforeSend,
    setSpeechReviewBeforeSend, mcpApiToken, setMcpApiToken, mobileApiToken, setMobileApiToken,
    mobileBindHost, setMobileBindHost, mobileBindPort, setMobileBindPort,
    twilioAccountSid, setTwilioAccountSid, twilioAuthToken, setTwilioAuthToken,
    twilioWhatsappFrom, setTwilioWhatsappFrom, twilioVoiceFrom, setTwilioVoiceFrom,
    twilioWebhookBaseUrl, setTwilioWebhookBaseUrl, plannerContactTarget,
    setPlannerContactTarget, plannerContactOpeningMessage, setPlannerContactOpeningMessage,
    plannerContactMsg, setPlannerContactMsg, plannerContactError, setPlannerContactError,
  };
}
