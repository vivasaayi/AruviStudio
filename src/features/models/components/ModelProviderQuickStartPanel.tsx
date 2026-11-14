import { styles } from "../lib/modelProviderPageStyles";

interface ModelProviderQuickStartPanelProps {
  onApplyPreset: (preset: "deepseek" | "lm_studio" | "model_host") => void;
}

export function ModelProviderQuickStartPanel({ onApplyPreset }: ModelProviderQuickStartPanelProps) {
  return (
    <div style={{ ...styles.form, marginBottom: 18 }}>
      <div style={{ ...styles.title, fontSize: 16, marginBottom: 10 }}>Quick Start</div>
      <div style={{ color: "#9aa0a6", fontSize: 13, lineHeight: 1.5, marginBottom: 14 }}>
        Connect Aruvi Model Host for centrally hosted local coding models, or use another compatible provider. Managed Whisper installs below support local desktop transcription.
      </div>
      <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
        <button style={styles.btn} onClick={() => onApplyPreset("model_host")}>Use Aruvi Model Host</button>
        <button style={styles.btn} onClick={() => onApplyPreset("deepseek")}>Use DeepSeek Hosted</button>
        <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => onApplyPreset("lm_studio")}>Use LM Studio Local</button>
      </div>
    </div>
  );
}
