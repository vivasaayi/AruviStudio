import React, { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  clearDatabasePathOverride,
  getActiveDatabasePath,
  getDatabaseHealth,
  getDatabasePathOverride,
  getMcpBridgeStatus,
  getMobileBridgeStatus,
  getSetting,
  routePlannerContact,
  sendTwilioWhatsappMessage,
  seedExampleProducts,
  startTwilioVoiceCall,
  setDatabasePathOverride,
  setSetting,
} from "../../../lib/tauri";
import type { McpBridgeStatus, MobileBridgeStatus } from "../../../lib/types";
import {
  McpBridgeStatusCard,
  MobileBridgeStatusCard,
} from "../components/SettingsBridgeStatusCards";
import { SettingsDatabaseSections } from "../components/SettingsDatabaseSections";
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
import { useSettingsModelOptions } from "../hooks/useSettingsModelOptions";
import { useSettingsPageState } from "../hooks/useSettingsPageState";
import { styles } from "../lib/settingsPageStyles";

export function SettingsPage() {
  const queryClient = useQueryClient();
  const {
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
  } = useSettingsPageState();
  const { data: mcpBridgeStatus } = useQuery<McpBridgeStatus>({
    queryKey: ["mcpBridgeStatus"],
    queryFn: getMcpBridgeStatus,
  });
  const { data: mobileBridgeStatus } = useQuery<MobileBridgeStatus>({
    queryKey: ["mobileBridgeStatus"],
    queryFn: getMobileBridgeStatus,
  });

  const { speechProviderOptions, speechModelOptions } = useSettingsModelOptions({
    speechProviderId,
  });

  useEffect(() => {
    getSetting("docker_host").then((v) => { if (v) setDockerHost(v); });
    getSetting("max_workflow_retries").then((v) => { if (v) setMaxRetries(v); });
    getSetting(AUTO_START_AFTER_APPROVAL_KEY).then((v) => setAutoStartAfterApproval(parseBooleanSetting(v, true)));
    getSetting(AUTO_APPROVE_PLAN_KEY).then((v) => setAutoApprovePlan(parseBooleanSetting(v, true)));
    getSetting(AUTO_APPROVE_TEST_REVIEW_KEY).then((v) => setAutoApproveTestReview(parseBooleanSetting(v, true)));
    getSetting(HIDE_EXAMPLE_PRODUCTS_KEY).then((v) => setHideExampleProducts(parseBooleanSetting(v, true)));
    getSetting(PLANNER_DEFAULT_PROVIDER_KEY).then((v) => { if (v) setPlannerDefaultProviderId(v); });
    getSetting(PLANNER_DEFAULT_MODEL_KEY).then((v) => { if (v) setPlannerDefaultModelName(v); });
    getSetting(PLANNER_CHANNEL_PREFERENCE_KEY).then((v) => { if (v) setPlannerChannelPreference(v); });
    getSetting(PLANNER_ESCALATE_TO_CALL_KEY).then((v) => setPlannerEscalateToCall(parseBooleanSetting(v, true)));
    getSetting(PLANNER_CALL_QUIET_HOURS_START_KEY).then((v) => { if (v) setPlannerCallQuietHoursStart(v); });
    getSetting(PLANNER_CALL_QUIET_HOURS_END_KEY).then((v) => { if (v) setPlannerCallQuietHoursEnd(v); });
    getSetting(SPEECH_PROVIDER_KEY).then((v) => { if (v) setSpeechProviderId(v); });
    getSetting(SPEECH_MODEL_KEY).then((v) => { if (v) setSpeechModelName(v); });
    getSetting(SPEECH_LOCALE_KEY).then((v) => { if (v) setSpeechLocale(v); });
    getSetting(SPEECH_NATIVE_VOICE_KEY).then((v) => { if (v) setSpeechNativeVoice(v); });
    getSetting(SPEECH_ENABLE_MIC_KEY).then((v) => setSpeechEnableMic(parseBooleanSetting(v, true)));
    getSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY).then((v) => setSpeechAutoSpeakReplies(parseBooleanSetting(v, false)));
    getSetting(SPEECH_REVIEW_BEFORE_SEND_KEY).then((v) => setSpeechReviewBeforeSend(parseBooleanSetting(v, false)));
    getSetting(MCP_API_TOKEN_KEY).then((v) => { if (v) setMcpApiToken(v); });
    getSetting(MOBILE_API_TOKEN_KEY).then((v) => { if (v) setMobileApiToken(v); });
    getSetting(MOBILE_BIND_HOST_KEY).then((v) => { if (v) setMobileBindHost(v); });
    getSetting(MOBILE_BIND_PORT_KEY).then((v) => { if (v) setMobileBindPort(v); });
    getSetting(TWILIO_ACCOUNT_SID_KEY).then((v) => { if (v) setTwilioAccountSid(v); });
    getSetting(TWILIO_AUTH_TOKEN_KEY).then((v) => { if (v) setTwilioAuthToken(v); });
    getSetting(TWILIO_WHATSAPP_FROM_KEY).then((v) => { if (v) setTwilioWhatsappFrom(v); });
    getSetting(TWILIO_VOICE_FROM_KEY).then((v) => { if (v) setTwilioVoiceFrom(v); });
    getSetting(TWILIO_WEBHOOK_BASE_URL_KEY).then((v) => { if (v) setTwilioWebhookBaseUrl(v); });
    getSetting(PLANNER_CONTACT_TARGET_KEY).then((v) => { if (v) setPlannerContactTarget(v); });
    getSetting(PLANNER_CONTACT_OPENING_MESSAGE_KEY).then((v) => { if (v) setPlannerContactOpeningMessage(v); });
    getActiveDatabasePath().then(setActiveDbPath).catch((error) => setDbPathOverrideError(String(error)));
    getDatabasePathOverride().then((v) => { if (v) setDbPathOverrideInput(v); });
    getDatabaseHealth()
      .then((health) => {
        setDbHealth(health);
        setDbHealthError(null);
      })
      .catch((error) => {
        setDbHealthError(String(error));
      });
  }, []);

  useEffect(() => {
    if (!speechProviderId || speechModelName === "") {
      return;
    }
    if (!speechModelOptions.some((model) => model.name === speechModelName)) {
      setSpeechModelName("");
    }
  }, [speechModelName, speechModelOptions, speechProviderId]);

  const saveSetting = async (key: string, value: string) => {
    await setSetting(key, value);
    await queryClient.invalidateQueries({ queryKey: ["setting"] });
    await queryClient.invalidateQueries({ queryKey: ["mcpBridgeStatus"] });
    await queryClient.invalidateQueries({ queryKey: ["mobileBridgeStatus"] });
    if (key === HIDE_EXAMPLE_PRODUCTS_KEY) {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["products"] }),
        queryClient.invalidateQueries({ queryKey: ["productTree"] }),
        queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
        queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
      ]);
    }
    setSavedMsg(key);
    setTimeout(() => setSavedMsg(null), 2000);
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
      await sendTwilioWhatsappMessage({ to: plannerContactTarget.trim(), content: plannerContactOpeningMessage.trim() });
      setPlannerContactMsg("WhatsApp message queued through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  const startPlannerVoiceCall = async () => {
    try {
      setPlannerContactError(null);
      setPlannerContactMsg(null);
      await startTwilioVoiceCall({ to: plannerContactTarget.trim(), initialPrompt: plannerContactOpeningMessage.trim() || undefined });
      setPlannerContactMsg("Voice call requested through Twilio.");
    } catch (error) {
      setPlannerContactError(String(error));
    }
  };

  return (
    <div style={styles.page}>
      <h1 style={styles.title}>Settings</h1>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Catalog</div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Hide Example Products</div>
            <div style={styles.desc}>Seeded example products stay in the database but remain hidden from the workspace by default.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: hideExampleProducts ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !hideExampleProducts;
              setHideExampleProducts(next);
              await saveSetting(HIDE_EXAMPLE_PRODUCTS_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Seed / Repair Example Products</div>
            <div style={styles.desc}>Safe to run multiple times. Creates missing examples and repairs the built-in catalog in the currently active database.</div>
          </div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <button
              style={styles.btn}
              onClick={async () => {
                try {
                  setCatalogActionError(null);
                  await seedExampleProducts();
                  await Promise.all([
                    queryClient.invalidateQueries({ queryKey: ["products"] }),
                    queryClient.invalidateQueries({ queryKey: ["productTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["workItems"] }),
                  ]);
                  setCatalogActionMsg("Example catalog is present and up to date.");
                  setTimeout(() => setCatalogActionMsg(null), 2500);
                } catch (error) {
                  setCatalogActionError(String(error));
                }
              }}
            >
              Seed / Repair
            </button>
          </div>
        </div>
        {catalogActionMsg && <div style={styles.saved}>{catalogActionMsg}</div>}
        {catalogActionError && <div style={{ ...styles.desc, color: "#f48771", marginTop: 8 }}>{catalogActionError}</div>}
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Execution</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Docker Host</div><div style={styles.desc}>Docker daemon URL for test execution</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={dockerHost} onChange={(e) => setDockerHost(e.target.value)} placeholder="unix:///var/run/docker.sock" /><button style={styles.btn} onClick={() => saveSetting("docker_host", dockerHost)}>Save</button>{savedMsg === "docker_host" && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Max Workflow Retries</div><div style={styles.desc}>Maximum retry attempts for failed workflow stages</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 80 }} type="number" value={maxRetries} onChange={(e) => setMaxRetries(e.target.value)} /><button style={styles.btn} onClick={() => saveSetting("max_workflow_retries", maxRetries)}>Save</button>{savedMsg === "max_workflow_retries" && <span style={styles.saved}>Saved!</span>}</div>
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Planner Defaults</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Default Provider Id</div><div style={styles.desc}>Used by WhatsApp/call planner sessions when no provider is specified in the UI.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={plannerDefaultProviderId} onChange={(e) => setPlannerDefaultProviderId(e.target.value)} placeholder="provider uuid" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_DEFAULT_PROVIDER_KEY, plannerDefaultProviderId)}>Save</button>{savedMsg === PLANNER_DEFAULT_PROVIDER_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Default Model Name</div><div style={styles.desc}>Model name used by external planner channels when a new session is created.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={plannerDefaultModelName} onChange={(e) => setPlannerDefaultModelName(e.target.value)} placeholder="gpt-4.1-mini or local model name" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_DEFAULT_MODEL_KEY, plannerDefaultModelName)}>Save</button>{savedMsg === PLANNER_DEFAULT_MODEL_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Outbound Channel Preference</div><div style={styles.desc}>Controls how planner outreach routes by default. Hybrid uses WhatsApp for routine updates and escalates ambiguous planning turns to calls.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <select style={styles.input} value={plannerChannelPreference} onChange={(e) => setPlannerChannelPreference(e.target.value)}>
              <option value="hybrid">Hybrid</option>
              <option value="whatsapp">Prefer WhatsApp</option>
              <option value="voice">Prefer Voice Calls</option>
            </select>
            <button style={styles.btn} onClick={() => saveSetting(PLANNER_CHANNEL_PREFERENCE_KEY, plannerChannelPreference)}>Save</button>
            {savedMsg === PLANNER_CHANNEL_PREFERENCE_KEY && <span style={styles.saved}>Saved!</span>}
          </div>
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Escalate Ambiguous Planning To Call</div>
            <div style={styles.desc}>When enabled, hybrid mode promotes exploratory or high-ambiguity outreach to a voice call instead of WhatsApp.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: plannerEscalateToCall ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !plannerEscalateToCall;
              setPlannerEscalateToCall(next);
              await saveSetting(PLANNER_ESCALATE_TO_CALL_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Call Quiet Hours Start</div><div style={styles.desc}>Calls auto-fall back to WhatsApp during quiet hours. Uses this machine&apos;s local time.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 120 }} value={plannerCallQuietHoursStart} onChange={(e) => setPlannerCallQuietHoursStart(e.target.value)} placeholder="21:00" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CALL_QUIET_HOURS_START_KEY, plannerCallQuietHoursStart)}>Save</button>{savedMsg === PLANNER_CALL_QUIET_HOURS_START_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Call Quiet Hours End</div><div style={styles.desc}>End of the quiet-hours window in `HH:MM` 24-hour format.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 120 }} value={plannerCallQuietHoursEnd} onChange={(e) => setPlannerCallQuietHoursEnd(e.target.value)} placeholder="08:00" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CALL_QUIET_HOURS_END_KEY, plannerCallQuietHoursEnd)}>Save</button>{savedMsg === PLANNER_CALL_QUIET_HOURS_END_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Speech</div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Enable Voice Input By Default</div>
            <div style={styles.desc}>Controls whether planner and chat start with microphone input enabled.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: speechEnableMic ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !speechEnableMic;
              setSpeechEnableMic(next);
              await saveSetting(SPEECH_ENABLE_MIC_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Speak Assistant Replies By Default</div>
            <div style={styles.desc}>Uses native macOS voice when available. This affects planner and direct chat voice replies.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: speechAutoSpeakReplies ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !speechAutoSpeakReplies;
              setSpeechAutoSpeakReplies(next);
              await saveSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Review Transcript Before Sending</div>
            <div style={styles.desc}>Turn this on if you want voice input to pause for transcript editing. Leave it off for a hands-free flow that sends speech straight to the planner.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: speechReviewBeforeSend ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !speechReviewBeforeSend;
              setSpeechReviewBeforeSend(next);
              await saveSetting(SPEECH_REVIEW_BEFORE_SEND_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Speech Provider</div><div style={styles.desc}>Explicit provider used for planner voice transcription. Leave blank to allow automatic discovery.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <select style={styles.input} value={speechProviderId} onChange={(e) => setSpeechProviderId(e.target.value)}>
              <option value="">Automatic</option>
              {speechProviderOptions.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                </option>
              ))}
            </select>
            <button style={styles.btn} onClick={() => saveSetting(SPEECH_PROVIDER_KEY, speechProviderId)}>Save</button>
            {savedMsg === SPEECH_PROVIDER_KEY && <span style={styles.saved}>Saved!</span>}
          </div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Speech Model</div><div style={styles.desc}>Pick a Whisper/transcription model explicitly so desktop and mobile voice use the same backend speech path.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <select style={styles.input} value={speechModelName} onChange={(e) => setSpeechModelName(e.target.value)}>
              <option value="">Automatic</option>
              {speechModelOptions.map((model) => (
                <option key={model.id} value={model.name}>
                  {model.name}
                </option>
              ))}
            </select>
            <button style={styles.btn} onClick={() => saveSetting(SPEECH_MODEL_KEY, speechModelName)}>Save</button>
            {savedMsg === SPEECH_MODEL_KEY && <span style={styles.saved}>Saved!</span>}
          </div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Speech Locale</div><div style={styles.desc}>Locale hint for transcription and spoken replies, for example `en-US`.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={speechLocale} onChange={(e) => setSpeechLocale(e.target.value)} placeholder="en-US" /><button style={styles.btn} onClick={() => saveSetting(SPEECH_LOCALE_KEY, speechLocale)}>Save</button>{savedMsg === SPEECH_LOCALE_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Native Speech Voice</div><div style={styles.desc}>Optional macOS `say` voice, for example `Samantha`, used for planner replies when native speech is enabled.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={speechNativeVoice} onChange={(e) => setSpeechNativeVoice(e.target.value)} placeholder="Samantha" /><button style={styles.btn} onClick={() => saveSetting(SPEECH_NATIVE_VOICE_KEY, speechNativeVoice)}>Save</button>{savedMsg === SPEECH_NATIVE_VOICE_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Agents / MCP</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>MCP API Token</div><div style={styles.desc}>Optional bearer token for agent hosts that connect to Aruvi over the embedded HTTP MCP endpoint. Required before exposing the bridge beyond localhost.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={mcpApiToken} onChange={(e) => setMcpApiToken(e.target.value)} placeholder="set-a-strong-token" /><button style={styles.btn} onClick={() => saveSetting(MCP_API_TOKEN_KEY, mcpApiToken)}>Save</button>{savedMsg === MCP_API_TOKEN_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={{ ...styles.desc, marginTop: 8 }}>
          Aruvi hosts MCP on the same embedded HTTP bridge as the mobile companion, so the bind host and port below also control the MCP endpoint agents connect to.
        </div>
        <McpBridgeStatusCard status={mcpBridgeStatus} onCopy={(value) => void copyText(value)} />
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Mobile Companion</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Mobile API Token</div><div style={styles.desc}>Bearer token used by the iPhone planner companion when it talks to the desktop planner bridge.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={mobileApiToken} onChange={(e) => setMobileApiToken(e.target.value)} placeholder="set-a-strong-token" /><button style={styles.btn} onClick={() => saveSetting(MOBILE_API_TOKEN_KEY, mobileApiToken)}>Save</button>{savedMsg === MOBILE_API_TOKEN_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Bind Host</div><div style={styles.desc}>Use `0.0.0.0` for same-LAN iPhone access. `127.0.0.1` keeps the mobile bridge local to this Mac. Restart required.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 180 }} value={mobileBindHost} onChange={(e) => setMobileBindHost(e.target.value)} placeholder="0.0.0.0" /><button style={styles.btn} onClick={() => saveSetting(MOBILE_BIND_HOST_KEY, mobileBindHost)}>Save</button>{savedMsg === MOBILE_BIND_HOST_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Bind Port</div><div style={styles.desc}>Port exposed by the desktop planner bridge. Restart required after changes.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 120 }} type="number" min="1" max="65535" value={mobileBindPort} onChange={(e) => setMobileBindPort(e.target.value)} placeholder="8787" /><button style={styles.btn} onClick={() => saveSetting(MOBILE_BIND_PORT_KEY, mobileBindPort)}>Save</button>{savedMsg === MOBILE_BIND_PORT_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={{ ...styles.desc, marginTop: 8 }}>
          The phone client uses the same planner and speech APIs as the desktop UI. To reach the desktop from an iPhone, expose the webhook server on a reachable host and connect with this token.
        </div>
        <MobileBridgeStatusCard status={mobileBridgeStatus} onCopy={(value) => void copyText(value)} />
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Twilio</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Account SID</div><div style={styles.desc}>Twilio account sid used for webhook validation and outbound API calls.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={twilioAccountSid} onChange={(e) => setTwilioAccountSid(e.target.value)} placeholder="AC..." /><button style={styles.btn} onClick={() => saveSetting(TWILIO_ACCOUNT_SID_KEY, twilioAccountSid)}>Save</button>{savedMsg === TWILIO_ACCOUNT_SID_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Auth Token</div><div style={styles.desc}>Used to validate inbound webhook signatures and authenticate outbound requests.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} type="password" value={twilioAuthToken} onChange={(e) => setTwilioAuthToken(e.target.value)} placeholder="Twilio auth token" /><button style={styles.btn} onClick={() => saveSetting(TWILIO_AUTH_TOKEN_KEY, twilioAuthToken)}>Save</button>{savedMsg === TWILIO_AUTH_TOKEN_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>WhatsApp From</div><div style={styles.desc}>Twilio WhatsApp sender, for example `whatsapp:+14155238886`.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={twilioWhatsappFrom} onChange={(e) => setTwilioWhatsappFrom(e.target.value)} placeholder="whatsapp:+14155238886" /><button style={styles.btn} onClick={() => saveSetting(TWILIO_WHATSAPP_FROM_KEY, twilioWhatsappFrom)}>Save</button>{savedMsg === TWILIO_WHATSAPP_FROM_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Voice From</div><div style={styles.desc}>Twilio voice-enabled caller id used when the planner starts a phone call.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={twilioVoiceFrom} onChange={(e) => setTwilioVoiceFrom(e.target.value)} placeholder="+15551234567" /><button style={styles.btn} onClick={() => saveSetting(TWILIO_VOICE_FROM_KEY, twilioVoiceFrom)}>Save</button>{savedMsg === TWILIO_VOICE_FROM_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Webhook Base URL</div><div style={styles.desc}>Public base URL Twilio will call, used for signature validation and outbound voice-call callback URLs.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 380 }} value={twilioWebhookBaseUrl} onChange={(e) => setTwilioWebhookBaseUrl(e.target.value)} placeholder="https://your-public-domain.example.com" /><button style={styles.btn} onClick={() => saveSetting(TWILIO_WEBHOOK_BASE_URL_KEY, twilioWebhookBaseUrl)}>Save</button>{savedMsg === TWILIO_WEBHOOK_BASE_URL_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Planner Contact</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Default Destination</div><div style={styles.desc}>Used when you want the planner to contact you by WhatsApp or voice call.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={plannerContactTarget} onChange={(e) => setPlannerContactTarget(e.target.value)} placeholder="whatsapp:+15551234567 or +15551234567" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CONTACT_TARGET_KEY, plannerContactTarget)}>Save</button>{savedMsg === PLANNER_CONTACT_TARGET_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Default Opening Message</div><div style={styles.desc}>Starter message used when the planner initiates contact.</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 380 }} value={plannerContactOpeningMessage} onChange={(e) => setPlannerContactOpeningMessage(e.target.value)} placeholder="Call me and ask what work should be prioritized next." /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CONTACT_OPENING_MESSAGE_KEY, plannerContactOpeningMessage)}>Save</button>{savedMsg === PLANNER_CONTACT_OPENING_MESSAGE_KEY && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={{ ...styles.desc, marginTop: 8 }}>
          Use these actions to test the same outbound contact flow that used to live in the planner surface.
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 12, flexWrap: "wrap" as const }}>
          <button style={styles.btn} onClick={() => void autoRoutePlannerContact()} disabled={!plannerContactTarget.trim() || !plannerContactOpeningMessage.trim()}>
            Auto Route
          </button>
          <button style={styles.btn} onClick={() => void sendPlannerWhatsapp()} disabled={!plannerContactTarget.trim()}>
            Send WhatsApp
          </button>
          <button style={styles.btn} onClick={() => void startPlannerVoiceCall()} disabled={!plannerContactTarget.trim()}>
            Start Voice Call
          </button>
        </div>
        {plannerContactMsg ? <div style={{ ...styles.desc, color: "#4ec9b0", marginTop: 10 }}>{plannerContactMsg}</div> : null}
        {plannerContactError ? <div style={{ ...styles.desc, color: "#f48771", marginTop: 10 }}>{plannerContactError}</div> : null}
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Workflow Automation</div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-start after work item approval</div>
            <div style={styles.desc}>When a work item is approved, queue its workflow immediately in the background. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoStartAfterApproval ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoStartAfterApproval;
              setAutoStartAfterApproval(next);
              await saveSetting(AUTO_START_AFTER_APPROVAL_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-approve planning</div>
            <div style={styles.desc}>After planning completes, record plan approval automatically and continue into coding. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoApprovePlan ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoApprovePlan;
              setAutoApprovePlan(next);
              await saveSetting(AUTO_APPROVE_PLAN_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-approve test review</div>
            <div style={styles.desc}>After validation, security, and performance stages complete, record test review approval automatically and continue into push preparation. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoApproveTestReview ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoApproveTestReview;
              setAutoApproveTestReview(next);
              await saveSetting(AUTO_APPROVE_TEST_REVIEW_KEY, String(next));
            }}
          />
        </div>
      </div>
      <SettingsDatabaseSections
        activeDbPath={activeDbPath}
        dbPathOverrideInput={dbPathOverrideInput}
        dbPathOverrideSaved={dbPathOverrideSaved}
        dbPathOverrideError={dbPathOverrideError}
        dbHealth={dbHealth}
        dbHealthError={dbHealthError}
        onDbPathOverrideInputChange={setDbPathOverrideInput}
        onSaveDbOverride={() => void saveDbOverride()}
        onClearDbOverride={() => void clearDbOverride()}
      />
    </div>
  );
}
