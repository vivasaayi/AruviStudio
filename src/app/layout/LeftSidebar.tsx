import React, { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { countHierarchyNodes, countLeafNodes, findHierarchyNodePath, flattenHierarchyNodes, getDirectWorkItemsForNode, getHierarchyNodeKey, getProductDirectWorkItems, getSubtreeWorkItemsForNode } from "../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel, supportsHierarchyChildren } from "../../lib/hierarchyLabels";
import { getProductTree, listProducts, listWorkItems, summarizeWorkItemsByProduct } from "../../lib/tauri";
import { useWorkspaceStore } from "../../state/workspaceStore";
import { useUIStore } from "../../state/uiStore";
import type { HierarchyNodeKind, HierarchyTreeNode, Product, ProductTree, WorkItem } from "../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  container: { height: "100%", backgroundColor: "#17191d", overflow: "auto", padding: 10 },
  header: { fontSize: 11, fontWeight: 700, textTransform: "uppercase" as const, color: "#8f96a3", letterSpacing: 1, marginBottom: 10 },
  summary: { padding: 12, borderRadius: 10, border: "1px solid #30343c", backgroundColor: "#20242a", marginBottom: 12 },
  summaryValue: { fontSize: 18, fontWeight: 800, color: "#ffffff" },
  summaryMeta: { fontSize: 11, color: "#8f96a3", marginTop: 4 },
  productItem: { padding: "10px 12px", fontSize: 13, cursor: "pointer", borderRadius: 10, marginBottom: 6, border: "1px solid #30343c", backgroundColor: "#20242a" },
  productItemActive: { padding: "10px 12px", fontSize: 13, cursor: "pointer", borderRadius: 10, marginBottom: 6, border: "1px solid #0e639c", background: "linear-gradient(135deg, rgba(14,99,156,0.18), rgba(32,36,42,1))" },
  productName: { fontWeight: 700, color: "#f3f3f3", marginBottom: 4 },
  productMeta: { fontSize: 11, color: "#8f96a3" },
  pickerSearch: { width: "100%", padding: "9px 10px", fontSize: 13, backgroundColor: "#141820", color: "#f1f3f7", border: "1px solid #3b4049", borderRadius: 10, marginBottom: 8, boxSizing: "border-box" as const },
  pickerPanel: { border: "1px solid #30343c", borderRadius: 10, backgroundColor: "#141820", marginBottom: 8, overflow: "auto" as const, maxHeight: 320 },
  pickerGroup: { padding: 8, borderTop: "1px solid #232831" },
  pickerGroupTitle: { fontSize: 10, fontWeight: 800, letterSpacing: 0.8, textTransform: "uppercase" as const, color: "#8f96a3", marginBottom: 6 },
  pickerOption: { width: "100%", textAlign: "left" as const, padding: "8px 10px", borderRadius: 8, border: "1px solid transparent", backgroundColor: "transparent", color: "#dbe2ed", cursor: "pointer", display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", marginBottom: 4 },
  pickerOptionActive: { width: "100%", textAlign: "left" as const, padding: "8px 10px", borderRadius: 8, border: "1px solid #0e639c", background: "linear-gradient(135deg, rgba(14,99,156,0.18), rgba(32,36,42,1))", color: "#ffffff", cursor: "pointer", display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", marginBottom: 4 },
  pickerOptionMeta: { fontSize: 10, color: "#8f96a3" },
  exampleBadge: { fontSize: 10, fontWeight: 700, color: "#d7ba7d", border: "1px solid #5a5034", borderRadius: 999, padding: "2px 6px", backgroundColor: "#2a2619" },
  selectedProductCard: { padding: "10px 12px", borderRadius: 10, border: "1px solid #0e639c", background: "linear-gradient(135deg, rgba(14,99,156,0.16), rgba(32,36,42,1))", marginBottom: 8 },
  treeSection: { marginTop: 14 },
  treeSearch: { width: "100%", padding: "8px 10px", fontSize: 12, backgroundColor: "#141820", color: "#f1f3f7", border: "1px solid #3b4049", borderRadius: 10, boxSizing: "border-box" as const },
  filterSelect: { width: "100%", padding: "8px 10px", fontSize: 12, backgroundColor: "#141820", color: "#f1f3f7", border: "1px solid #3b4049", borderRadius: 10, boxSizing: "border-box" as const },
  treeControls: { display: "grid", gridTemplateColumns: "1fr", gap: 8, marginBottom: 10 },
  toolRow: { display: "flex", gap: 6, flexWrap: "wrap" as const },
  treeRoot: { display: "flex", flexDirection: "column", gap: 6 },
  node: { borderRadius: 8, border: "1px solid transparent", backgroundColor: "#1d2025", color: "#ced4de", cursor: "pointer", padding: "8px 10px" },
  nodeActive: { borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#1f2a35", color: "#ffffff", cursor: "pointer", padding: "8px 10px" },
  nodeHeader: { display: "flex", alignItems: "flex-start", gap: 8 },
  nodeToggle: { width: 20, height: 20, borderRadius: 6, border: "1px solid #38404d", backgroundColor: "#141820", color: "#cfd6e4", fontSize: 10, cursor: "pointer", flexShrink: 0 },
  nodeBody: { flex: 1, minWidth: 0 },
  nodeTitleRow: { display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" as const },
  nodeTitle: { fontSize: 12, fontWeight: 600 },
  nodeKindPill: { fontSize: 10, fontWeight: 700, color: "#8fc8ff", borderRadius: 999, padding: "2px 6px", border: "1px solid #36516e", backgroundColor: "#1a2736" },
  nodeMeta: { fontSize: 10, color: "#8f96a3", marginTop: 3 },
  childWrap: { marginLeft: 12, paddingLeft: 8, borderLeft: "1px solid #2c3139", display: "flex", flexDirection: "column", gap: 6, marginTop: 6 },
  taskItem: { borderRadius: 8, border: "1px solid #2c3139", backgroundColor: "#1a1d22", color: "#d8dde6", cursor: "pointer", padding: "7px 9px" },
  taskItemActive: { borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#1c2733", color: "#ffffff", cursor: "pointer", padding: "7px 9px" },
  taskTitle: { fontSize: 11, fontWeight: 600 },
  taskMeta: { fontSize: 10, color: "#8f96a3", marginTop: 3 },
  recentSection: { border: "1px solid #30343c", borderRadius: 10, backgroundColor: "#141820", padding: 8, marginBottom: 10 },
  recentWrap: { display: "flex", flexDirection: "column", gap: 6 },
  recentBtn: { width: "100%", textAlign: "left" as const, padding: "7px 8px", borderRadius: 8, border: "1px solid #2f3641", backgroundColor: "#1b2028", color: "#d8e1ef", cursor: "pointer", fontSize: 11 },
  empty: { fontSize: 12, color: "#666", padding: 8 },
  headerRow: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, marginBottom: 10 },
  addBtn: { padding: "4px 8px", borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#e0e0e0", fontSize: 11, fontWeight: 700, cursor: "pointer" },
  collapseBtn: { padding: "4px 8px", borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#181c23", color: "#aeb7c5", fontSize: 11, fontWeight: 700, cursor: "pointer" },
  actionRow: { display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" },
  actionBtn: { padding: "3px 6px", borderRadius: 6, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#e0e0e0", fontSize: 10, fontWeight: 700, cursor: "pointer" },
  actionRowHidden: { opacity: 0, pointerEvents: "none" as const },
  actionRowVisible: { opacity: 1 },
};

export function LeftSidebar() {
  const navigate = useNavigate();
  const {
    activeProductId,
    activeModuleId,
    activeCapabilityId,
    activeNodeId,
    activeNodeType,
    activeWorkItemId,
    setActiveProduct,
    setActiveModule,
    setActiveCapability,
    setActiveHierarchyNode,
    setActiveWorkItem,
  } = useWorkspaceStore();
  const {
    expandedModules,
    expandedCapabilities,
    showHierarchyWorkItems,
    toggleModuleExpanded,
    toggleCapabilityExpanded,
    toggleHierarchyWorkItems,
    setModuleExpanded,
    setCapabilityExpanded,
    setProductWorkspaceTab,
    openProductDialog,
    openModuleDialog,
    openCapabilityDialog,
    openWorkItemCreateDialog,
    productPickerCollapsed,
    setWorkItemWorkspaceTab,
    setActiveView,
    toggleProductPickerCollapsed,
  } = useUIStore();

  const [productSearch, setProductSearch] = useState("");
  const [treeSearchTerm, setTreeSearchTerm] = useState("");
  const [nodeKindFilter, setNodeKindFilter] = useState<HierarchyNodeKind | "">("");
  const [recentNodeKeys, setRecentNodeKeys] = useState<string[]>([]);
  const nodeRefs = React.useRef<Record<string, HTMLDivElement | null>>({});

  const { data: products } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const selectedProductId = activeProductId ?? products?.[0]?.id ?? null;
  const { data: workItems } = useQuery({ queryKey: ["sidebarWorkItems", selectedProductId], queryFn: () => listWorkItems({ productId: selectedProductId ?? undefined }), enabled: !!selectedProductId });
  const { data: productSummaries } = useQuery({ queryKey: ["productWorkItemSummaries"], queryFn: summarizeWorkItemsByProduct });
  const { data: tree } = useQuery({
    queryKey: ["sidebarProductTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProductId,
  });

  useEffect(() => {
    if (!activeProductId && products?.[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  const selectedProduct = (products ?? []).find((product) => product.id === selectedProductId) ?? null;
  const filteredProducts = useMemo(() => {
    const normalized = productSearch.trim().toLowerCase();
    if (!normalized) {
      return products ?? [];
    }
    return (products ?? []).filter((product: Product) => {
      const haystack = [product.name, product.description, ...product.tags].join(" ").toLowerCase();
      return haystack.includes(normalized);
    });
  }, [productSearch, products]);
  const groupedProducts = useMemo(() => {
    const examples: Product[] = [];
    const custom: Product[] = [];
    filteredProducts.forEach((product: Product) => {
      if (product.tags.includes("example_product")) {
        examples.push(product);
      } else {
        custom.push(product);
      }
    });
    return { custom, examples };
  }, [filteredProducts]);
  const productCounts = useMemo(() => {
    const counts = new Map<string, { total: number; active: number }>();
    (productSummaries ?? []).forEach((summary) => {
      counts.set(summary.product_id, {
        total: summary.total_count,
        active: summary.active_count,
      });
    });
    return counts;
  }, [productSummaries]);
  const scopedSidebarWorkItems = useMemo(
    () => (workItems ?? []).filter((workItem) => workItem.product_id === selectedProductId),
    [selectedProductId, workItems],
  );
  const activeWorkItemCount = selectedProductId
    ? productCounts.get(selectedProductId)?.active ?? 0
    : 0;
  const allTreeNodes = useMemo(() => (tree ? flattenHierarchyNodes(tree.roots) : []), [tree]);
  const nodeLookup = useMemo(
    () => new Map(allTreeNodes.map((node) => [getHierarchyNodeKey(node), node])),
    [allTreeNodes],
  );
  const selectedNodeKey = activeNodeId && activeNodeType ? `${activeNodeType}:${activeNodeId}` : null;
  const selectedNodePath = useMemo(
    () => (tree ? findHierarchyNodePath(tree.roots, activeNodeId, activeNodeType) : []),
    [tree, activeNodeId, activeNodeType],
  );
  const selectedPathKeySet = useMemo(
    () => new Set(selectedNodePath.map((node) => getHierarchyNodeKey(node))),
    [selectedNodePath],
  );
  const rootSectionCount = tree?.roots.length ?? 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const leafNodeCount = tree ? countLeafNodes(tree.roots) : 0;
  const nodeKindOptions = useMemo(
    () => Array.from(new Set(allTreeNodes.map((node) => node.node_kind))),
    [allTreeNodes],
  );
  const hasTreeFilter = treeSearchTerm.trim().length > 0 || nodeKindFilter.length > 0;

  const filteredRoots = useMemo(() => {
    if (!tree) {
      return [];
    }
    if (!hasTreeFilter) {
      return tree.roots;
    }

    const normalizedSearch = treeSearchTerm.trim().toLowerCase();
    const filterNode = (node: HierarchyTreeNode): HierarchyTreeNode | null => {
      const childMatches = node.children
        .map(filterNode)
        .filter(Boolean) as HierarchyTreeNode[];
      const matchesSearch = normalizedSearch.length === 0
        || [node.name, ...node.path, node.description, node.summary].join(" ").toLowerCase().includes(normalizedSearch);
      const matchesKind = !nodeKindFilter || node.node_kind === nodeKindFilter;
      if ((matchesSearch && matchesKind) || childMatches.length > 0) {
        return {
          ...node,
          children: childMatches,
        };
      }
      return null;
    };

    return tree.roots
      .map(filterNode)
      .filter(Boolean) as HierarchyTreeNode[];
  }, [hasTreeFilter, nodeKindFilter, tree, treeSearchTerm]);

  useEffect(() => {
    if (!selectedNodeKey) {
      return;
    }
    setRecentNodeKeys((current) => [selectedNodeKey, ...current.filter((key) => key !== selectedNodeKey)].slice(0, 6));
  }, [selectedNodeKey]);

  const goToProductStructure = () => {
    setActiveView("products");
    setProductWorkspaceTab("structure");
    navigate("/products");
  };

  const goToTaskIntake = () => {
    setActiveView("work-items");
    setWorkItemWorkspaceTab("backlog");
    navigate("/work-items");
    openWorkItemCreateDialog();
  };

  const setNodeExpandedState = (node: HierarchyTreeNode, expanded: boolean) => {
    if (node.node_type === "module") {
      setModuleExpanded(node.id, expanded);
      return;
    }
    setCapabilityExpanded(node.id, expanded);
  };

  const collapseAllNodes = () => {
    allTreeNodes.forEach((node) => setNodeExpandedState(node, false));
  };

  const expandSelectedPath = () => {
    selectedNodePath.forEach((node) => setNodeExpandedState(node, true));
  };

  const jumpToSelectedNode = () => {
    if (!selectedNodeKey) {
      return;
    }
    expandSelectedPath();
    requestAnimationFrame(() => {
      nodeRefs.current[selectedNodeKey]?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  };

  const openHierarchyNode = (node: HierarchyTreeNode) => {
    setActiveHierarchyNode({
      nodeId: node.id,
      nodeType: node.node_type,
      moduleId: node.module_id,
      capabilityId: node.capability_id,
    });
    goToProductStructure();
  };

  const renderTreeNode = (node: HierarchyTreeNode, depth = 0): React.ReactNode => {
    const nodeKey = getHierarchyNodeKey(node);
    const isActive = selectedNodeKey === nodeKey;
    const isExpanded = hasTreeFilter
      ? true
      : node.node_type === "module"
        ? expandedModules[node.id] ?? true
        : expandedCapabilities[node.id] ?? true;
    const directWorkItemCount = getDirectWorkItemsForNode(node, scopedSidebarWorkItems).length;
    const totalWorkItemCount = getSubtreeWorkItemsForNode(node, scopedSidebarWorkItems).length;

    return (
      <div key={nodeKey}>
        <div
          ref={(element) => {
            nodeRefs.current[nodeKey] = element;
          }}
          style={{
            ...(isActive ? styles.nodeActive : styles.node),
            marginLeft: depth * 10,
          }}
        >
          <div style={styles.nodeHeader}>
            {node.children.length > 0 ? (
              <button
                style={styles.nodeToggle}
                onClick={(event) => {
                  event.stopPropagation();
                  if (node.node_type === "module") {
                    toggleModuleExpanded(node.id);
                  } else {
                    toggleCapabilityExpanded(node.id);
                  }
                }}
              >
                {isExpanded ? "−" : "+"}
              </button>
            ) : (
              <div style={styles.nodeToggle}>•</div>
            )}
            <div style={styles.nodeBody} onClick={() => openHierarchyNode(node)}>
              <div style={styles.nodeTitleRow}>
                <div style={styles.nodeTitle}>{node.name}</div>
                <span style={styles.nodeKindPill}>{getHierarchyNodeKindLabel(node.node_kind)}</span>
              </div>
              <div style={styles.nodeMeta}>
                {node.children.length} {node.children.length === 1 ? "child" : "children"} · {directWorkItemCount} direct work items · {totalWorkItemCount} total
              </div>
              {node.summary || node.description ? <div style={styles.nodeMeta}>{node.summary || node.description}</div> : null}
            </div>
          </div>
          <div style={styles.actionRow}>
            <button
              style={styles.actionBtn}
              onClick={(event) => {
                event.stopPropagation();
                openHierarchyNode(node);
                goToTaskIntake();
              }}
            >
              + Work Item
            </button>
            {supportsHierarchyChildren(node.node_kind) ? (
              <button
                style={styles.actionBtn}
                onClick={(event) => {
                  event.stopPropagation();
                  openHierarchyNode(node);
                  openCapabilityDialog("create");
                }}
              >
                + Child Node
              </button>
            ) : null}
            <button
              style={styles.actionBtn}
              onClick={(event) => {
                event.stopPropagation();
                openHierarchyNode(node);
                if (node.node_type === "module") {
                  openModuleDialog("edit");
                } else {
                  openCapabilityDialog("edit");
                }
              }}
            >
              Edit
            </button>
          </div>
        </div>
        {isExpanded && node.children.length > 0 ? (
          <div style={styles.childWrap}>
            {node.children.map((child) => renderTreeNode(child, depth + 1))}
            {showHierarchyWorkItems
              ? getDirectWorkItemsForNode(node, scopedSidebarWorkItems).slice(0, 6).map((workItem) => (
                  <div
                    key={workItem.id}
                    style={activeWorkItemId === workItem.id ? styles.taskItemActive : styles.taskItem}
                    onClick={() => setActiveWorkItem(workItem.id)}
                  >
                    <div style={styles.taskTitle}>{workItem.title}</div>
                    <div style={styles.taskMeta}>{workItem.status.replace(/_/g, " ")}</div>
                  </div>
                ))
              : null}
          </div>
        ) : null}
      </div>
    );
  };

  return (
    <div style={styles.container}>
      <div style={styles.summary}>
        <div style={styles.header}>{selectedProduct ? selectedProduct.name : "Workspace"}</div>
        <div style={styles.summaryValue}>{selectedProduct ? totalNodeCount : products?.length ?? 0}</div>
        <div style={styles.summaryMeta}>
          {selectedProduct
            ? `${rootSectionCount} root sections · ${leafNodeCount} leaf nodes · ${activeWorkItemCount} active work items`
            : "Visible products"}
        </div>
      </div>

      <div style={styles.headerRow}>
        <div style={{ ...styles.header, marginBottom: 0 }}>Products</div>
        <div style={{ display: "flex", gap: 6 }}>
          <button style={styles.collapseBtn} onClick={toggleProductPickerCollapsed}>
            {productPickerCollapsed ? "Expand" : "Collapse"}
          </button>
          <button
            style={styles.addBtn}
            onClick={() => {
              setActiveView("products");
              navigate("/products");
              openProductDialog("create");
            }}
          >
            + New
          </button>
        </div>
      </div>
      {products && products.length > 0 ? (
        <>
          {!productPickerCollapsed ? (
            <>
              <input
                style={styles.pickerSearch}
                value={productSearch}
                onChange={(e) => setProductSearch(e.target.value)}
                placeholder="Find a product"
              />
              <div style={styles.pickerPanel}>
                {groupedProducts.custom.length > 0 ? (
                  <div style={styles.pickerGroup}>
                    <div style={styles.pickerGroupTitle}>Your Products</div>
                    {groupedProducts.custom.map((product) => {
                      const counts = productCounts.get(product.id) ?? { total: 0, active: 0 };
                      return (
                        <button
                          key={product.id}
                          style={selectedProductId === product.id ? styles.pickerOptionActive : styles.pickerOption}
                          onClick={() => setActiveProduct(product.id)}
                        >
                          <span>{product.name}</span>
                          <span style={styles.pickerOptionMeta}>{counts.active} active · {counts.total} total</span>
                        </button>
                      );
                    })}
                  </div>
                ) : null}
                {groupedProducts.examples.length > 0 ? (
                  <div style={styles.pickerGroup}>
                    <div style={styles.pickerGroupTitle}>Example Products</div>
                    {groupedProducts.examples.map((product) => {
                      const counts = productCounts.get(product.id) ?? { total: 0, active: 0 };
                      return (
                        <button
                          key={product.id}
                          style={selectedProductId === product.id ? styles.pickerOptionActive : styles.pickerOption}
                          onClick={() => setActiveProduct(product.id)}
                        >
                          <span>{product.name}</span>
                          <span style={{ display: "flex", gap: 6, alignItems: "center" }}>
                            <span style={styles.pickerOptionMeta}>{counts.active} active</span>
                            <span style={styles.exampleBadge}>Example</span>
                          </span>
                        </button>
                      );
                    })}
                  </div>
                ) : null}
                {groupedProducts.custom.length === 0 && groupedProducts.examples.length === 0 ? (
                  <div style={styles.empty}>No matching products.</div>
                ) : null}
              </div>
            </>
          ) : null}
          {selectedProduct && productPickerCollapsed ? (
            <div style={styles.selectedProductCard}>
              <div style={styles.productName}>{selectedProduct.name}</div>
              <div style={styles.productMeta}>
                {selectedProduct.status} · {selectedProduct.tags.includes("example_product") ? "Example product" : selectedProduct.tags.length > 0 ? selectedProduct.tags.join(", ") : "No tags"}
              </div>
              <div style={styles.actionRow}>
                <button
                  style={styles.actionBtn}
                  onClick={() => {
                    setActiveProduct(selectedProduct.id);
                    setActiveModule(null);
                    setActiveCapability(null);
                    goToProductStructure();
                    openModuleDialog("create");
                  }}
                >
                  + Root Section
                </button>
                <button
                  style={styles.actionBtn}
                  onClick={() => {
                    setActiveProduct(selectedProduct.id);
                    setActiveModule(null);
                    setActiveCapability(null);
                    setActiveView("products");
                    navigate("/products");
                    openProductDialog("edit");
                  }}
                >
                  Edit
                </button>
              </div>
            </div>
          ) : null}
        </>
      ) : (
        <div style={styles.empty}>No visible products yet. Use + New or turn off Hide Example Products in Settings.</div>
      )}

      {tree && (
        <div style={styles.treeSection}>
          <div style={styles.headerRow}>
            <div style={{ ...styles.header, marginBottom: 0 }}>Outline</div>
            <button style={styles.addBtn} onClick={toggleHierarchyWorkItems}>
              {showHierarchyWorkItems ? "Hide Work Items" : "Show Work Items"}
            </button>
          </div>
          <div style={styles.treeControls}>
            <input
              style={styles.treeSearch}
              value={treeSearchTerm}
              onChange={(event) => setTreeSearchTerm(event.target.value)}
              placeholder="Search nodes"
            />
            <select
              style={styles.filterSelect}
              value={nodeKindFilter}
              onChange={(event) => setNodeKindFilter(event.target.value as HierarchyNodeKind | "")}
            >
              <option value="">All node kinds</option>
              {nodeKindOptions.map((nodeKind) => (
                <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
              ))}
            </select>
            <div style={styles.toolRow}>
              <button style={styles.collapseBtn} onClick={collapseAllNodes}>Collapse All</button>
              <button style={styles.collapseBtn} onClick={expandSelectedPath} disabled={!selectedNodeKey}>Expand Path</button>
              <button style={styles.collapseBtn} onClick={jumpToSelectedNode} disabled={!selectedNodeKey}>Jump To Selected</button>
            </div>
          </div>
          {recentNodeKeys.length > 0 ? (
            <div style={styles.recentSection}>
              <div style={styles.pickerGroupTitle}>Recent Nodes</div>
              <div style={styles.recentWrap}>
                {recentNodeKeys
                  .map((key) => nodeLookup.get(key))
                  .filter((node): node is HierarchyTreeNode => Boolean(node))
                  .map((node) => (
                    <button
                      key={getHierarchyNodeKey(node)}
                      style={styles.recentBtn}
                      onClick={() => {
                        openHierarchyNode(node);
                        requestAnimationFrame(() => {
                          nodeRefs.current[getHierarchyNodeKey(node)]?.scrollIntoView({ block: "center", behavior: "smooth" });
                        });
                      }}
                    >
                      {node.path.join(" / ")}
                    </button>
                  ))}
              </div>
            </div>
          ) : null}
          <div style={styles.treeRoot}>
            {filteredRoots.length > 0 ? (
              filteredRoots.map((node) => renderTreeNode(node))
            ) : (
              <div style={styles.empty}>
                {hasTreeFilter ? "No nodes match the current filters." : "Create root sections to begin building the hierarchy."}
              </div>
            )}
            {showHierarchyWorkItems && getProductDirectWorkItems(scopedSidebarWorkItems).length > 0 ? (
              <div style={styles.recentSection}>
                <div style={styles.pickerGroupTitle}>Product Work Items</div>
                {getProductDirectWorkItems(scopedSidebarWorkItems).slice(0, 6).map((workItem) => (
                  <div
                    key={workItem.id}
                    style={activeWorkItemId === workItem.id ? styles.taskItemActive : styles.taskItem}
                    onClick={() => setActiveWorkItem(workItem.id)}
                  >
                    <div style={styles.taskTitle}>{workItem.title}</div>
                    <div style={styles.taskMeta}>{workItem.status.replace(/_/g, " ")}</div>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}
