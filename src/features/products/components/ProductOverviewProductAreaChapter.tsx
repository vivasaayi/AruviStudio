import { useEffect, useMemo, useState } from "react";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, Product, ProductArea, ProductAreaTree, ProductReference, WorkItem } from "../../../lib/types";
import {
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getProductAreaSectionId,
} from "../lib/productOverview";
import { getProductAreaScopedWorkItems } from "../lib/productOverviewScopedWorkItems";
import { filterReferencesForScope, getProductAreaReferenceScope } from "../lib/productReferences";
import type { ProductOverviewPlannerAction } from "./ProductOverviewDocument";
import { MetricPills, WorkItemTree } from "./ProductOverviewDelivery";
import { styles } from "./ProductOverviewDocument.styles";
import { CapabilityChapter } from "./ProductOverviewCapabilityChapter";
import { ReferenceList } from "./ProductOverviewReferences";

type ProductOverviewProductAreaChapterProps = {
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
};

export function ProductOverviewProductAreaChapter({
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
}: ProductOverviewProductAreaChapterProps) {
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
