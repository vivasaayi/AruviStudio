import { useEffect, useState } from "react";
import { getSetting, setSetting } from "../../../lib/tauri";
import type { ModelProvider } from "../../../lib/types";
import { styles } from "../lib/modelProviderPageStyles";

interface TestResult {
  status: string;
  message: string;
}

interface ModelProviderGridProps {
  providers: ModelProvider[];
  testResults: Record<string, TestResult>;
  onEditProvider: (provider: ModelProvider) => void;
  onTestProvider: (providerId: string) => void;
}

function StreamUsageSetting({ provider }: { provider: ModelProvider }) {
  const [value, setValue] = useState(false);
  const [available, setAvailable] = useState(false);
  const [message, setMessage] = useState("");
  const key = `model.provider.${provider.id}.stream_usage`;
  useEffect(() => {
    let active = true;
    setAvailable(false);
    getSetting(key).then((raw) => {
      if (!active) return;
      if (raw !== null && raw !== "true" && raw !== "false") throw new Error("Invalid stream usage setting");
      setValue(raw === "true"); setAvailable(true);
    }).catch((error) => { if (active) setMessage(String(error)); });
    return () => { active = false; };
  }, [key]);
  async function update(next: boolean) {
    setAvailable(false); setMessage("");
    try { await setSetting(key, String(next)); setValue(next); }
    catch (error) { setMessage(String(error)); }
    finally { setAvailable(true); }
  }
  if (provider.provider_type !== "openai_compatible" || provider.protocol === "anthropic") return null;
  return <div><label><input type="checkbox" aria-label={`Request streaming usage for ${provider.name}`} checked={value} disabled={!available} onChange={(event) => void update(event.target.checked)} /> Request streaming usage if supported</label>{message && <div role="alert">{message}</div>}</div>;
}

function TranscriptionLanguageSetting({ provider }: { provider: ModelProvider }) {
  const [value, setValue] = useState("auto");
  const [available, setAvailable] = useState(false);
  const [message, setMessage] = useState("");
  const key = `model.provider.${provider.id}.transcription_language_mode`;
  useEffect(() => {
    let active = true;
    setAvailable(false);
    getSetting(key).then((raw) => {
      if (!active) return;
      if (raw !== null && !["auto", "singular", "array"].includes(raw)) throw new Error("Invalid transcription language mode");
      setValue(raw ?? "auto"); setAvailable(true);
    }).catch((error) => { if (active) setMessage(String(error)); });
    return () => { active = false; };
  }, [key]);
  async function update(next: string) {
    setAvailable(false); setMessage("");
    try { await setSetting(key, next); setValue(next); }
    catch (error) { setMessage(String(error)); }
    finally { setAvailable(true); }
  }
  if (provider.provider_type !== "openai_compatible" || provider.protocol === "anthropic") return null;
  return <div><label>Transcription language field <select aria-label={`Transcription language field for ${provider.name}`} value={value} disabled={!available} onChange={(event) => void update(event.target.value)}><option value="auto">Automatic</option><option value="singular">language</option><option value="array">languages[]</option></select></label>{message && <div role="alert">{message}</div>}</div>;
}

export function ModelProviderGrid({
  providers,
  testResults,
  onEditProvider,
  onTestProvider,
}: ModelProviderGridProps) {
  return (
    <div style={styles.grid}>
      {providers.map((provider) => (
        <div key={provider.id} style={styles.card}>
          <div style={styles.name}>{provider.name}</div>
          <div style={styles.type}>{provider.provider_type === "local_runtime" ? provider.provider_type : (provider.protocol ?? provider.provider_type)}</div>
          <div style={styles.url}>{provider.base_url}</div>
          <StreamUsageSetting provider={provider} />
          <TranscriptionLanguageSetting provider={provider} />
          <div style={styles.cardFooter}>
            <span style={{ ...styles.badge, backgroundColor: provider.enabled ? "#1b3a2d" : "#3a1b1b", color: provider.enabled ? "#4ec9b0" : "#f44747" }}>
              {provider.enabled ? "Enabled" : "Disabled"}
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              <button style={{ ...styles.btnTest, backgroundColor: "#2c3139" }} onClick={() => onEditProvider(provider)}>Edit</button>
              <button style={styles.btnTest} onClick={() => onTestProvider(provider.id)}>Test Connection</button>
            </div>
          </div>
          {testResults[provider.id] ? (
            <div
              style={{
                ...styles.testResult,
                backgroundColor: testResults[provider.id].status === "success" ? "#1b3a2d" : testResults[provider.id].status === "error" ? "#3a1b1b" : "#2a2a2a",
                color: testResults[provider.id].status === "success" ? "#4ec9b0" : testResults[provider.id].status === "error" ? "#f44747" : "#888",
              }}
            >
              {testResults[provider.id].message}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
