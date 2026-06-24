import type { Repository } from "../../../lib/types";
import { styles } from "../lib/plannerPageStyles";

export function PlannerRepositoryModal({
  repositories,
  selectedRepositoryId,
  repositoryPathDraft,
  isProductSelected,
  isPlannerBusy,
  hasPlannerModel,
  repoAnalysisMessage,
  repoAnalysisError,
  onClose,
  onSelectedRepositoryIdChange,
  onRepositoryPathDraftChange,
  onBrowseRepositoryPath,
  onRegisterRepository,
  onAnalyzeRepository,
}: {
  repositories: Repository[];
  selectedRepositoryId: string;
  repositoryPathDraft: string;
  isProductSelected: boolean;
  isPlannerBusy: boolean;
  hasPlannerModel: boolean;
  repoAnalysisMessage: string | null;
  repoAnalysisError: string | null;
  onClose: () => void;
  onSelectedRepositoryIdChange: (repositoryId: string) => void;
  onRepositoryPathDraftChange: (repositoryPath: string) => void;
  onBrowseRepositoryPath: () => void;
  onRegisterRepository: () => void;
  onAnalyzeRepository: () => void;
}) {
  return (
    <div style={styles.modalOverlay} onClick={onClose}>
      <div style={styles.modalCard} onClick={(event) => event.stopPropagation()}>
        <div style={styles.modalHeader}>
          <div>
            <div style={styles.modalTitle}>Reverse Engineer Repository</div>
            <div style={styles.helper}>
              Point the planner at an existing repository and let the model infer a staged product area, capability, feature, story, and task tree from the codebase.
            </div>
          </div>
          <button style={styles.btnGhost} onClick={onClose}>
            Close
          </button>
        </div>
        <label style={styles.label}>Registered Repository</label>
        <select
          style={styles.select}
          value={selectedRepositoryId}
          onChange={(event) => onSelectedRepositoryIdChange(event.target.value)}
        >
          <option value="">Select a repository</option>
          {repositories.map((repository) => (
            <option key={repository.id} value={repository.id}>
              {repository.name}
            </option>
          ))}
        </select>
        <div style={{ height: 10 }} />
        <label style={styles.label}>Add Existing Repo Path</label>
        <input
          style={styles.input}
          value={repositoryPathDraft}
          onChange={(event) => onRepositoryPathDraftChange(event.target.value)}
          placeholder="/absolute/path/to/repository"
        />
        <div style={styles.inlineButtonRow}>
          <button style={styles.btnGhost} onClick={onBrowseRepositoryPath}>
            Browse Path
          </button>
          <button
            style={styles.btnGhost}
            onClick={onRegisterRepository}
            disabled={!repositoryPathDraft.trim()}
          >
            Register Repo
          </button>
          <button
            style={styles.btn}
            onClick={onAnalyzeRepository}
            disabled={!selectedRepositoryId || !isProductSelected || isPlannerBusy || !hasPlannerModel}
          >
            Analyze Repo Into Design
          </button>
        </div>
        {!isProductSelected ? (
          <div style={{ ...styles.helper, marginTop: 10 }}>
            Select a product in the Planner toolbar before analyzing a repository.
          </div>
        ) : null}
        {!hasPlannerModel ? (
          <div style={{ ...styles.helper, marginTop: 10 }}>
            Configure a planner model first. Repository reverse engineering depends on the selected LLM.
          </div>
        ) : null}
        {repoAnalysisMessage ? <div style={{ ...styles.success, marginTop: 10 }}>{repoAnalysisMessage}</div> : null}
        {repoAnalysisError ? <div style={{ ...styles.error, marginTop: 10 }}>{repoAnalysisError}</div> : null}
      </div>
    </div>
  );
}
