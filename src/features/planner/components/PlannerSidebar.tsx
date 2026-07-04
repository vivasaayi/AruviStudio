import {
  PLANNER_WORK_ITEM_PAGE_SIZE,
  SelectableTreeNodeView,
  type PendingPlan,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";

type PlannerSidebarProps = {
  isCompactScreen: boolean;
  hasTreeData: boolean;
  plannerWorkItemsHasMore: boolean;
  draftTreeNodes: PlannerTreeNode[];
  selectedDraftNodeId: string | null;
  onSelectDraftNode: (nodeId: string) => void;
  expandedDraftNodeIdSet: Set<string>;
  onToggleDraftNodeExpanded: (nodeId: string) => void;
  pendingPlan: PendingPlan | null;
};

export function PlannerSidebar({
  isCompactScreen,
  hasTreeData,
  plannerWorkItemsHasMore,
  draftTreeNodes,
  selectedDraftNodeId,
  onSelectDraftNode,
  expandedDraftNodeIdSet,
  onToggleDraftNodeExpanded,
  pendingPlan,
}: PlannerSidebarProps) {
  return (
    <div style={styles.panel}>
      <div style={isCompactScreen ? styles.compactPanelBody : styles.panelBody}>
        <div style={styles.sectionTitle}>Planner Controls</div>
        <div style={styles.sideCard}>
          <div style={styles.helper}>
            {hasTreeData ? "Product area context is loaded. Full capability trees load only for packet export or backend planner actions." : "Product area context will activate once product structure finishes loading."}
          </div>
          {plannerWorkItemsHasMore ? (
            <div style={{ ...styles.warning, marginTop: 8 }}>
              Planner context is showing the first {PLANNER_WORK_ITEM_PAGE_SIZE} story/task items. Use Work Items for full paged delivery browsing.
            </div>
          ) : null}
        </div>

        <div style={styles.sideCard}>
          <div style={styles.label}>Design Tree</div>
          <div style={styles.helper}>
            Build the plan here first. Select a node, then ask follow-up questions like "expand this capability" or "add stories under this feature."
          </div>
          <div style={{ height: 10 }} />
          {draftTreeNodes.length > 0 ? (
            <div style={styles.treePanel}>
              <div style={styles.treeExplorer}>
                {draftTreeNodes.map((node) => (
                  <SelectableTreeNodeView
                    key={node.id}
                    node={node}
                    selectedNodeId={selectedDraftNodeId}
                    onSelect={onSelectDraftNode}
                    expandedNodeIds={expandedDraftNodeIdSet}
                    onToggle={onToggleDraftNodeExpanded}
                  />
                ))}
              </div>
            </div>
          ) : (
            <div style={styles.helper}>No staged design yet. Select a product, then ask the planner to design product areas, capabilities, features, and starter stories inside it.</div>
          )}
        </div>

        {pendingPlan || draftTreeNodes.length > 0 ? (
          <div style={styles.sideCard}>
            <div style={styles.label}>Design Snapshot</div>
            <div style={styles.helper}>
              The planner stages structure here first. Generate a review packet, keep refining the tree, then apply when the design looks right.
            </div>
            {pendingPlan ? (
              <div style={styles.list}>
                {pendingPlan.plan.actions.map((action, index) => (
                  <div key={`${action.type}-${index}`} style={styles.listItem}>
                    <div style={styles.listItemTitle}>{action.type}</div>
                    <div style={styles.listItemMeta}>{JSON.stringify(action, null, 2)}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ ...styles.helper, marginTop: 10 }}>
                The current staged design is active in the tree above. Select a node and keep iterating, or apply it when approved.
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
