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
          <div style={styles.type}>{provider.provider_type}</div>
          <div style={styles.url}>{provider.base_url}</div>
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
