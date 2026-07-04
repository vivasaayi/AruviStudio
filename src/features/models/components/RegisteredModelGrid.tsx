import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { styles } from "../lib/modelProviderPageStyles";

interface RegisteredModelGridProps {
  models: ModelDefinition[];
  visibleProviders: ModelProvider[];
  providers: ModelProvider[];
  onEditModel: (model: ModelDefinition) => void;
}

export function RegisteredModelGrid({
  models,
  visibleProviders,
  providers,
  onEditModel,
}: RegisteredModelGridProps) {
  return (
    <div style={{ ...styles.form, marginTop: 18 }}>
      <div style={styles.sectionTitle}>Registered Models</div>
      {models.length > 0 ? (
        <div style={styles.subGrid}>
          {models.map((model) => {
            const provider = visibleProviders.find((entry) => entry.id === model.provider_id)
              ?? providers.find((entry) => entry.id === model.provider_id);
            return (
              <div key={model.id} style={styles.modelCard}>
                <div style={styles.modelName}>{model.name}</div>
                <div style={styles.modelMeta}>{provider?.name ?? "Unknown provider"}</div>
                <div style={styles.modelMeta}>Context: {model.context_window ?? "not set"}</div>
                <div style={styles.modelMeta}>Tags: {model.capability_tags.length > 0 ? model.capability_tags.join(", ") : "none"}</div>
                {model.notes ? <div style={styles.modelMeta}>{model.notes}</div> : null}
                <span style={{ ...styles.badge, backgroundColor: model.enabled ? "#1b3a2d" : "#3a1b1b", color: model.enabled ? "#4ec9b0" : "#f44747" }}>
                  {model.enabled ? "Enabled" : "Disabled"}
                </span>
                <div style={{ marginTop: 8 }}>
                  <button
                    style={{ ...styles.btnTest, backgroundColor: "#2c3139" }}
                    onClick={() => onEditModel(model)}
                  >
                    Edit
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div style={styles.empty}>No model definitions yet. Add one so agents can be bound to a concrete model.</div>
      )}
    </div>
  );
}
