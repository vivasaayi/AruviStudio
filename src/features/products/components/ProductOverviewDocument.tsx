import React, { useMemo, useState } from "react";
import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, CapabilityTree, Module, ModuleTree, Product, ProductTree, WorkItem } from "../../../lib/types";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildProductOverviewToc,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getModuleSectionId,
  getWorkItemPresentation,
  sortWorkItems,
  type WorkItemMetrics,
  type WorkItemNode,
} from "../lib/productOverview";

type TocGroup = {
  item: { id: string; title: string; level: number };
  children: { id: string; title: string; level: number }[];
};

const styles: Record<string, React.CSSProperties> = {
  layout: { display: "flex", gap: 24, alignItems: "flex-start" },
  layoutCollapsed: { display: "block" },
  article: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 18 },
  aside: { width: 268, flexShrink: 0, position: "sticky" as const, top: 12, display: "flex", flexDirection: "column", gap: 14, height: "calc(100vh - 24px)", minHeight: 0 },
  asideCollapsed: { display: "none" },
  panel: { borderRadius: 18, border: "1px solid #2a3340", backgroundColor: "#141b24", padding: 16, boxShadow: "0 18px 40px rgba(0,0,0,0.22)" },
  panelScrollable: { flex: 1, minHeight: 0, display: "flex", flexDirection: "column" as const },
  panelTitle: { fontSize: 11, fontWeight: 800, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "#8fb8ff", marginBottom: 10 },
  tocWrap: { overflowY: "auto" as const, paddingRight: 4, display: "flex", flexDirection: "column", gap: 6, flex: 1, minHeight: 0 },
  tocList: { display: "flex", flexDirection: "column", gap: 4 },
  tocLink: { display: "block", padding: "7px 10px", borderRadius: 10, color: "#c8d5e8", textDecoration: "none", fontSize: 13, lineHeight: 1.45 },
  tocGroup: { borderRadius: 12, border: "1px solid #233041", backgroundColor: "#111821", overflow: "hidden" },
  tocSummary: { listStyle: "none" as const, cursor: "pointer", padding: "8px 10px", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, color: "#d7e3f3", fontSize: 13, fontWeight: 700 },
  tocChildren: { display: "flex", flexDirection: "column", gap: 3, padding: "0 6px 8px" },
  legendRow: { display: "flex", alignItems: "center", gap: 10, fontSize: 12, color: "#a8b5c8" },
  legendDot: { width: 10, height: 10, borderRadius: 999 },
  hero: {
    borderRadius: 22,
    border: "1px solid #2d3c51",
    background: "radial-gradient(circle at top right, rgba(102, 140, 214, 0.26), transparent 28%), linear-gradient(145deg, #1a2940 0%, #162433 52%, #111924 100%)",
    padding: 24,
    boxShadow: "0 24px 48px rgba(0,0,0,0.28)",
  },
  eyebrow: { fontSize: 11, fontWeight: 800, letterSpacing: "0.14em", textTransform: "uppercase" as const, color: "#8fb8ff", marginBottom: 8 },
  heroTop: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, marginBottom: 12 },
  title: { fontSize: 30, fontWeight: 900, color: "#f6f8fc", margin: 0, lineHeight: 1.05 },
  subtitle: { fontSize: 14, color: "#d6deea", lineHeight: 1.7, margin: 0 },
  prose: { fontSize: 14, color: "#dce5f2", lineHeight: 1.75, whiteSpace: "pre-wrap" as const },
  button: { padding: "7px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#22344a", color: "#f4f8ff", border: "1px solid #406183", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  subtleButton: { padding: "6px 10px", fontSize: 12, fontWeight: 700, backgroundColor: "#182433", color: "#cfe0f7", border: "1px solid #31465f", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  toggleButton: { padding: "7px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#182433", color: "#d9e7fa", border: "1px solid #35506f", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  readerToolbar: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, flexWrap: "wrap" as const },
  toolbarHint: { fontSize: 12, color: "#92a2b8" },
  progressPanel: { marginTop: 18, borderRadius: 16, border: "1px solid rgba(255,255,255,0.1)", backgroundColor: "rgba(255,255,255,0.06)", padding: 14 },
  progressRow: { display: "flex", justifyContent: "space-between", gap: 16, fontSize: 12, color: "#dce4f1" },
  progressTrack: { width: "100%", height: 10, borderRadius: 999, backgroundColor: "rgba(255,255,255,0.12)", overflow: "hidden", marginTop: 10 },
  progressFill: { height: "100%", borderRadius: 999, background: "linear-gradient(90deg, #61d48c 0%, #8ff2bc 100%)" },
  metricGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 10, marginTop: 16 },
  metricCard: { borderRadius: 14, padding: 12, backgroundColor: "rgba(255,255,255,0.08)", border: "1px solid rgba(255,255,255,0.08)" },
  metricLabel: { fontSize: 11, fontWeight: 800, color: "#b8cae4", textTransform: "uppercase" as const, letterSpacing: "0.08em" },
  metricValue: { fontSize: 24, fontWeight: 900, color: "#ffffff", marginTop: 4 },
  summaryGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 14 },
  summaryCard: { borderRadius: 18, border: "1px solid #293341", backgroundColor: "#141b24", padding: 16 },
  sectionTitle: { fontSize: 11, fontWeight: 800, letterSpacing: "0.1em", textTransform: "uppercase" as const, color: "#95a7c0", marginBottom: 10 },
  summaryHeading: { fontSize: 20, fontWeight: 800, color: "#f5f7fb", margin: "0 0 8px" },
  list: { margin: 0, paddingLeft: 18, color: "#d6deeb", display: "flex", flexDirection: "column", gap: 8, lineHeight: 1.65 },
  chipRow: { display: "flex", gap: 8, flexWrap: "wrap" as const },
  chip: { fontSize: 11, padding: "4px 8px", borderRadius: 999, backgroundColor: "#1b2a3c", color: "#b9d3ff" },
  section: { borderRadius: 18, border: "1px solid #293341", backgroundColor: "#141b24", padding: 18 },
  sectionHeader: { display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 },
  sectionHeading: { fontSize: 24, fontWeight: 900, color: "#f5f7fb", margin: 0 },
  sectionSubtitle: { fontSize: 13, color: "#9ca9bc", lineHeight: 1.65, margin: 0 },
  empty: { fontSize: 13, color: "#79879b", fontStyle: "italic" as const, lineHeight: 1.6 },
  detailsShell: { borderRadius: 18, border: "1px solid #293341", backgroundColor: "#141b24", overflow: "hidden" },
  summary: { padding: 18, display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16, cursor: "pointer", listStyle: "none" as const },
  summaryLeft: { minWidth: 0 },
  summaryRight: { display: "flex", gap: 8, flexWrap: "wrap" as const, justifyContent: "flex-end" as const },
  chapterLabel: { fontSize: 11, fontWeight: 800, color: "#8fb8ff", letterSpacing: "0.14em", textTransform: "uppercase" as const, marginBottom: 6 },
  chapterTitle: { fontSize: 24, fontWeight: 900, color: "#f4f7fd", margin: 0, lineHeight: 1.15 },
  chapterSubtitle: { fontSize: 13, color: "#a5b0c0", lineHeight: 1.65, marginTop: 8 },
  summaryPill: { display: "inline-flex", alignItems: "center", borderRadius: 999, padding: "5px 9px", fontSize: 11, fontWeight: 700, backgroundColor: "#1b2431", border: "1px solid #324256", color: "#c8d5e8" },
  statePill: { display: "inline-flex", alignItems: "center", borderRadius: 999, padding: "5px 9px", fontSize: 11, fontWeight: 800, border: "1px solid currentColor", backgroundColor: "rgba(255,255,255,0.04)" },
  detailsBody: { padding: "0 18px 18px", display: "flex", flexDirection: "column", gap: 14 },
  noteGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 12 },
  noteCard: { borderRadius: 14, border: "1px solid #263142", backgroundColor: "#111821", padding: 14 },
  noteHeading: { fontSize: 11, fontWeight: 800, letterSpacing: "0.08em", textTransform: "uppercase" as const, color: "#95a7c0", marginBottom: 8 },
  noteText: { fontSize: 13, color: "#d6deea", lineHeight: 1.65, whiteSpace: "pre-wrap" as const },
  metaRow: { display: "flex", gap: 8, flexWrap: "wrap" as const },
  metaPill: { fontSize: 11, padding: "4px 8px", borderRadius: 999, backgroundColor: "#1a2737", color: "#bed3ee" },
  pathText: { fontSize: 12, color: "#9fb0c5", lineHeight: 1.6, marginTop: 6 },
  nested: { marginLeft: 18, paddingLeft: 18, borderLeft: "1px solid #253141", display: "flex", flexDirection: "column", gap: 14 },
  workItemList: { display: "flex", flexDirection: "column", gap: 10 },
  workItemCard: { borderRadius: 15, border: "1px solid #334152", padding: 14, cursor: "pointer", backgroundColor: "#101721" },
  workItemHeader: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  workItemTitle: { fontSize: 14, fontWeight: 800, color: "#f4f7fd", margin: 0, lineHeight: 1.35 },
  workItemText: { fontSize: 12, color: "#c5cedb", lineHeight: 1.65, marginTop: 8 },
  workItemChildren: { marginTop: 12, marginLeft: 14, paddingLeft: 14, borderLeft: "1px solid #293341" },
};

type ProductOverviewDocumentProps = {
  product: Product;
  tree?: ProductTree;
  workItems?: WorkItem[];
  isLoading?: boolean;
  onEditProduct: () => void;
  onEditModule: (module: Module) => void;
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
};

export function ProductOverviewDocument({
  product,
  tree,
  workItems,
  isLoading = false,
  onEditProduct,
  onEditModule,
  onEditCapability,
  onOpenWorkItem,
}: ProductOverviewDocumentProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
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
  const tocItems = useMemo(
    () => buildProductOverviewToc(tree, productLevelWorkItems.length > 0),
    [productLevelWorkItems.length, tree],
  );
  const tocGroups = useMemo(() => groupTocItems(tocItems), [tocItems]);

  return (
    <div style={sidebarCollapsed ? { ...styles.layout, ...styles.layoutCollapsed } : styles.layout}>
      <div style={styles.article}>
        <div style={styles.readerToolbar}>
          <div style={styles.toolbarHint}>
            Reader controls
          </div>
          <button style={styles.toggleButton} onClick={() => setSidebarCollapsed((current) => !current)}>
            {sidebarCollapsed ? "Show Sidebar" : "Hide Sidebar"}
          </button>
        </div>
        <section id={PRODUCT_OVERVIEW_TOP_ID} style={styles.hero}>
          <div style={styles.eyebrow}>Product Overview</div>
          <div style={styles.heroTop}>
            <div style={{ minWidth: 0 }}>
              <h2 style={styles.title}>{product.name}</h2>
              <p style={styles.subtitle}>
                Reader mode for the product: semantic root sections, nested nodes, and delivery work aligned to the same structural tree.
              </p>
            </div>
            <button style={styles.button} onClick={onEditProduct}>Edit Product</button>
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
            <WorkItemTree nodes={productLevelWorkItems} onOpenWorkItem={onOpenWorkItem} />
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
              productName={product.name}
              moduleTree={moduleTree}
              chapterNumber={index + 1}
              allWorkItems={allWorkItems}
              onEditModule={onEditModule}
              onEditCapability={onEditCapability}
              onOpenWorkItem={onOpenWorkItem}
            />
          ))
        ) : null}
      </div>

      <aside style={sidebarCollapsed ? { ...styles.aside, ...styles.asideCollapsed } : styles.aside}>
        <div style={{ ...styles.panel, ...styles.panelScrollable }}>
          <div style={styles.panelTitle}>On This Page</div>
          <div style={styles.tocWrap}>
            {tocGroups.map((group) => (
              group.children.length === 0 ? (
                <a
                  key={group.item.id}
                  href={`#${group.item.id}`}
                  style={styles.tocLink}
                >
                  {group.item.title}
                </a>
              ) : (
                <details key={group.item.id} open style={styles.tocGroup}>
                  <summary style={styles.tocSummary}>
                    <span>{group.item.title}</span>
                    <span>{group.children.length}</span>
                  </summary>
                  <div style={styles.tocChildren}>
                    <a href={`#${group.item.id}`} style={styles.tocLink}>
                      Section overview
                    </a>
                    {group.children.map((item) => (
                      <a
                        key={item.id}
                        href={`#${item.id}`}
                        style={{ ...styles.tocLink, paddingLeft: 10 + Math.min(item.level, 3) * 14 }}
                      >
                        {item.title}
                      </a>
                    ))}
                  </div>
                </details>
              )
            ))}
          </div>
        </div>
        <div style={styles.panel}>
          <div style={styles.panelTitle}>Work Status</div>
          <LegendRow label="Done" color="#4aa37c" />
          <LegendRow label="WIP" color="#d1a643" />
          <LegendRow label="TBD" color="#6797d8" />
          <LegendRow label="Blocked" color="#cb6469" />
        </div>
      </aside>
    </div>
  );
}

function groupTocItems(items: { id: string; title: string; level: number }[]): TocGroup[] {
  const groups: TocGroup[] = [];

  items.forEach((item) => {
    if (item.level === 0) {
      groups.push({ item, children: [] });
      return;
    }

    const currentGroup = groups[groups.length - 1];
    if (currentGroup) {
      currentGroup.children.push(item);
    }
  });

  return groups;
}

function MetricCard({ label, value }: { label: string; value: number }) {
  return (
    <div style={styles.metricCard}>
      <div style={styles.metricLabel}>{label}</div>
      <div style={styles.metricValue}>{value}</div>
    </div>
  );
}

function LegendRow({ label, color }: { label: string; color: string }) {
  return (
    <div style={styles.legendRow}>
      <span style={{ ...styles.legendDot, backgroundColor: color }} />
      {label}
    </div>
  );
}

function ModuleChapter({
  productName,
  moduleTree,
  chapterNumber,
  allWorkItems,
  onEditModule,
  onEditCapability,
  onOpenWorkItem,
}: {
  productName: string;
  moduleTree: ModuleTree;
  chapterNumber: number;
  allWorkItems: WorkItem[];
  onEditModule: (module: Module) => void;
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
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
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button style={styles.subtleButton} onClick={() => onEditModule(moduleTree.module)}>Edit {rootLabel}</button>
          </div>

          {moduleTree.module.purpose ? (
            <div style={styles.noteCard}>
              <div style={styles.noteHeading}>Purpose</div>
              <div style={styles.noteText}>{moduleTree.module.purpose}</div>
            </div>
          ) : null}

          {moduleWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Direct Work</div>
              <WorkItemTree nodes={moduleWorkItems} onOpenWorkItem={onOpenWorkItem} />
            </div>
          ) : null}

          {moduleTree.features.length > 0 ? (
            moduleTree.features.map((capabilityTree, index) => (
              <CapabilityChapter
                key={capabilityTree.capability.id}
                path={[productName, moduleTree.module.name]}
                capabilityTree={capabilityTree}
                numbering={`${chapterNumber}.${index + 1}`}
                allWorkItems={allWorkItems}
                onEditCapability={onEditCapability}
                onOpenWorkItem={onOpenWorkItem}
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
  path,
  capabilityTree,
  numbering,
  allWorkItems,
  onEditCapability,
  onOpenWorkItem,
}: {
  path: string[];
  capabilityTree: CapabilityTree;
  numbering: string;
  allWorkItems: WorkItem[];
  onEditCapability: (capability: Capability) => void;
  onOpenWorkItem: (workItem: WorkItem) => void;
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
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button style={styles.subtleButton} onClick={() => onEditCapability(capabilityTree.capability)}>
              Edit {capabilityType}
            </button>
          </div>

          {capabilityTree.capability.acceptance_criteria || capabilityTree.capability.technical_notes ? (
            <div style={styles.noteGrid}>
              {capabilityTree.capability.acceptance_criteria ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Acceptance Criteria</div>
                  <div style={styles.noteText}>{capabilityTree.capability.acceptance_criteria}</div>
                </div>
              ) : null}
              {capabilityTree.capability.technical_notes ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Technical Notes</div>
                  <div style={styles.noteText}>{capabilityTree.capability.technical_notes}</div>
                </div>
              ) : null}
            </div>
          ) : (
            <div style={styles.empty}>No acceptance criteria or technical notes recorded yet.</div>
          )}

          <div style={styles.metaRow}>
            <span style={styles.metaPill}>status: {capabilityTree.capability.status.replace(/_/g, " ")}</span>
            <span style={styles.metaPill}>priority: {capabilityTree.capability.priority}</span>
            <span style={styles.metaPill}>risk: {capabilityTree.capability.risk}</span>
          </div>

          {directWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Delivery Work</div>
              <WorkItemTree nodes={directWorkItems} onOpenWorkItem={onOpenWorkItem} />
            </div>
          ) : (
            <div style={styles.empty}>No work items attached to this {capabilityType.toLowerCase()} yet.</div>
          )}

          {capabilityTree.children.length > 0 ? (
            <div style={styles.nested}>
              {capabilityTree.children.map((child, index) => (
                <CapabilityChapter
                  key={child.capability.id}
                  path={[...path, capabilityTree.capability.name]}
                  capabilityTree={child}
                  numbering={`${numbering}.${index + 1}`}
                  allWorkItems={allWorkItems}
                  onEditCapability={onEditCapability}
                  onOpenWorkItem={onOpenWorkItem}
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
  nodes,
  onOpenWorkItem,
}: {
  nodes: WorkItemNode[];
  onOpenWorkItem: (workItem: WorkItem) => void;
}) {
  return (
    <div style={styles.workItemList}>
      {nodes.map((node) => (
        <WorkItemCard key={node.workItem.id} node={node} onOpenWorkItem={onOpenWorkItem} />
      ))}
    </div>
  );
}

function WorkItemCard({
  node,
  onOpenWorkItem,
}: {
  node: WorkItemNode;
  onOpenWorkItem: (workItem: WorkItem) => void;
}) {
  const presentation = getWorkItemPresentation(node.workItem.status);
  const excerpt = summarizeText(node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria || "No delivery notes captured yet.");

  return (
    <div
      style={{
        ...styles.workItemCard,
        borderColor: presentation.borderColor,
        backgroundColor: presentation.backgroundColor,
        borderLeft: `4px solid ${presentation.accentColor}`,
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
                backgroundColor: presentation.badgeBackground,
                color: presentation.badgeColor,
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
      </div>
      <div style={styles.workItemText}>{excerpt}</div>
      {node.children.length > 0 ? (
        <div style={styles.workItemChildren}>
          <WorkItemTree nodes={node.children} onOpenWorkItem={onOpenWorkItem} />
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
      {metrics.done > 0 ? <StatusTonePill label={`${metrics.done} done`} tone="#4aa37c" /> : null}
      {metrics.wip > 0 ? <StatusTonePill label={`${metrics.wip} WIP`} tone="#d1a643" /> : null}
      {metrics.tbd > 0 ? <StatusTonePill label={`${metrics.tbd} TBD`} tone="#6797d8" /> : null}
      {metrics.blocked > 0 ? <StatusTonePill label={`${metrics.blocked} blocked`} tone="#cb6469" /> : null}
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
