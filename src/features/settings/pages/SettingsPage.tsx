import React, { useState, useEffect, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  clearDatabasePathOverride,
  getActiveDatabasePath,
  getDatabaseHealth,
  getDatabasePathOverride,
  getMobileBridgeStatus,
  getSetting,
  listModelDefinitions,
  listProviders,
  seedExampleProducts,
  setDatabasePathOverride,
  setSetting,
} from "../../../lib/tauri";
import type { DatabaseHealth, MobileBridgeStatus, ModelDefinition, ModelProvider } from "../../../lib/types";
import { useUIStore } from "../../../state/uiStore";

const AUTO_START_AFTER_APPROVAL_KEY = "workflow.auto_start_after_work_item_approval";
const AUTO_APPROVE_PLAN_KEY = "workflow.auto_approve_plan";
const AUTO_APPROVE_TEST_REVIEW_KEY = "workflow.auto_approve_test_review";
const HIDE_EXAMPLE_PRODUCTS_KEY = "catalog.hide_example_products";
const PLANNER_DEFAULT_PROVIDER_KEY = "planner.default_provider_id";
const PLANNER_DEFAULT_MODEL_KEY = "planner.default_model_name";
const PLANNER_CHANNEL_PREFERENCE_KEY = "planner.channel_preference";
const PLANNER_ESCALATE_TO_CALL_KEY = "planner.escalate_to_call_on_ambiguity";
const PLANNER_CALL_QUIET_HOURS_START_KEY = "planner.call_quiet_hours_start";
const PLANNER_CALL_QUIET_HOURS_END_KEY = "planner.call_quiet_hours_end";
const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
const MOBILE_API_TOKEN_KEY = "mobile.api_token";
const MOBILE_BIND_HOST_KEY = "mobile.bind_host";
const MOBILE_BIND_PORT_KEY = "mobile.bind_port";
const TWILIO_ACCOUNT_SID_KEY = "twilio.account_sid";
const TWILIO_AUTH_TOKEN_KEY = "twilio.auth_token";
const TWILIO_WHATSAPP_FROM_KEY = "twilio.whatsapp_from";
const TWILIO_VOICE_FROM_KEY = "twilio.voice_from";
const TWILIO_WEBHOOK_BASE_URL_KEY = "twilio.webhook_base_url";

function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 700, margin: "0 auto" },
  title: { fontSize: 20, fontWeight: 600, color: "#e0e0e0", marginBottom: 24 },
  section: { marginBottom: 32 },
  sectionTitle: { fontSize: 14, fontWeight: 600, color: "#cccccc", marginBottom: 12, borderBottom: "1px solid #333", paddingBottom: 8 },
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid #2a2a2a" },
  label: { fontSize: 13, color: "#e0e0e0" },
  desc: { fontSize: 11, color: "#888" },
  toggle: { width: 40, height: 22, borderRadius: 11, border: "none", cursor: "pointer", position: "relative" as const, transition: "background 0.2s" },
  settingRow: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 0", borderBottom: "1px solid #2a2a2a" },
  input: { width: 300, padding: "6px 10px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13 },
  btn: { padding: "6px 16px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer", marginLeft: 8 },
  saved: { fontSize: 12, color: "#4ec9b0", marginLeft: 8 },
  healthCard: { backgroundColor: "#1f1f1f", border: "1px solid #333", borderRadius: 8, padding: 16, marginTop: 12 },
  healthGrid: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12, marginBottom: 16 },
  healthLabel: { fontSize: 11, color: "#888", textTransform: "uppercase" as const, letterSpacing: 0.6 },
  healthValue: { fontSize: 18, fontWeight: 700, color: "#e0e0e0", marginTop: 4 },
  migrationList: { display: "flex", flexDirection: "column" as const, gap: 8, maxHeight: 220, overflowY: "auto" as const },
  migrationRow: { display: "flex", justifyContent: "space-between", gap: 12, padding: "8px 10px", backgroundColor: "#181818", border: "1px solid #2a2a2a", borderRadius: 6 },
  badge: { padding: "2px 8px", borderRadius: 999, fontSize: 11, fontWeight: 700 },
  codeBox: { fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12, color: "#d7e3ff", backgroundColor: "#171b24", border: "1px solid #2d3a52", borderRadius: 6, padding: "8px 10px" },
};

export function SettingsPage() {
  const queryClient = useQueryClient();
  const { leftSidebarVisible, rightSidebarVisible, bottomPanelVisible, toggleLeftSidebar, toggleRightSidebar, toggleBottomPanel } = useUIStore();
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
  const [mobileApiToken, setMobileApiToken] = useState("");
  const [mobileBindHost, setMobileBindHost] = useState("127.0.0.1");
  const [mobileBindPort, setMobileBindPort] = useState("8787");
  const [twilioAccountSid, setTwilioAccountSid] = useState("");
  const [twilioAuthToken, setTwilioAuthToken] = useState("");
  const [twilioWhatsappFrom, setTwilioWhatsappFrom] = useState("");
  const [twilioVoiceFrom, setTwilioVoiceFrom] = useState("");
  const [twilioWebhookBaseUrl, setTwilioWebhookBaseUrl] = useState("");
  const { data: providers = [] } = useQuery<ModelProvider[]>({ queryKey: ["settingsProviders"], queryFn: listProviders });
  const { data: models = [] } = useQuery<ModelDefinition[]>({ queryKey: ["settingsModels"], queryFn: listModelDefinitions });
  const { data: mobileBridgeStatus } = useQuery<MobileBridgeStatus>({
    queryKey: ["mobileBridgeStatus"],
    queryFn: getMobileBridgeStatus,
  });

  const speechProviderOptions = useMemo(
    () => providers.filter((provider) => provider.enabled),
    [providers],
  );
  const speechModelOptions = useMemo(() => {
    const looksLikeSpeechModel = (model: ModelDefinition) =>
      model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
      || /whisper|transcrib/i.test(model.name);
    return models.filter((model) => model.enabled && looksLikeSpeechModel(model) && (!speechProviderId || model.provider_id === speechProviderId));
  }, [models, speechProviderId]);

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
    getSetting(MOBILE_API_TOKEN_KEY).then((v) => { if (v) setMobileApiToken(v); });
    getSetting(MOBILE_BIND_HOST_KEY).then((v) => { if (v) setMobileBindHost(v); });
    getSetting(MOBILE_BIND_PORT_KEY).then((v) => { if (v) setMobileBindPort(v); });
    getSetting(TWILIO_ACCOUNT_SID_KEY).then((v) => { if (v) setTwilioAccountSid(v); });
    getSetting(TWILIO_AUTH_TOKEN_KEY).then((v) => { if (v) setTwilioAuthToken(v); });
    getSetting(TWILIO_WHATSAPP_FROM_KEY).then((v) => { if (v) setTwilioWhatsappFrom(v); });
    getSetting(TWILIO_VOICE_FROM_KEY).then((v) => { if (v) setTwilioVoiceFrom(v); });
    getSetting(TWILIO_WEBHOOK_BASE_URL_KEY).then((v) => { if (v) setTwilioWebhookBaseUrl(v); });
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

  return (
    <div style={styles.page}>
      <h1 style={styles.title}>Settings</h1>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Layout</div>
        <div style={styles.row}><div><div style={styles.label}>Left Sidebar</div><div style={styles.desc}>Product tree and navigation</div></div><button style={{ ...styles.toggle, backgroundColor: leftSidebarVisible ? "#0e639c" : "#444" }} onClick={toggleLeftSidebar} /></div>
        <div style={styles.row}><div><div style={styles.label}>Right Sidebar</div><div style={styles.desc}>Context panel for work item details</div></div><button style={{ ...styles.toggle, backgroundColor: rightSidebarVisible ? "#0e639c" : "#444" }} onClick={toggleRightSidebar} /></div>
        <div style={styles.row}><div><div style={styles.label}>Bottom Panel</div><div style={styles.desc}>Terminal, logs, and test results</div></div><button style={{ ...styles.toggle, backgroundColor: bottomPanelVisible ? "#0e639c" : "#444" }} onClick={toggleBottomPanel} /></div>
      </div>
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
        <div style={styles.healthCard}>
          <div style={{ ...styles.label, marginBottom: 8 }}>LAN Ready Status</div>
          {mobileBridgeStatus ? (
            <>
              <div style={styles.healthGrid}>
                <div>
                  <div style={styles.healthLabel}>Bind Scope</div>
                  <div style={styles.healthValue}>{mobileBridgeStatus.bind_scope}</div>
                </div>
                <div>
                  <div style={styles.healthLabel}>Detected Mac LAN IP</div>
                  <div style={styles.healthValue}>{mobileBridgeStatus.detected_lan_ip ?? "Unavailable"}</div>
                </div>
              </div>
              <div style={{ ...styles.desc, marginBottom: 8 }}>
                {mobileBridgeStatus.guidance}
              </div>
              <div style={{ marginBottom: 10 }}>
                <div style={styles.healthLabel}>Desktop Base URL</div>
                <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
                  <div style={{ ...styles.codeBox, flex: 1 }}>{mobileBridgeStatus.desktop_base_url}</div>
                  <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => copyText(mobileBridgeStatus.desktop_base_url)}>Copy</button>
                </div>
              </div>
              <div style={{ marginBottom: 10 }}>
                <div style={styles.healthLabel}>Phone Base URL</div>
                <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
                  <div style={{ ...styles.codeBox, flex: 1 }}>{mobileBridgeStatus.phone_base_url ?? "Set bind host to 0.0.0.0 and restart to enable same-LAN access."}</div>
                  {mobileBridgeStatus.phone_base_url && <button style={{ ...styles.btn, marginLeft: 0 }} onClick={() => copyText(mobileBridgeStatus.phone_base_url!)}>Copy</button>}
                </div>
              </div>
              <div style={styles.desc}>
                Bind host source: {mobileBridgeStatus.host_source}. Port source: {mobileBridgeStatus.port_source}. {mobileBridgeStatus.env_overrides_settings ? "Environment variables currently override these settings. " : ""}{mobileBridgeStatus.bind_changes_require_restart ? "Restart AruviStudio after changing bind host or port." : ""}
              </div>
            </>
          ) : (
            <div style={styles.desc}>Loading mobile bridge status…</div>
          )}
        </div>
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
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Database Source</div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Active Database</div>
            <div style={styles.desc}>{activeDbPath || "Unknown"}</div>
          </div>
        </div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Override Database Path</div>
            <div style={styles.desc}>Set an absolute SQLite path for next app launch. Restart required.</div>
          </div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <input
              style={{ ...styles.input, width: 380 }}
              value={dbPathOverrideInput}
              onChange={(e) => setDbPathOverrideInput(e.target.value)}
              placeholder="/absolute/path/to/aruvi-live.db"
            />
            <button style={styles.btn} onClick={saveDbOverride}>Save</button>
            <button style={{ ...styles.btn, backgroundColor: "#3a4556" }} onClick={clearDbOverride}>Clear</button>
          </div>
        </div>
        {dbPathOverrideSaved === "saved" && <div style={styles.saved}>DB override saved. Restart AruviStudio to apply.</div>}
        {dbPathOverrideSaved === "cleared" && <div style={styles.saved}>DB override cleared. Restart AruviStudio to use default DB.</div>}
        {dbPathOverrideError && <div style={{ ...styles.desc, color: "#f48771", marginTop: 8 }}>{dbPathOverrideError}</div>}
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Database Health</div>
        <div style={styles.healthCard}>
          {dbHealthError && <div style={{ ...styles.desc, color: "#f48771" }}>{dbHealthError}</div>}
          {dbHealth && (
            <>
              <div style={styles.healthGrid}>
                <div>
                  <div style={styles.healthLabel}>Applied Migrations</div>
                  <div style={styles.healthValue}>{dbHealth.applied_migrations}</div>
                </div>
                <div>
                  <div style={styles.healthLabel}>Latest Version</div>
                  <div style={styles.healthValue}>{dbHealth.latest_version ?? "N/A"}</div>
                </div>
              </div>
              <div style={styles.migrationList}>
                {dbHealth.migrations.map((migration) => (
                  <div key={migration.version} style={styles.migrationRow}>
                    <div>
                      <div style={styles.label}>v{migration.version} · {migration.description}</div>
                      <div style={styles.desc}>Installed {migration.installed_on}</div>
                    </div>
                    <span
                      style={{
                        ...styles.badge,
                        color: migration.success ? "#4ec9b0" : "#f48771",
                        backgroundColor: migration.success ? "rgba(78, 201, 176, 0.12)" : "rgba(244, 135, 113, 0.12)",
                      }}
                    >
                      {migration.success ? "Applied" : "Failed"}
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}
          {!dbHealth && !dbHealthError && <div style={styles.desc}>Loading migration metadata…</div>}
        </div>
      </div>
    </div>
  );
}
