import React, { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getMcpBridgeStatus,
  getMobileBridgeStatus,
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
  PLANNER_CONTACT_TARGET_KEY,
  PLANNER_CONTACT_OPENING_MESSAGE_KEY,
  TWILIO_ACCOUNT_SID_KEY,
  TWILIO_AUTH_TOKEN_KEY,
  TWILIO_VOICE_FROM_KEY,
  TWILIO_WEBHOOK_BASE_URL_KEY,
  TWILIO_WHATSAPP_FROM_KEY,
} from "../lib/settingsKeys";
import { useSettingsModelOptions } from "../hooks/useSettingsModelOptions";
import { useSettingsPageState } from "../hooks/useSettingsPageState";
import {
  useSettingsLoader,
  useSettingsPageActions,
} from "../hooks/useSettingsPersistence";
import { SettingsPlannerDefaultsSection } from "../components/SettingsPlannerDefaultsSection";
import { SettingsSpeechSection } from "../components/SettingsSpeechSection";
import { styles } from "../lib/settingsPageStyles";

export function SettingsPage() {
  const queryClient = useQueryClient();
  const settingsState = useSettingsPageState();
  const {
    dockerHost, setDockerHost, maxRetries, setMaxRetries,
    autoStartAfterApproval, setAutoStartAfterApproval,
    autoApprovePlan, setAutoApprovePlan, autoApproveTestReview, setAutoApproveTestReview,
    hideExampleProducts, setHideExampleProducts, savedMsg,
    dbHealth, dbHealthError, activeDbPath,
    dbPathOverrideInput, setDbPathOverrideInput, dbPathOverrideSaved,
    dbPathOverrideError, catalogActionMsg,
    catalogActionError, plannerDefaultProviderId,
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
    plannerContactMsg, plannerContactError,
  } = settingsState;
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

  useSettingsLoader(settingsState);

  useEffect(() => {
    if (!speechProviderId || speechModelName === "") {
      return;
    }
    if (!speechModelOptions.some((model) => model.name === speechModelName)) {
      setSpeechModelName("");
    }
  }, [speechModelName, speechModelOptions, speechProviderId]);

  const {
    autoRoutePlannerContact,
    clearDbOverride,
    copyText,
    saveDbOverride,
    saveSetting,
    seedExampleCatalog,
    sendPlannerWhatsapp,
    startPlannerVoiceCall,
  } = useSettingsPageActions({ queryClient, settings: settingsState });

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
              onClick={() => void seedExampleCatalog()}
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
      <SettingsPlannerDefaultsSection
        plannerDefaultProviderId={plannerDefaultProviderId}
        onPlannerDefaultProviderIdChange={setPlannerDefaultProviderId}
        plannerDefaultModelName={plannerDefaultModelName}
        onPlannerDefaultModelNameChange={setPlannerDefaultModelName}
        plannerChannelPreference={plannerChannelPreference}
        onPlannerChannelPreferenceChange={setPlannerChannelPreference}
        plannerEscalateToCall={plannerEscalateToCall}
        onPlannerEscalateToCallChange={setPlannerEscalateToCall}
        plannerCallQuietHoursStart={plannerCallQuietHoursStart}
        onPlannerCallQuietHoursStartChange={setPlannerCallQuietHoursStart}
        plannerCallQuietHoursEnd={plannerCallQuietHoursEnd}
        onPlannerCallQuietHoursEndChange={setPlannerCallQuietHoursEnd}
        savedMsg={savedMsg}
        saveSetting={saveSetting}
      />
      <SettingsSpeechSection
        speechProviderId={speechProviderId}
        onSpeechProviderIdChange={setSpeechProviderId}
        speechModelName={speechModelName}
        onSpeechModelNameChange={setSpeechModelName}
        speechLocale={speechLocale}
        onSpeechLocaleChange={setSpeechLocale}
        speechNativeVoice={speechNativeVoice}
        onSpeechNativeVoiceChange={setSpeechNativeVoice}
        speechEnableMic={speechEnableMic}
        onSpeechEnableMicChange={setSpeechEnableMic}
        speechAutoSpeakReplies={speechAutoSpeakReplies}
        onSpeechAutoSpeakRepliesChange={setSpeechAutoSpeakReplies}
        speechReviewBeforeSend={speechReviewBeforeSend}
        onSpeechReviewBeforeSendChange={setSpeechReviewBeforeSend}
        speechProviderOptions={speechProviderOptions}
        speechModelOptions={speechModelOptions}
        savedMsg={savedMsg}
        saveSetting={saveSetting}
      />
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
