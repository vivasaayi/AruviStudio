import type { DraftValidationSummary, PlannerTreeNode } from "../lib/plannerPageModel";
import type { ModelProvider, Product } from "../../../lib/types";
import { styles } from "../lib/plannerPageStyles";

type PlannerView = "conversation" | "draft" | "trace";

type PlannerModelPickerOption = {
  value: string;
  label: string;
};

export function PlannerHeader({
  plannerView,
  selectedProductId,
  products,
  plannerModelPickerValue,
  plannerModelPickerOptions,
  providerId,
  providers,
  modelName,
  selectedDraftNode,
  draftTreeNodesLength,
  latestTraceEventsLength,
  plannerStatusSummary,
  isCompactScreen,
  showCompactTools,
  draftValidation,
  pendingVoiceTranscript,
  isPlannerBusy,
  hasPendingPlan,
  onOpenRepositoryModal,
  onProductChange,
  onCreateProduct,
  onPlannerModelChange,
  onPlannerViewChange,
  onToggleCompactTools,
}: {
  plannerView: PlannerView;
  selectedProductId: string | null;
  products: Product[];
  plannerModelPickerValue: string;
  plannerModelPickerOptions: PlannerModelPickerOption[];
  providerId: string;
  providers: ModelProvider[];
  modelName: string;
  selectedDraftNode: PlannerTreeNode | null;
  draftTreeNodesLength: number;
  latestTraceEventsLength: number;
  plannerStatusSummary: { title: string; detail: string };
  isCompactScreen: boolean;
  showCompactTools: boolean;
  draftValidation: DraftValidationSummary;
  pendingVoiceTranscript: string | null;
  isPlannerBusy: boolean;
  hasPendingPlan: boolean;
  onOpenRepositoryModal: () => void;
  onProductChange: (productId: string | null) => void;
  onCreateProduct: () => void;
  onPlannerModelChange: (providerId: string, modelName: string) => void;
  onPlannerViewChange: (view: PlannerView) => void;
  onToggleCompactTools: () => void;
}) {
  return (
    <>
      <div style={styles.sectionHeader}>
        <div style={styles.sectionTitle}>
          {plannerView === "draft" ? "Design Review" : plannerView === "trace" ? "Planner Trace" : "Conversation"}
        </div>
        <div style={styles.viewToggleRow}>
          <button
            aria-label="Reverse engineer repository"
            style={styles.iconButton}
            onClick={onOpenRepositoryModal}
          >
            ⌕ Repo
          </button>
          <select
            aria-label="Planner product"
            style={{ ...styles.select, width: 240 }}
            value={selectedProductId ?? ""}
            onChange={(event) => onProductChange(event.target.value || null)}
          >
            <option value="">Select product</option>
            {products.map((product) => (
              <option key={product.id} value={product.id}>
                {product.name}
              </option>
            ))}
          </select>
          <button style={styles.btnGhost} onClick={onCreateProduct}>
            Create Product
          </button>
          <select
            aria-label="Planner model"
            style={{ ...styles.select, width: 260 }}
            value={plannerModelPickerValue}
            onChange={(event) => {
              const [nextProviderId, nextModelName] = event.target.value.split("::");
              onPlannerModelChange(nextProviderId ?? "", nextModelName ?? "");
            }}
          >
            <option value="">Select model</option>
            {plannerModelPickerOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button
            data-testid="planner-view-conversation"
            style={plannerView === "conversation" ? styles.btn : styles.btnGhost}
            onClick={() => onPlannerViewChange("conversation")}
          >
            Conversation
          </button>
          <button
            data-testid="planner-view-draft"
            style={plannerView === "draft" ? styles.btn : styles.btnGhost}
            onClick={() => onPlannerViewChange("draft")}
            disabled={draftTreeNodesLength === 0}
          >
            View Design
          </button>
          <button
            data-testid="planner-view-trace"
            style={plannerView === "trace" ? styles.btn : styles.btnGhost}
            onClick={() => onPlannerViewChange("trace")}
            disabled={latestTraceEventsLength === 0}
          >
            View Trace
          </button>
        </div>
      </div>

      <div style={styles.statusBanner}>
        <div>
          <div style={styles.statusBannerStrong}>{plannerStatusSummary.title}</div>
          <div style={styles.statusBannerMeta}>{plannerStatusSummary.detail}</div>
        </div>
        <div style={styles.chipRow}>
          {providerId ? <div style={styles.chip}>{providers.find((provider) => provider.id === providerId)?.name ?? "provider selected"}</div> : null}
          {modelName ? <div style={styles.chip}>{modelName}</div> : null}
          {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
        </div>
      </div>

      {isCompactScreen && plannerView === "conversation" ? (
        <>
          <div style={styles.compactControlStrip}>
            <button style={styles.btnGhost} onClick={onToggleCompactTools}>
              {showCompactTools ? "Hide Tools" : "Show Tools"}
            </button>
            <button style={styles.btnGhost} onClick={() => onPlannerViewChange("draft")} disabled={draftTreeNodesLength === 0}>
              Open Design
            </button>
            <button style={styles.btnGhost} onClick={() => onPlannerViewChange("trace")} disabled={latestTraceEventsLength === 0}>
              Open Trace
            </button>
          </div>
          <div style={styles.compactSummaryCard}>
            <div style={styles.compactSummaryGrid}>
              <div style={styles.compactSummaryItem}>
                <div style={styles.compactSummaryLabel}>Design</div>
                <div style={styles.compactSummaryValue}>{draftTreeNodesLength > 0 ? `${draftValidation.counts["product area"]} product areas staged` : "No active design"}</div>
              </div>
              <div style={styles.compactSummaryItem}>
                <div style={styles.compactSummaryLabel}>Selection</div>
                <div style={styles.compactSummaryValue}>{selectedDraftNode?.label ?? "None"}</div>
              </div>
              <div style={styles.compactSummaryItem}>
                <div style={styles.compactSummaryLabel}>Readiness</div>
                <div style={styles.compactSummaryValue}>{draftTreeNodesLength > 0 ? `${draftValidation.score}` : "n/a"}</div>
              </div>
              <div style={styles.compactSummaryItem}>
                <div style={styles.compactSummaryLabel}>State</div>
                <div style={styles.compactSummaryValue}>
                  {pendingVoiceTranscript ? "Review transcript" : isPlannerBusy ? "Working" : hasPendingPlan ? "Need confirm" : "Ready"}
                </div>
              </div>
            </div>
          </div>
        </>
      ) : null}
    </>
  );
}
