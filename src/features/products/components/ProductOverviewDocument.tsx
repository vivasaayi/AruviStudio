import React, { useEffect, useMemo, useState } from "react";
import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, CapabilityTree, ProductArea, ProductAreaTree, Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  sortWorkItems,
  type WorkItemMetrics,
  type WorkItemNode,
} from "../lib/productOverview";
import { filterReferencesForProductBook, filterReferencesForScope, getCapabilityReferenceScope, getProductAreaReferenceScope, getReferenceKindLabel } from "../lib/productReferences";
import { styles } from "./ProductOverviewDocument.styles";

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
  metricsOverride?: WorkItemMetrics;
  nodeCountsOverride?: {
    productAreaCount: number;
    totalNodeCount: number;
    leafNodeCount: number;
  };
  activeWorkItemCountOverride?: number;
  references?: ProductReference[];
  isLoading?: boolean;
  onEditProduct: () => void;
  onEditProductArea: (product_area: ProductArea) => void;
  onEditCapability: (capability: Capability) => void;
  onLoadProductAreaTree?: (productArea: ProductArea) => Promise<ProductAreaTree>;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
};

export type ProductOverviewPlannerAction =
  | { kind: "enhance_product"; product: Product }
  | { kind: "add_product_child"; product: Product }
  | { kind: "enhance_product_area"; product: Product; product_area: ProductArea }
  | { kind: "add_product_area_child"; product: Product; product_area: ProductArea }
  | { kind: "enhance_capability"; product: Product; productAreaName: string; capability: Capability }
  | { kind: "add_capability_child"; product: Product; productAreaName: string; capability: Capability }
  | { kind: "add_capability_work_item"; product: Product; productAreaName: string; capability: Capability }
  | { kind: "enhance_work_item"; product: Product; workItem: WorkItem };

export function ProductOverviewDocument({
  product,
  tree,
  workItems,
  metricsOverride,
  nodeCountsOverride,
  activeWorkItemCountOverride,
  references = [],
  isLoading = false,
  onEditProduct,
  onEditProductArea,
  onEditCapability,
  onLoadProductAreaTree,
  onOpenWorkItem,
  onPlanFromItem,
}: ProductOverviewDocumentProps) {
  const allWorkItems = useMemo(() => sortWorkItems(workItems ?? []), [workItems]);
  const derivedMetrics = useMemo(() => buildWorkItemMetrics(allWorkItems), [allWorkItems]);
  const metrics = metricsOverride ?? derivedMetrics;
  const productLevelWorkItems = useMemo(
    () => buildScopedWorkItemTree(getProductDirectWorkItems(allWorkItems)),
    [allWorkItems],
  );
  const rootSectionCount = nodeCountsOverride?.productAreaCount ?? tree?.roots.length ?? 0;
  const totalNodeCount = nodeCountsOverride?.totalNodeCount ?? (tree ? countHierarchyNodes(tree.roots) : 0);
  const leafNodeCount = nodeCountsOverride?.leafNodeCount ?? (tree ? countLeafNodes(tree.roots) : 0);
  const bookReferences = useMemo(
    () => filterReferencesForProductBook(product.id, tree, references),
    [product.id, references, tree],
  );
  const productReferences = useMemo(
    () => filterReferencesForScope(bookReferences, { scopeType: "product", scopeId: product.id }),
    [bookReferences, product.id],
  );
  const derivedActiveWorkItemCount = useMemo(
    () => allWorkItems.filter((workItem) => workItem.status !== "done" && workItem.status !== "cancelled").length,
    [allWorkItems],
  );
  const activeWorkItemCount = activeWorkItemCountOverride ?? derivedActiveWorkItemCount;

  return (
    <div style={styles.layout}>
      <div style={styles.article}>
        <section id={PRODUCT_OVERVIEW_TOP_ID} style={styles.hero}>
          <div style={styles.eyebrow}>Product Book</div>
          <div style={styles.heroTop}>
            <div style={{ minWidth: 0 }}>
              <h2 style={styles.title}>{product.name}</h2>
              <p style={styles.subtitle}>
                Product areas read as chapters. Capabilities and features read as sections. Stories and tasks remain delivery notes.
              </p>
            </div>
            <div style={styles.nodeActionRow}>
              <button style={styles.plannerButton} onClick={() => onPlanFromItem({ kind: "enhance_product", product })}>Improve Book</button>
              <button style={styles.subtleButton} onClick={() => onPlanFromItem({ kind: "add_product_child", product })}>Add Chapter</button>
              <button style={styles.button} onClick={onEditProduct}>Edit Book</button>
            </div>
          </div>

          <div style={styles.prose}>
            {product.description || "Add a product description to anchor the book before coding starts."}
          </div>

          <ReferenceList references={productReferences} title="Product References" />

          <div style={styles.progressPanel}>
            <div style={styles.progressRow}>
              <span>{metrics.done} of {metrics.total} stories complete</span>
              <strong>{metrics.completion}% complete</strong>
            </div>
            <div style={styles.progressTrack}>
              <div style={{ ...styles.progressFill, width: `${metrics.completion}%` }} />
            </div>
          </div>

          <div style={styles.metricGrid}>
            <MetricCard label="Product Areas" value={rootSectionCount} />
            <MetricCard label="Total Nodes" value={totalNodeCount} />
            <MetricCard label="Leaf Nodes" value={leafNodeCount} />
            <MetricCard label="Active Stories" value={activeWorkItemCount} />
            <MetricCard label="References" value={bookReferences.length} />
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
                Cross-cutting work attached directly to the book instead of a specific chapter or feature.
              </p>
            </div>
            <WorkItemTree product={product} nodes={productLevelWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
          </section>
        ) : null}

        {!isLoading && (tree?.product_areas.length ?? 0) === 0 ? (
          <section style={styles.section}>
            <div style={styles.sectionHeader}>
              <div style={styles.eyebrow}>Product</div>
              <h3 style={styles.sectionHeading}>No Product Areas Yet</h3>
              <p style={styles.sectionSubtitle}>
                Create the first product area to turn the product into a navigable system map.
              </p>
            </div>
          </section>
        ) : null}

        {!isLoading ? (
          (tree?.product_areas ?? []).map((productAreaTree, index) => (
            <ProductAreaChapter
              key={productAreaTree.product_area.id}
              product={product}
              productName={product.name}
              productAreaTree={productAreaTree}
              chapterNumber={index + 1}
              allWorkItems={allWorkItems}
              references={bookReferences}
              onEditProductArea={onEditProductArea}
              onEditCapability={onEditCapability}
              onLoadProductAreaTree={onLoadProductAreaTree}
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

function ProductAreaChapter({
  product,
  productName,
  productAreaTree,
  chapterNumber,
  allWorkItems,
  references,
  onEditProductArea,
  onEditCapability,
  onLoadProductAreaTree,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  productName: string;
  productAreaTree: ProductAreaTree;
  chapterNumber: number;
  allWorkItems: WorkItem[];
  references: ProductReference[];
  onEditProductArea: (product_area: ProductArea) => void;
  onEditCapability: (capability: Capability) => void;
  onLoadProductAreaTree?: (productArea: ProductArea) => Promise<ProductAreaTree>;
  onOpenWorkItem: (workItem: WorkItem) => void;
  onPlanFromItem: (action: ProductOverviewPlannerAction) => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const [loadedProductAreaTree, setLoadedProductAreaTree] = useState<ProductAreaTree | null>(null);
  const [loadState, setLoadState] = useState<"idle" | "loading" | "loaded" | "error">("idle");
  const rootLabel = getHierarchyNodeKindLabel(productAreaTree.product_area.node_kind);
  const renderedProductAreaTree = loadedProductAreaTree ?? productAreaTree;
  const shouldLazyLoad = productAreaTree.features.length === 0 && !!onLoadProductAreaTree;

  useEffect(() => {
    if (!isOpen || !shouldLazyLoad || loadState !== "idle" || !onLoadProductAreaTree) {
      return;
    }

    let cancelled = false;
    setLoadState("loading");
    onLoadProductAreaTree(productAreaTree.product_area)
      .then((nextTree) => {
        if (cancelled) {
          return;
        }
        setLoadedProductAreaTree(nextTree);
        setLoadState("loaded");
      })
      .catch(() => {
        if (!cancelled) {
          setLoadState("error");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [isOpen, loadState, onLoadProductAreaTree, productAreaTree.product_area, shouldLazyLoad]);

  const productAreaWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.product_area_id === renderedProductAreaTree.product_area.id && !workItem.capability_id),
  );
  const metrics = useMemo(() => buildWorkItemMetrics(getProductAreaScopedWorkItems(renderedProductAreaTree, allWorkItems)), [allWorkItems, renderedProductAreaTree]);
  const productAreaReferences = useMemo(
    () => filterReferencesForScope(references, getProductAreaReferenceScope(renderedProductAreaTree.product_area.id)),
    [renderedProductAreaTree.product_area.id, references],
  );
  const loadedChildCount = renderedProductAreaTree.features.length;

  return (
    <section id={getProductAreaSectionId(renderedProductAreaTree.product_area)} style={styles.detailsShell}>
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
          <div style={styles.chapterLabel}>{rootLabel} {chapterNumber}</div>
          <h3 style={styles.chapterTitle}>{renderedProductAreaTree.product_area.name}</h3>
          <div style={styles.chapterSubtitle}>
            {renderedProductAreaTree.product_area.description || renderedProductAreaTree.product_area.purpose || `Document this ${rootLabel.toLowerCase()} so the product architecture stays readable.`}
          </div>
          <div style={styles.pathText}>{productName} / {renderedProductAreaTree.product_area.name}</div>
        </div>
        <div style={styles.summaryRight}>
          <div style={styles.summaryPillRow}>
            <span style={styles.summaryPill}>
              {loadState === "loading"
                ? "Loading sections"
                : shouldLazyLoad && loadState === "idle"
                  ? "Open to load sections"
                  : `${loadedChildCount} ${loadedChildCount === 1 ? "child node" : "child nodes"}`}
            </span>
            {shouldLazyLoad && loadState !== "loaded" ? null : <MetricPills metrics={metrics} />}
          </div>
          <div style={styles.nodeActionRow}>
            <button
              style={styles.plannerButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlanFromItem({ kind: "enhance_product_area", product, product_area: renderedProductAreaTree.product_area });
              }}
            >
              Improve Chapter
            </button>
            <button
              style={styles.subtleButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlanFromItem({ kind: "add_product_area_child", product, product_area: renderedProductAreaTree.product_area });
              }}
            >
              Add Section
            </button>
            <button
              style={styles.subtleButton}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onEditProductArea(renderedProductAreaTree.product_area);
              }}
            >
              Edit {rootLabel}
            </button>
          </div>
        </div>
      </div>

      {isOpen ? (
        <div style={styles.detailsBody}>
          {renderedProductAreaTree.product_area.purpose
            || renderedProductAreaTree.product_area.explanation
            || renderedProductAreaTree.product_area.examples
            || renderedProductAreaTree.product_area.implementation_notes
            || renderedProductAreaTree.product_area.test_guidance ? (
            <div style={styles.noteGrid}>
              {renderedProductAreaTree.product_area.purpose ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Purpose</div>
                  <div style={styles.noteText}>{renderedProductAreaTree.product_area.purpose}</div>
                </div>
              ) : null}
              {renderedProductAreaTree.product_area.explanation ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Explanation</div>
                  <div style={styles.noteText}>{renderedProductAreaTree.product_area.explanation}</div>
                </div>
              ) : null}
              {renderedProductAreaTree.product_area.examples ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Examples</div>
                  <div style={styles.noteText}>{renderedProductAreaTree.product_area.examples}</div>
                </div>
              ) : null}
              {renderedProductAreaTree.product_area.implementation_notes ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Implementation Notes</div>
                  <div style={styles.noteText}>{renderedProductAreaTree.product_area.implementation_notes}</div>
                </div>
              ) : null}
              {renderedProductAreaTree.product_area.test_guidance ? (
                <div style={styles.noteCard}>
                  <div style={styles.noteHeading}>Test Guidance</div>
                  <div style={styles.noteText}>{renderedProductAreaTree.product_area.test_guidance}</div>
                </div>
              ) : null}
            </div>
          ) : null}

          <ReferenceList references={productAreaReferences} title={`${rootLabel} References`} />

          {productAreaWorkItems.length > 0 ? (
            <div>
              <div style={styles.sectionTitle}>Direct Work</div>
              <WorkItemTree product={product} nodes={productAreaWorkItems} onOpenWorkItem={onOpenWorkItem} onPlanFromItem={onPlanFromItem} />
            </div>
          ) : null}

          {loadState === "loading" ? (
            <div style={styles.empty}>Loading chapter sections...</div>
          ) : loadState === "error" ? (
            <div style={styles.empty}>Unable to load chapter sections. Collapse and reopen the chapter to try again.</div>
          ) : renderedProductAreaTree.features.length > 0 ? (
            renderedProductAreaTree.features.map((capabilityTree, index) => (
              <CapabilityChapter
                key={capabilityTree.capability.id}
                product={product}
                path={[productName, renderedProductAreaTree.product_area.name]}
                capabilityTree={capabilityTree}
                numbering={`${chapterNumber}.${index + 1}`}
                allWorkItems={allWorkItems}
                references={references}
                onEditCapability={onEditCapability}
                onOpenWorkItem={onOpenWorkItem}
                onPlanFromItem={onPlanFromItem}
              />
            ))
          ) : (
            <div style={styles.empty}>No capabilities defined for this product area yet.</div>
          )}
        </div>
      ) : null}
    </section>
  );
}

function CapabilityChapter({
  product,
  path,
  capabilityTree,
  numbering,
  allWorkItems,
  references,
  onEditCapability,
  onOpenWorkItem,
  onPlanFromItem,
}: {
  product: Product;
  path: string[];
  capabilityTree: CapabilityTree;
  numbering: string;
  allWorkItems: WorkItem[];
  references: ProductReference[];
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
  const noteText = node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria;
  const excerpt = noteText ? summarizeText(noteText) : "";

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
          <div style={{ ...styles.metaRow, marginTop: 6 }}>
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
          Improve
        </button>
      </div>
      {excerpt ? <div style={styles.workItemText}>{excerpt}</div> : null}
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
    return <span style={styles.summaryPill}>No stories</span>;
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

function ReferenceList({ references, title }: { references: ProductReference[]; title: string }) {
  if (references.length === 0) {
    return null;
  }

  return (
    <div>
      <div style={styles.sectionTitle}>{title}</div>
      <div style={styles.referenceList}>
        {references.map((reference) => (
          <div key={reference.id} style={styles.referenceCard}>
            <div style={styles.noteHeading}>{getReferenceKindLabel(reference.reference_kind)}</div>
            <h5 style={styles.referenceTitle}>{reference.title}</h5>
            {reference.content ? <div style={{ ...styles.noteText, marginTop: 8 }}>{reference.content}</div> : null}
            {reference.uri ? (
              <a style={styles.referenceUri} href={reference.uri} target="_blank" rel="noreferrer">
                {reference.uri}
              </a>
            ) : null}
          </div>
        ))}
      </div>
    </div>
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

function getProductAreaScopedWorkItems(productAreaTree: ProductAreaTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds(productAreaTree.features);
  return allWorkItems.filter(
    (workItem) => workItem.product_area_id === productAreaTree.product_area.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false),
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
