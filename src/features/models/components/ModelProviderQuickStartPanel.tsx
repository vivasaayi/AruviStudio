import { styles } from "../lib/modelProviderPageStyles";

interface ModelProviderQuickStartPanelProps {
  onApplyPreset: (preset: "deepseek" | "lm_studio") => void;
}

export function ModelProviderQuickStartPanel({ onApplyPreset }: ModelProviderQuickStartPanelProps) {
  return (
    <div style={{ ...styles.form, marginBottom: 18 }}>
      <div style={{ ...styles.title, fontSize: 16, marginBottom: 10 }}>Quick Start</div>
      <div style={{ color: "#9aa0a6", fontSize: 13, lineHeight: 1.5, marginBottom: 14 }}>
        Use DeepSeek hosted first if you want a quick end-to-end path. For fully local desktop voice, use the managed Whisper installs below instead of creating manual placeholder providers.
      </div>
      <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
        <button style={styles.btn} onClick={() => onApplyPreset("deepseek")}>Use DeepSeek Hosted</button>
        <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => onApplyPreset("lm_studio")}>Use LM Studio Local</button>
      </div>
    </div>
  );
}
