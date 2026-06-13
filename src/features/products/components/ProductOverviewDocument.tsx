import React, { useMemo, useState } from "react";
import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, CapabilityTree, Module, ModuleTree, Product, ProductTree, WorkItem } from "../../../lib/types";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getModuleSectionId,
  getWorkItemPresentation,
  sortWorkItems,
  type WorkItemMetrics,
  type WorkItemNode,
} from "../lib/productOverview";

const styles: Record<string, React.CSSProperties> = {
  layout: { display: "block" },
  article: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 18 },
  hero: {
    borderRadius: 16,
    border: "1px solid #bfdbfe",
    background: "linear-gradient(145deg, #eff6ff 0%, #ffffff 68%, #f8fafc 100%)",
    padding: 18,
  },
  eyebrow: { fontSize: 11, fontWeight: 800, letterSpacing: "0.14em", textTransform: "uppercase" as const, color: "#2563eb", marginBottom: 8 },
  heroTop: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 12 },
  title: { fontSize: 26, fontWeight: 900, color: "#0f172a", margin: 0, lineHeight: 1.05 },
  subtitle: { fontSize: 13, color: "#475569", lineHeight: 1.5, margin: 0 },
  prose: { fontSize: 13, color: "#334155", lineHeight: 1.55, whiteSpace: "pre-wrap" as const },
  button: { padding: "7px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#f8fafc", color: "#1e3a8a", border: "1px solid #93c5fd", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  plannerButton: { padding: "7px 12px", fontSize: 12, fontWeight: 800, backgroundColor: "#2563eb", color: "#ffffff", border: "1px solid #1d4ed8", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  subtleButton: { padding: "6px 10px", fontSize: 12, fontWeight: 700, backgroundColor: "#eff6ff", color: "#1d4ed8", border: "1px solid #93c5fd", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  toggleButton: { padding: "7px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#eff6ff", color: "#1d4ed8", border: "1px solid #93c5fd", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  progressPanel: { marginTop: 12, borderRadius: 12, border: "1px solid #dbeafe", backgroundColor: "#ffffff", padding: 10 },
  progressRow: { display: "flex", justifyContent: "space-between", gap: 16, fontSize: 12, color: "#475569" },
  progressTrack: { width: "100%", height: 10, borderRadius: 999, backgroundColor: "#e2e8f0", overflow: "hidden", marginTop: 10 },
  progressFill: { height: "100%", borderRadius: 999, background: "linear-gradient(90deg, #16a34a 0%, #22c55e 100%)" },
  metricGrid: { display: "grid", gridTemplateColumns: "repeat(6, minmax(0, 1fr))", gap: 8, marginTop: 12 },
  metricCard: { borderRadius: 10, padding: 10, backgroundColor: "#ffffff", border: "1px solid #dbeafe" },
  metricLabel: { fontSize: 11, fontWeight: 800, color: "#64748b", textTransform: "uppercase" as const, letterSpacing: "0.08em" },
  metricValue: { fontSize: 20, fontWeight: 900, color: "#0f172a", marginTop: 4 },
  summaryGrid: { display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: 10 },
  summaryCard: { borderRadius: 12, border: "1px solid #d8dee8", backgroundColor: "#ffffff", padding: 12 },
  sectionTitle: { fontSize: 11, fontWeight: 800, letterSpacing: "0.1em", textTransform: "uppercase" as const, color: "#64748b", marginBottom: 10 },
  summaryHeading: { fontSize: 16, fontWeight: 800, color: "#111827", margin: "0 0 6px" },
  list: { margin: 0, paddingLeft: 18, color: "#334155", display: "flex", flexDirection: "column", gap: 8, lineHeight: 1.65 },
  chipRow: { display: "flex", gap: 8, flexWrap: "wrap" as const },
  chip: { fontSize: 11, padding: "4px 8px", borderRadius: 999, backgroundColor: "#eff6ff", color: "#1d4ed8" },
  section: { borderRadius: 12, border: "1px solid #d8dee8", backgroundColor: "#ffffff", padding: 14 },
  sectionHeader: { display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 },
  sectionHeading: { fontSize: 20, fontWeight: 900, color: "#111827", margin: 0 },
  sectionSubtitle: { fontSize: 12, color: "#64748b", lineHeight: 1.45, margin: 0 },
  empty: { fontSize: 13, color: "#64748b", fontStyle: "italic" as const, lineHeight: 1.6 },
  detailsShell: { borderRadius: 12, border: "1px solid #d8dee8", backgroundColor: "#ffffff", overflow: "hidden" },
  summary: { padding: 14, display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12, cursor: "pointer", listStyle: "none" as const },
  summaryLeft: { minWidth: 0 },
  summaryRight: { display: "flex", gap: 8, flexWrap: "wrap" as const, justifyContent: "flex-end" as const },
  chapterLabel: { fontSize: 11, fontWeight: 800, color: "#2563eb", letterSpacing: "0.14em", textTransform: "uppercase" as const, marginBottom: 6 },
  chapterTitle: { fontSize: 20, fontWeight: 900, color: "#111827", margin: 0, lineHeight: 1.15 },
  chapterSubtitle: { fontSize: 12, color: "#475569", lineHeight: 1.45, marginTop: 6 },
  summaryPill: { display: "inline-flex", alignItems: "center", borderRadius: 999, padding: "5px 9px", fontSize: 11, fontWeight: 700, backgroundColor: "#f8fafc", border: "1px solid #cbd5e1", color: "#475569" },
  statePill: { display: "inline-flex", alignItems: "center", borderRadius: 999, padding: "5px 9px", fontSize: 11, fontWeight: 800, border: "1px solid currentColor", backgroundColor: "#f8fafc" },
  detailsBody: { padding: "0 14px 14px", display: "flex", flexDirection: "column", gap: 12 },
  nodeActionRow: { display: "flex", justifyContent: "flex-end", gap: 8, flexWrap: "wrap" as const },
  noteGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 10 },
  noteCard: { borderRadius: 10, border: "1px solid #d8dee8", backgroundColor: "#f8fafc", padding: 12 },
  noteHeading: { fontSize: 11, fontWeight: 800, letterSpacing: "0.08em", textTransform: "uppercase" as const, color: "#64748b", marginBottom: 8 },
  noteText: { fontSize: 12, color: "#334155", lineHeight: 1.5, whiteSpace: "pre-wrap" as const },
  metaRow: { display: "flex", gap: 8, flexWrap: "wrap" as const },
  metaPill: { fontSize: 11, padding: "4px 8px", borderRadius: 999, backgroundColor: "#eff6ff", color: "#1d4ed8" },
  pathText: { fontSize: 12, color: "#64748b", lineHeight: 1.6, marginTop: 6 },
  nested: { marginLeft: 10, paddingLeft: 22, borderLeft: "2px solid #cbd5e1", display: "flex", flexDirection: "column", gap: 16 },
  workItemList: { display: "flex", flexDirection: "column", gap: 10 },
  workItemCard: { borderRadius: 12, border: "1px solid #d8dee8", padding: 12, cursor: "pointer", backgroundColor: "#ffffff" },
  workItemHeader: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  workItemTitle: { fontSize: 14, fontWeight: 800, color: "#111827", margin: 0, lineHeight: 1.35 },
  workItemText: { fontSize: 12, color: "#475569", lineHeight: 1.55, marginTop: 8 },
  workItemChildren: { marginTop: 12, marginLeft: 14, paddingLeft: 14, borderLeft: "1px solid #d8dee8" },
};

type LightWorkItemTone = {
  accentColor: string;
  borderColor: string;
  backgroundColor: string;
  badgeBackground: string;
  badgeColor: string;
};

function getLightWorkItemTone(bucket: ReturnType<typeof getWorkItemPresentation>["bucket"]): LightWorkItemTone {
  if (bucket === "done") {
    return {
      accentColor: "#16a34a",
      borderColor: "#bbf7d0",
      backgroundColor: "#f0fdf4",
      badgeBackground: "#dcfce7",
      badgeColor: "#166534",
    };
  }

  if (bucket === "wip") {
    return {
      accentColor: "#ca8a04",
      borderColor: "#fde68a",
      backgroundColor: "#fffbeb",
      badgeBackground: "#fef3c7",
      badgeColor: "#854d0e",
    };
  }

  if (bucket === "blocked") {
    return {
      accentColor: "#dc2626",
      borderColor: "#fecaca",
      backgroundColor: "#fff1f2",
      badgeBackground: "#fee2e2",
      badgeColor: "#991b1b",
    };
  }

  return {
    accentColor: "#2563eb",
    borderColor: "#bfdbfe",
    backgroundColor: "#eff6ff",
    badgeBackground: "#dbeafe",
    badgeColor: "#1e40af",
  };
}

type ProductOverviewDocumentProps = {
  product: Product;
  tree?: ProductTree;
  workItems?: WorkItem[];
  isLoading?: boolean;
  onEditProduct: () => void;
  onEditModule: (module: Module) => void;
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
};

export type ProductOverviewPlannerAction =
  | { kind: "enhance_product"; product: Product }
  | { kind: "add_product_child"; product: Product }
  | { kind: "enhance_module"; product: Product; module: Module }
  | { kind: "add_module_child"; product: Product; module: Module }
  | { kind: "enhance_capability"; product: Product; moduleName: string; capability: Capability }
  | { kind: "add_capability_child"; product: Product; moduleName: string; capability: Capability }
  | { kind: "add_capability_work_item"; product: Product; moduleName: string; capability: Capability }
  | { kind: "enhance_work_item"; product: Product; workItem: WorkItem };

export function ProductOverviewDocument({
  product,
  tree,
  workItems,
  isLoading = false,
  onEditProduct,
  onEditModule,
  onEditCapability,
  onOpenWorkItem,
  onPlanFromItem,
}: ProductOverviewDocumentProps) {
  const allWorkItems = useMemo(() => sortWorkItems(workItems ?? []), [workItems]);
  const metrics = useMemo(() => buildWorkItemMetrics(allWorkItems), [allWorkItems]);
  const productLevelWorkItems = useMemo(
    () => buildScopedWorkItemTree(getProductDirectWorkItems(allWorkItems)),
    [allWorkItems],
  );
  const rootSectionCount = tree?.roots.length ?? 0;
  const totalNodeCount = useMemo(() => (tree ? countHierarchyNodes(tree.roots) : 0), [tree]);
  const leafNodeCount = useMemo(() => (tree ? countLeafNodes(tree.roots) : 0), [tree]);
  const activeWorkItemCount = useMemo(
    () => allWorkItems.filter((workItem) => workItem.status !== "done" && workItem.status !== "cancelled").length,
    [allWorkItems],
  );

  return (
    <div style={styles.layout}>
      <div style={styles.article}>
        <section id={PRODUCT_OVERVIEW_TOP_ID} style={styles.hero}>
          <div style={styles.eyebrow}>Product Overview</div>
          <div style={styles.heroTop}>
            <div style={{ minWidth: 0 }}>
              <h2 style={styles.title}>{product.name}</h2>
              <p style={styles.subtitle}>
                Reader mode for the product: semantic root sections, nested nodes, and delivery work aligned to the same structural tree.
              </p>
            </div>
            <div style={styles.nodeActionRow}>
              <button style={styles.plannerButton} onClick={() => onPlanFromItem({ kind: "enhance_product", product })}>Enhance</button>
              <button style={styles.subtleButton} onClick={() => onPlanFromItem({ kind: "add_product_child", product })}>Add Child</button>
              <button style={styles.button} onClick={onEditProduct}>Edit Product</button>
            </div>
          </div>

          <div style={styles.prose}>
            {product.description || "Add a product description so this page reads like durable documentation instead of a thin status screen."}
          </div>

          <div style={styles.progressPanel}>
            <div style={styles.progressRow}>
              <span>{metrics.done} of {metrics.total} work items complete</span>
              <strong>{metrics.completion}% complete</strong>
            </div>
            <div style={styles.progressTrack}>
              <div style={{ ...styles.progressFill, width: `${metrics.completion}%` }} />
            </div>
          </div>

          <div style={styles.metricGrid}>
            <MetricCard label="Root Sections" value={rootSectionCount} />
            <MetricCard label="Total Nodes" value={totalNodeCount} />
            <MetricCard label="Leaf Nodes" value={leafNodeCount} />
            <MetricCard label="Active Work Items" value={activeWorkItemCount} />
            <MetricCard label="Done" value={metrics.done} />
            <MetricCard label="Blocked" value={metrics.blocked} />
          </div>
        </section>

        <div style={styles.summaryGrid}>
          <div style={styles.summaryCard}>
            <div style={styles.sectionTitle}>Direction</div>
            <h3 style={styles.summaryHeading}>Vision</h3>
            <div style={styles.prose}>
              {product.vision || "No product vision recorded yet."}
            </div>
          </div>

          <div style={styles.summaryCard}>
            <div style={styles.sectionTitle}>Intent</div>
            <h3 style={styles.summaryHeading}>Goals</h3>
            {product.goals.length > 0 ? (
              <ol style={styles.list}>
                {product.goals.map((goal) => (
                  <li key={goal}>{goal}</li>
                ))}
              </ol>
            ) : (
              <div style={styles.empty}>No goals recorded yet.</div>
            )}
          </div>

          <div style={styles.summaryCard}>
            <div style={styles.sectionTitle}>Metadata</div>
            <h3 style={styles.summaryHeading}>Tags</h3>
            {product.tags.length > 0 ? (
              <div style={styles.chipRow}>
                {product.tags.map((tag) => (
                  <span key={tag} style={styles.chip}>{tag}</span>
                ))}
              </div>
            ) : (
              <div style={styles.empty}>No product tags recorded yet.</div>
            )}
          </div>
        </div>

        {isLoading ? <div style={styles.section}><div style={styles.empty}>Loading product documentation…</div></div> : null}

        {!isLoading && productLevelWorkItems.length > 0 ? (
          <section id={PRODUCT_DELIVERY_ID} style={styles.section}>
            <div style={styles.sectionHeader}>
              <div style={styles.eyebrow}>Product</div>
              <h3 style={styles.sectionHeading}>Product Delivery</h3>
              <p style={styles.sectionSubtitle}>
                Cross-cutting work attached directly to the product instead of a specific module or capability.
              </p>
            </div>
            <WorkItemTree product={product} nodes={productLevelWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
          </section>
        ) : null}

        {!isLoading && (tree?.modules.length ?? 0) === 0 ? (
          <section style={styles.section}>
            <div style={styles.sectionHeader}>
              <div style={styles.eyebrow}>Product</div>
              <h3 style={styles.sectionHeading}>No Root Sections Yet</h3>
              <p style={styles.sectionSubtitle}>
                Create the first semantic root section to turn the product into a navigable system map.
              </p>
            </div>
          </section>
        ) : null}

        {!isLoading ? (
          (tree?.modules ?? []).map((moduleTree, index) => (
            <ModuleChapter
              key={moduleTree.module.id}
              product={product}
              productName={product.name}
              moduleTree={moduleTree}
              chapterNumber={index + 1}
              allWorkItems={allWorkItems}
              onEditModule={onEditModule}
              onEditCapability={onEditCapability}
              onOpenWorkItem={onOpenWorkItem}
              onPlanFromItem={onPlanFromItem}
            />
          ))
        ) : null}
      </div>
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: number }) {
  return (
    <div style={styles.metricCard}>
      <div style={styles.metricLabel}>{label}</div>
      <div style={styles.metricValue}>{value}</div>
    </div>
  );
}

function ModuleChapter({
  product,
  productName,
  moduleTree,
  chapterNumber,
  allWorkItems,
  onEditModule,
  onEditCapability,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  productName: string;
  moduleTree: ModuleTree;
  chapterNumber: number;
  allWorkItems: WorkItem[];
  onEditModule: (module: Module) => void;
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  const [isOpen, setIsOpen] = useState(true);
  const rootLabel = getHierarchyNodeKindLabel(moduleTree.module.node_kind);
  const moduleWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.module_id === moduleTree.module.id && !workItem.capability_id),
  );
  const metrics = useMemo(() => buildWorkItemMetrics(getModuleScopedWorkItems(moduleTree, allWorkItems)), [allWorkItems, moduleTree]);

  return (
    <section id={getModuleSectionId(moduleTree.module)} style={styles.detailsShell}>
      <details open={isOpen} onToggle={(event) => setIsOpen(event.currentTarget.open)}>
        <summary style={styles.summary}>
          <div style={styles.summaryLeft}>
            <div style={styles.chapterLabel}>{rootLabel} {chapterNumber}</div>
            <h3 style={styles.chapterTitle}>{moduleTree.module.name}</h3>
            <div style={styles.chapterSubtitle}>
              {moduleTree.module.description || moduleTree.module.purpose || `Document this ${rootLabel.toLowerCase()} so the product architecture stays readable.`}
            </div>
            <div style={styles.pathText}>{productName} / {moduleTree.module.name}</div>
          </div>
          <div style={styles.summaryRight}>
            <span style={styles.summaryPill}>{moduleTree.features.length} {moduleTree.features.length === 1 ? "child node" : "child nodes"}</span>
            <MetricPills metrics={metrics} />
          </div>
        </summary>

        <div style={styles.detailsBody}>
          <div style={styles.nodeActionRow}>
            <button style={styles.plannerButton} onClick={() => onPlanFromItem({ kind: "enhance_module", product, module: moduleTree.module })}>Enhance</button>
            <button style={styles.subtleButton} onClick={() => onPlanFromItem({ kind: "add_module_child", product, module: moduleTree.module })}>Add Child</button>
            <button style={styles.subtleButton} onClick={() => onEditModule(moduleTree.module)}>Edit {rootLabel}</button>
          </div>

          {moduleTree.module.purpose
            || moduleTree.module.explanation
            || moduleTree.module.examples
            || moduleTree.module.implementation_notes
            || moduleTree.module.test_guidance ? (
            <div style={styles.noteGrid}>
              {moduleTree.module.purpose ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Purpose</div>
                  <div style={styles.noteText}>{moduleTree.module.purpose}</div>
                </div>
              ) : null}
              {moduleTree.module.explanation ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Explanation</div>
                  <div style={styles.noteText}>{moduleTree.module.explanation}</div>
                </div>
              ) : null}
              {moduleTree.module.examples ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Examples</div>
                  <div style={styles.noteText}>{moduleTree.module.examples}</div>
                </div>
              ) : null}
              {moduleTree.module.implementation_notes ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Implementation Notes</div>
                  <div style={styles.noteText}>{moduleTree.module.implementation_notes}</div>
                </div>
              ) : null}
              {moduleTree.module.test_guidance ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Test Guidance</div>
                  <div style={styles.noteText}>{moduleTree.module.test_guidance}</div>
                </div>
              ) : null}
            </div>
          ) : null}

          {moduleWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Direct Work</div>
              <WorkItemTree product={product} nodes={moduleWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
            </div>
          ) : null}

          {moduleTree.features.length > 0 ? (
            moduleTree.features.map((capabilityTree, index) => (
              <CapabilityChapter
                key={capabilityTree.capability.id}
                product={product}
                path={[productName, moduleTree.module.name]}
                capabilityTree={capabilityTree}
                numbering={`${chapterNumber}.${index + 1}`}
                allWorkItems={allWorkItems}
                onEditCapability={onEditCapability}
                onOpenWorkItem={onOpenWorkItem}
                onPlanFromItem={onPlanFromItem}
              />
            ))
          ) : (
            <div style={styles.empty}>No capabilities defined for this module yet.</div>
          )}
        </div>
      </details>
    </section>
  );
}

function CapabilityChapter({
  product,
  path,
  capabilityTree,
  numbering,
  allWorkItems,
  onEditCapability,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  path: string[];
  capabilityTree: CapabilityTree;
  numbering: string;
  allWorkItems: WorkItem[];
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
  const directWorkItems = useMemo(
    () => buildScopedWorkItemTree(allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id)),
    [allWorkItems, capabilityTree.capability.id],
  );
  const metrics = useMemo(() => buildWorkItemMetrics(getCapabilityScopedWorkItems(capabilityTree, allWorkItems)), [allWorkItems, capabilityTree]);

  return (
    <div id={getCapabilitySectionId(capabilityTree.capability)} style={styles.detailsShell}>
      <details open={isOpen} onToggle={(event) => setIsOpen(event.currentTarget.open)}>
        <summary style={styles.summary}>
          <div style={styles.summaryLeft}>
            <div style={styles.chapterLabel}>{capabilityType} {numbering}</div>
            <h4 style={{ ...styles.chapterTitle, fontSize: 19 }}>{capabilityTree.capability.name}</h4>
            <div style={styles.chapterSubtitle}>
              {capabilityTree.capability.description || `Document what this ${capabilityType.toLowerCase()} is responsible for.`}
            </div>
            <div style={styles.pathText}>{[...path, capabilityTree.capability.name].join(" / ")}</div>
          </div>
          <div style={styles.summaryRight}>
            <span style={styles.summaryPill}>{capabilityTree.capability.status.replace(/_/g, " ")}</span>
            <span style={styles.summaryPill}>{capabilityTree.capability.priority} priority</span>
            <MetricPills metrics={metrics} />
          </div>
        </summary>

        <div style={styles.detailsBody}>
          <div style={styles.nodeActionRow}>
            <button
              style={styles.plannerButton}
              onClick={() => onPlanFromItem({ kind: "enhance_capability", product, moduleName: path[1] ?? "", capability: capabilityTree.capability })}
            >
              Enhance
            </button>
            <button
              style={styles.subtleButton}
              onClick={() => onPlanFromItem({ kind: "add_capability_child", product, moduleName: path[1] ?? "", capability: capabilityTree.capability })}
            >
              Add Child
            </button>
            <button
              style={styles.subtleButton}
              onClick={() => onPlanFromItem({ kind: "add_capability_work_item", product, moduleName: path[1] ?? "", capability: capabilityTree.capability })}
            >
              Add Work Item
            </button>
            <button style={styles.subtleButton} onClick={() => onEditCapability(capabilityTree.capability)}>
              Edit {capabilityType}
            </button>
          </div>

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

          <div style={styles.metaRow}>
            <span style={styles.metaPill}>status: {capabilityTree.capability.status.replace(/_/g, " ")}</span>
            <span style={styles.metaPill}>priority: {capabilityTree.capability.priority}</span>
            <span style={styles.metaPill}>risk: {capabilityTree.capability.risk}</span>
          </div>

          {directWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Delivery Work</div>
              <WorkItemTree product={product} nodes={directWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
            </div>
          ) : (
            <div style={styles.empty}>No work items attached to this {capabilityType.toLowerCase()} yet.</div>
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
                  onEditCapability={onEditCapability}
                  onOpenWorkItem={onOpenWorkItem}
                  onPlanFromItem={onPlanFromItem}
                />
              ))}
            </div>
          ) : null}
        </div>
      </details>
    </div>
  );
}

function WorkItemTree({
  product,
  nodes,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  nodes: WorkItemNode[];
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  return (
    <div style={styles.workItemList}>
      {nodes.map((node) => (
        <WorkItemCard key={node.workItem.id} product={product} node={node} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
      ))}
    </div>
  );
}

function WorkItemCard({
  product,
  node,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  node: WorkItemNode;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  const presentation = getWorkItemPresentation(node.workItem.status);
  const tone = getLightWorkItemTone(presentation.bucket);
  const excerpt = summarizeText(node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria || "No delivery notes captured yet.");

  return (
    <div
      style={{
        ...styles.workItemCard,
        borderColor: tone.borderColor,
        backgroundColor: tone.backgroundColor,
        borderLeft: `4px solid ${tone.accentColor}`,
      }}
      onClick={() => onOpenWorkItem(node.workItem)}
    >
      <div style={styles.workItemHeader}>
        <div style={{ minWidth: 0 }}>
          <h5 style={styles.workItemTitle}>{node.workItem.title}</h5>
          <div style={{ ...styles.metaRow, marginTop: 8 }}>
            <span
              style={{
                ...styles.statePill,
                backgroundColor: tone.badgeBackground,
                color: tone.badgeColor,
              }}
            >
              {presentation.label}
            </span>
            <span style={styles.metaPill}>{node.workItem.work_item_type.replace(/_/g, " ")}</span>
            <span style={styles.metaPill}>{node.workItem.priority} priority</span>
            <span style={styles.metaPill}>{node.workItem.complexity.replace(/_/g, " ")}</span>
            {node.children.length > 0 ? <span style={styles.metaPill}>{node.children.length} sub-item{node.children.length === 1 ? "" : "s"}</span> : null}
          </div>
        </div>
        <button
          style={styles.subtleButton}
          onClick={(event) => {
            event.stopPropagation();
            onPlanFromItem({ kind: "enhance_work_item", product, workItem: node.workItem });
          }}
        >
          Enhance
        </button>
      </div>
      <div style={styles.workItemText}>{excerpt}</div>
      {node.children.length > 0 ? (
        <div style={styles.workItemChildren}>
          <WorkItemTree product={product} nodes={node.children} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
        </div>
      ) : null}
    </div>
  );
}

function MetricPills({ metrics }: { metrics: WorkItemMetrics }) {
  if (metrics.total === 0) {
    return <span style={styles.summaryPill}>No work items</span>;
  }

  return (
    <>
      {metrics.done > 0 ? <StatusTonePill label={`${metrics.done} done`} tone="#15803d" /> : null}
      {metrics.wip > 0 ? <StatusTonePill label={`${metrics.wip} WIP`} tone="#a16207" /> : null}
      {metrics.tbd > 0 ? <StatusTonePill label={`${metrics.tbd} TBD`} tone="#1d4ed8" /> : null}
      {metrics.blocked > 0 ? <StatusTonePill label={`${metrics.blocked} blocked`} tone="#b91c1c" /> : null}
    </>
  );
}

function StatusTonePill({ label, tone }: { label: string; tone: string }) {
  return (
    <span style={{ ...styles.statePill, color: tone }}>
      {label}
    </span>
  );
}

function collectCapabilityIds(capabilities: CapabilityTree[]): Set<string> {
  const ids = new Set<string>();
  capabilities.forEach((capabilityTree) => {
    ids.add(capabilityTree.capability.id);
    collectCapabilityIds(capabilityTree.children).forEach((id) => ids.add(id));
  });
  return ids;
}

function getModuleScopedWorkItems(moduleTree: ModuleTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds(moduleTree.features);
  return allWorkItems.filter(
    (workItem) => workItem.module_id === moduleTree.module.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false),
  );
}

function getCapabilityScopedWorkItems(capabilityTree: CapabilityTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds([capabilityTree]);
  return allWorkItems.filter((workItem) => workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false);
}

function summarizeText(text: string, maxLength = 220) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 1).trimEnd()}…`;
}
