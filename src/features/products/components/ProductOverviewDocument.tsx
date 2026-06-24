import { useEffect, useMemo, useState } from "react";
import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, ProductArea, ProductAreaTree, Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getProductAreaSectionId,
  sortWorkItems,
  type WorkItemMetrics,
} from "../lib/productOverview";
import { filterReferencesForProductBook, filterReferencesForScope, getProductAreaReferenceScope } from "../lib/productReferences";
import { getProductAreaScopedWorkItems } from "../lib/productOverviewScopedWorkItems";
import { MetricPills, WorkItemTree } from "./ProductOverviewDelivery";
import { styles } from "./ProductOverviewDocument.styles";
import { CapabilityChapter } from "./ProductOverviewCapabilityChapter";
import { ReferenceList } from "./ProductOverviewReferences";

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
