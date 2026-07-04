import {
  PLANNER_CALL_QUIET_HOURS_END_KEY,
  PLANNER_CALL_QUIET_HOURS_START_KEY,
  PLANNER_CHANNEL_PREFERENCE_KEY,
  PLANNER_DEFAULT_MODEL_KEY,
  PLANNER_DEFAULT_PROVIDER_KEY,
  PLANNER_ESCALATE_TO_CALL_KEY,
} from "../lib/settingsKeys";
import { styles } from "../lib/settingsPageStyles";

type SettingsPlannerDefaultsSectionProps = {
  plannerDefaultProviderId: string;
  onPlannerDefaultProviderIdChange: (value: string) => void;
  plannerDefaultModelName: string;
  onPlannerDefaultModelNameChange: (value: string) => void;
  plannerChannelPreference: string;
  onPlannerChannelPreferenceChange: (value: string) => void;
  plannerEscalateToCall: boolean;
  onPlannerEscalateToCallChange: (value: boolean) => void;
  plannerCallQuietHoursStart: string;
  onPlannerCallQuietHoursStartChange: (value: string) => void;
  plannerCallQuietHoursEnd: string;
  onPlannerCallQuietHoursEndChange: (value: string) => void;
  savedMsg: string | null;
  saveSetting: (key: string, value: string) => Promise<void>;
};

export function SettingsPlannerDefaultsSection({
  plannerDefaultProviderId,
  onPlannerDefaultProviderIdChange,
  plannerDefaultModelName,
  onPlannerDefaultModelNameChange,
  plannerChannelPreference,
  onPlannerChannelPreferenceChange,
  plannerEscalateToCall,
  onPlannerEscalateToCallChange,
  plannerCallQuietHoursStart,
  onPlannerCallQuietHoursStartChange,
  plannerCallQuietHoursEnd,
  onPlannerCallQuietHoursEndChange,
  savedMsg,
  saveSetting,
}: SettingsPlannerDefaultsSectionProps) {
  return (
    <div style={styles.section}>
      <div style={styles.sectionTitle}>Planner Defaults</div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Default Provider Id</div><div style={styles.desc}>Used by WhatsApp/call planner sessions when no provider is specified in the UI.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={plannerDefaultProviderId} onChange={(e) => onPlannerDefaultProviderIdChange(e.target.value)} placeholder="provider uuid" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_DEFAULT_PROVIDER_KEY, plannerDefaultProviderId)}>Save</button>{savedMsg === PLANNER_DEFAULT_PROVIDER_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Default Model Name</div><div style={styles.desc}>Model name used by external planner channels when a new session is created.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={plannerDefaultModelName} onChange={(e) => onPlannerDefaultModelNameChange(e.target.value)} placeholder="gpt-4.1-mini or local model name" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_DEFAULT_MODEL_KEY, plannerDefaultModelName)}>Save</button>{savedMsg === PLANNER_DEFAULT_MODEL_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Outbound Channel Preference</div><div style={styles.desc}>Controls how planner outreach routes by default. Hybrid uses WhatsApp for routine updates and escalates ambiguous planning turns to calls.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}>
          <select style={styles.input} value={plannerChannelPreference} onChange={(e) => onPlannerChannelPreferenceChange(e.target.value)}>
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
            onPlannerEscalateToCallChange(next);
            await saveSetting(PLANNER_ESCALATE_TO_CALL_KEY, String(next));
          }}
        />
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Call Quiet Hours Start</div><div style={styles.desc}>Calls auto-fall back to WhatsApp during quiet hours. Uses this machine&apos;s local time.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 120 }} value={plannerCallQuietHoursStart} onChange={(e) => onPlannerCallQuietHoursStartChange(e.target.value)} placeholder="21:00" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CALL_QUIET_HOURS_START_KEY, plannerCallQuietHoursStart)}>Save</button>{savedMsg === PLANNER_CALL_QUIET_HOURS_START_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Call Quiet Hours End</div><div style={styles.desc}>End of the quiet-hours window in `HH:MM` 24-hour format.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 120 }} value={plannerCallQuietHoursEnd} onChange={(e) => onPlannerCallQuietHoursEndChange(e.target.value)} placeholder="08:00" /><button style={styles.btn} onClick={() => saveSetting(PLANNER_CALL_QUIET_HOURS_END_KEY, plannerCallQuietHoursEnd)}>Save</button>{savedMsg === PLANNER_CALL_QUIET_HOURS_END_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
    </div>
  );
}
