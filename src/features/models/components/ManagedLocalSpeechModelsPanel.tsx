import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { MANAGED_LOCAL_MODELS } from "../lib/modelProviderConstants";
import { styles } from "../lib/modelProviderPageStyles";

type ManagedLocalModel = (typeof MANAGED_LOCAL_MODELS)[number];

interface ManagedLocalSpeechModelsPanelProps {
  providers: ModelProvider[];
  modelDefinitions: ModelDefinition[];
  isBusy: boolean;
  isInstallPending: boolean;
  onInstall: (entry: ManagedLocalModel) => void;
  onRegister: (entry: ManagedLocalModel) => void;
  onUseForSpeech: (providerId: string, modelName: string) => void;
}

export function ManagedLocalSpeechModelsPanel({
  providers,
  modelDefinitions,
  isBusy,
  isInstallPending,
  onInstall,
  onRegister,
  onUseForSpeech,
}: ManagedLocalSpeechModelsPanelProps) {
  return (
    <div style={{ ...styles.form, marginBottom: 18 }}>
      <div style={{ ...styles.title, fontSize: 16, marginBottom: 10 }}>Managed Local Speech Models</div>
      <div style={{ color: "#9aa0a6", fontSize: 13, lineHeight: 1.5, marginBottom: 14 }}>
        Install Whisper models into AruviStudio-managed storage, or register an existing local model file without typing absolute paths by hand.
      </div>
      <div style={styles.managedGrid}>
        {MANAGED_LOCAL_MODELS.map((entry) => {
          const installedProvider = providers.find((provider) => provider.name === entry.providerName);
          const installedModel = modelDefinitions.find(
            (model) => model.provider_id === installedProvider?.id && model.name === entry.modelName,
          );
          const isInstalled = Boolean(installedProvider && installedModel);

          return (
            <div key={entry.id} style={styles.managedCard}>
              <div style={styles.managedTitle}>{entry.displayName}</div>
              <div style={styles.managedBadgeRow}>
                <span style={styles.managedBadge}>{entry.sizeLabel}</span>
                <span style={styles.managedBadge}>speech_to_text</span>
                <span style={styles.managedBadge}>{isInstalled ? "installed" : "not installed"}</span>
              </div>
              <div style={styles.managedMeta}>{entry.notes}</div>
              {installedProvider ? (
                <div style={{ ...styles.managedMeta, fontFamily: "monospace" }}>
                  Provider path: {installedProvider.base_url}
                </div>
              ) : null}
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
                <button
                  style={styles.btn}
                  onClick={() => onInstall(entry)}
                  disabled={isBusy}
                >
                  {isInstallPending ? "Installing..." : isInstalled ? "Reinstall / Reuse" : "Install & Register"}
                </button>
                <button
                  style={{ ...styles.btn, backgroundColor: "#2c3139" }}
                  onClick={() => onRegister(entry)}
                  disabled={isBusy}
                >
                  Use Existing File
                </button>
                {installedProvider && installedModel ? (
                  <button
                    style={{ ...styles.btn, backgroundColor: "#355c2b" }}
                    onClick={() => onUseForSpeech(installedProvider.id, installedModel.name)}
                    disabled={isBusy}
                  >
                    Use for Speech
                  </button>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
      <div style={{ color: "#8f96a3", fontSize: 12, marginTop: 12 }}>
        This release manages local Whisper models for speech. Local in-app hosting for GGUF chat/coding models is the next runtime step after speech.
      </div>
    </div>
  );
}
