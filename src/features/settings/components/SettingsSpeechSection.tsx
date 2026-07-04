import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  SPEECH_AUTO_SPEAK_REPLIES_KEY,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
  SPEECH_REVIEW_BEFORE_SEND_KEY,
} from "../lib/settingsKeys";
import { styles } from "../lib/settingsPageStyles";

type SettingsSpeechSectionProps = {
  speechProviderId: string;
  onSpeechProviderIdChange: (value: string) => void;
  speechModelName: string;
  onSpeechModelNameChange: (value: string) => void;
  speechLocale: string;
  onSpeechLocaleChange: (value: string) => void;
  speechNativeVoice: string;
  onSpeechNativeVoiceChange: (value: string) => void;
  speechEnableMic: boolean;
  onSpeechEnableMicChange: (value: boolean) => void;
  speechAutoSpeakReplies: boolean;
  onSpeechAutoSpeakRepliesChange: (value: boolean) => void;
  speechReviewBeforeSend: boolean;
  onSpeechReviewBeforeSendChange: (value: boolean) => void;
  speechProviderOptions: ModelProvider[];
  speechModelOptions: ModelDefinition[];
  savedMsg: string | null;
  saveSetting: (key: string, value: string) => Promise<void>;
};

export function SettingsSpeechSection({
  speechProviderId,
  onSpeechProviderIdChange,
  speechModelName,
  onSpeechModelNameChange,
  speechLocale,
  onSpeechLocaleChange,
  speechNativeVoice,
  onSpeechNativeVoiceChange,
  speechEnableMic,
  onSpeechEnableMicChange,
  speechAutoSpeakReplies,
  onSpeechAutoSpeakRepliesChange,
  speechReviewBeforeSend,
  onSpeechReviewBeforeSendChange,
  speechProviderOptions,
  speechModelOptions,
  savedMsg,
  saveSetting,
}: SettingsSpeechSectionProps) {
  return (
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
            onSpeechEnableMicChange(next);
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
            onSpeechAutoSpeakRepliesChange(next);
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
            onSpeechReviewBeforeSendChange(next);
            await saveSetting(SPEECH_REVIEW_BEFORE_SEND_KEY, String(next));
          }}
        />
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Speech Provider</div><div style={styles.desc}>Explicit provider used for planner voice transcription. Leave blank to allow automatic discovery.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}>
          <select style={styles.input} value={speechProviderId} onChange={(e) => onSpeechProviderIdChange(e.target.value)}>
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
          <select style={styles.input} value={speechModelName} onChange={(e) => onSpeechModelNameChange(e.target.value)}>
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
        <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={speechLocale} onChange={(e) => onSpeechLocaleChange(e.target.value)} placeholder="en-US" /><button style={styles.btn} onClick={() => saveSetting(SPEECH_LOCALE_KEY, speechLocale)}>Save</button>{savedMsg === SPEECH_LOCALE_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
      <div style={styles.settingRow}>
        <div><div style={styles.label}>Native Speech Voice</div><div style={styles.desc}>Optional macOS `say` voice, for example `Samantha`, used for planner replies when native speech is enabled.</div></div>
        <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={speechNativeVoice} onChange={(e) => onSpeechNativeVoiceChange(e.target.value)} placeholder="Samantha" /><button style={styles.btn} onClick={() => saveSetting(SPEECH_NATIVE_VOICE_KEY, speechNativeVoice)}>Save</button>{savedMsg === SPEECH_NATIVE_VOICE_KEY && <span style={styles.saved}>Saved!</span>}</div>
      </div>
    </div>
  );
}
