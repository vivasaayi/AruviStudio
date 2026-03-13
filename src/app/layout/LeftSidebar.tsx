import React, { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { getProductTree, listProducts, listWorkItems, summarizeWorkItemsByProduct } from "../../lib/tauri";
import { useWorkspaceStore } from "../../state/workspaceStore";
import { useUIStore } from "../../state/uiStore";
import type { CapabilityTree, Product, ProductTree, WorkItem } from "../../lib/types";

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
  treeRoot: { display: "flex", flexDirection: "column", gap: 6 },
  node: { borderRadius: 8, border: "1px solid transparent", backgroundColor: "#1d2025", color: "#ced4de", cursor: "pointer", padding: "8px 10px" },
  nodeActive: { borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#1f2a35", color: "#ffffff", cursor: "pointer", padding: "8px 10px" },
  nodeTitle: { fontSize: 12, fontWeight: 600 },
  nodeMeta: { fontSize: 10, color: "#8f96a3", marginTop: 3 },
  childWrap: { marginLeft: 12, paddingLeft: 8, borderLeft: "1px solid #2c3139", display: "flex", flexDirection: "column", gap: 6, marginTop: 6 },
  taskItem: { borderRadius: 8, border: "1px solid #2c3139", backgroundColor: "#1a1d22", color: "#d8dde6", cursor: "pointer", padding: "7px 9px" },
  taskItemActive: { borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#1c2733", color: "#ffffff", cursor: "pointer", padding: "7px 9px" },
  taskTitle: { fontSize: 11, fontWeight: 600 },
  taskMeta: { fontSize: 10, color: "#8f96a3", marginTop: 3 },
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
    activeWorkItemId,
    setActiveProduct,
    setActiveModule,
    setActiveCapability,
    setActiveWorkItem,
  } = useWorkspaceStore();
  const {
    expandedModules,
    expandedCapabilities,
    showHierarchyWorkItems,
    toggleModuleExpanded,
    toggleCapabilityExpanded,
    toggleHierarchyWorkItems,
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

  const [hoveredModuleId, setHoveredModuleId] = useState<string | null>(null);
  const [hoveredCapabilityId, setHoveredCapabilityId] = useState<string | null>(null);
  const [productSearch, setProductSearch] = useState("");

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
  const workItemsByModule = useMemo(() => {
    const map = new Map<string, WorkItem[]>();
    scopedSidebarWorkItems.forEach((workItem) => {
      const key = workItem.module_id ?? "unscoped";
      const existing = map.get(key) ?? [];
      existing.push(workItem);
      map.set(key, existing);
    });
    return map;
  }, [scopedSidebarWorkItems]);
  const workItemsByCapability = useMemo(() => {
    const map = new Map<string, WorkItem[]>();
    scopedSidebarWorkItems.forEach((workItem) => {
      if (!workItem.capability_id) {
        return;
      }
      const existing = map.get(workItem.capability_id) ?? [];
      existing.push(workItem);
      map.set(workItem.capability_id, existing);
    });
    return map;
  }, [scopedSidebarWorkItems]);

  const getCapabilityAggregateMeta = useMemo(() => {
    const cache = new Map<string, { outcomeCount: number; workItemCount: number }>();

    const walk = (capabilityTree: CapabilityTree): { outcomeCount: number; workItemCount: number } => {
      const cached = cache.get(capabilityTree.capability.id);
      if (cached) {
        return cached;
      }
      const directWorkItemCount = (workItemsByCapability.get(capabilityTree.capability.id) ?? []).length;
      const childMeta = capabilityTree.children.map((child) => walk(child));
      const meta = {
        outcomeCount: capabilityTree.children.length,
        workItemCount: directWorkItemCount + childMeta.reduce((sum, child) => sum + child.workItemCount, 0),
      };
      cache.set(capabilityTree.capability.id, meta);
      return meta;
    };

    return walk;
  }, [workItemsByCapability]);

  const getModuleAggregateMeta = useMemo(() => {
    return (moduleTree: ProductTree["modules"][number]) => {
      const directWorkItemCount = (workItemsByModule.get(moduleTree.module.id) ?? [])
        .filter((workItem) => !workItem.capability_id)
        .length;
      const capabilityMeta = moduleTree.features.map((featureTree: CapabilityTree) => getCapabilityAggregateMeta(featureTree));
      return {
        capabilityCount: moduleTree.features.length,
        workItemCount: directWorkItemCount + capabilityMeta.reduce((sum: number, meta: { workItemCount: number }) => sum + meta.workItemCount, 0),
      };
    };
  }, [getCapabilityAggregateMeta, workItemsByModule]);

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

  return (
    <div style={styles.container}>
      <div style={styles.summary}>
        <div style={styles.header}>Workspace</div>
        <div style={styles.summaryValue}>{products?.length ?? 0}</div>
        <div style={styles.summaryMeta}>Products · {activeWorkItemCount} active work items in scope</div>
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
                  + Module
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
            <div style={{ ...styles.header, marginBottom: 0 }}>Hierarchy</div>
            <button style={styles.addBtn} onClick={toggleHierarchyWorkItems}>
              {showHierarchyWorkItems ? "Hide Work Items" : "Show Work Items"}
            </button>
          </div>
          <div style={styles.treeRoot}>
            {tree.modules.length > 0 ? (
              tree.modules.map((moduleTree) => {
                const moduleOpen = expandedModules[moduleTree.module.id] ?? true;
                const moduleTasks = workItemsByModule.get(moduleTree.module.id) ?? [];
                const moduleMeta = getModuleAggregateMeta(moduleTree);
                return (
                  <div key={moduleTree.module.id}>
                    <div
                      style={activeModuleId === moduleTree.module.id ? styles.nodeActive : styles.node}
                      onMouseEnter={() => setHoveredModuleId(moduleTree.module.id)}
                      onMouseLeave={() => setHoveredModuleId((current) => (current === moduleTree.module.id ? null : current))}
                      onClick={() => {
                        setActiveModule(moduleTree.module.id);
                        toggleModuleExpanded(moduleTree.module.id);
                      }}
                    >
                      <div style={styles.nodeTitle}>{moduleTree.module.name}</div>
                      <div style={styles.nodeMeta}>{moduleMeta.capabilityCount} capabilities · {moduleMeta.workItemCount} work items</div>
                      <div style={{ ...styles.actionRow, ...(hoveredModuleId === moduleTree.module.id ? styles.actionRowVisible : styles.actionRowHidden) }}>
                          <button
                            style={styles.actionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setActiveModule(moduleTree.module.id);
                              setActiveCapability(null);
                              goToTaskIntake();
                            }}
                          >
                            + Work Item
                          </button>
                          <button
                            style={styles.actionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setActiveModule(moduleTree.module.id);
                              setActiveCapability(null);
                              goToProductStructure();
                              openCapabilityDialog("create");
                            }}
                          >
                            + Capability
                          </button>
                          <button
                            style={styles.actionBtn}
                            onClick={(e) => {
                              e.stopPropagation();
                              setActiveModule(moduleTree.module.id);
                              setActiveCapability(null);
                              goToProductStructure();
                              openModuleDialog("edit");
                            }}
                          >
                            Edit
                          </button>
                      </div>
                    </div>
                    {moduleOpen && (
                      <div style={styles.childWrap}>
                        {moduleTree.features.map((capabilityTree) =>
                          renderCapabilityNode(capabilityTree, {
                            activeCapabilityId,
                            activeWorkItemId,
                            expandedCapabilities,
                            hoveredCapabilityId,
                            setHoveredCapabilityId,
                            toggleCapabilityExpanded,
                            setActiveCapability,
                            setActiveWorkItem,
                            setActiveModule,
                            openCapabilityDialog,
                            goToProductStructure,
                            goToTaskIntake,
                            workItemsByCapability,
                            getCapabilityAggregateMeta,
                            showHierarchyWorkItems,
                            depth: 0,
                            maxDepth: 1,
                          }),
                        )}
                        {showHierarchyWorkItems && moduleTasks.filter((workItem) => !workItem.capability_id).slice(0, 6).map((workItem) => (
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
                    )}
                  </div>
                );
              })
            ) : (
              <div style={styles.empty}>Create modules to begin building the hierarchy.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function renderCapabilityNode(
  capabilityTree: CapabilityTree,
  context: {
    activeCapabilityId: string | null;
    activeWorkItemId: string | null;
    expandedCapabilities: Record<string, boolean>;
    hoveredCapabilityId: string | null;
    setHoveredCapabilityId: React.Dispatch<React.SetStateAction<string | null>>;
    toggleCapabilityExpanded: (id: string) => void;
    setActiveCapability: (id: string | null) => void;
    setActiveWorkItem: (id: string | null) => void;
    setActiveModule: (id: string | null) => void;
    openCapabilityDialog: (mode: "create" | "edit") => void;
    goToProductStructure: () => void;
    goToTaskIntake: () => void;
    workItemsByCapability: Map<string, Array<{ id: string; title: string; status: string }>>;
    getCapabilityAggregateMeta: (capabilityTree: CapabilityTree) => { outcomeCount: number; workItemCount: number };
    showHierarchyWorkItems: boolean;
    depth: number;
    maxDepth: number;
  },
): React.ReactNode {
  const {
    activeCapabilityId,
    activeWorkItemId,
    expandedCapabilities,
    hoveredCapabilityId,
    setHoveredCapabilityId,
    toggleCapabilityExpanded,
    setActiveCapability,
    setActiveWorkItem,
    setActiveModule,
    openCapabilityDialog,
    goToProductStructure,
    goToTaskIntake,
    workItemsByCapability,
    getCapabilityAggregateMeta,
    showHierarchyWorkItems,
    depth,
    maxDepth,
  } = context;
  const isOpen = expandedCapabilities[capabilityTree.capability.id] ?? true;
  const workItems = workItemsByCapability.get(capabilityTree.capability.id) ?? [];
  const capabilityMeta = getCapabilityAggregateMeta(capabilityTree);

  return (
    <div key={capabilityTree.capability.id}>
      <div
        style={activeCapabilityId === capabilityTree.capability.id ? styles.nodeActive : styles.node}
        onMouseEnter={() => setHoveredCapabilityId(capabilityTree.capability.id)}
        onMouseLeave={() => setHoveredCapabilityId((current: string | null) => (current === capabilityTree.capability.id ? null : current))}
        onClick={() => {
          setActiveModule(capabilityTree.capability.module_id);
          setActiveCapability(capabilityTree.capability.id);
          toggleCapabilityExpanded(capabilityTree.capability.id);
        }}
      >
        <div style={styles.nodeTitle}>{capabilityTree.capability.name}</div>
        <div style={styles.nodeMeta}>{capabilityMeta.outcomeCount} outcomes · {capabilityMeta.workItemCount} work items</div>
        <div style={{ ...styles.actionRow, ...(hoveredCapabilityId === capabilityTree.capability.id ? styles.actionRowVisible : styles.actionRowHidden) }}>
            <button
              style={styles.actionBtn}
              onClick={(e) => {
                e.stopPropagation();
                setActiveModule(capabilityTree.capability.module_id);
                setActiveCapability(capabilityTree.capability.id);
                goToTaskIntake();
              }}
            >
              + Work Item
            </button>
            <button
              style={styles.actionBtn}
              onClick={(e) => {
                e.stopPropagation();
                setActiveModule(capabilityTree.capability.module_id);
                setActiveCapability(capabilityTree.capability.id);
                goToProductStructure();
                openCapabilityDialog("create");
              }}
            >
              + Outcome
            </button>
            <button
              style={styles.actionBtn}
              onClick={(e) => {
                e.stopPropagation();
                setActiveModule(capabilityTree.capability.module_id);
                setActiveCapability(capabilityTree.capability.id);
                goToProductStructure();
                openCapabilityDialog("edit");
              }}
            >
              Edit
            </button>
        </div>
      </div>
      {isOpen && (
        <div style={styles.childWrap}>
          {depth < maxDepth ? capabilityTree.children.map((child) => renderCapabilityNode(child, { ...context, depth: depth + 1 })) : null}
          {showHierarchyWorkItems && workItems.slice(0, 6).map((workItem) => (
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
      )}
    </div>
  );
}
