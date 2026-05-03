import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import {
  archiveProduct,
  createCapability,
  createLocalWorkspace,
  createModule,
  createProduct,
  createWorkItem,
  getProductTree,
  listProducts,
  listWorkItems,
  reorderCapabilities,
  reorderModules,
  revealInFinder,
  resolveRepositoryForScope,
  updateCapability,
  updateModule,
  updateProduct,
} from "../../../lib/tauri";
import {
  countDescendantNodes,
  countHierarchyNodes,
  countLeafNodes,
  findHierarchyNode,
  findHierarchyNodePath,
  getDirectChildNodes,
  getDirectWorkItemsForNode,
  getHierarchyNodeSectionId,
  getProductDirectWorkItems,
  getSubtreeWorkItemsForNode,
  isDirectProductWorkItem,
} from "../../../lib/hierarchyTree";
import {
  getAllowedChildNodeKinds,
  getDefaultChildNodeKind,
  getHierarchyChildLabel,
  getHierarchyNodeKindLabel,
  ROOT_NODE_KINDS,
  supportsHierarchyChildren,
} from "../../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import type { CapabilityNode, CapabilityTree, HierarchyNodeKind, HierarchyTreeNode, ModuleTree, ProductTree, Repository, WorkItem } from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", height: "100%", gap: 12 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 },
  titleBlock: { display: "flex", flexDirection: "column", gap: 3 },
  title: { fontSize: 18, fontWeight: 800, color: "#f3f3f3", margin: 0 },
  subtitle: { fontSize: 12, color: "#8f96a3" },
  workspace: { display: "grid", gridTemplateColumns: "minmax(0, 1fr)", gap: 12, minHeight: 0, flex: 1 },
  panel: { backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 12, minHeight: 0, overflow: "hidden" },
  panelInner: { padding: 14, height: "100%", overflow: "auto" },
  tabBar: { display: "flex", gap: 8, marginBottom: 14, borderBottom: "1px solid #32353d", paddingBottom: 10 },
  tab: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#cfd6e4", cursor: "pointer" },
  tabActive: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#173247", color: "#ffffff", cursor: "pointer" },
  btn: { padding: "7px 12px", fontSize: 12, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  ghostBtn: { padding: "6px 10px", fontSize: 12, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3b4049", borderRadius: 8, cursor: "pointer" },
  btnDanger: { padding: "5px 10px", fontSize: 12, backgroundColor: "#6c2020", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  hero: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 220px", gap: 10, marginBottom: 12 },
  heroCard: { backgroundColor: "#26292f", borderRadius: 12, border: "1px solid #32353d", padding: 14 },
  heroName: { fontSize: 24, fontWeight: 800, color: "#ffffff", marginBottom: 6 },
  heroDesc: { fontSize: 13, color: "#aab2bf", lineHeight: 1.45 },
  metricGrid: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 },
  metricCard: { backgroundColor: "#26292f", borderRadius: 12, border: "1px solid #32353d", padding: 12 },
  metricLabel: { fontSize: 10, color: "#8f96a3", textTransform: "uppercase" as const, marginBottom: 4 },
  metricValue: { fontSize: 20, fontWeight: 700, color: "#f3f3f3" },
  section: { marginTop: 14 },
  sectionTitle: { fontSize: 13, fontWeight: 700, color: "#d8dde6", marginBottom: 10, display: "flex", alignItems: "center", justifyContent: "space-between" },
  contextCard: { border: "1px solid #32353d", borderRadius: 12, padding: 12, backgroundColor: "#26292f", marginBottom: 10 },
  contextTitle: { fontSize: 13, fontWeight: 700, color: "#f3f3f3", marginBottom: 6 },
  contextText: { fontSize: 12, color: "#aab2bf", lineHeight: 1.5 },
  contextLabel: { fontSize: 11, color: "#8f96a3", textTransform: "uppercase" as const, marginBottom: 4 },
  moduleCard: { border: "1px solid #32353d", borderRadius: 12, backgroundColor: "#26292f", padding: 12, marginBottom: 10 },
  moduleHeader: { display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start" },
  moduleName: { fontSize: 14, fontWeight: 700, color: "#f3f3f3", marginBottom: 4 },
  moduleDesc: { fontSize: 12, color: "#8f96a3", lineHeight: 1.45, marginBottom: 8 },
  featureNode: { padding: "10px 12px", borderRadius: 10, backgroundColor: "#1b1d22", border: "1px solid #2d3139", marginTop: 8 },
  featureNodeActive: { padding: "10px 12px", borderRadius: 10, backgroundColor: "#1f2a35", border: "1px solid #0e639c", marginTop: 8 },
  featureTitle: { fontWeight: 700, color: "#e9eef8", fontSize: 12 },
  featureMeta: { fontSize: 11, color: "#8f96a3", marginTop: 4 },
  childWrap: { marginLeft: 16 },
  inlineMeta: { display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 },
  badge: { fontSize: 11, padding: "3px 8px", borderRadius: 999, backgroundColor: "#163d2f", color: "#59d6b2" },
  badgeMuted: { fontSize: 11, padding: "3px 8px", borderRadius: 999, backgroundColor: "#2a3140", color: "#a9c4f5" },
  badgeKind: { fontSize: 11, padding: "3px 8px", borderRadius: 999, backgroundColor: "#223147", color: "#8fc8ff", border: "1px solid #38506f" },
  taskRow: { border: "1px solid #32353d", borderRadius: 10, padding: 10, backgroundColor: "#26292f", marginBottom: 8, cursor: "pointer" },
  taskTitle: { fontSize: 12, fontWeight: 700, color: "#f3f3f3" },
  taskMeta: { fontSize: 11, color: "#8f96a3", marginTop: 4 },
  metricHelp: { fontSize: 11, color: "#8f96a3", marginTop: 6 },
  chipRow: { display: "flex", gap: 8, flexWrap: "wrap" as const, marginTop: 10 },
  empty: { textAlign: "center" as const, color: "#666", padding: 32, fontSize: 14 },
  dropTarget: { outline: "1px dashed #0e639c", outlineOffset: 2 },
  dragHandle: { fontSize: 13, color: "#8f96a3", cursor: "grab", userSelect: "none" as const, padding: "2px 4px" },
  modalBackdrop: { position: "fixed", inset: 0, backgroundColor: "rgba(8, 10, 14, 0.72)", display: "flex", alignItems: "center", justifyContent: "center", padding: 24, zIndex: 40 },
  modal: { width: "min(720px, 100%)", maxHeight: "80vh", backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 14, overflow: "hidden", boxShadow: "0 24px 60px rgba(0,0,0,0.45)" },
  modalHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, padding: "14px 16px", borderBottom: "1px solid #32353d" },
  modalTitle: { fontSize: 14, fontWeight: 800, color: "#f3f3f3" },
  modalBody: { padding: 16, maxHeight: "calc(80vh - 61px)", overflow: "auto" },
  formRow: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  input: { width: "100%", padding: "9px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 13, marginBottom: 10, boxSizing: "border-box" as const },
  textarea: { width: "100%", padding: "9px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 13, marginBottom: 10, minHeight: 84, resize: "vertical" as const, boxSizing: "border-box" as const },
  label: { fontSize: 12, color: "#999", display: "block", marginBottom: 4 },
  errorText: { fontSize: 12, color: "#ff7b72", marginBottom: 10 },
  table: { border: "1px solid #32353d", borderRadius: 12, overflow: "hidden", backgroundColor: "#26292f" },
  tableHeader: { display: "grid", gridTemplateColumns: "minmax(0, 1.6fr) 110px 90px 110px", gap: 10, padding: "10px 12px", borderBottom: "1px solid #32353d", fontSize: 11, fontWeight: 700, letterSpacing: "0.06em", textTransform: "uppercase" as const, color: "#8f96a3" },
  tableRow: { display: "grid", gridTemplateColumns: "minmax(0, 1.6fr) 110px 90px 110px", gap: 10, padding: "12px", borderBottom: "1px solid #2d3139", alignItems: "center" },
  rowPrimary: { fontSize: 13, fontWeight: 700, color: "#f3f3f3" },
  rowSecondary: { fontSize: 12, color: "#8f96a3", marginTop: 4 },
  rowCell: { fontSize: 12, color: "#cfd6e4" },
};

export function ProductListPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const {
    activeProductId,
    activeModuleId,
    activeCapabilityId,
    activeNodeId,
    activeNodeType,
    activeWorkspacePath,
    setActiveProduct,
    setActiveModule,
    setActiveCapability,
    setActiveHierarchyNode,
    setActiveWorkItem,
  } = useWorkspaceStore();
  const {
    productDialogMode,
    moduleDialogMode,
    capabilityDialogMode,
    productWorkspaceTab,
    closeProductDialog,
    openProductDialog,
    closeModuleDialog,
    openModuleDialog,
    closeCapabilityDialog,
    openCapabilityDialog,
    setProductWorkspaceTab,
    setActiveView,
  } = useUIStore();

  const [showWorkItemForm, setShowWorkItemForm] = useState(false);
  const [productForm, setProductForm] = useState({ name: "", description: "", vision: "", goals: "", tags: "" });
  const [productDraft, setProductDraft] = useState({ name: "", description: "", vision: "", goals: "", tags: "" });
  const [moduleForm, setModuleForm] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "area" });
  const [moduleDraft, setModuleDraft] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "area" });
  const [capabilityForm, setCapabilityForm] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [capabilityDraft, setCapabilityDraft] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [workItemForm, setWorkItemForm] = useState({ title: "", description: "", problemStatement: "", acceptanceCriteria: "", constraints: "" });
  const [structureViewMode, setStructureViewMode] = useState<"children" | "work_items">("children");
  const [formError, setFormError] = useState<string | null>(null);
  const [workspaceActionMsg, setWorkspaceActionMsg] = useState<string | null>(null);
  const [workspaceActionError, setWorkspaceActionError] = useState<string | null>(null);
  const [draggedModuleId, setDraggedModuleId] = useState<string | null>(null);
  const [draggedFeature, setDraggedFeature] = useState<null | { id: string; moduleId: string; parentCapabilityId?: string | null; siblingIds: string[] }>(null);
  const [moduleOrderIds, setModuleOrderIds] = useState<string[]>([]);
  const [capabilityOrderMap, setFeatureOrderMap] = useState<Record<string, string[]>>({});

  const { data: products, isLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const visibleActiveProductId = products?.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;
  const selectedProductId = visibleActiveProductId ?? products?.[0]?.id ?? null;
  const selectedProduct = useMemo(
    () => products?.find((product) => product.id === selectedProductId) ?? null,
    [products, selectedProductId],
  );

  useEffect(() => {
    if (isLoading) {
      return;
    }
    if (activeProductId !== selectedProductId) {
      setActiveProduct(selectedProductId);
    }
  }, [activeProductId, isLoading, selectedProductId, setActiveProduct]);

  const { data: tree } = useQuery({
    queryKey: ["productTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProduct,
  });

  const { data: productWorkItems } = useQuery({
    queryKey: ["productAllTasks", selectedProductId],
    queryFn: () => listWorkItems({ productId: selectedProductId ?? undefined }),
    enabled: !!selectedProduct,
  });

  const { data: scopedTasks } = useQuery({
    queryKey: ["productTasks", selectedProductId, activeNodeId, activeNodeType],
    queryFn: () =>
      listWorkItems({
        productId: selectedProductId ?? undefined,
        sourceNodeId: activeNodeId ?? undefined,
        sourceNodeType: activeNodeType ?? undefined,
      }),
    enabled: !!selectedProduct,
  });

  const { data: resolvedWorkspace } = useQuery<Repository | null>({
    queryKey: ["productScopeRepo", selectedProductId, activeModuleId],
    queryFn: () => resolveRepositoryForScope({ productId: selectedProductId, moduleId: activeModuleId }),
    enabled: !!selectedProduct,
  });
  const effectiveWorkspacePath = resolvedWorkspace?.local_path ?? activeWorkspacePath ?? null;

  const allProductTasks = useMemo(() => {
    if (!selectedProductId) {
      return [];
    }
    return (productWorkItems ?? []).filter((workItem) => workItem.product_id === selectedProductId);
  }, [productWorkItems, selectedProductId]);

  const filteredScopedTasks = useMemo(() => {
    if (!selectedProductId) {
      return [];
    }
    return (scopedTasks ?? []).filter((workItem) => workItem.product_id === selectedProductId);
  }, [scopedTasks, selectedProductId]);

  const openWorkspaceInIde = () => {
    if (resolvedWorkspace) {
      useWorkspaceStore.getState().setActiveRepo(resolvedWorkspace.id);
      useWorkspaceStore.getState().setActiveWorkspace(resolvedWorkspace.local_path);
    } else if (effectiveWorkspacePath) {
      useWorkspaceStore.getState().setActiveWorkspace(effectiveWorkspacePath);
    }
    setWorkspaceActionError(null);
    setActiveView("ide");
    navigate("/ide");
  };

  useEffect(() => {
    if (!activeProductId && products?.[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  useEffect(() => {
    setActiveWorkItem(null);
    setFormError(null);
    setWorkspaceActionMsg(null);
    setWorkspaceActionError(null);
  }, [selectedProductId, activeModuleId, activeCapabilityId, setActiveWorkItem]);

  useEffect(() => {
    if (selectedProduct) {
      setProductDraft({
        name: selectedProduct.name,
        description: selectedProduct.description,
        vision: selectedProduct.vision,
        goals: selectedProduct.goals.join(", "),
        tags: selectedProduct.tags.join(", "),
      });
    }
  }, [selectedProduct]);

  useEffect(() => {
    setFormError(null);
    if (productDialogMode === "create") {
      setProductForm({ name: "", description: "", vision: "", goals: "", tags: "" });
    }
  }, [productDialogMode]);

  useEffect(() => {
    if (!tree) {
      return;
    }
    setModuleOrderIds(tree.modules.map((moduleTree) => moduleTree.module.id));
    const nextCapabilityMap: Record<string, string[]> = {};
    tree.modules.forEach((moduleTree) => {
      nextCapabilityMap[getCapabilityOrderKey(moduleTree.module.id, null)] = moduleTree.features.map((capabilityTree) => capabilityTree.capability.id);
      seedCapabilityOrderMap(nextCapabilityMap, moduleTree.features);
    });
    setFeatureOrderMap(nextCapabilityMap);
  }, [tree]);

  const selectedModule = useMemo(
    () => tree?.modules.find((moduleTree) => moduleTree.module.id === activeModuleId)?.module ?? null,
    [tree, activeModuleId],
  );
  const selectedCapabilityTree = useMemo(
    () => (tree ? findCapabilityTree(tree.modules, activeCapabilityId) : null),
    [tree, activeCapabilityId],
  );
  const selectedCapability = selectedCapabilityTree?.capability ?? null;
  const selectedModuleTree = useMemo(
    () => tree?.modules.find((moduleTree) => moduleTree.module.id === activeModuleId) ?? null,
    [tree, activeModuleId],
  );
  const selectedCapabilityParentKind = useMemo(() => {
    if (!selectedCapability) {
      return selectedModule?.node_kind ?? null;
    }
    if (!selectedCapability.parent_capability_id) {
      return selectedModule?.node_kind ?? null;
    }
    return findCapabilityTree(tree?.modules ?? [], selectedCapability.parent_capability_id)?.capability.node_kind ?? null;
  }, [selectedCapability, selectedModule, tree]);

  useEffect(() => {
    if (moduleDialogMode === "create") {
      setModuleForm({ name: "", description: "", purpose: "", nodeKind: "area" });
      return;
    }
    if (moduleDialogMode === "edit" && selectedModule) {
      setModuleDraft({
        name: selectedModule.name,
        description: selectedModule.description,
        purpose: selectedModule.purpose,
        nodeKind: selectedModule.node_kind,
      });
    }
  }, [moduleDialogMode, selectedModule]);

  useEffect(() => {
    if (capabilityDialogMode === "create") {
      setCapabilityForm({
        name: "",
        description: "",
        acceptanceCriteria: "",
        technicalNotes: "",
        nodeKind: getDefaultChildNodeKind(selectedCapability?.node_kind ?? selectedModule?.node_kind),
      });
      setFormError(null);
      return;
    }
    if (capabilityDialogMode === "edit" && selectedCapability) {
      setCapabilityDraft({
        name: selectedCapability.name,
        description: selectedCapability.description,
        acceptanceCriteria: selectedCapability.acceptance_criteria,
        technicalNotes: selectedCapability.technical_notes,
        nodeKind: selectedCapability.node_kind,
      });
    }
  }, [capabilityDialogMode, selectedCapability, selectedModule]);

  useEffect(() => {
    if (showWorkItemForm) {
      setFormError(null);
    }
  }, [showWorkItemForm]);

  const invalidateHierarchy = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["products"] }),
      queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarProductTree", selectedProductId] }),
    ]);
  };

  const invalidateTasks = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["productAllTasks", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productTasks", selectedProductId, activeNodeId, activeNodeType] }),
      queryClient.invalidateQueries({ queryKey: ["workItems"] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems", selectedProductId] }),
    ]);
  };

  const createProductMutation = useMutation({
    mutationFn: () => createProduct(productForm),
    onSuccess: async (createdProduct) => {
      await invalidateHierarchy();
      setProductForm({ name: "", description: "", vision: "", goals: "", tags: "" });
      setActiveProduct(createdProduct.id);
      closeProductDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateProductMutation = useMutation({
    mutationFn: () =>
      updateProduct({
        id: selectedProductId!,
        name: productDraft.name,
        description: productDraft.description,
        vision: productDraft.vision,
        goals: productDraft.goals,
        tags: productDraft.tags,
      }),
    onSuccess: async () => {
      await invalidateHierarchy();
      closeProductDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createModuleMutation = useMutation({
    mutationFn: () => createModule({ productId: selectedProductId!, ...moduleForm }),
    onSuccess: async (createdModule) => {
      await invalidateHierarchy();
      setModuleForm({ name: "", description: "", purpose: "", nodeKind: "area" });
      setProductWorkspaceTab("structure");
      setActiveModule(createdModule.id);
      closeModuleDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateModuleMutation = useMutation({
    mutationFn: () =>
      updateModule({
        id: activeModuleId!,
        name: moduleDraft.name,
        description: moduleDraft.description,
        purpose: moduleDraft.purpose,
        nodeKind: moduleDraft.nodeKind,
      }),
    onSuccess: async (updatedModule) => {
      await invalidateHierarchy();
      setActiveModule(updatedModule.id);
      closeModuleDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createCapabilityMutation = useMutation({
    mutationFn: () =>
      createCapability({
        moduleId: activeModuleId ?? selectedCapability?.module_id ?? selectedModule?.id ?? "",
        parentCapabilityId: activeCapabilityId ?? undefined,
        name: capabilityForm.name,
        description: capabilityForm.description,
        acceptanceCriteria: capabilityForm.acceptanceCriteria,
        priority: "medium",
        risk: "low",
        technicalNotes: capabilityForm.technicalNotes,
        nodeKind: capabilityForm.nodeKind,
      }),
    onSuccess: async (createdCapability) => {
      await invalidateHierarchy();
      setCapabilityForm({
        name: "",
        description: "",
        acceptanceCriteria: "",
        technicalNotes: "",
        nodeKind: getDefaultChildNodeKind(selectedCapability?.node_kind ?? selectedModule?.node_kind),
      });
      setProductWorkspaceTab("structure");
      setActiveCapability(createdCapability.id);
      closeCapabilityDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateCapabilityMutation = useMutation({
    mutationFn: () =>
      updateCapability({
        id: activeCapabilityId!,
        name: capabilityDraft.name,
        description: capabilityDraft.description,
        acceptanceCriteria: capabilityDraft.acceptanceCriteria,
        technicalNotes: capabilityDraft.technicalNotes,
        nodeKind: capabilityDraft.nodeKind,
      }),
    onSuccess: async (updatedCapability) => {
      await invalidateHierarchy();
      setActiveCapability(updatedCapability.id);
      closeCapabilityDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createWorkItemMutation = useMutation({
    mutationFn: () =>
      createWorkItem({
        productId: selectedProductId!,
        moduleId: activeModuleId ?? selectedCapability?.module_id ?? undefined,
        capabilityId: activeCapabilityId ?? undefined,
        sourceNodeId: activeNodeId ?? undefined,
        sourceNodeType: activeNodeType ?? undefined,
        title: workItemForm.title,
        description: workItemForm.description,
        problemStatement: workItemForm.problemStatement,
        acceptanceCriteria: workItemForm.acceptanceCriteria,
        constraints: workItemForm.constraints,
        workItemType: "feature",
        priority: "medium",
        complexity: "medium",
      }),
    onSuccess: async (createdWorkItem) => {
      queryClient.setQueryData<WorkItem[] | undefined>(["productTasks", selectedProductId, activeNodeId, activeNodeType], (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
      );
      queryClient.setQueryData<WorkItem[] | undefined>(["productAllTasks", selectedProductId], (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
      );
      queryClient.setQueryData<WorkItem[] | undefined>(["sidebarWorkItems", selectedProductId], (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
      );
      setActiveWorkItem(createdWorkItem.id);
      await invalidateTasks();
      setWorkItemForm({ title: "", description: "", problemStatement: "", acceptanceCriteria: "", constraints: "" });
      setShowWorkItemForm(false);
      setProductWorkspaceTab("delivery");
    },
    onError: (error) => setFormError(String(error)),
  });

  const archiveMutation = useMutation({
    mutationFn: (id: string) => archiveProduct(id),
    onSuccess: async (_, archivedId) => {
      await invalidateHierarchy();
      if (selectedProductId === archivedId) {
        setActiveProduct(null);
      }
    },
  });

  const reorderModulesMutation = useMutation({
    mutationFn: (orderedIds: string[]) => reorderModules(selectedProductId!, orderedIds),
    onSuccess: async () => invalidateHierarchy(),
  });

  const reorderCapabilitiesMutation = useMutation({
    mutationFn: (data: { moduleId: string; parentCapabilityId?: string; orderedIds: string[] }) => reorderCapabilities(data),
    onSuccess: async () => invalidateHierarchy(),
  });

  const createWorkspaceMutation = useMutation({
    mutationFn: () =>
      createLocalWorkspace({
        productId: selectedProductId,
        moduleId: activeModuleId,
      }),
    onSuccess: async (provisioned) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["repositories"] }),
        queryClient.invalidateQueries({ queryKey: ["productScopeRepo", selectedProductId, activeModuleId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
        queryClient.invalidateQueries({ queryKey: ["sidebarProductTree", selectedProductId] }),
      ]);
      setWorkspaceActionError(null);
      setWorkspaceActionMsg(`Workspace ready at ${provisioned.created_path}. Opening IDE.`);
      setActiveView("ide");
      navigate("/ide");
      useWorkspaceStore.getState().setActiveWorkspace(provisioned.created_path);
      useWorkspaceStore.getState().setActiveRepo(provisioned.repository.id);
    },
    onError: (error) => {
      setWorkspaceActionMsg(null);
      setWorkspaceActionError(String(error));
    },
  });

  const capabilityCount = tree ? countCapabilities(tree.modules) : 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const leafNodeCount = tree ? countLeafNodes(tree.roots) : 0;
  const activeWorkItemCount = allProductTasks.filter((workItem) => workItem.status !== "done" && workItem.status !== "cancelled").length;
  const completedWorkItemCount = allProductTasks.filter((workItem) => workItem.status === "done").length;
  const selectedHierarchyNode = useMemo(
    () => (tree ? findHierarchyNode(tree.roots, activeNodeId, activeNodeType) : null),
    [tree, activeNodeId, activeNodeType],
  );
  const selectedHierarchyPath = useMemo(
    () => (tree ? findHierarchyNodePath(tree.roots, activeNodeId, activeNodeType) : []),
    [tree, activeNodeId, activeNodeType],
  );
  const selectedNodeKind = selectedHierarchyNode?.node_kind ?? selectedCapability?.node_kind ?? selectedModule?.node_kind ?? null;
  const selectedNodeTitle = selectedHierarchyNode?.name ?? selectedProduct?.name ?? "Product";
  const selectedNodeSummary = selectedHierarchyNode?.summary
    || selectedHierarchyNode?.description
    || selectedProduct?.description
    || "Add a durable description so the selected node reads like a documented section instead of a blank planning shell.";
  const selectedScopePath = selectedProduct
    ? [selectedProduct.name, ...selectedHierarchyPath.map((node) => node.name)]
    : [];
  const selectedNodeEntityLabel = selectedCapability
    ? getHierarchyNodeKindLabel(selectedCapability.node_kind)
    : selectedModule
      ? getHierarchyNodeKindLabel(selectedModule.node_kind)
      : "Product";
  const selectedAllowedChildKinds = selectedHierarchyNode ? selectedHierarchyNode.allowed_child_kinds : ROOT_NODE_KINDS;
  const selectedChildNodeKinds = selectedHierarchyNode ? selectedAllowedChildKinds : [];
  const canCreateChildCapability = selectedChildNodeKinds.length > 0;
  const nextCapabilityEntityLabel = selectedChildNodeKinds.length > 0
    ? getHierarchyNodeKindLabel(getDefaultChildNodeKind(selectedNodeKind))
    : "Child Node";
  const selectedDirectChildren = useMemo(
    () => getDirectChildNodes(tree, selectedHierarchyNode),
    [tree, selectedHierarchyNode],
  );
  const selectedDirectWorkItems = useMemo(
    () => (selectedHierarchyNode
      ? getDirectWorkItemsForNode(selectedHierarchyNode, allProductTasks)
      : getProductDirectWorkItems(allProductTasks)),
    [allProductTasks, selectedHierarchyNode],
  );
  const selectedSubtreeWorkItems = useMemo(
    () => (selectedHierarchyNode
      ? getSubtreeWorkItemsForNode(selectedHierarchyNode, allProductTasks)
      : allProductTasks),
    [allProductTasks, selectedHierarchyNode],
  );
  const selectedMetricCards = selectedHierarchyNode
    ? [
        { label: "Direct Children", value: selectedDirectChildren.length, help: `${selectedDirectChildren.length} immediate child ${selectedDirectChildren.length === 1 ? "node" : "nodes"}` },
        { label: "Subtree Nodes", value: countDescendantNodes(selectedHierarchyNode) + 1, help: "Selected node plus all nested descendants" },
        { label: "Direct Work Items", value: selectedDirectWorkItems.length, help: "Attached directly to this node" },
        { label: "Total Work Items", value: selectedSubtreeWorkItems.length, help: "Across this node and its descendants" },
      ]
    : [
        { label: "Root Sections", value: tree?.roots.length ?? 0, help: "Top-level structural sections in the product tree" },
        { label: "Total Nodes", value: totalNodeCount, help: "All structural nodes in the product" },
        { label: "Leaf Nodes", value: leafNodeCount, help: "Nodes without structural children" },
        { label: "Active Work Items", value: activeWorkItemCount, help: "Open delivery work across the product" },
      ];
  const editableCapabilityNodeKinds = useMemo(() => {
    if (!selectedCapability) {
      return [] as HierarchyNodeKind[];
    }
    const allowedKinds = getAllowedChildNodeKinds(selectedCapabilityParentKind);
    return allowedKinds.includes(selectedCapability.node_kind)
      ? allowedKinds
      : [selectedCapability.node_kind, ...allowedKinds];
  }, [selectedCapability, selectedCapabilityParentKind]);
  const orderedModules = useMemo(() => {
    if (!tree) {
      return [];
    }
    return orderItemsByIds(tree.modules, moduleOrderIds, (moduleTree) => moduleTree.module.id);
  }, [tree, moduleOrderIds]);
  const structureRows = useMemo(() => {
    if (!tree) {
      return [];
    }
    return selectedDirectChildren.map((node) => ({
      id: node.id,
      name: node.name,
      subtitle: node.summary || node.description || getHierarchyNodeKindLabel(node.node_kind),
      type: getHierarchyNodeKindLabel(node.node_kind, { lowercase: true }),
      directChildren: node.children.length,
      directWorkItems: getDirectWorkItemsForNode(node, allProductTasks).length,
      totalWorkItems: getSubtreeWorkItemsForNode(node, allProductTasks).length,
      onSelect: () => {
        setActiveHierarchyNode({
          nodeId: node.id,
          nodeType: node.node_type,
          moduleId: node.module_id,
          capabilityId: node.capability_id,
        });
      },
      onEdit: () => {
        setActiveHierarchyNode({
          nodeId: node.id,
          nodeType: node.node_type,
          moduleId: node.module_id,
          capabilityId: node.capability_id,
        });
        if (node.node_type === "module") {
          const moduleMatch = tree.modules.find((moduleTree) => moduleTree.module.id === node.id)?.module;
          if (!moduleMatch) {
            return;
          }
          setModuleDraft({
            name: moduleMatch.name,
            description: moduleMatch.description,
            purpose: moduleMatch.purpose,
            nodeKind: moduleMatch.node_kind,
          });
          useUIStore.getState().openModuleDialog("edit");
          return;
        }
        const capabilityMatch = findCapabilityTree(tree.modules, node.id)?.capability;
        if (!capabilityMatch) {
          return;
        }
        setCapabilityDraft({
          name: capabilityMatch.name,
          description: capabilityMatch.description,
          acceptanceCriteria: capabilityMatch.acceptance_criteria,
          technicalNotes: capabilityMatch.technical_notes,
          nodeKind: capabilityMatch.node_kind,
        });
        useUIStore.getState().openCapabilityDialog("edit");
      },
    }));
  }, [allProductTasks, selectedDirectChildren, setActiveHierarchyNode, tree]);

  const openSelectedSectionInBook = () => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveView("product-overview");
    navigate(`/product-overview#${getHierarchyNodeSectionId(selectedHierarchyNode)}`);
  };

  const editSelectedScope = () => {
    if (!selectedProduct) {
      return;
    }
    if (!selectedHierarchyNode) {
      openProductDialog("edit");
      return;
    }
    if (selectedHierarchyNode.node_type === "module") {
      openModuleDialog("edit");
      return;
    }
    openCapabilityDialog("edit");
  };

  const openCreateInSelectedScope = () => {
    if (!selectedHierarchyNode) {
      useUIStore.getState().openModuleDialog("create");
      return;
    }
    if (!canCreateChildCapability) {
      return;
    }
    useUIStore.getState().openCapabilityDialog("create");
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Product Workspace</h1>
          <div style={styles.subtitle}>Book reads the product as documentation. Structure edits the semantic tree. Delivery keeps work attached to the current node.</div>
        </div>
      </div>

      <div style={styles.workspace}>
        <div style={styles.panel}>
          <div style={styles.panelInner}>
            {selectedProduct ? (
              <>
                <div style={styles.tabBar}>
                  <button style={productWorkspaceTab === "book" ? styles.tabActive : styles.tab} onClick={() => setProductWorkspaceTab("book")}>Book</button>
                  <button style={productWorkspaceTab === "structure" ? styles.tabActive : styles.tab} onClick={() => setProductWorkspaceTab("structure")}>Structure</button>
                  <button style={productWorkspaceTab === "delivery" ? styles.tabActive : styles.tab} onClick={() => setProductWorkspaceTab("delivery")}>Delivery</button>
                </div>

                <div style={styles.hero}>
                  <div style={styles.heroCard}>
                    <ScopeBreadcrumb
                      label="Selected Path"
                      productName={selectedProduct.name}
                      path={selectedScopePath}
                    />
                    <div style={styles.heroName}>{selectedNodeTitle}</div>
                    <div style={styles.heroDesc}>{selectedNodeSummary}</div>
                    <div style={styles.inlineMeta}>
                      <span style={styles.badgeKind}>{selectedHierarchyNode ? getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind) : "Product"}</span>
                      {!selectedHierarchyNode ? <span style={styles.badge}>{selectedProduct.status}</span> : null}
                      {!selectedHierarchyNode && selectedProduct.tags.map((tag) => (
                        <span key={tag} style={styles.badgeMuted}>{tag}</span>
                      ))}
                    </div>
                    <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                      <button style={styles.ghostBtn} onClick={editSelectedScope}>
                        {selectedHierarchyNode ? `Edit ${selectedNodeEntityLabel}` : "Edit Product"}
                      </button>
                      <button style={styles.ghostBtn} onClick={openSelectedSectionInBook}>Read This Section</button>
                      {effectiveWorkspacePath ? (
                        <>
                          <button style={styles.ghostBtn} onClick={openWorkspaceInIde}>Open Workspace</button>
                          <button style={styles.ghostBtn} onClick={() => revealInFinder(effectiveWorkspacePath).catch((error) => setWorkspaceActionError(String(error)))}>Reveal in Finder</button>
                        </>
                      ) : (
                        <button
                          style={styles.btn}
                          onClick={() => createWorkspaceMutation.mutate()}
                          disabled={createWorkspaceMutation.isPending}
                        >
                          {createWorkspaceMutation.isPending ? "Creating Workspace..." : "Create Workspace"}
                        </button>
                      )}
                      {!selectedHierarchyNode ? <button style={styles.btnDanger} onClick={() => archiveMutation.mutate(selectedProduct.id)}>Archive</button> : null}
                    </div>
                    {workspaceActionMsg && <div style={{ ...styles.contextText, color: "#4ec9b0", marginTop: 10 }}>{workspaceActionMsg}</div>}
                    {workspaceActionError && <div style={{ ...styles.errorText, marginTop: 10, marginBottom: 0 }}>{workspaceActionError}</div>}
                  </div>
                  <div style={styles.metricGrid}>
                    {selectedMetricCards.map((metric) => (
                      <div key={metric.label} style={styles.metricCard}>
                        <div style={styles.metricLabel}>{metric.label}</div>
                        <div style={styles.metricValue}>{metric.value}</div>
                        <div style={styles.metricHelp}>{metric.help}</div>
                      </div>
                    ))}
                  </div>
                </div>

                {productWorkspaceTab === "book" && (
                  <>
                    <div style={styles.section}>
                      <div style={styles.sectionTitle}>Book View</div>
                      <div style={styles.contextCard}>
                        <div style={styles.contextLabel}>Section Summary</div>
                        <div style={styles.contextTitle}>{selectedNodeTitle}</div>
                        <div style={styles.contextText}>{selectedNodeSummary}</div>
                        <div style={styles.chipRow}>
                          <span style={styles.badgeKind}>{selectedHierarchyNode ? getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind) : "Product"}</span>
                          <span style={styles.badgeMuted}>{selectedDirectChildren.length} direct {selectedDirectChildren.length === 1 ? "child" : "children"}</span>
                          <span style={styles.badgeMuted}>{selectedDirectWorkItems.length} direct work items</span>
                          <span style={styles.badgeMuted}>{selectedSubtreeWorkItems.length} total work items</span>
                        </div>
                      </div>
                      <div style={styles.contextCard}>
                        <div style={styles.contextLabel}>Allowed Child Kinds</div>
                        <div style={styles.contextTitle}>
                          {selectedAllowedChildKinds.length > 0 ? "This section can grow structurally." : "This section is a structural leaf."}
                        </div>
                        <div style={styles.contextText}>
                          {selectedAllowedChildKinds.length > 0
                            ? selectedAllowedChildKinds.map((nodeKind) => getHierarchyNodeKindLabel(nodeKind)).join(", ")
                            : "No deeper structural children are allowed here."}
                        </div>
                      </div>
                      <div style={styles.contextCard}>
                        <div style={styles.contextLabel}>Book Alignment</div>
                        <div style={styles.contextText}>The current path, ordering, and node labels are used as the section spine for the reader/exported book.</div>
                        <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                          <button style={styles.ghostBtn} onClick={openSelectedSectionInBook}>Open In Book</button>
                          <button style={styles.ghostBtn} onClick={() => setProductWorkspaceTab("structure")}>Inspect Structure</button>
                        </div>
                      </div>
                    </div>
                    <div style={styles.section}>
                      <div style={styles.sectionTitle}>Direct Delivery Notes</div>
                      {selectedDirectWorkItems.length > 0 ? (
                        selectedDirectWorkItems.slice(0, 8).map((workItem: WorkItem) => (
                          <div key={workItem.id} style={styles.taskRow} onClick={() => setActiveWorkItem(workItem.id)}>
                            <div style={styles.taskTitle}>{workItem.title}</div>
                            <div style={styles.taskMeta}>{workItem.status.replace(/_/g, " ")} · {workItem.priority}</div>
                          </div>
                        ))
                      ) : (
                        <div style={styles.empty}>No direct work items are attached to this section yet.</div>
                      )}
                    </div>
                  </>
                )}

                {productWorkspaceTab === "structure" && (
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>
                      <span>Structure</span>
                      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                        <button
                          style={structureViewMode === "children" ? styles.tabActive : styles.tab}
                          onClick={() => setStructureViewMode("children")}
                        >
                          Children
                        </button>
                        <button
                          style={structureViewMode === "work_items" ? styles.tabActive : styles.tab}
                          onClick={() => setStructureViewMode("work_items")}
                        >
                          Direct Work Items
                        </button>
                        <button
                          style={styles.ghostBtn}
                          onClick={openCreateInSelectedScope}
                          disabled={!selectedHierarchyNode && !selectedProductId ? true : selectedHierarchyNode ? !canCreateChildCapability : false}
                        >
                          {selectedHierarchyNode ? `+ ${nextCapabilityEntityLabel}` : "+ Root Section"}
                        </button>
                        <button style={styles.ghostBtn} onClick={() => setShowWorkItemForm(true)}>+ Work Item</button>
                        {!selectedHierarchyNode ? null : <button style={styles.ghostBtn} onClick={() => useUIStore.getState().openModuleDialog("create")}>+ Root Section</button>}
                      </div>
                    </div>

                    <div style={styles.contextCard}>
                      <div style={styles.contextLabel}>Selected Node</div>
                      <div style={styles.contextTitle}>{selectedNodeTitle}</div>
                      <div style={styles.contextText}>
                        {structureViewMode === "work_items"
                          ? "Direct work items attached to the selected node are listed below."
                          : selectedHierarchyNode
                            ? supportsHierarchyChildren(selectedHierarchyNode.node_kind)
                              ? `${getHierarchyChildLabel(selectedHierarchyNode.node_kind, { plural: true })} for the selected ${getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind, { lowercase: true })} are listed below.`
                              : `This ${getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind, { lowercase: true })} cannot contain deeper structural children.`
                            : "Root sections for the selected product are listed below."}
                      </div>
                      <div style={styles.chipRow}>
                        <span style={styles.badgeKind}>{selectedHierarchyNode ? getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind) : "Product"}</span>
                        {selectedAllowedChildKinds.map((nodeKind) => (
                          <span key={nodeKind} style={styles.badgeMuted}>{getHierarchyNodeKindLabel(nodeKind)}</span>
                        ))}
                      </div>
                    </div>

                    {structureViewMode === "children" ? (
                      structureRows.length > 0 ? (
                      <div style={styles.table}>
                        <div style={styles.tableHeader}>
                          <div>Name</div>
                          <div>Kind</div>
                          <div>Direct</div>
                          <div>Aggregate</div>
                        </div>
                        {structureRows.map((row) => (
                          <div
                            key={row.id}
                            style={styles.tableRow}
                            onClick={row.onSelect}
                          >
                            <div>
                              <div style={styles.rowPrimary}>{row.name}</div>
                              <div style={styles.rowSecondary}>{row.subtitle}</div>
                            </div>
                            <div style={styles.rowCell}>{row.type}</div>
                            <div style={styles.rowCell}>{row.directChildren} children · {row.directWorkItems} work items</div>
                            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
                              <span style={styles.rowCell}>{row.totalWorkItems} total work items</span>
                              <button
                                style={styles.ghostBtn}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  row.onEdit();
                                }}
                              >
                                Edit
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div style={styles.empty}>
                        {selectedHierarchyNode
                          ? supportsHierarchyChildren(selectedHierarchyNode.node_kind)
                            ? `No ${getHierarchyChildLabel(selectedHierarchyNode.node_kind, { plural: true, lowercase: true })} yet.`
                            : `${getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind)} nodes cannot contain deeper hierarchy.`
                          : "No root sections yet. Start with the first section and build from there."}
                      </div>
                      )
                    ) : selectedDirectWorkItems.length > 0 ? (
                      <div style={styles.table}>
                        <div style={styles.tableHeader}>
                          <div>Name</div>
                          <div>Type</div>
                          <div>Status</div>
                          <div>Priority</div>
                        </div>
                        {selectedDirectWorkItems.map((item: WorkItem) => (
                          <div key={item.id} style={styles.tableRow} onClick={() => setActiveWorkItem(item.id)}>
                            <div>
                              <div style={styles.rowPrimary}>{item.title}</div>
                              <div style={styles.rowSecondary}>{item.description || item.problem_statement || "Work item"}</div>
                            </div>
                            <div style={styles.rowCell}>{item.work_item_type}</div>
                            <div style={styles.rowCell}>{item.status.replace(/_/g, " ")}</div>
                            <div style={styles.rowCell}>{item.priority}</div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div style={styles.empty}>No work items in the selected scope yet.</div>
                    )}
                  </div>
                )}

                {productWorkspaceTab === "delivery" && (
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>
                      <span>Delivery</span>
                      <button style={styles.ghostBtn} onClick={() => setShowWorkItemForm(true)}>+ Work Item Here</button>
                    </div>
                    <div style={styles.contextCard}>
                      <div style={styles.contextLabel}>Owner Scope</div>
                      <div style={styles.contextTitle}>{selectedNodeTitle}</div>
                      <div style={styles.contextText}>New work items created here stay attached directly to the current node so ownership remains structurally meaningful.</div>
                      <div style={styles.chipRow}>
                        <span style={styles.badgeKind}>{selectedHierarchyNode ? getHierarchyNodeKindLabel(selectedHierarchyNode.node_kind) : "Product"}</span>
                        <span style={styles.badgeMuted}>{selectedDirectWorkItems.length} direct work items</span>
                        <span style={styles.badgeMuted}>{selectedSubtreeWorkItems.length} total in subtree</span>
                      </div>
                    </div>
                    {selectedDirectWorkItems.length > 0 ? (
                      selectedDirectWorkItems.map((workItem: WorkItem) => (
                        <div key={workItem.id} style={styles.taskRow} onClick={() => setActiveWorkItem(workItem.id)}>
                          <div style={styles.taskTitle}>{workItem.title}</div>
                          <div style={styles.taskMeta}>{workItem.status.replace(/_/g, " ")} · {workItem.priority}</div>
                        </div>
                      ))
                    ) : (
                      <div style={styles.empty}>No work items in the current scope yet.</div>
                    )}
                  </div>
                )}
              </>
            ) : (
              <div style={styles.empty}>
                {isLoading
                  ? "Loading products..."
                  : products && products.length > 0
                    ? "Select a product from the left sidebar to start refining the hierarchy."
                    : "No visible products yet. Use + New in the left product rail or disable Hide Example Products in Settings."}
              </div>
            )}
          </div>
        </div>
      </div>

      {productDialogMode !== "closed" && (
        <ModalShell title={productDialogMode === "create" ? "Create Product" : "Edit Product"} onClose={closeProductDialog}>
          <label style={styles.label}>Name</label>
          <input
            style={styles.input}
            value={productDialogMode === "create" ? productForm.name : productDraft.name}
            onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, name: e.target.value }) : setProductDraft({ ...productDraft, name: e.target.value }))}
          />
          <label style={styles.label}>Description</label>
          <textarea
            style={styles.textarea}
            value={productDialogMode === "create" ? productForm.description : productDraft.description}
            onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, description: e.target.value }) : setProductDraft({ ...productDraft, description: e.target.value }))}
          />
          <label style={styles.label}>Vision</label>
          <textarea
            style={styles.textarea}
            value={productDialogMode === "create" ? productForm.vision : productDraft.vision}
            onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, vision: e.target.value }) : setProductDraft({ ...productDraft, vision: e.target.value }))}
          />
          <div style={styles.formRow}>
            <div>
              <label style={styles.label}>Goals (comma-separated)</label>
              <input
                style={styles.input}
                value={productDialogMode === "create" ? productForm.goals : productDraft.goals}
                onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, goals: e.target.value }) : setProductDraft({ ...productDraft, goals: e.target.value }))}
              />
            </div>
            <div>
              <label style={styles.label}>Tags (comma-separated)</label>
              <input
                style={styles.input}
                value={productDialogMode === "create" ? productForm.tags : productDraft.tags}
                onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, tags: e.target.value }) : setProductDraft({ ...productDraft, tags: e.target.value }))}
              />
            </div>
          </div>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={closeProductDialog}>Cancel</button>
            <button
              style={styles.btn}
              onClick={() => (productDialogMode === "create" ? createProductMutation.mutate() : updateProductMutation.mutate())}
              disabled={!(productDialogMode === "create" ? productForm.name : productDraft.name)}
            >
              {productDialogMode === "create"
                ? createProductMutation.isPending ? "Creating..." : "Create Product"
                : updateProductMutation.isPending ? "Saving..." : "Save Product"}
            </button>
          </div>
        </ModalShell>
      )}

      {moduleDialogMode !== "closed" && (
        <ModalShell
          title={moduleDialogMode === "create"
            ? `Create ${getHierarchyNodeKindLabel(moduleForm.nodeKind)}`
            : `Edit ${selectedModule ? getHierarchyNodeKindLabel(selectedModule.node_kind) : "Root Section"}: ${selectedModule?.name ?? ""}`}
          onClose={closeModuleDialog}
        >
          {moduleDialogMode === "create" ? (
            <>
              <label style={styles.label}>Root Kind</label>
              <select style={styles.input} value={moduleForm.nodeKind} onChange={(e) => setModuleForm({ ...moduleForm, nodeKind: e.target.value as HierarchyNodeKind })}>
                {ROOT_NODE_KINDS.map((nodeKind) => (
                  <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                ))}
              </select>
              <label style={styles.label}>{getHierarchyNodeKindLabel(moduleForm.nodeKind)} Name</label>
              <input style={styles.input} value={moduleForm.name} onChange={(e) => setModuleForm({ ...moduleForm, name: e.target.value })} />
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={moduleForm.description} onChange={(e) => setModuleForm({ ...moduleForm, description: e.target.value })} />
              <label style={styles.label}>Purpose</label>
              <input style={styles.input} value={moduleForm.purpose} onChange={(e) => setModuleForm({ ...moduleForm, purpose: e.target.value })} />
            </>
          ) : (
            <>
              <label style={styles.label}>Root Kind</label>
              <select style={styles.input} value={moduleDraft.nodeKind} onChange={(e) => setModuleDraft({ ...moduleDraft, nodeKind: e.target.value as HierarchyNodeKind })}>
                {ROOT_NODE_KINDS.map((nodeKind) => (
                  <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                ))}
              </select>
              <label style={styles.label}>{getHierarchyNodeKindLabel(moduleDraft.nodeKind)} Name</label>
              <input style={styles.input} value={moduleDraft.name} onChange={(e) => setModuleDraft({ ...moduleDraft, name: e.target.value })} />
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={moduleDraft.description} onChange={(e) => setModuleDraft({ ...moduleDraft, description: e.target.value })} />
              <label style={styles.label}>Purpose</label>
              <input style={styles.input} value={moduleDraft.purpose} onChange={(e) => setModuleDraft({ ...moduleDraft, purpose: e.target.value })} />
            </>
          )}
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={closeModuleDialog}>Cancel</button>
            <button
              style={styles.btn}
              onClick={() => (moduleDialogMode === "create" ? createModuleMutation.mutate() : updateModuleMutation.mutate())}
              disabled={!(moduleDialogMode === "create" ? moduleForm.name : moduleDraft.name) || !selectedProductId}
            >
              {moduleDialogMode === "create"
                ? createModuleMutation.isPending ? "Saving..." : `Create ${getHierarchyNodeKindLabel(moduleForm.nodeKind)}`
                : updateModuleMutation.isPending ? "Saving..." : `Save ${getHierarchyNodeKindLabel(moduleDraft.nodeKind)}`}
            </button>
          </div>
        </ModalShell>
      )}

      {capabilityDialogMode !== "closed" && (
        <ModalShell
          title={capabilityDialogMode === "create"
            ? `Create ${getHierarchyNodeKindLabel(capabilityForm.nodeKind)}`
            : `Edit ${selectedCapability ? getHierarchyNodeKindLabel(selectedCapability.node_kind) : "Node"}: ${selectedCapability?.name ?? ""}`}
          onClose={closeCapabilityDialog}
        >
          {capabilityDialogMode === "create" ? (
            <>
              <label style={styles.label}>Parent Root</label>
              <input style={styles.input} value={selectedModule?.name ?? ""} readOnly />
              <label style={styles.label}>Parent Node</label>
              <input
                style={styles.input}
                value={selectedCapability?.name ?? ""}
                readOnly
                placeholder={`Create a top-level child under ${selectedModule?.name ?? "the selected root"}`}
              />
              <label style={styles.label}>Node Kind</label>
              <select style={styles.input} value={capabilityForm.nodeKind} onChange={(e) => setCapabilityForm({ ...capabilityForm, nodeKind: e.target.value as HierarchyNodeKind })}>
                {getAllowedChildNodeKinds(selectedCapability?.node_kind ?? selectedModule?.node_kind).map((nodeKind) => (
                  <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                ))}
              </select>
              <div style={styles.contextText}>
                Allowed child kinds: {getAllowedChildNodeKinds(selectedCapability?.node_kind ?? selectedModule?.node_kind).map((nodeKind) => getHierarchyNodeKindLabel(nodeKind)).join(", ")}.
              </div>
              <label style={styles.label}>{getHierarchyNodeKindLabel(capabilityForm.nodeKind)} Name</label>
              <input style={styles.input} value={capabilityForm.name} onChange={(e) => setCapabilityForm({ ...capabilityForm, name: e.target.value })} />
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={capabilityForm.description} onChange={(e) => setCapabilityForm({ ...capabilityForm, description: e.target.value })} />
              <label style={styles.label}>Acceptance Criteria</label>
              <textarea style={styles.textarea} value={capabilityForm.acceptanceCriteria} onChange={(e) => setCapabilityForm({ ...capabilityForm, acceptanceCriteria: e.target.value })} />
              <label style={styles.label}>Technical Notes</label>
              <textarea style={styles.textarea} value={capabilityForm.technicalNotes} onChange={(e) => setCapabilityForm({ ...capabilityForm, technicalNotes: e.target.value })} />
            </>
          ) : (
            <>
              <label style={styles.label}>Node Kind</label>
              <select style={styles.input} value={capabilityDraft.nodeKind} onChange={(e) => setCapabilityDraft((current) => ({ ...current, nodeKind: e.target.value as HierarchyNodeKind }))}>
                {editableCapabilityNodeKinds.map((nodeKind) => (
                  <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                ))}
              </select>
              <label style={styles.label}>Name</label>
              <input style={styles.input} value={capabilityDraft.name} onChange={(e) => setCapabilityDraft((current) => ({ ...current, name: e.target.value }))} />
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={capabilityDraft.description} onChange={(e) => setCapabilityDraft((current) => ({ ...current, description: e.target.value }))} />
              <label style={styles.label}>Acceptance Criteria</label>
              <textarea style={styles.textarea} value={capabilityDraft.acceptanceCriteria} onChange={(e) => setCapabilityDraft((current) => ({ ...current, acceptanceCriteria: e.target.value }))} />
              <label style={styles.label}>Technical Notes</label>
              <textarea style={styles.textarea} value={capabilityDraft.technicalNotes} onChange={(e) => setCapabilityDraft((current) => ({ ...current, technicalNotes: e.target.value }))} />
            </>
          )}
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={closeCapabilityDialog}>Cancel</button>
            <button
              style={styles.btn}
              onClick={() => (capabilityDialogMode === "create" ? createCapabilityMutation.mutate() : updateCapabilityMutation.mutate())}
              disabled={!(capabilityDialogMode === "create" ? capabilityForm.name : capabilityDraft.name) || !activeModuleId}
            >
              {capabilityDialogMode === "create"
                ? createCapabilityMutation.isPending ? "Saving..." : `Create ${getHierarchyNodeKindLabel(capabilityForm.nodeKind)}`
                : updateCapabilityMutation.isPending ? "Saving..." : `Save ${selectedCapability ? getHierarchyNodeKindLabel(selectedCapability.node_kind) : "Node"}`}
            </button>
          </div>
        </ModalShell>
      )}

      {showWorkItemForm && (
        <ModalShell title="Create Scoped Work Item" onClose={() => setShowWorkItemForm(false)}>
          <label style={styles.label}>Title</label>
          <input style={styles.input} value={workItemForm.title} onChange={(e) => setWorkItemForm({ ...workItemForm, title: e.target.value })} />
          <label style={styles.label}>Description</label>
          <textarea style={styles.textarea} value={workItemForm.description} onChange={(e) => setWorkItemForm({ ...workItemForm, description: e.target.value })} />
          <label style={styles.label}>Problem Statement</label>
          <textarea style={styles.textarea} value={workItemForm.problemStatement} onChange={(e) => setWorkItemForm({ ...workItemForm, problemStatement: e.target.value })} />
          <label style={styles.label}>Acceptance Criteria</label>
          <textarea style={styles.textarea} value={workItemForm.acceptanceCriteria} onChange={(e) => setWorkItemForm({ ...workItemForm, acceptanceCriteria: e.target.value })} />
          <label style={styles.label}>Constraints</label>
          <textarea style={styles.textarea} value={workItemForm.constraints} onChange={(e) => setWorkItemForm({ ...workItemForm, constraints: e.target.value })} />
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setShowWorkItemForm(false)}>Cancel</button>
            <button style={styles.btn} onClick={() => createWorkItemMutation.mutate()} disabled={!selectedProductId || !workItemForm.title}>
              {createWorkItemMutation.isPending ? "Creating..." : "Create Work Item"}
            </button>
          </div>
        </ModalShell>
      )}
    </div>
  );
}

function renderCapabilityTreeNode(
  capabilityTree: CapabilityTree,
  context: {
    activeCapabilityId: string | null;
    setActiveModule: (id: string | null) => void;
    setActiveCapability: (id: string | null) => void;
    onEdit: (capability: CapabilityNode) => void;
    onDropCapability: (targetCapability: CapabilityNode, siblingIds: string[]) => void;
    onDragCapabilityStart: (capability: CapabilityNode, siblingIds: string[]) => void;
    onDragCapabilityEnd: () => void;
    draggedCapabilityId: string | null;
    capabilityOrderMap: Record<string, string[]>;
  },
  siblingIds: string[],
): React.ReactNode {
  const { activeCapabilityId, setActiveModule, setActiveCapability, onEdit, onDropCapability, onDragCapabilityStart, onDragCapabilityEnd, draggedCapabilityId, capabilityOrderMap } = context;
  const isActive = activeCapabilityId === capabilityTree.capability.id;
  const orderedChildren = getOrderedCapabilityTrees(
    capabilityTree.children,
    capabilityOrderMap[getCapabilityOrderKey(capabilityTree.capability.module_id, capabilityTree.capability.id)],
  );

  return (
    <div key={capabilityTree.capability.id} style={styles.childWrap}>
      <div
        style={{
          ...(isActive ? styles.featureNodeActive : styles.featureNode),
          ...(draggedCapabilityId === capabilityTree.capability.id ? styles.dropTarget : null),
        }}
        draggable
        onDragStart={() => onDragCapabilityStart(capabilityTree.capability, siblingIds)}
        onDragEnd={onDragCapabilityEnd}
        onDragOver={(e) => e.preventDefault()}
        onDrop={() => onDropCapability(capabilityTree.capability, siblingIds)}
      >
        <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "flex-start" }}>
          <div
            onClick={() => {
              setActiveModule(capabilityTree.capability.module_id);
              setActiveCapability(capabilityTree.capability.id);
            }}
            style={{ cursor: "pointer", flex: 1 }}
          >
            <div style={styles.featureTitle}>{capabilityTree.capability.name}</div>
            <div style={styles.featureMeta}>{capabilityTree.capability.status} · {capabilityTree.capability.priority} priority</div>
            <div style={styles.featureMeta}>{capabilityTree.capability.description || "No description yet."}</div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={styles.dragHandle} title="Drag to reorder">::</span>
            <button style={styles.ghostBtn} onClick={() => onEdit(capabilityTree.capability)}>Edit</button>
          </div>
        </div>
      </div>
      {orderedChildren.map((child) =>
        renderCapabilityTreeNode(
          child,
          context,
          capabilityOrderMap[getCapabilityOrderKey(capabilityTree.capability.module_id, capabilityTree.capability.id)] ?? capabilityTree.children.map((item) => item.capability.id),
        ),
      )}
    </div>
  );
}

function ModalShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div style={styles.modalBackdrop} onClick={onClose}>
      <div style={styles.modal} onClick={(e) => e.stopPropagation()}>
        <div style={styles.modalHeader}>
          <div style={styles.modalTitle}>{title}</div>
          <button style={styles.ghostBtn} onClick={onClose}>Close</button>
        </div>
        <div style={styles.modalBody}>{children}</div>
      </div>
    </div>
  );
}

function countCapabilities(modules: ModuleTree[]) {
  return modules.reduce((total, moduleTree) => total + moduleTree.features.reduce((sum, capabilityTree) => sum + countCapabilityTree(capabilityTree), 0), 0);
}

function countCapabilityTree(capabilityTree: CapabilityTree): number {
  return 1 + capabilityTree.children.reduce((sum, child) => sum + countCapabilityTree(child), 0);
}

function findCapabilityTree(modules: ModuleTree[], capabilityId: string | null): CapabilityTree | null {
  if (!capabilityId) {
    return null;
  }

  for (const moduleTree of modules) {
    for (const capabilityTree of moduleTree.features) {
      const found = searchCapabilityTree(capabilityTree, capabilityId);
      if (found) {
        return found;
      }
    }
  }

  return null;
}

function searchCapabilityTree(capabilityTree: CapabilityTree, capabilityId: string | null): CapabilityTree | null {
  if (!capabilityId) {
    return null;
  }
  if (capabilityTree.capability.id === capabilityId) {
    return capabilityTree;
  }

  for (const child of capabilityTree.children) {
    const found = searchCapabilityTree(child, capabilityId);
      if (found) {
        return found;
      }
  }

  return null;
}

function moveId(ids: string[], id: string, direction: -1 | 1): string[] {
  const currentIndex = ids.indexOf(id);
  if (currentIndex === -1) {
    return ids;
  }
  return moveIdToIndex(ids, id, currentIndex + direction);
}

function addCapabilityToTree(tree: ProductTree, capability: CapabilityNode): ProductTree {
  return {
    ...tree,
    modules: tree.modules.map((moduleTree) =>
      moduleTree.module.id === capability.module_id
        ? {
            ...moduleTree,
            features: insertCapabilityTree(moduleTree.features, capability),
          }
        : moduleTree,
    ),
  };
}

function insertCapabilityTree(nodes: CapabilityTree[], capability: CapabilityNode): CapabilityTree[] {
  if (!capability.parent_capability_id) {
    return [...nodes, { capability, children: [] }];
  }

  return nodes.map((node) =>
    node.capability.id === capability.parent_capability_id
      ? { ...node, children: [...node.children, { capability, children: [] }] }
      : { ...node, children: insertCapabilityTree(node.children, capability) },
  );
}

function getCapabilityOrderKey(moduleId: string, parentCapabilityId: string | null) {
  return `${moduleId}:${parentCapabilityId ?? "root"}`;
}

function seedCapabilityOrderMap(target: Record<string, string[]>, nodes: CapabilityTree[]) {
  nodes.forEach((node) => {
    target[getCapabilityOrderKey(node.capability.module_id, node.capability.id)] = node.children.map((child) => child.capability.id);
    seedCapabilityOrderMap(target, node.children);
  });
}

function getOrderedCapabilityTrees(nodes: CapabilityTree[], orderedIds?: string[]) {
  return orderItemsByIds(nodes, orderedIds ?? [], (node) => node.capability.id);
}

function orderItemsByIds<T>(items: T[], orderedIds: string[], getId: (item: T) => string) {
  if (orderedIds.length === 0) {
    return items;
  }
  const rank = new Map(orderedIds.map((id, index) => [id, index]));
  return [...items].sort((a, b) => (rank.get(getId(a)) ?? Number.MAX_SAFE_INTEGER) - (rank.get(getId(b)) ?? Number.MAX_SAFE_INTEGER));
}

function moveIdToIndex(ids: string[], id: string, nextIndex: number): string[] {
  const currentIndex = ids.indexOf(id);
  if (currentIndex === -1 || nextIndex < 0 || nextIndex >= ids.length) {
    return ids;
  }
  const nextIds = [...ids];
  const [item] = nextIds.splice(currentIndex, 1);
  nextIds.splice(nextIndex, 0, item);
  return nextIds;
}
