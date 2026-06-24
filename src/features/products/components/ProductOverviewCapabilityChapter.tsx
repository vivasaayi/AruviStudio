import { useMemo, useState } from "react";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, CapabilityTree, Product, ProductReference, WorkItem } from "../../../lib/types";
import {
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
} from "../lib/productOverview";
import { getCapabilityReferenceScope, filterReferencesForScope } from "../lib/productReferences";
import { getCapabilityScopedWorkItems } from "../lib/productOverviewScopedWorkItems";
import { MetricPills, WorkItemTree } from "./ProductOverviewDelivery";
import { styles } from "./ProductOverviewDocument.styles";
import type { ProductOverviewPlannerAction } from "./ProductOverviewDocument";
import { ReferenceList } from "./ProductOverviewReferences";

type CapabilityChapterProps = {
  product: Product;
  path: string[];
  capabilityTree: CapabilityTree;
  numbering: string;
  allWorkItems: WorkItem[];
  references: ProductReference[];
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
};

export function CapabilityChapter({
  product,
  path,
  capabilityTree,
  numbering,
  allWorkItems,
  references,
  onEditCapability,
  onOpenWorkItem,
  onPlanFromItem,
}: CapabilityChapterProps) {
  const [isOpen, setIsOpen] = useState(false);
  const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
  const directWorkItems = useMemo(
    () => buildScopedWorkItemTree(allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id)),
    [allWorkItems, capabilityTree.capability.id],
  );
  const metrics = useMemo(() => buildWorkItemMetrics(getCapabilityScopedWorkItems(capabilityTree, allWorkItems)), [allWorkItems, capabilityTree]);
  const capabilityReferences = useMemo(
    () => filterReferencesForScope(references, getCapabilityReferenceScope(capabilityTree.capability)),
    [capabilityTree.capability, references],
  );

  return (
    <div id={getCapabilitySectionId(capabilityTree.capability)} style={styles.detailsShell}>
      <div
        style={styles.summary}
        role="button"
        tabIndex={0}
        aria-expanded={isOpen}
        onClick={() => setIsOpen((current) => !current)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            setIsOpen((current) => !current);
          }
        }}
      >
        <span style={styles.summaryToggle}>{isOpen ? "▾" : "▸"}</span>
        <div style={styles.summaryLeft}>
          <div style={styles.chapterLabel}>{capabilityType} {numbering}</div>
          <h4 style={{ ...styles.chapterTitle, fontSize: 19 }}>{capabilityTree.capability.name}</h4>
          <div style={styles.chapterSubtitle}>
            {capabilityTree.capability.description || `Document what this ${capabilityType.toLowerCase()} is responsible for.`}
          </div>
          <div style={styles.pathText}>{[...path, capabilityTree.capability.name].join(" / ")}</div>
        </div>
        <div style={styles.summaryRight}>
          <div style={styles.summaryPillRow}>
            <span style={styles.summaryPill}>{capabilityTree.capability.status.replace(/_/g, " ")}</span>
            <span style={styles.summaryPill}>{capabilityTree.capability.priority} priority</span>
            <span style={styles.summaryPill}>{capabilityTree.capability.risk} risk</span>
            <MetricPills metrics={metrics} />
          </div>
          <div style={styles.nodeActionRow}>
            <button
              style={styles.plannerButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlanFromItem({ kind: "enhance_capability", product, productAreaName: path[1] ?? "", capability: capabilityTree.capability });
              }}
            >
              Improve Section
            </button>
            <button
              style={styles.subtleButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlanFromItem({ kind: "add_capability_child", product, productAreaName: path[1] ?? "", capability: capabilityTree.capability });
              }}
            >
              Add Child Section
            </button>
            <button
              style={styles.subtleButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlanFromItem({ kind: "add_capability_work_item", product, productAreaName: path[1] ?? "", capability: capabilityTree.capability });
              }}
            >
              Add Story
            </button>
            <button
              style={styles.subtleButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onEditCapability(capabilityTree.capability);
              }}
            >
              Edit {capabilityType}
            </button>
          </div>
        </div>
      </div>

      {isOpen ? (
        <div style={styles.detailsBody}>
          {capabilityTree.capability.acceptance_criteria
            || capabilityTree.capability.technical_notes
            || capabilityTree.capability.explanation
            || capabilityTree.capability.examples
            || capabilityTree.capability.implementation_notes
            || capabilityTree.capability.test_guidance ? (
            <div style={styles.noteGrid}>
              {capabilityTree.capability.acceptance_criteria ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Acceptance Criteria</div>
                  <div style={styles.noteText}>{capabilityTree.capability.acceptance_criteria}</div>
                </div>
              ) : null}
              {capabilityTree.capability.explanation ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Explanation</div>
                  <div style={styles.noteText}>{capabilityTree.capability.explanation}</div>
                </div>
              ) : null}
              {capabilityTree.capability.examples ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Examples</div>
                  <div style={styles.noteText}>{capabilityTree.capability.examples}</div>
                </div>
              ) : null}
              {capabilityTree.capability.technical_notes ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Technical Notes</div>
                  <div style={styles.noteText}>{capabilityTree.capability.technical_notes}</div>
                </div>
              ) : null}
              {capabilityTree.capability.implementation_notes ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Implementation Notes</div>
                  <div style={styles.noteText}>{capabilityTree.capability.implementation_notes}</div>
                </div>
              ) : null}
              {capabilityTree.capability.test_guidance ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Test Guidance</div>
                  <div style={styles.noteText}>{capabilityTree.capability.test_guidance}</div>
                </div>
              ) : null}
            </div>
          ) : (
            <div style={styles.empty}>No chapter guidance recorded yet.</div>
          )}

          <ReferenceList references={capabilityReferences} title={`${capabilityType} References`} />

          {directWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Delivery Stories</div>
              <WorkItemTree product={product} nodes={directWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
            </div>
          ) : (
            <div style={styles.empty}>No stories attached to this {capabilityType.toLowerCase()} yet.</div>
          )}

          {capabilityTree.children.length > 0 ? (
            <div style={styles.nested}>
              {capabilityTree.children.map((child, index) => (
                <CapabilityChapter
                  key={child.capability.id}
                  product={product}
                  path={[...path, capabilityTree.capability.name]}
                  capabilityTree={child}
                  numbering={`${numbering}.${index + 1}`}
                  allWorkItems={allWorkItems}
                  references={references}
                  onEditCapability={onEditCapability}
                  onOpenWorkItem={onOpenWorkItem}
                  onPlanFromItem={onPlanFromItem}
                />
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
