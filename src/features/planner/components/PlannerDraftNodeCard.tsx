import type { PlannerDraftChildType } from "../../../lib/types";
import {
  formatDraftChildTypeLabel,
  getPlannerNodeType,
  type PlannerAction,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";
import { PlannerDraftActionList } from "./PlannerDraftActionList";

type PlannerDraftNodeCardProps = {
  selectedDraftNode: PlannerTreeNode | null;
  selectedDraftNodePath: PlannerTreeNode[];
  renameDraftName: string;
  onRenameDraftNameChange: (name: string) => void;
  onRenameSelectedDraftNode: () => void;
  onDeleteSelectedDraftNode: () => void;
  isPlannerBusy: boolean;
  allowedDraftChildTypes: PlannerDraftChildType[];
  draftChildType: PlannerDraftChildType;
  onDraftChildTypeChange: (childType: PlannerDraftChildType) => void;
  draftChildName: string;
  onDraftChildNameChange: (name: string) => void;
  draftChildSummary: string;
  onDraftChildSummaryChange: (summary: string) => void;
  onAddChildToSelectedDraftNode: () => void;
  draftEditMessage: string | null;
  draftEditError: string | null;
  selectedDraftNodePrompts: string[];
  onApplyPromptSuggestion: (prompt: string) => void;
  selectedNodeRecentActions: PlannerAction[];
};

export function PlannerDraftNodeCard({
  selectedDraftNode,
  selectedDraftNodePath,
  renameDraftName,
  onRenameDraftNameChange,
  onRenameSelectedDraftNode,
  onDeleteSelectedDraftNode,
  isPlannerBusy,
  allowedDraftChildTypes,
  draftChildType,
  onDraftChildTypeChange,
  draftChildName,
  onDraftChildNameChange,
  draftChildSummary,
  onDraftChildSummaryChange,
  onAddChildToSelectedDraftNode,
  draftEditMessage,
  draftEditError,
  selectedDraftNodePrompts,
  onApplyPromptSuggestion,
  selectedNodeRecentActions,
}: PlannerDraftNodeCardProps) {
  return (
    <div style={styles.sideCard}>
      <div style={styles.label}>Selected Node</div>
      {selectedDraftNode ? (
        <>
          <div style={styles.cardTitle}>{selectedDraftNode.label}</div>
          <div style={styles.helper}>Type: {getPlannerNodeType(selectedDraftNode)}</div>
          {selectedDraftNode.summary ? <div style={{ ...styles.helper, marginTop: 8 }}>{selectedDraftNode.summary}</div> : null}
          {selectedDraftNode.source || selectedDraftNode.confidence ? (
            <div style={styles.chipRow}>
              {selectedDraftNode.source ? <div style={styles.chip}>source: {selectedDraftNode.source.replace("_", " ")}</div> : null}
              {selectedDraftNode.confidence ? <div style={styles.chip}>{selectedDraftNode.confidence} confidence</div> : null}
            </div>
          ) : null}
          {selectedDraftNodePath.length > 0 ? (
            <div style={styles.pathText}>
              Path: {selectedDraftNodePath.map((node) => node.label).join(" / ")}
            </div>
          ) : null}
          {selectedDraftNode.evidence && selectedDraftNode.evidence.length > 0 ? (
            <>
              <div style={styles.sectionDivider} />
              <div style={styles.label}>Evidence</div>
              <div style={styles.list}>
                {selectedDraftNode.evidence.map((line) => (
                  <div key={line} style={styles.listItem}>
                    <div style={styles.listItemMeta}>{line}</div>
                  </div>
                ))}
              </div>
            </>
          ) : null}
          <DraftNodeEditor
            renameDraftName={renameDraftName}
            onRenameDraftNameChange={onRenameDraftNameChange}
            onRenameSelectedDraftNode={onRenameSelectedDraftNode}
            onDeleteSelectedDraftNode={onDeleteSelectedDraftNode}
            isPlannerBusy={isPlannerBusy}
            allowedDraftChildTypes={allowedDraftChildTypes}
            draftChildType={draftChildType}
            onDraftChildTypeChange={onDraftChildTypeChange}
            draftChildName={draftChildName}
            onDraftChildNameChange={onDraftChildNameChange}
            draftChildSummary={draftChildSummary}
            onDraftChildSummaryChange={onDraftChildSummaryChange}
            onAddChildToSelectedDraftNode={onAddChildToSelectedDraftNode}
            draftEditMessage={draftEditMessage}
            draftEditError={draftEditError}
          />
          <div style={styles.sectionDivider} />
          <div style={styles.label}>Suggested Next Prompts</div>
          <div style={styles.promptList}>
            {selectedDraftNodePrompts.map((prompt) => (
              <button key={prompt} style={styles.promptButton} onClick={() => onApplyPromptSuggestion(prompt)}>
                {prompt}
              </button>
            ))}
          </div>
          <div style={styles.sectionDivider} />
          <div style={styles.label}>Recent AI Changes For This Node</div>
          {selectedNodeRecentActions.length > 0 ? (
            <PlannerDraftActionList actions={selectedNodeRecentActions} />
          ) : (
            <div style={styles.helper}>
              No recent planner operations are directly tied to this node yet.
            </div>
          )}
        </>
      ) : (
        <div style={styles.helper}>
          Select a node in the tree to anchor follow-up planning turns to that part of the design.
        </div>
      )}
    </div>
  );
}

function DraftNodeEditor({
  renameDraftName,
  onRenameDraftNameChange,
  onRenameSelectedDraftNode,
  onDeleteSelectedDraftNode,
  isPlannerBusy,
  allowedDraftChildTypes,
  draftChildType,
  onDraftChildTypeChange,
  draftChildName,
  onDraftChildNameChange,
  draftChildSummary,
  onDraftChildSummaryChange,
  onAddChildToSelectedDraftNode,
  draftEditMessage,
  draftEditError,
}: Pick<
  PlannerDraftNodeCardProps,
  | "renameDraftName"
  | "onRenameDraftNameChange"
  | "onRenameSelectedDraftNode"
  | "onDeleteSelectedDraftNode"
  | "isPlannerBusy"
  | "allowedDraftChildTypes"
  | "draftChildType"
  | "onDraftChildTypeChange"
  | "draftChildName"
  | "onDraftChildNameChange"
  | "draftChildSummary"
  | "onDraftChildSummaryChange"
  | "onAddChildToSelectedDraftNode"
  | "draftEditMessage"
  | "draftEditError"
>) {
  return (
    <>
      <div style={styles.sectionDivider} />
      <div style={styles.label}>Edit Node</div>
      <div style={styles.fieldGroup}>
        <input
          data-testid="draft-node-rename-input"
          style={styles.input}
          value={renameDraftName}
          onChange={(event) => onRenameDraftNameChange(event.target.value)}
          placeholder="Rename this node"
        />
        <div style={styles.inlineButtonRow}>
          <button
            data-testid="draft-node-rename-save"
            style={styles.mutedButton}
            onClick={onRenameSelectedDraftNode}
            disabled={!renameDraftName.trim() || isPlannerBusy}
          >
            Save Name
          </button>
          <button
            data-testid="draft-node-delete"
            style={styles.btnDanger}
            onClick={onDeleteSelectedDraftNode}
            disabled={isPlannerBusy}
          >
            Delete Node
          </button>
        </div>
      </div>
      <div style={styles.sectionDivider} />
      <div style={styles.label}>Add Child</div>
      {allowedDraftChildTypes.length > 0 ? (
        <div style={styles.fieldGroup}>
          <select
            data-testid="draft-node-add-child-type"
            style={styles.select}
            value={draftChildType}
            onChange={(event) => onDraftChildTypeChange(event.target.value as PlannerDraftChildType)}
          >
            {allowedDraftChildTypes.map((option) => (
              <option key={option} value={option}>
                {formatDraftChildTypeLabel(option)}
              </option>
            ))}
          </select>
          <input
            data-testid="draft-node-add-child-name"
            style={styles.input}
            value={draftChildName}
            onChange={(event) => onDraftChildNameChange(event.target.value)}
            placeholder={`Name the new ${formatDraftChildTypeLabel(draftChildType).toLowerCase()}`}
          />
          <textarea
            data-testid="draft-node-add-child-summary"
            style={styles.compactTextarea}
            value={draftChildSummary}
            onChange={(event) => onDraftChildSummaryChange(event.target.value)}
            placeholder="Optional summary or brief description"
          />
          <button
            data-testid="draft-node-add-child-save"
            style={styles.btnGhost}
            onClick={onAddChildToSelectedDraftNode}
            disabled={!draftChildName.trim() || isPlannerBusy}
          >
            Add {formatDraftChildTypeLabel(draftChildType)}
          </button>
        </div>
      ) : (
        <div style={styles.helper}>
          This node is a leaf in the staged hierarchy. Use rename or delete, or select a higher branch to add more structure.
        </div>
      )}
      {draftEditMessage ? <div style={styles.success}>{draftEditMessage}</div> : null}
      {draftEditError ? <div style={styles.error}>{draftEditError}</div> : null}
    </>
  );
}
