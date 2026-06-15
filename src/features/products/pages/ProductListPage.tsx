import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  archiveProduct,
  createCapability,
  createLocalWorkspace,
  createModule,
  createProduct,
  createProductDependency,
  createProductReference,
  createWorkItem,
  deleteCapability,
  deleteModule,
  deleteProductReference,
  deleteWorkItem,
  getProductTree,
  getSetting,
  listProductDependencies,
  listProductReferences,
  listProducts,
  listWorkItems,
  reorderCapabilities,
  reorderModules,
  revealInFinder,
  resolveRepositoryForScope,
  setSetting,
  updateCapability,
  updateModule,
  updateProduct,
  updateWorkItem,
} from "../../../lib/tauri";
import {
  countDescendantNodes,
  countHierarchyNodes,
  countLeafNodes,
  findHierarchyNode,
  findHierarchyNodePath,
  flattenHierarchyNodes,
  getDirectChildNodes,
  getDirectWorkItemsForNode,
  getHierarchyNodeKey,
  getHierarchyNodeSectionId,
  getSubtreeWorkItemsForNode,
  isDirectProductWorkItem,
} from "../../../lib/hierarchyTree";
import {
  getAllowedChildNodeKinds,
  getDefaultChildNodeKind,
  groupHierarchyNodeKinds,
  getHierarchyChildLabel,
  getHierarchyNodeKindGuidance,
  getHierarchyNodeKindLabel,
  orderHierarchyNodeKinds,
  ROOT_NODE_KINDS,
  supportsHierarchyChildren,
} from "../../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { ProductOverviewPage } from "./ProductOverviewPage";
import type { CapabilityNode, CapabilityTree, HierarchyNodeKind, HierarchyTreeNode, ModuleTree, Product, ProductDependency, ProductDependencyKind, ProductReference, ProductTree, Repository, WorkItem } from "../../../lib/types";

const HIDE_EXAMPLE_PRODUCTS_KEY = "catalog.hide_example_products";

type ProductFormState = {
  name: string;
  description: string;
  vision: string;
  goals: string;
  tags: string;
  lifecycle: Product["lifecycle"];
  health: Product["health"];
  ownerLabel: string;
  investmentStatus: Product["investment_status"];
  roadmap: string;
  evidence: string;
};

const emptyProductForm: ProductFormState = {
  name: "",
  description: "",
  vision: "",
  goals: "",
  tags: "",
  lifecycle: "incubating",
  health: "unknown",
  ownerLabel: "",
  investmentStatus: "evaluate",
  roadmap: "",
  evidence: "",
};

function productToForm(product: Product): ProductFormState {
  return {
    name: product.name,
    description: product.description,
    vision: product.vision,
    goals: product.goals.join(", "),
    tags: product.tags.join(", "),
    lifecycle: product.lifecycle,
    health: product.health,
    ownerLabel: product.owner_label,
    investmentStatus: product.investment_status,
    roadmap: product.roadmap,
    evidence: product.evidence,
  };
}

const productLifecycleOptions: Product["lifecycle"][] = ["idea", "incubating", "active", "maturing", "sunsetting", "retired"];
const productHealthOptions: Product["health"][] = ["unknown", "healthy", "watch", "at_risk", "blocked"];
const productInvestmentOptions: Product["investment_status"][] = ["evaluate", "invest", "maintain", "pause", "retire"];
const referenceKindOptions: ProductReference["reference_kind"][] = ["note", "external_doc", "architecture", "customer_evidence", "regulatory", "design_packet", "standard", "other"];

type WorkItemDraftState = {
  title: string;
  problemStatement: string;
  description: string;
  acceptanceCriteria: string;
  constraints: string;
  status: WorkItem["status"];
  priority: WorkItem["priority"];
  complexity: WorkItem["complexity"];
};

const emptyWorkItemDraft: WorkItemDraftState = {
  title: "",
  problemStatement: "",
  description: "",
  acceptanceCriteria: "",
  constraints: "",
  status: "draft",
  priority: "medium",
  complexity: "medium",
};

const workItemStatusOptions: WorkItem["status"][] = ["draft", "ready_for_review", "approved", "in_planning", "in_progress", "in_validation", "waiting_human_review", "done", "blocked", "failed", "cancelled"];
const workItemPriorityOptions: WorkItem["priority"][] = ["critical", "high", "medium", "low"];
const workItemComplexityOptions: WorkItem["complexity"][] = ["trivial", "low", "medium", "high", "very_high"];

function workItemToDraft(workItem: WorkItem): WorkItemDraftState {
  return {
    title: workItem.title,
    problemStatement: workItem.problem_statement,
    description: workItem.description,
    acceptanceCriteria: workItem.acceptance_criteria,
    constraints: workItem.constraints,
    status: workItem.status,
    priority: workItem.priority,
    complexity: workItem.complexity,
  };
}

function formatWorkItemMeta(value: string): string {
  return value.replace(/_/g, " ");
}

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
  pageTabs: { display: "flex", gap: 8, padding: 4, border: "1px solid #32353d", borderRadius: 10, backgroundColor: "#1b1d22", flexWrap: "wrap" as const },
  pageTabGroup: { display: "flex", gap: 6, alignItems: "center", border: "1px solid #2d3139", borderRadius: 8, padding: 6, flexWrap: "wrap" as const },
  pageTabGroupLabel: { fontSize: 10, color: "#8f96a3", fontWeight: 800, textTransform: "uppercase" as const, letterSpacing: "0.06em", padding: "0 4px" },
  pageTabProductSelect: { minWidth: 190, padding: "7px 10px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 12 },
  pageTab: { padding: "8px 12px", fontSize: 12, fontWeight: 800, borderRadius: 8, border: "1px solid transparent", backgroundColor: "transparent", color: "#aeb7c6", cursor: "pointer" },
  pageTabActive: { padding: "8px 12px", fontSize: 12, fontWeight: 800, borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#173247", color: "#ffffff", cursor: "pointer" },
  btn: { padding: "7px 12px", fontSize: 12, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  ghostBtn: { padding: "6px 10px", fontSize: 12, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3b4049", borderRadius: 8, cursor: "pointer" },
  btnDanger: { padding: "5px 10px", fontSize: 12, backgroundColor: "#6c2020", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  compactActionBtn: { padding: "5px 8px", minHeight: 30, fontSize: 11, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3b4049", borderRadius: 7, cursor: "pointer", whiteSpace: "nowrap" as const },
  compactDangerBtn: { padding: "5px 8px", minHeight: 30, fontSize: 11, backgroundColor: "#6c2020", color: "#fff", border: "1px solid #8a2b2b", borderRadius: 7, cursor: "pointer", whiteSpace: "nowrap" as const },
  toolbar: { display: "grid", gridTemplateColumns: "minmax(260px, 1fr) repeat(4, minmax(120px, 160px)) auto", gap: 10, alignItems: "end", marginBottom: 10 },
  toolbarCompact: { display: "flex", gap: 10, flexWrap: "wrap" as const, alignItems: "flex-end", marginBottom: 12 },
  statusToolbar: { display: "grid", gridTemplateColumns: "minmax(220px, 360px) 150px 180px minmax(360px, 1fr)", gap: 10, alignItems: "end", marginBottom: 10 },
  statusMetrics: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 8 },
  statusMetric: { border: "1px solid #32353d", borderRadius: 8, backgroundColor: "#26292f", padding: "8px 10px", minHeight: 52 },
  statusMetricValue: { fontSize: 18, fontWeight: 800, color: "#f3f3f3", lineHeight: 1.1 },
  statusMetricHelp: { fontSize: 10, color: "#8f96a3", marginTop: 3, lineHeight: 1.3 },
  toggleRow: { display: "flex", gap: 12, flexWrap: "wrap" as const, alignItems: "center", marginBottom: 12 },
  checkboxLabel: { display: "flex", gap: 6, alignItems: "center", fontSize: 12, color: "#cfd6e4" },
  controlLabel: { fontSize: 11, color: "#8f96a3", textTransform: "uppercase" as const, fontWeight: 800, letterSpacing: "0.06em", marginBottom: 4 },
  select: { width: "100%", padding: "9px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 13, boxSizing: "border-box" as const },
  workspaceWithOutline: { display: "grid", gridTemplateColumns: "minmax(280px, 360px) minmax(0, 1fr)", gap: 12, minHeight: 0 },
  outlinePanel: { border: "1px solid #32353d", borderRadius: 12, backgroundColor: "#1b1d22", padding: 12, minHeight: 0, overflow: "auto" },
  outlineSummary: { border: "1px solid #32353d", borderRadius: 10, backgroundColor: "#26292f", padding: 12, marginBottom: 12 },
  outlineTitle: { fontSize: 14, fontWeight: 800, color: "#f3f3f3", marginBottom: 4 },
  outlineMeta: { fontSize: 11, color: "#8f96a3", lineHeight: 1.45 },
  outlineControls: { display: "grid", gridTemplateColumns: "1fr", gap: 8, marginBottom: 10 },
  outlineToolRow: { display: "flex", gap: 6, flexWrap: "wrap" as const },
  outlineRecent: { border: "1px solid #32353d", borderRadius: 10, backgroundColor: "#26292f", padding: 8, marginBottom: 10 },
  outlineRecentBtn: { width: "100%", textAlign: "left" as const, padding: "7px 8px", borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#1b1d22", color: "#d8e1ef", cursor: "pointer", fontSize: 11, marginTop: 6 },
  outlineTree: { display: "flex", flexDirection: "column", gap: 6 },
  outlineNode: { borderRadius: 8, border: "1px solid #303640", backgroundColor: "#26292f", color: "#ced4de", padding: "8px 10px" },
  outlineNodeActive: { borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#1f2a35", color: "#ffffff", padding: "8px 10px" },
  outlineNodeHeader: { display: "flex", alignItems: "flex-start", gap: 8 },
  outlineToggle: { width: 20, height: 20, borderRadius: 6, border: "1px solid #38404d", backgroundColor: "#181a1f", color: "#cfd6e4", fontSize: 10, cursor: "pointer", flexShrink: 0 },
  outlineNodeBody: { flex: 1, minWidth: 0, cursor: "pointer" },
  outlineNodeTitle: { fontSize: 12, fontWeight: 700, color: "#e9eef8" },
  outlineNodeMeta: { fontSize: 10, color: "#8f96a3", marginTop: 3, lineHeight: 1.4 },
  outlineChildWrap: { marginLeft: 12, paddingLeft: 8, borderLeft: "1px solid #2c3139", display: "flex", flexDirection: "column", gap: 6, marginTop: 6 },
  outlineActionRow: { display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" },
  outlineActionBtn: { padding: "3px 6px", borderRadius: 6, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#e0e0e0", fontSize: 10, fontWeight: 700, cursor: "pointer" },
  outlineTask: { borderRadius: 8, border: "1px solid #2c3139", backgroundColor: "#1a1d22", color: "#d8dde6", cursor: "pointer", padding: "7px 9px" },
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
  workItemDetailGrid: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12, marginTop: 10 },
  managementTabs: { display: "flex", gap: 8, paddingBottom: 10, borderBottom: "1px solid #32353d", marginBottom: 12, flexWrap: "wrap" as const },
  managementLayout: { display: "grid", gridTemplateColumns: "minmax(220px, 300px) minmax(0, 1fr)", gap: 12, minHeight: 0 },
  managementThreePane: { display: "grid", gridTemplateColumns: "minmax(220px, 280px) minmax(240px, 340px) minmax(0, 1fr)", gap: 12, minHeight: 0 },
  managementPane: { border: "1px solid #32353d", borderRadius: 12, backgroundColor: "#1b1d22", padding: 10, minHeight: 0 },
  managementPaneHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, marginBottom: 8 },
  managementList: { display: "flex", flexDirection: "column", gap: 6 },
  managementListButton: { textAlign: "left" as const, border: "1px solid #303640", borderRadius: 8, backgroundColor: "#26292f", color: "#d8e1ef", padding: "8px 10px", cursor: "pointer" },
  managementListButtonActive: { textAlign: "left" as const, border: "1px solid #0e639c", borderRadius: 8, backgroundColor: "#1f2a35", color: "#ffffff", padding: "8px 10px", cursor: "pointer" },
  managementItemSelect: { width: "100%", textAlign: "left" as const, border: "none", backgroundColor: "transparent", color: "inherit", padding: 0, cursor: "pointer" },
  managementTableHeader: { display: "grid", gridTemplateColumns: "minmax(0, 1.5fr) 110px 110px 190px", gap: 10, padding: "10px 12px", borderBottom: "1px solid #32353d", fontSize: 11, fontWeight: 700, letterSpacing: "0.06em", textTransform: "uppercase" as const, color: "#8f96a3" },
  managementTableRow: { display: "grid", gridTemplateColumns: "minmax(0, 1.5fr) 110px 110px 190px", gap: 10, padding: "12px", borderBottom: "1px solid #2d3139", alignItems: "center" },
  managementActions: { display: "flex", gap: 6, alignItems: "center", justifyContent: "flex-end", flexWrap: "wrap" as const },
  inlineActionRow: { display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" as const },
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
  chipRow: { display: "flex", gap: 6, flexWrap: "wrap" as const, marginTop: 8 },
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
  productTableHeader: { display: "grid", gridTemplateColumns: "minmax(360px, 1.9fr) 86px 112px 142px 142px 210px", gap: 12, padding: "9px 14px", borderBottom: "1px solid #32353d", fontSize: 10, fontWeight: 800, letterSpacing: "0.06em", textTransform: "uppercase" as const, color: "#8f96a3" },
  productTableRow: { display: "grid", gridTemplateColumns: "minmax(360px, 1.9fr) 86px 112px 142px 142px 210px", gap: 12, padding: "12px 14px", borderBottom: "1px solid #2d3139", alignItems: "center", minHeight: 112 },
  productCell: { minWidth: 0 },
  productDescription: { fontSize: 12, color: "#8f96a3", marginTop: 4, maxWidth: 720, overflow: "hidden", display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical" as const, lineHeight: 1.35 },
  productMetaCell: { fontSize: 12, color: "#cfd6e4", lineHeight: 1.35, minWidth: 0 },
  productActions: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 6, alignItems: "center" },
  statusTableHeader: { display: "grid", gridTemplateColumns: "minmax(260px, 1.9fr) 58px 110px 120px 115px 150px", gap: 8, padding: "8px 10px", borderBottom: "1px solid #32353d", fontSize: 10, fontWeight: 800, letterSpacing: "0.06em", textTransform: "uppercase" as const, color: "#8f96a3" },
  statusTableRow: { display: "grid", gridTemplateColumns: "minmax(260px, 1.9fr) 58px 110px 120px 115px 150px", gap: 8, padding: "8px 10px", borderBottom: "1px solid #2d3139", alignItems: "center" },
  progressTrack: { height: 6, borderRadius: 999, backgroundColor: "#171a20", border: "1px solid #303640", overflow: "hidden" },
  progressFill: { height: "100%", borderRadius: 999, backgroundColor: "#4ec9b0" },
  rowPrimary: { fontSize: 13, fontWeight: 700, color: "#f3f3f3" },
  rowSecondary: { fontSize: 12, color: "#8f96a3", marginTop: 4 },
  rowCell: { fontSize: 12, color: "#cfd6e4" },
};

export function ProductListPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const isProductDetailRoute = location.pathname.startsWith("/products/");
  const {
    activeProductId,
    activeModuleId,
    activeCapabilityId,
    activeNodeId,
    activeNodeType,
    activeWorkItemId,
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
    expandedModules,
    expandedCapabilities,
    closeProductDialog,
    openProductDialog,
    closeModuleDialog,
    openModuleDialog,
    closeCapabilityDialog,
    openCapabilityDialog,
    setProductWorkspaceTab,
    toggleModuleExpanded,
    toggleCapabilityExpanded,
    setModuleExpanded,
    setCapabilityExpanded,
    setActiveView,
  } = useUIStore();

  const [productForm, setProductForm] = useState<ProductFormState>(emptyProductForm);
  const [productDraft, setProductDraft] = useState<ProductFormState>(emptyProductForm);
  const [moduleForm, setModuleForm] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "area" });
  const [moduleDraft, setModuleDraft] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "area" });
  const [capabilityForm, setCapabilityForm] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [capabilityDraft, setCapabilityDraft] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [referenceDraft, setReferenceDraft] = useState<{ title: string; referenceKind: ProductReference["reference_kind"]; uri: string; content: string }>({ title: "", referenceKind: "note", uri: "", content: "" });
  const [structureViewMode, setStructureViewMode] = useState<"children" | "references">("children");
  const [productManagementTab, setProductManagementTab] = useState<"areas" | "capabilities" | "features" | "work_items">("areas");
  const [formError, setFormError] = useState<string | null>(null);
  const [workspaceActionMsg, setWorkspaceActionMsg] = useState<string | null>(null);
  const [workspaceActionError, setWorkspaceActionError] = useState<string | null>(null);
  const [draggedModuleId, setDraggedModuleId] = useState<string | null>(null);
  const [draggedFeature, setDraggedFeature] = useState<null | { id: string; moduleId: string; parentCapabilityId?: string | null; siblingIds: string[] }>(null);
  const [moduleOrderIds, setModuleOrderIds] = useState<string[]>([]);
  const [capabilityOrderMap, setFeatureOrderMap] = useState<Record<string, string[]>>({});
  const [productPageTab, setProductPageTab] = useState<"list" | "status" | "overview" | "design" | "dependencies">(() => isProductDetailRoute ? "design" : "list");
  const [productSearch, setProductSearch] = useState("");
  const [productStatusFilter, setProductStatusFilter] = useState<"all" | Product["status"]>("all");
  const [productSourceFilter, setProductSourceFilter] = useState<"all" | "default" | "custom">("all");
  const [productTagFilter, setProductTagFilter] = useState("all");
  const [productSort, setProductSort] = useState<"name" | "updated" | "progress" | "work">("name");
  const [showDefaultProductsInTable, setShowDefaultProductsInTable] = useState(true);
  const [showCustomProductsInTable, setShowCustomProductsInTable] = useState(true);
  const [catalogFilterMsg, setCatalogFilterMsg] = useState<string | null>(null);
  const [catalogFilterError, setCatalogFilterError] = useState<string | null>(null);
  const [statusProductId, setStatusProductId] = useState<string>("all");
  const [statusDepth, setStatusDepth] = useState(1);
  const [statusGroupBy, setStatusGroupBy] = useState<"node" | "kind" | "work_status">("node");
  const [dependencyDraft, setDependencyDraft] = useState({
    capabilityId: "",
    dependsOnProductId: "",
    dependsOnCapabilityId: "",
    dependencyKind: "platform" as ProductDependencyKind,
    description: "",
  });
  const [deleteProductCandidate, setDeleteProductCandidate] = useState<Product | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [deleteConfirmArchive, setDeleteConfirmArchive] = useState(false);
  const [resetPlanCandidate, setResetPlanCandidate] = useState<Product | null>(null);
  const [resetPlanConfirmName, setResetPlanConfirmName] = useState("");
  const [resetPlanConfirmTree, setResetPlanConfirmTree] = useState(false);
  const [resetPlanDeleteDelivery, setResetPlanDeleteDelivery] = useState(false);
  const [deleteHierarchyCandidate, setDeleteHierarchyCandidate] = useState<null | {
    kind: "area" | "capability" | "feature";
    id: string;
    name: string;
  }>(null);
  const [deleteHierarchyConfirmName, setDeleteHierarchyConfirmName] = useState("");
  const [deleteHierarchyConfirmChecked, setDeleteHierarchyConfirmChecked] = useState(false);
  const [selectedManagementStoryId, setSelectedManagementStoryId] = useState<string | null>(null);
  const [storyDialogMode, setStoryDialogMode] = useState<"closed" | "create" | "edit">("closed");
  const [taskDialogMode, setTaskDialogMode] = useState<"closed" | "create" | "edit">("closed");
  const [editingStory, setEditingStory] = useState<WorkItem | null>(null);
  const [editingTask, setEditingTask] = useState<WorkItem | null>(null);
  const [deleteWorkItemCandidate, setDeleteWorkItemCandidate] = useState<null | { workItem: WorkItem; kind: "story" | "task" }>(null);
  const [deleteWorkItemConfirmName, setDeleteWorkItemConfirmName] = useState("");
  const [deleteWorkItemConfirmChecked, setDeleteWorkItemConfirmChecked] = useState(false);
  const [storyDraft, setStoryDraft] = useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [taskDraft, setTaskDraft] = useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [outlineSearchTerm, setOutlineSearchTerm] = useState("");
  const [outlineKindFilter, setOutlineKindFilter] = useState<HierarchyNodeKind | "">("");
  const [recentNodeKeys, setRecentNodeKeys] = useState<string[]>([]);
  const outlineNodeRefs = React.useRef<Record<string, HTMLDivElement | null>>({});

  const { data: products, isLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: productDependencies = [] } = useQuery({ queryKey: ["product-dependencies"], queryFn: listProductDependencies });
  const { data: productReferences = [] } = useQuery({ queryKey: ["product-references"], queryFn: () => listProductReferences() });
  const { data: hideExampleProductsSetting } = useQuery({
    queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY],
    queryFn: () => getSetting(HIDE_EXAMPLE_PRODUCTS_KEY),
  });
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

  useEffect(() => {
    if (statusProductId === "all") {
      return;
    }
    if (!products?.some((product) => product.id === statusProductId)) {
      setStatusProductId("all");
    }
  }, [products, statusProductId]);

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

  const productTreeQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productTree", product.id],
      queryFn: () => getProductTree(product.id),
      enabled: !!product.id,
    })),
  });

  const productWorkItemQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productAllTasks", product.id],
      queryFn: () => listWorkItems({ productId: product.id }),
      enabled: !!product.id,
    })),
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

  const productTreeById = useMemo(() => {
    const map = new Map<string, ProductTree>();
    (products ?? []).forEach((product, index) => {
      const result = productTreeQueries[index]?.data;
      if (result) {
        map.set(product.id, result);
      }
    });
    return map;
  }, [productTreeQueries, products]);

  const productTasksById = useMemo(() => {
    const map = new Map<string, WorkItem[]>();
    (products ?? []).forEach((product, index) => {
      map.set(product.id, productWorkItemQueries[index]?.data ?? []);
    });
    return map;
  }, [productWorkItemQueries, products]);

  const allProductTags = useMemo(() => {
    const tags = new Set<string>();
    (products ?? []).forEach((product) => product.tags.forEach((tag) => tags.add(tag)));
    return Array.from(tags).sort((a, b) => a.localeCompare(b));
  }, [products]);

  const includeDefaultProductsInCatalog = !parseBooleanSetting(hideExampleProductsSetting, true);
  const productTableRows = useMemo(() => {
    const rows = (products ?? []).map((product) => {
      const treeForProduct = productTreeById.get(product.id);
      const tasksForProduct = productTasksById.get(product.id) ?? [];
      const progress = getProgressSummary(tasksForProduct);
      return {
        product,
        source: isExampleProduct(product) ? "default" as const : "custom" as const,
        rootCount: treeForProduct?.roots.length ?? 0,
        nodeCount: treeForProduct ? countHierarchyNodes(treeForProduct.roots) : 0,
        workItemCount: tasksForProduct.length,
        activeWorkItemCount: tasksForProduct.filter(isActiveWorkItem).length,
        progress,
      };
    });

    const search = productSearch.trim().toLowerCase();
    return rows
      .filter((row) => {
        if (!showDefaultProductsInTable && row.source === "default") return false;
        if (!showCustomProductsInTable && row.source === "custom") return false;
        if (productStatusFilter !== "all" && row.product.status !== productStatusFilter) return false;
        if (productSourceFilter !== "all" && row.source !== productSourceFilter) return false;
        if (productTagFilter !== "all" && !row.product.tags.includes(productTagFilter)) return false;
        if (!search) return true;
        return [
          row.product.name,
          row.product.description,
          row.product.vision,
          row.product.status,
          row.product.lifecycle,
          row.product.health,
          row.product.owner_label,
          row.product.investment_status,
          row.product.roadmap,
          row.product.evidence,
          row.source,
          ...row.product.tags,
        ].join(" ").toLowerCase().includes(search);
      })
      .sort((a, b) => {
        switch (productSort) {
          case "updated":
            return Date.parse(b.product.updated_at) - Date.parse(a.product.updated_at);
          case "progress":
            return b.progress.percent - a.progress.percent || a.product.name.localeCompare(b.product.name);
          case "work":
            return b.workItemCount - a.workItemCount || a.product.name.localeCompare(b.product.name);
          case "name":
          default:
            return a.product.name.localeCompare(b.product.name);
        }
      });
  }, [
    productSearch,
    productSort,
    productSourceFilter,
    productStatusFilter,
    productTagFilter,
    productTasksById,
    productTreeById,
    products,
    showCustomProductsInTable,
    showDefaultProductsInTable,
  ]);

  const selectedStatusProduct = statusProductId === "all"
    ? null
    : products?.find((product) => product.id === statusProductId) ?? null;
  const selectedStatusProducts = selectedStatusProduct ? [selectedStatusProduct] : (products ?? []);
  const statusSummary = useMemo(
    () => buildProductStatusSummary(selectedStatusProducts, productTreeById, productTasksById),
    [productTasksById, productTreeById, selectedStatusProducts],
  );
  const statusRows = useMemo(
    () => buildStatusRows(selectedStatusProducts, productTreeById, productTasksById, statusDepth, statusGroupBy),
    [productTasksById, productTreeById, selectedStatusProducts, statusDepth, statusGroupBy],
  );

  const updateDefaultProductVisibility = async (includeDefaultProducts: boolean) => {
    try {
      setCatalogFilterMsg(null);
      setCatalogFilterError(null);
      await setSetting(HIDE_EXAMPLE_PRODUCTS_KEY, includeDefaultProducts ? "false" : "true");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["products"] }),
        queryClient.invalidateQueries({ queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY] }),
      ]);
      setCatalogFilterMsg(includeDefaultProducts ? "Default products are included." : "Default products are hidden.");
    } catch (error) {
      setCatalogFilterError(String(error));
    }
  };

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
      setProductDraft(productToForm(selectedProduct));
    }
  }, [selectedProduct]);

  useEffect(() => {
    setFormError(null);
    if (productDialogMode === "create") {
      setProductForm(emptyProductForm);
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
      setProductForm(emptyProductForm);
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
        lifecycle: productDraft.lifecycle,
        health: productDraft.health,
        ownerLabel: productDraft.ownerLabel,
        investmentStatus: productDraft.investmentStatus,
        roadmap: productDraft.roadmap,
        evidence: productDraft.evidence,
      }),
    onSuccess: async () => {
      await invalidateHierarchy();
      closeProductDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createProductDependencyMutation = useMutation({
    mutationFn: () => createProductDependency({
      productId: selectedProductId!,
      capabilityId: dependencyDraft.capabilityId || null,
      dependsOnProductId: dependencyDraft.dependsOnProductId,
      dependsOnCapabilityId: dependencyDraft.dependsOnCapabilityId || null,
      dependencyKind: dependencyDraft.dependencyKind,
      description: dependencyDraft.description.trim(),
      status: "active",
    }),
    onSuccess: async () => {
      setDependencyDraft({
        capabilityId: "",
        dependsOnProductId: "",
        dependsOnCapabilityId: "",
        dependencyKind: "platform",
        description: "",
      });
      setFormError(null);
      await queryClient.invalidateQueries({ queryKey: ["product-dependencies"] });
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
        nodeKind: "area",
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

  const archiveMutation = useMutation({
    mutationFn: (id: string) => archiveProduct(id),
    onSuccess: async (_, archivedId) => {
      await invalidateHierarchy();
      if (selectedProductId === archivedId) {
        setActiveProduct(null);
      }
      if (statusProductId === archivedId) {
        setStatusProductId("all");
      }
      setDeleteProductCandidate(null);
      setDeleteConfirmName("");
      setDeleteConfirmArchive(false);
    },
    onError: (error) => setFormError(String(error)),
  });

  const resetProductPlanMutation = useMutation({
    mutationFn: async (data: { productId: string; deleteDelivery: boolean }) => {
      if (data.productId !== selectedProductId) {
        throw new Error("Select the product before resetting its plan.");
      }
      const modulesToDelete = tree?.modules ?? [];
      await Promise.all(modulesToDelete.map((moduleTree) => deleteModule(moduleTree.module.id)));
      if (data.deleteDelivery) {
        await Promise.all(allProductTasks.map((workItem) => deleteWorkItem(workItem.id)));
      }
    },
    onSuccess: async () => {
      await invalidateHierarchy();
      await invalidateTasks();
      setActiveModule(null);
      setActiveCapability(null);
      setResetPlanCandidate(null);
      setResetPlanConfirmName("");
      setResetPlanConfirmTree(false);
      setResetPlanDeleteDelivery(false);
      setProductWorkspaceTab("structure");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteHierarchyMutation = useMutation({
    mutationFn: async (candidate: NonNullable<typeof deleteHierarchyCandidate>) => {
      if (candidate.kind === "area") {
        await deleteModule(candidate.id);
        return;
      }
      await deleteCapability(candidate.id);
    },
    onSuccess: async () => {
      await invalidateHierarchy();
      setActiveModule(null);
      setActiveCapability(null);
      setDeleteHierarchyCandidate(null);
      setDeleteHierarchyConfirmName("");
      setDeleteHierarchyConfirmChecked(false);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const reorderModulesMutation = useMutation({
    mutationFn: (orderedIds: string[]) => reorderModules(selectedProductId!, orderedIds),
    onSuccess: async () => invalidateHierarchy(),
  });

  const reorderCapabilitiesMutation = useMutation({
    mutationFn: (data: { moduleId: string; parentCapabilityId?: string; orderedIds: string[] }) => reorderCapabilities(data),
    onSuccess: async () => invalidateHierarchy(),
  });

  const createManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode) {
        throw new Error("Select a feature before adding a story.");
      }
      return createWorkItem({
        productId: selectedProductId,
        moduleId: selectedManagementFeatureNode.module_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        title: storyDraft.title.trim(),
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
        workItemType: "feature",
        priority: storyDraft.priority,
        complexity: storyDraft.complexity,
      });
    },
    onSuccess: async (createdStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(createdStory.id);
      setActiveWorkItem(createdStory.id);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!editingStory) {
        throw new Error("Select a story before editing.");
      }
      return updateWorkItem({
        id: editingStory.id,
        title: storyDraft.title.trim(),
        status: storyDraft.status,
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
      });
    },
    onSuccess: async (updatedStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(updatedStory.id);
      setActiveWorkItem(updatedStory.id);
      setEditingStory(null);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const createManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode || !selectedManagementStory) {
        throw new Error("Select a story before adding a task.");
      }
      return createWorkItem({
        productId: selectedProductId,
        moduleId: selectedManagementFeatureNode.module_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        parentWorkItemId: selectedManagementStory.id,
        title: taskDraft.title.trim(),
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
        workItemType: "feature",
        priority: taskDraft.priority,
        complexity: taskDraft.complexity,
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!editingTask) {
        throw new Error("Select a task before editing.");
      }
      return updateWorkItem({
        id: editingTask.id,
        title: taskDraft.title.trim(),
        status: taskDraft.status,
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setEditingTask(null);
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteManagementWorkItemMutation = useMutation({
    mutationFn: async (candidate: { workItem: WorkItem; kind: "story" | "task" }) => {
      if (candidate.kind === "story") {
        const childTasks = allProductTasks.filter((workItem) => workItem.parent_work_item_id === candidate.workItem.id);
        await Promise.all(childTasks.map((workItem) => deleteWorkItem(workItem.id)));
      }
      await deleteWorkItem(candidate.workItem.id);
    },
    onSuccess: async () => {
      await invalidateTasks();
      setDeleteWorkItemCandidate(null);
      setDeleteWorkItemConfirmName("");
      setDeleteWorkItemConfirmChecked(false);
      setSelectedManagementStoryId(null);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
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
  const selectedHierarchyNode = useMemo(
    () => (tree ? findHierarchyNode(tree.roots, activeNodeId, activeNodeType) : null),
    [tree, activeNodeId, activeNodeType],
  );
  const selectedHierarchyPath = useMemo(
    () => (tree ? findHierarchyNodePath(tree.roots, activeNodeId, activeNodeType) : []),
    [tree, activeNodeId, activeNodeType],
  );
  const allTreeNodes = useMemo(() => (tree ? flattenHierarchyNodes(tree.roots) : []), [tree]);
  const canonicalManagementNodeCount = useMemo(
    () => allTreeNodes.filter((node) => node.node_kind === "area" || node.node_kind === "capability" || node.node_kind === "feature").length,
    [allTreeNodes],
  );
  const selectedCapabilityOptions = useMemo(
    () => allTreeNodes
      .filter((node) => node.node_type === "capability")
      .map((node) => ({ id: node.id, label: node.path.join(" / ") })),
    [allTreeNodes],
  );
  const dependencyTargetCapabilityOptions = useMemo(() => {
    const targetTree = productTreeById.get(dependencyDraft.dependsOnProductId);
    return targetTree ? flattenHierarchyNodes(targetTree.roots)
      .filter((node) => node.node_type === "capability")
      .map((node) => ({ id: node.id, label: node.path.join(" / ") })) : [];
  }, [dependencyDraft.dependsOnProductId, productTreeById]);
  const selectedProductDependencies = useMemo(
    () => productDependencies.filter((dependency) => dependency.product_id === selectedProductId),
    [productDependencies, selectedProductId],
  );
  const productNameById = useMemo(
    () => new Map((products ?? []).map((product) => [product.id, product.name])),
    [products],
  );
  const capabilityLabelById = useMemo(() => {
    const map = new Map<string, string>();
    productTreeById.forEach((productTree) => {
      flattenHierarchyNodes(productTree.roots)
        .filter((node) => node.node_type === "capability")
        .forEach((node) => map.set(node.id, node.path.join(" / ")));
    });
    allTreeNodes
      .filter((node) => node.node_type === "capability")
      .forEach((node) => map.set(node.id, node.path.join(" / ")));
    return map;
  }, [allTreeNodes, productTreeById]);
  const nodeLookup = useMemo(
    () => new Map(allTreeNodes.map((node) => [getHierarchyNodeKey(node), node])),
    [allTreeNodes],
  );
  const selectedNodeKey = activeNodeId && activeNodeType ? `${activeNodeType}:${activeNodeId}` : null;
  const outlineNodeKindOptions = useMemo(
    () => orderHierarchyNodeKinds(Array.from(new Set(allTreeNodes.map((node) => node.node_kind)))),
    [allTreeNodes],
  );
  const outlineNodeKindGroups = useMemo(() => groupHierarchyNodeKinds(outlineNodeKindOptions), [outlineNodeKindOptions]);
  const hasOutlineFilter = outlineSearchTerm.trim().length > 0 || outlineKindFilter.length > 0;
  const filteredOutlineRoots = useMemo(() => {
    if (!tree) {
      return [];
    }
    if (!hasOutlineFilter) {
      return tree.roots;
    }

    const normalizedSearch = outlineSearchTerm.trim().toLowerCase();
    const filterNode = (node: HierarchyTreeNode): HierarchyTreeNode | null => {
      const childMatches = node.children
        .map(filterNode)
        .filter(Boolean) as HierarchyTreeNode[];
      const matchesSearch = normalizedSearch.length === 0
        || [node.name, ...node.path, node.description, node.summary].join(" ").toLowerCase().includes(normalizedSearch);
      const matchesKind = !outlineKindFilter || node.node_kind === outlineKindFilter;
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
  }, [hasOutlineFilter, outlineKindFilter, outlineSearchTerm, tree]);
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
  const selectedReferenceScope = useMemo(() => {
    if (!selectedProductId) {
      return null;
    }
    if (selectedHierarchyNode?.node_type === "capability" && selectedHierarchyNode.capability_id) {
      return {
        scopeType: selectedHierarchyNode.node_kind === "feature" ? "feature" as const : "capability" as const,
        scopeId: selectedHierarchyNode.capability_id,
      };
    }
    return { scopeType: "product" as const, scopeId: selectedProductId };
  }, [selectedHierarchyNode, selectedProductId]);
  const selectedReferences = useMemo(
    () => selectedReferenceScope
      ? productReferences.filter((reference) => reference.scope_type === selectedReferenceScope.scopeType && reference.scope_id === selectedReferenceScope.scopeId)
      : [],
    [productReferences, selectedReferenceScope],
  );
  const createProductReferenceMutation = useMutation({
    mutationFn: () => {
      if (!selectedReferenceScope) {
        throw new Error("Select a product management scope before adding a reference.");
      }
      return createProductReference({
        scopeType: selectedReferenceScope.scopeType,
        scopeId: selectedReferenceScope.scopeId,
        title: referenceDraft.title.trim(),
        referenceKind: referenceDraft.referenceKind,
        uri: referenceDraft.uri.trim(),
        content: referenceDraft.content.trim(),
      });
    },
    onSuccess: async () => {
      setReferenceDraft({ title: "", referenceKind: "note", uri: "", content: "" });
      setFormError(null);
      await queryClient.invalidateQueries({ queryKey: ["product-references"] });
    },
    onError: (error) => setFormError(String(error)),
  });
  const deleteProductReferenceMutation = useMutation({
    mutationFn: (id: string) => deleteProductReference(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["product-references"] });
    },
    onError: (error) => setFormError(String(error)),
  });
  const selectedMetricCards = selectedHierarchyNode
    ? [
        { label: "Direct Children", value: selectedDirectChildren.length, help: `${selectedDirectChildren.length} immediate child ${selectedDirectChildren.length === 1 ? "node" : "nodes"}` },
        { label: "Subtree Nodes", value: countDescendantNodes(selectedHierarchyNode) + 1, help: "Selected node plus all nested descendants" },
        { label: "References", value: selectedReferences.length, help: "Attached context for this management scope" },
        { label: "Dependencies", value: selectedProductDependencies.length, help: "Cross-product dependencies for this product" },
      ]
    : [
        { label: "Product Areas", value: tree?.roots.filter((node) => node.node_kind === "area").length ?? 0, help: "Top-level product management areas" },
        { label: "Management Nodes", value: canonicalManagementNodeCount, help: "Product areas, capabilities, and features" },
        { label: "References", value: selectedReferences.length, help: "Attached product context" },
        { label: "Dependencies", value: selectedProductDependencies.length, help: "Cross-product dependencies" },
      ];
  const editableCapabilityNodeKinds = useMemo(() => {
    if (!selectedCapability) {
      return [] as HierarchyNodeKind[];
    }
    const allowedKinds = getAllowedChildNodeKinds(selectedCapabilityParentKind);
    return orderHierarchyNodeKinds(allowedKinds.includes(selectedCapability.node_kind)
      ? allowedKinds
      : [selectedCapability.node_kind, ...allowedKinds]);
  }, [selectedCapability, selectedCapabilityParentKind]);
  const selectedCapabilityAllowedKindGroups = useMemo(
    () => groupHierarchyNodeKinds(getAllowedChildNodeKinds(selectedCapability?.node_kind ?? selectedModule?.node_kind)),
    [selectedCapability?.node_kind, selectedModule?.node_kind],
  );
  const editableCapabilityNodeKindGroups = useMemo(
    () => groupHierarchyNodeKinds(editableCapabilityNodeKinds),
    [editableCapabilityNodeKinds],
  );
  const orderedModules = useMemo(() => {
    if (!tree) {
      return [];
    }
    return orderItemsByIds(tree.modules, moduleOrderIds, (moduleTree) => moduleTree.module.id);
  }, [tree, moduleOrderIds]);
  const productAreaModules = useMemo(
    () => orderedModules.filter((moduleTree) => moduleTree.module.node_kind === "area"),
    [orderedModules],
  );
  const selectedProductAreaTree = useMemo(
    () => productAreaModules.find((moduleTree) => moduleTree.module.id === activeModuleId) ?? productAreaModules[0] ?? null,
    [activeModuleId, productAreaModules],
  );
  const selectedProductAreaNode = useMemo(
    () => selectedProductAreaTree ? tree?.roots.find((node) => node.id === selectedProductAreaTree.module.id) ?? null : null,
    [selectedProductAreaTree, tree],
  );
  const managementCapabilities = useMemo(
    () => selectedProductAreaTree
      ? getOrderedCapabilityTrees(
          selectedProductAreaTree.features,
          capabilityOrderMap[getCapabilityOrderKey(selectedProductAreaTree.module.id, null)],
        ).filter((capabilityTree) => capabilityTree.capability.node_kind === "capability")
      : [],
    [capabilityOrderMap, selectedProductAreaTree],
  );
  const selectedManagementCapabilityTree = useMemo(() => {
    const selectedTopLevelCapability = managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === activeCapabilityId);
    if (selectedTopLevelCapability) {
      return selectedTopLevelCapability;
    }
    if (selectedCapability?.parent_capability_id) {
      return managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === selectedCapability.parent_capability_id) ?? managementCapabilities[0] ?? null;
    }
    return managementCapabilities[0] ?? null;
  }, [activeCapabilityId, managementCapabilities, selectedCapability?.parent_capability_id]);
  const managementFeatures = useMemo(
    () => selectedManagementCapabilityTree
      ? getOrderedCapabilityTrees(
          selectedManagementCapabilityTree.children,
          capabilityOrderMap[getCapabilityOrderKey(selectedManagementCapabilityTree.capability.module_id, selectedManagementCapabilityTree.capability.id)],
        ).filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
      : [],
    [capabilityOrderMap, selectedManagementCapabilityTree],
  );
  const allManagementFeatures = useMemo(
    () => productAreaModules.flatMap((moduleTree) =>
      flattenCapabilityTreeList(moduleTree.features)
        .filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
        .map((capabilityTree) => ({
          capabilityTree,
          productArea: moduleTree.module,
          parentCapability: capabilityTree.capability.parent_capability_id
            ? findCapabilityTree(productAreaModules, capabilityTree.capability.parent_capability_id)?.capability ?? null
            : null,
        })),
    ),
    [productAreaModules],
  );
  const selectedManagementFeature = useMemo(() => {
    const activeFeature = allManagementFeatures.find((entry) => entry.capabilityTree.capability.id === activeCapabilityId);
    if (activeFeature) {
      return activeFeature;
    }
    return allManagementFeatures[0] ?? null;
  }, [activeCapabilityId, allManagementFeatures]);
  const selectedManagementFeatureNode = useMemo(
    () => selectedManagementFeature ? findHierarchyNode(tree?.roots ?? [], selectedManagementFeature.capabilityTree.capability.id, "capability") : null,
    [selectedManagementFeature, tree],
  );
  const featureStories = useMemo(() => {
    if (!selectedManagementFeatureNode) {
      return [];
    }
    return getDirectWorkItemsForNode(selectedManagementFeatureNode, allProductTasks)
      .filter((workItem) => !workItem.parent_work_item_id);
  }, [allProductTasks, selectedManagementFeatureNode]);
  const selectedManagementStory = useMemo(
    () => featureStories.find((workItem) => workItem.id === selectedManagementStoryId)
      ?? featureStories.find((workItem) => workItem.id === activeWorkItemId)
      ?? featureStories[0]
      ?? null,
    [activeWorkItemId, featureStories, selectedManagementStoryId],
  );
  const selectedManagementTasks = useMemo(
    () => selectedManagementStory
      ? allProductTasks.filter((workItem) => workItem.parent_work_item_id === selectedManagementStory.id)
      : [],
    [allProductTasks, selectedManagementStory],
  );
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
      references: productReferences.filter((reference) => {
        const scopeType = node.node_kind === "feature" ? "feature" : node.node_type === "capability" ? "capability" : "product";
        const scopeId = node.node_type === "capability" ? node.id : selectedProductId;
        return reference.scope_type === scopeType && reference.scope_id === scopeId;
      }).length,
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
  }, [productReferences, selectedDirectChildren, selectedProductId, setActiveHierarchyNode, tree]);

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

  const selectProductArea = (moduleTree: ModuleTree) => {
    setActiveHierarchyNode({
      nodeId: moduleTree.module.id,
      nodeType: "module",
      moduleId: moduleTree.module.id,
      capabilityId: null,
    });
  };

  const selectCapabilityForManagement = (capabilityTree: CapabilityTree) => {
    setActiveHierarchyNode({
      nodeId: capabilityTree.capability.id,
      nodeType: "capability",
      moduleId: capabilityTree.capability.module_id,
      capabilityId: capabilityTree.capability.id,
    });
  };

  const openCreateCapabilityForArea = (moduleTree: ModuleTree) => {
    selectProductArea(moduleTree);
    setCapabilityForm({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
    openCapabilityDialog("create");
  };

  const openCreateFeatureForCapability = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityForm({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "feature" });
    openCapabilityDialog("create");
  };

  const openEditProductArea = (moduleTree: ModuleTree) => {
    selectProductArea(moduleTree);
    setModuleDraft({
      name: moduleTree.module.name,
      description: moduleTree.module.description,
      purpose: moduleTree.module.purpose,
      nodeKind: moduleTree.module.node_kind,
    });
    openModuleDialog("edit");
  };

  const openEditCapabilityNode = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityDraft({
      name: capabilityTree.capability.name,
      description: capabilityTree.capability.description,
      acceptanceCriteria: capabilityTree.capability.acceptance_criteria,
      technicalNotes: capabilityTree.capability.technical_notes,
      nodeKind: capabilityTree.capability.node_kind,
    });
    openCapabilityDialog("edit");
  };

  const requestDeleteHierarchyNode = (candidate: NonNullable<typeof deleteHierarchyCandidate>) => {
    setDeleteHierarchyCandidate(candidate);
    setDeleteHierarchyConfirmName("");
    setDeleteHierarchyConfirmChecked(false);
    setFormError(null);
  };

  const openFeatureInBuilder = (featureNode: HierarchyTreeNode | null) => {
    if (featureNode) {
      setActiveHierarchyNode({
        nodeId: featureNode.id,
        nodeType: featureNode.node_type,
        moduleId: featureNode.module_id,
        capabilityId: featureNode.capability_id,
      });
    }
    setActiveView("work-items");
    navigate("/work-items");
  };

  const openStoryInBuilder = (story: WorkItem) => {
    setSelectedManagementStoryId(story.id);
    setActiveWorkItem(story.id);
    setActiveView("work-items");
    navigate("/work-items");
  };

  useEffect(() => {
    if (!selectedNodeKey) {
      return;
    }
    setRecentNodeKeys((current) => [selectedNodeKey, ...current.filter((key) => key !== selectedNodeKey)].slice(0, 6));
  }, [selectedNodeKey]);

  const setOutlineNodeExpandedState = (node: HierarchyTreeNode, expanded: boolean) => {
    if (node.node_type === "module") {
      setModuleExpanded(node.id, expanded);
      return;
    }
    setCapabilityExpanded(node.id, expanded);
  };

  const collapseOutlineNodes = () => {
    allTreeNodes.forEach((node) => setOutlineNodeExpandedState(node, false));
  };

  const expandSelectedOutlinePath = () => {
    selectedHierarchyPath.forEach((node) => setOutlineNodeExpandedState(node, true));
  };

  const jumpToSelectedOutlineNode = () => {
    if (!selectedNodeKey) {
      return;
    }
    expandSelectedOutlinePath();
    requestAnimationFrame(() => {
      outlineNodeRefs.current[selectedNodeKey]?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  };

  const openOutlineNode = (node: HierarchyTreeNode) => {
    setActiveHierarchyNode({
      nodeId: node.id,
      nodeType: node.node_type,
      moduleId: node.module_id,
      capabilityId: node.capability_id,
    });
    setProductWorkspaceTab("structure");
  };

  const createChildForOutlineNode = (node: HierarchyTreeNode) => {
    openOutlineNode(node);
    openCapabilityDialog("create");
  };

  const editOutlineNode = (node: HierarchyTreeNode) => {
    openOutlineNode(node);
    if (node.node_type === "module") {
      openModuleDialog("edit");
      return;
    }
    openCapabilityDialog("edit");
  };

  const renderReferencesPanel = () => (
    <div style={styles.section}>
      <div style={styles.sectionTitle}>
        <span>References</span>
        <span style={styles.badgeMuted}>{selectedReferences.length}</span>
      </div>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>Attached Context</div>
        <div style={styles.contextTitle}>{selectedNodeTitle}</div>
        <div style={styles.contextText}>
          References stay with product management scopes and are available as context for product owner, builder, and agent work.
        </div>
        <div style={styles.chipRow}>
          <span style={styles.badgeKind}>{selectedReferenceScope?.scopeType.replace("_", " ") ?? "product"}</span>
          <span style={styles.badgeMuted}>{selectedScopePath.join(" / ") || selectedProduct?.name}</span>
        </div>
      </div>
      <div style={styles.contextCard}>
        <div style={styles.formRow}>
          <div>
            <label style={styles.label}>Title</label>
            <input
              style={styles.input}
              value={referenceDraft.title}
              onChange={(event) => setReferenceDraft({ ...referenceDraft, title: event.target.value })}
              placeholder="Architecture note, standard, evidence packet"
            />
          </div>
          <div>
            <label style={styles.label}>Kind</label>
            <select
              style={styles.select}
              value={referenceDraft.referenceKind}
              onChange={(event) => setReferenceDraft({ ...referenceDraft, referenceKind: event.target.value as ProductReference["reference_kind"] })}
            >
              {referenceKindOptions.map((kind) => (
                <option key={kind} value={kind}>{kind.replace("_", " ")}</option>
              ))}
            </select>
          </div>
        </div>
        <label style={styles.label}>URI</label>
        <input
          style={styles.input}
          value={referenceDraft.uri}
          onChange={(event) => setReferenceDraft({ ...referenceDraft, uri: event.target.value })}
          placeholder="https://..., repo path, or document id"
        />
        <label style={styles.label}>Notes</label>
        <textarea
          style={styles.textarea}
          value={referenceDraft.content}
          onChange={(event) => setReferenceDraft({ ...referenceDraft, content: event.target.value })}
          placeholder="Relevant context, constraints, or evidence"
        />
        {formError && <div style={styles.errorText}>{formError}</div>}
        <button
          style={styles.btn}
          onClick={() => createProductReferenceMutation.mutate()}
          disabled={!selectedReferenceScope || !referenceDraft.title.trim() || createProductReferenceMutation.isPending}
        >
          {createProductReferenceMutation.isPending ? "Adding..." : "Add Reference"}
        </button>
      </div>
      {selectedReferences.length > 0 ? (
        <div style={styles.table}>
          <div style={styles.tableHeader}>
            <div>Title</div>
            <div>Kind</div>
            <div>URI</div>
            <div>Action</div>
          </div>
          {selectedReferences.map((reference) => (
            <div key={reference.id} style={styles.tableRow}>
              <div>
                <div style={styles.rowPrimary}>{reference.title}</div>
                {reference.content ? <div style={styles.rowSecondary}>{reference.content}</div> : null}
              </div>
              <div style={styles.rowCell}>{reference.reference_kind.replace("_", " ")}</div>
              <div style={styles.rowCell}>{reference.uri || "None"}</div>
              <button
                style={styles.ghostBtn}
                onClick={() => deleteProductReferenceMutation.mutate(reference.id)}
                disabled={deleteProductReferenceMutation.isPending}
              >
                Delete
              </button>
            </div>
          ))}
        </div>
      ) : (
        <div style={styles.empty}>No references are attached to this scope yet.</div>
      )}
    </div>
  );

  const renderOutlineNode = (node: HierarchyTreeNode, depth = 0): React.ReactNode => {
    const nodeKey = getHierarchyNodeKey(node);
    const isActive = selectedNodeKey === nodeKey;
    const isExpanded = hasOutlineFilter
      ? true
      : node.node_type === "module"
        ? expandedModules[node.id] ?? true
        : expandedCapabilities[node.id] ?? true;
    return (
      <div key={nodeKey}>
        <div
          ref={(element) => {
            outlineNodeRefs.current[nodeKey] = element;
          }}
          style={{
            ...(isActive ? styles.outlineNodeActive : styles.outlineNode),
            marginLeft: depth * 10,
          }}
        >
          <div style={styles.outlineNodeHeader}>
            {node.children.length > 0 ? (
              <button
                style={styles.outlineToggle}
                onClick={(event) => {
                  event.stopPropagation();
                  if (node.node_type === "module") {
                    toggleModuleExpanded(node.id);
                  } else {
                    toggleCapabilityExpanded(node.id);
                  }
                }}
              >
                {isExpanded ? "-" : "+"}
              </button>
            ) : (
              <div style={styles.outlineToggle}>.</div>
            )}
            <div style={styles.outlineNodeBody} onClick={() => openOutlineNode(node)}>
              <div style={styles.outlineNodeTitle}>{node.name}</div>
              <div style={styles.outlineNodeMeta}>
                {getHierarchyNodeKindLabel(node.node_kind)} · {node.children.length} {node.children.length === 1 ? "child" : "children"}
              </div>
              {node.summary || node.description ? <div style={styles.outlineNodeMeta}>{node.summary || node.description}</div> : null}
            </div>
          </div>
          <div style={styles.outlineActionRow}>
            {supportsHierarchyChildren(node.node_kind) ? (
              <button
                style={styles.outlineActionBtn}
                onClick={(event) => {
                  event.stopPropagation();
                  createChildForOutlineNode(node);
                }}
              >
                + Child Node
              </button>
            ) : null}
            <button
              style={styles.outlineActionBtn}
              onClick={(event) => {
                event.stopPropagation();
                editOutlineNode(node);
              }}
            >
              Edit
            </button>
          </div>
        </div>
        {isExpanded && node.children.length > 0 ? (
          <div style={styles.outlineChildWrap}>
            {node.children.map((child) => renderOutlineNode(child, depth + 1))}
          </div>
        ) : null}
      </div>
    );
  };

  const editProductFromList = (product: Product) => {
    setActiveProduct(product.id);
    setProductDraft(productToForm(product));
    setFormError(null);
    openProductDialog("edit");
  };

  const openProductDesign = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("design");
    navigate(`/products/${product.id}`);
  };

  const openProductOverview = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("overview");
  };

  const openProductStatus = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("status");
  };

  const openProductDependencies = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("dependencies");
  };

  const requestArchiveProduct = (product: Product) => {
    setDeleteProductCandidate(product);
    setDeleteConfirmName("");
    setDeleteConfirmArchive(false);
    setFormError(null);
  };

  const requestResetProductPlan = (product: Product) => {
    setResetPlanCandidate(product);
    setResetPlanConfirmName("");
    setResetPlanConfirmTree(false);
    setResetPlanDeleteDelivery(false);
    setFormError(null);
  };

  const openCreateStoryDialog = () => {
    setEditingStory(null);
    setStoryDraft(emptyWorkItemDraft);
    setStoryDialogMode("create");
    setFormError(null);
  };

  const openEditStoryDialog = (story: WorkItem) => {
    setEditingStory(story);
    setStoryDraft(workItemToDraft(story));
    setStoryDialogMode("edit");
    setFormError(null);
  };

  const openCreateTaskDialog = () => {
    setEditingTask(null);
    setTaskDraft(emptyWorkItemDraft);
    setTaskDialogMode("create");
    setFormError(null);
  };

  const openEditTaskDialog = (task: WorkItem) => {
    setEditingTask(task);
    setTaskDraft(workItemToDraft(task));
    setTaskDialogMode("edit");
    setFormError(null);
  };

  const requestDeleteWorkItem = (workItem: WorkItem, kind: "story" | "task") => {
    setDeleteWorkItemCandidate({ workItem, kind });
    setDeleteWorkItemConfirmName("");
    setDeleteWorkItemConfirmChecked(false);
    setFormError(null);
  };

  const deleteConfirmationReady = !!deleteProductCandidate
    && deleteConfirmName.trim() === deleteProductCandidate.name
    && deleteConfirmArchive;
  const resetPlanReady = !!resetPlanCandidate
    && resetPlanConfirmName.trim() === resetPlanCandidate.name
    && resetPlanConfirmTree;
  const deleteHierarchyReady = !!deleteHierarchyCandidate
    && deleteHierarchyConfirmName.trim() === deleteHierarchyCandidate.name
    && deleteHierarchyConfirmChecked;
  const deleteManagementWorkItemReady = !!deleteWorkItemCandidate
    && deleteWorkItemConfirmName.trim() === deleteWorkItemCandidate.workItem.title
    && deleteWorkItemConfirmChecked;

  const renderProductManagementConsole = () => (
    <div>
      <div style={styles.managementTabs}>
        <button style={productManagementTab === "areas" ? styles.tabActive : styles.tab} onClick={() => setProductManagementTab("areas")}>Product Areas</button>
        <button style={productManagementTab === "capabilities" ? styles.tabActive : styles.tab} onClick={() => setProductManagementTab("capabilities")}>Capabilities</button>
        <button style={productManagementTab === "features" ? styles.tabActive : styles.tab} onClick={() => setProductManagementTab("features")}>Features</button>
        <button style={productManagementTab === "work_items" ? styles.tabActive : styles.tab} onClick={() => setProductManagementTab("work_items")}>Work Items</button>
      </div>

      {productManagementTab === "areas" && (
        <div style={styles.section}>
          <div style={styles.sectionTitle}>
            <span>Product Areas</span>
            <div style={styles.managementActions}>
              <button style={styles.ghostBtn} onClick={() => requestResetProductPlan(selectedProduct!)}>Reset Plan</button>
              <button style={styles.btn} onClick={() => openModuleDialog("create")}>+ Product Area</button>
            </div>
          </div>
          {productAreaModules.length > 0 ? (
            <div style={styles.table}>
              <div style={styles.managementTableHeader}>
                <div>Product Area</div>
                <div>Capabilities</div>
                <div>Features</div>
                <div>Actions</div>
              </div>
              {productAreaModules.map((moduleTree) => (
                <div key={moduleTree.module.id} style={styles.managementTableRow}>
                  <div>
                    <div style={styles.rowPrimary}>{moduleTree.module.name}</div>
                    <div style={styles.rowSecondary}>{moduleTree.module.description || moduleTree.module.purpose || "No description yet."}</div>
                  </div>
                  <div style={styles.rowCell}>{moduleTree.features.filter((node) => node.capability.node_kind === "capability").length}</div>
                  <div style={styles.rowCell}>{flattenCapabilityTreeList(moduleTree.features).filter((node) => node.capability.node_kind === "feature").length}</div>
                  <div style={styles.managementActions}>
                    <button style={styles.compactActionBtn} onClick={() => {
                      selectProductArea(moduleTree);
                      setProductManagementTab("capabilities");
                    }}>Open</button>
                    <button style={styles.compactActionBtn} onClick={() => openEditProductArea(moduleTree)}>Edit</button>
                    <button
                      style={styles.compactDangerBtn}
                      onClick={() => requestDeleteHierarchyNode({ kind: "area", id: moduleTree.module.id, name: moduleTree.module.name })}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div style={styles.empty}>No product areas yet. Add a product area to start the management model.</div>
          )}
        </div>
      )}

      {productManagementTab === "capabilities" && (
        <div style={styles.managementLayout}>
          <div style={styles.managementPane}>
            <div style={styles.managementPaneHeader}>
              <div style={styles.controlLabel}>Product Areas</div>
            </div>
            <div style={styles.managementList}>
              {productAreaModules.map((moduleTree) => (
                <button
                  key={moduleTree.module.id}
                  style={selectedProductAreaTree?.module.id === moduleTree.module.id ? styles.managementListButtonActive : styles.managementListButton}
                  onClick={() => selectProductArea(moduleTree)}
                >
                  <div style={styles.rowPrimary}>{moduleTree.module.name}</div>
                  <div style={styles.rowSecondary}>{moduleTree.features.length} child nodes</div>
                </button>
              ))}
            </div>
          </div>
          <div style={styles.managementPane}>
            <div style={styles.sectionTitle}>
              <span>{selectedProductAreaTree?.module.name ?? "Capabilities"}</span>
              <button
                style={styles.btn}
                onClick={() => selectedProductAreaTree && openCreateCapabilityForArea(selectedProductAreaTree)}
                disabled={!selectedProductAreaTree}
              >
                + Capability
              </button>
            </div>
            {managementCapabilities.length > 0 ? (
              <div style={styles.table}>
                <div style={styles.managementTableHeader}>
                  <div>Capability</div>
                  <div>Features</div>
                  <div>Stories</div>
                  <div>Actions</div>
                </div>
                {managementCapabilities.map((capabilityTree) => {
                  const capabilityNode = findHierarchyNode(tree?.roots ?? [], capabilityTree.capability.id, "capability");
                  const storyCount = capabilityNode ? getSubtreeWorkItemsForNode(capabilityNode, allProductTasks).filter((workItem) => !workItem.parent_work_item_id).length : 0;
                  return (
                    <div key={capabilityTree.capability.id} style={styles.managementTableRow}>
                      <div>
                        <div style={styles.rowPrimary}>{capabilityTree.capability.name}</div>
                        <div style={styles.rowSecondary}>{capabilityTree.capability.description || "No description yet."}</div>
                      </div>
                      <div style={styles.rowCell}>{capabilityTree.children.filter((node) => node.capability.node_kind === "feature").length}</div>
                      <div style={styles.rowCell}>{storyCount}</div>
                      <div style={styles.managementActions}>
                        <button style={styles.compactActionBtn} onClick={() => {
                          selectCapabilityForManagement(capabilityTree);
                          setProductManagementTab("features");
                        }}>Open</button>
                        <button style={styles.compactActionBtn} onClick={() => openEditCapabilityNode(capabilityTree)}>Edit</button>
                        <button
                          style={styles.compactDangerBtn}
                          onClick={() => requestDeleteHierarchyNode({ kind: "capability", id: capabilityTree.capability.id, name: capabilityTree.capability.name })}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={styles.empty}>No capabilities in this product area yet.</div>
            )}
          </div>
        </div>
      )}

      {productManagementTab === "features" && (
        <div style={styles.managementLayout}>
          <div style={styles.managementPane}>
            <div style={styles.controlLabel}>Capabilities</div>
            <div style={styles.managementList}>
              {managementCapabilities.map((capabilityTree) => (
                <button
                  key={capabilityTree.capability.id}
                  style={selectedManagementCapabilityTree?.capability.id === capabilityTree.capability.id ? styles.managementListButtonActive : styles.managementListButton}
                  onClick={() => selectCapabilityForManagement(capabilityTree)}
                >
                  <div style={styles.rowPrimary}>{capabilityTree.capability.name}</div>
                  <div style={styles.rowSecondary}>{capabilityTree.children.filter((node) => node.capability.node_kind === "feature").length} features</div>
                </button>
              ))}
            </div>
          </div>
          <div style={styles.managementPane}>
            <div style={styles.sectionTitle}>
              <span>{selectedManagementCapabilityTree?.capability.name ?? "Features"}</span>
              <button
                style={styles.btn}
                onClick={() => selectedManagementCapabilityTree && openCreateFeatureForCapability(selectedManagementCapabilityTree)}
                disabled={!selectedManagementCapabilityTree}
              >
                + Feature
              </button>
            </div>
            {managementFeatures.length > 0 ? (
              <div style={styles.table}>
                <div style={styles.managementTableHeader}>
                  <div>Feature</div>
                  <div>Status</div>
                  <div>Stories</div>
                  <div>Actions</div>
                </div>
                {managementFeatures.map((featureTree) => {
                  const featureNode = findHierarchyNode(tree?.roots ?? [], featureTree.capability.id, "capability");
                  const stories = featureNode ? getDirectWorkItemsForNode(featureNode, allProductTasks).filter((workItem) => !workItem.parent_work_item_id) : [];
                  return (
                    <div key={featureTree.capability.id} style={styles.managementTableRow}>
                      <div>
                        <div style={styles.rowPrimary}>{featureTree.capability.name}</div>
                        <div style={styles.rowSecondary}>{featureTree.capability.description || "No description yet."}</div>
                      </div>
                      <div style={styles.rowCell}>{featureTree.capability.status}</div>
                      <div style={styles.rowCell}>{stories.length}</div>
                      <div style={styles.managementActions}>
                        <button style={styles.compactActionBtn} onClick={() => {
                          selectCapabilityForManagement(featureTree);
                          setProductManagementTab("work_items");
                        }}>Stories</button>
                        <button style={styles.compactActionBtn} onClick={() => openEditCapabilityNode(featureTree)}>Edit</button>
                        <button
                          style={styles.compactDangerBtn}
                          onClick={() => requestDeleteHierarchyNode({ kind: "feature", id: featureTree.capability.id, name: featureTree.capability.name })}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={styles.empty}>No features in this capability yet.</div>
            )}
          </div>
        </div>
      )}

      {productManagementTab === "work_items" && (
        <div style={styles.managementThreePane}>
          <div style={styles.managementPane}>
            <div style={styles.controlLabel}>Features</div>
            <div style={styles.managementList}>
              {allManagementFeatures.map((entry) => (
                <button
                  key={entry.capabilityTree.capability.id}
                  style={selectedManagementFeature?.capabilityTree.capability.id === entry.capabilityTree.capability.id ? styles.managementListButtonActive : styles.managementListButton}
                  onClick={() => {
                    selectCapabilityForManagement(entry.capabilityTree);
                    setSelectedManagementStoryId(null);
                  }}
                >
                  <div style={styles.rowPrimary}>{entry.capabilityTree.capability.name}</div>
                  <div style={styles.rowSecondary}>{entry.productArea.name}{entry.parentCapability ? ` / ${entry.parentCapability.name}` : ""}</div>
                </button>
              ))}
            </div>
          </div>
          <div style={styles.managementPane}>
            <div style={styles.sectionTitle}>
              <span>Stories</span>
              <div style={styles.managementActions}>
                <button
                  style={styles.btn}
                  onClick={openCreateStoryDialog}
                  disabled={!selectedManagementFeatureNode}
                >
                  + Story
                </button>
                <button style={styles.ghostBtn} onClick={() => openFeatureInBuilder(selectedManagementFeatureNode)}>Open Builder</button>
              </div>
            </div>
            <div style={styles.managementList}>
              {featureStories.length > 0 ? featureStories.map((story) => (
                <div key={story.id} style={selectedManagementStory?.id === story.id ? styles.managementListButtonActive : styles.managementListButton}>
                  <button
                    style={styles.managementItemSelect}
                    onClick={() => {
                      setSelectedManagementStoryId(story.id);
                      setActiveWorkItem(story.id);
                    }}
                  >
                    <div style={styles.rowPrimary}>{story.title}</div>
                    <div style={styles.rowSecondary}>{formatWorkItemMeta(story.status)} · {story.priority} · {story.complexity}</div>
                  </button>
                  <div style={styles.inlineActionRow}>
                    <button style={styles.outlineActionBtn} onClick={() => openEditStoryDialog(story)}>Edit</button>
                    <button style={styles.compactDangerBtn} onClick={() => requestDeleteWorkItem(story, "story")}>Delete</button>
                  </div>
                </div>
              )) : (
                <div style={styles.empty}>No stories for this feature yet.</div>
              )}
            </div>
          </div>
          <div style={styles.managementPane}>
            {selectedManagementStory ? (
              <>
                <div style={styles.sectionTitle}>
                  <span>Story Details</span>
                  <div style={styles.managementActions}>
                    <button style={styles.ghostBtn} onClick={() => openEditStoryDialog(selectedManagementStory)}>Edit</button>
                    <button style={styles.ghostBtn} onClick={() => openStoryInBuilder(selectedManagementStory)}>Open Story</button>
                  </div>
                </div>
                <div style={styles.contextCard}>
                  <div style={styles.contextLabel}>{formatWorkItemMeta(selectedManagementStory.status)} · {formatWorkItemMeta(selectedManagementStory.work_item_type)} · {selectedManagementStory.priority} priority · {formatWorkItemMeta(selectedManagementStory.complexity)} complexity</div>
                  <div style={styles.contextTitle}>{selectedManagementStory.title}</div>
                  <div style={styles.workItemDetailGrid}>
                    <div>
                      <div style={styles.contextLabel}>Problem</div>
                      <div style={styles.contextText}>{selectedManagementStory.problem_statement || "No problem statement captured yet."}</div>
                    </div>
                    <div>
                      <div style={styles.contextLabel}>Description</div>
                      <div style={styles.contextText}>{selectedManagementStory.description || "No story description yet."}</div>
                    </div>
                    <div>
                      <div style={styles.contextLabel}>Acceptance Criteria</div>
                      <div style={styles.contextText}>{selectedManagementStory.acceptance_criteria || "No acceptance criteria captured yet."}</div>
                    </div>
                    <div>
                      <div style={styles.contextLabel}>Constraints</div>
                      <div style={styles.contextText}>{selectedManagementStory.constraints || "No constraints captured yet."}</div>
                    </div>
                  </div>
                </div>
                <div style={styles.sectionTitle}>
                  <span>Tasks</span>
                  <div style={styles.managementActions}>
                    <span style={styles.badgeMuted}>{selectedManagementTasks.length}</span>
                    <button
                      style={styles.btn}
                      onClick={openCreateTaskDialog}
                    >
                      + Task
                    </button>
                  </div>
                </div>
                {selectedManagementTasks.length > 0 ? selectedManagementTasks.map((task) => (
                  <div key={task.id} style={styles.contextCard}>
                    <div style={styles.moduleHeader}>
                      <div>
                        <div style={styles.contextTitle}>{task.title}</div>
                        <div style={styles.contextText}>{formatWorkItemMeta(task.status)} · {task.priority} · {formatWorkItemMeta(task.complexity)}</div>
                        {task.description && <div style={styles.contextText}>{task.description}</div>}
                      </div>
                      <div style={styles.managementActions}>
                        <button style={styles.compactActionBtn} onClick={() => openEditTaskDialog(task)}>Edit</button>
                        <button style={styles.compactDangerBtn} onClick={() => requestDeleteWorkItem(task, "task")}>Delete</button>
                      </div>
                    </div>
                  </div>
                )) : (
                  <div style={styles.empty}>No tasks under this story yet.</div>
                )}
              </>
            ) : (
              <div style={styles.empty}>Select a story to inspect details and tasks.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Products</h1>
          <div style={styles.subtitle}>Manage products, shape product areas, capabilities, and features, then track delivery stories and tasks separately.</div>
        </div>
      </div>

      <div style={styles.pageTabs}>
        <div style={styles.pageTabGroup}>
          <span style={styles.pageTabGroupLabel}>Catalog</span>
          <button style={productPageTab === "list" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("list")}>Product List</button>
          <button style={productPageTab === "status" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("status")}>Product Status</button>
        </div>
        <div style={styles.pageTabGroup}>
          <span style={styles.pageTabGroupLabel}>Selected Product</span>
          <select
            aria-label="Selected product"
            style={styles.pageTabProductSelect}
            value={selectedProductId ?? ""}
            onChange={(event) => {
              const nextProductId = event.target.value || null;
              setActiveProduct(nextProductId);
              if (nextProductId && (productPageTab === "list" || productPageTab === "status")) {
                setProductPageTab("overview");
              }
            }}
          >
            <option value="">Select product</option>
            {(products ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
          </select>
          <button style={productPageTab === "overview" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("overview")} disabled={!selectedProduct}>Product Overview</button>
          <button style={productPageTab === "design" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("design")} disabled={!selectedProduct}>Product Management</button>
          <button style={productPageTab === "dependencies" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("dependencies")} disabled={!selectedProduct}>Dependencies</button>
        </div>
      </div>

      <div style={styles.workspace}>
        <div style={styles.panel}>
          <div style={styles.panelInner}>
            {productPageTab === "list" ? (
              <>
                <div style={styles.toolbar}>
                  <div>
                    <div style={styles.controlLabel}>Search</div>
                    <input
                      style={styles.input}
                      value={productSearch}
                      onChange={(event) => setProductSearch(event.target.value)}
                      placeholder="Filter by name, tag, status, or description"
                    />
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Status</div>
                    <select style={styles.select} value={productStatusFilter} onChange={(event) => setProductStatusFilter(event.target.value as typeof productStatusFilter)}>
                      <option value="all">All statuses</option>
                      <option value="active">Active</option>
                      <option value="archived">Archived</option>
                    </select>
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Source</div>
                    <select style={styles.select} value={productSourceFilter} onChange={(event) => setProductSourceFilter(event.target.value as typeof productSourceFilter)}>
                      <option value="all">All sources</option>
                      <option value="custom">Custom</option>
                      <option value="default">Default</option>
                    </select>
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Tag</div>
                    <select style={styles.select} value={productTagFilter} onChange={(event) => setProductTagFilter(event.target.value)}>
                      <option value="all">All tags</option>
                      {allProductTags.map((tag) => <option key={tag} value={tag}>{tag}</option>)}
                    </select>
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Sort</div>
                    <select style={styles.select} value={productSort} onChange={(event) => setProductSort(event.target.value as typeof productSort)}>
                      <option value="name">Name</option>
                      <option value="updated">Recently updated</option>
                      <option value="progress">Progress</option>
                      <option value="work">Work items</option>
                    </select>
                  </div>
                  <button style={{ ...styles.btn, alignSelf: "center", justifySelf: "end" }} onClick={() => openProductDialog("create")}>+ Add Product</button>
                </div>
                <div style={styles.toggleRow}>
                  <label style={styles.checkboxLabel}>
                    <input type="checkbox" checked={showCustomProductsInTable} onChange={(event) => setShowCustomProductsInTable(event.target.checked)} />
                    Show custom products
                  </label>
                  <label style={styles.checkboxLabel}>
                    <input type="checkbox" checked={showDefaultProductsInTable} onChange={(event) => setShowDefaultProductsInTable(event.target.checked)} />
                    Show default products in table
                  </label>
                  <label style={styles.checkboxLabel}>
                    <input
                      type="checkbox"
                      checked={includeDefaultProductsInCatalog}
                      onChange={(event) => updateDefaultProductVisibility(event.target.checked)}
                    />
                    Include default products from catalog
                  </label>
                  {catalogFilterMsg && <span style={{ ...styles.contextText, color: "#4ec9b0" }}>{catalogFilterMsg}</span>}
                  {catalogFilterError && <span style={styles.errorText}>{catalogFilterError}</span>}
                </div>
                <div style={styles.table}>
                  <div style={styles.productTableHeader}>
                    <div>Product</div>
                    <div>Source</div>
                    <div>Status</div>
                    <div>Management</div>
                    <div>Progress</div>
                    <div>Actions</div>
                  </div>
                  {productTableRows.length > 0 ? productTableRows.map((row) => (
                    <div key={row.product.id} style={styles.productTableRow}>
                      <div style={styles.productCell}>
                        <div style={styles.rowPrimary}>{row.product.name}</div>
                        <div style={styles.productDescription}>{row.product.description || row.product.vision || "No description yet."}</div>
                        <div style={styles.chipRow}>
                          {row.product.tags.slice(0, 4).map((tag) => <span key={tag} style={styles.badgeMuted}>{tag}</span>)}
                        </div>
                      </div>
                      <div style={styles.productMetaCell}>{row.source}</div>
                      <div style={styles.productMetaCell}>
                        <div>{row.product.lifecycle}</div>
                        <div style={styles.rowSecondary}>{row.product.health}</div>
                      </div>
                      <div style={styles.productMetaCell}>
                        <div>{row.rootCount} product areas</div>
                        <div style={styles.rowSecondary}>{row.nodeCount} management nodes</div>
                      </div>
                      <div>
                        <div style={styles.rowCell}>{row.progress.percent}%</div>
                        <div style={styles.progressTrack}><div style={{ ...styles.progressFill, width: `${row.progress.percent}%` }} /></div>
                        <div style={styles.rowSecondary}>{row.progress.done}/{row.progress.total} done</div>
                        <div style={styles.rowSecondary}>{row.activeWorkItemCount} active stories</div>
                      </div>
                      <div style={styles.productActions}>
                        <button style={styles.compactActionBtn} onClick={() => editProductFromList(row.product)}>Edit</button>
                        <button style={styles.compactActionBtn} onClick={() => openProductStatus(row.product)}>Status</button>
                        <button style={styles.compactActionBtn} onClick={() => openProductOverview(row.product)}>Overview</button>
                        <button style={styles.compactActionBtn} onClick={() => openProductDesign(row.product)}>Manage</button>
                        <button style={styles.compactActionBtn} onClick={() => openProductDependencies(row.product)}>Dependencies</button>
                        <button style={styles.compactDangerBtn} onClick={() => requestArchiveProduct(row.product)}>Delete</button>
                      </div>
                    </div>
                  )) : (
                    <div style={styles.empty}>{isLoading ? "Loading products..." : "No products match the current filters."}</div>
                  )}
                </div>
              </>
            ) : productPageTab === "status" ? (
              <>
                <div style={styles.statusToolbar}>
                  <div>
                    <div style={styles.controlLabel}>Product</div>
                    <select
                      style={styles.select}
                      value={statusProductId}
                      onChange={(event) => {
                        const nextProductId = event.target.value;
                        setStatusProductId(nextProductId);
                        if (nextProductId !== "all") {
                          setActiveProduct(nextProductId);
                        }
                      }}
                    >
                      <option value="all">All visible products</option>
                      {(products ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
                    </select>
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Visible levels</div>
                    <select style={styles.select} value={statusDepth} onChange={(event) => setStatusDepth(Number(event.target.value))}>
                      {[1, 2, 3, 4, 5, 6].map((depth) => <option key={depth} value={depth}>{depth} {depth === 1 ? "level" : "levels"}</option>)}
                    </select>
                  </div>
                  <div>
                    <div style={styles.controlLabel}>Pivot</div>
                    <select style={styles.select} value={statusGroupBy} onChange={(event) => setStatusGroupBy(event.target.value as typeof statusGroupBy)}>
                      <option value="node">Tree nodes</option>
                      <option value="kind">Node kind</option>
                      <option value="work_status">Work status</option>
                    </select>
                  </div>
                  <div style={styles.statusMetrics}>
                    <div style={styles.statusMetric}>
                      <div style={styles.metricLabel}>Products</div>
                      <div style={styles.statusMetricValue}>{statusSummary.productCount}</div>
                      <div style={styles.statusMetricHelp}>included</div>
                    </div>
                    <div style={styles.statusMetric}>
                      <div style={styles.metricLabel}>Nodes</div>
                      <div style={styles.statusMetricValue}>{statusSummary.nodeCount}</div>
                      <div style={styles.statusMetricHelp}>{statusSummary.leafCount} leaf</div>
                    </div>
                    <div style={styles.statusMetric}>
                      <div style={styles.metricLabel}>Stories</div>
                      <div style={styles.statusMetricValue}>{statusSummary.workItemCount}</div>
                      <div style={styles.statusMetricHelp}>{statusSummary.activeWorkItemCount} active · {statusSummary.doneWorkItemCount} done stories</div>
                    </div>
                    <div style={styles.statusMetric}>
                      <div style={styles.metricLabel}>Progress</div>
                      <div style={styles.statusMetricValue}>{statusSummary.progress.percent}%</div>
                      <div style={styles.statusMetricHelp}>{statusSummary.progress.done}/{statusSummary.progress.total}</div>
                    </div>
                  </div>
                </div>
                <div style={styles.table}>
                  <div style={styles.statusTableHeader}>
                    <div>{statusGroupBy === "node" ? "Scope" : "Group"}</div>
                    <div>Level</div>
                    <div>Kind</div>
                    <div>Nodes</div>
                    <div>Work</div>
                    <div>Progress</div>
                  </div>
                  {statusRows.length > 0 ? statusRows.map((row) => (
                    <div
                      key={row.id}
                      style={styles.statusTableRow}
                      onClick={() => {
                        if (row.productId) {
                          setActiveProduct(row.productId);
                        }
                        if (row.nodeId && row.nodeType) {
                          setActiveHierarchyNode({
                            nodeId: row.nodeId,
                            nodeType: row.nodeType,
                            moduleId: row.moduleId ?? null,
                            capabilityId: row.capabilityId ?? null,
                          });
                          setProductPageTab("design");
                        }
                      }}
                    >
                      <div style={{ paddingLeft: statusGroupBy === "node" ? Math.max(0, row.level - 1) * 16 : 0 }}>
                        <div style={{ ...styles.rowPrimary, fontSize: 12 }}>{row.name}</div>
                        <div style={{ ...styles.rowSecondary, fontSize: 10, marginTop: 2 }}>{row.subtitle}</div>
                      </div>
                      <div style={styles.rowCell}>{row.level}</div>
                      <div style={styles.rowCell}>{row.kind}</div>
                      <div style={styles.rowCell}>{row.nodeCount} · {row.childCount} child</div>
                      <div style={styles.rowCell}>{row.workItemCount} · {row.activeWorkItemCount} active stories</div>
                      <div>
                        <div style={{ display: "grid", gridTemplateColumns: "38px minmax(0, 1fr) 46px", gap: 6, alignItems: "center" }}>
                          <span style={styles.rowCell}>{row.progress.percent}%</span>
                          <div style={styles.progressTrack}><div style={{ ...styles.progressFill, width: `${row.progress.percent}%` }} /></div>
                          <span style={{ ...styles.rowSecondary, marginTop: 0 }}>{row.progress.done}/{row.progress.total}</span>
                        </div>
                      </div>
                    </div>
                  )) : (
                    <div style={styles.empty}>{isLoading ? "Loading status..." : "No status rows are available for this selection."}</div>
                  )}
                </div>
              </>
            ) : productPageTab === "overview" ? (
              selectedProduct ? (
                <ProductOverviewPage />
              ) : (
                <div style={styles.empty}>Select a product to view the product overview.</div>
              )
            ) : productPageTab === "dependencies" ? (
              selectedProduct ? (
                <>
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>Dependencies</div>
                    <div style={styles.contextCard}>
                      <div style={styles.contextLabel}>Product Owner Lens</div>
                      <div style={styles.contextTitle}>{selectedProduct.name}</div>
                      <div style={styles.contextText}>
                        Capture cross-product dependencies here. Use the optional capability fields when one product capability depends on a specific platform capability.
                      </div>
                    </div>
                    <div style={styles.formRow}>
                      <div>
                        <label style={styles.label}>Source Capability</label>
                        <select
                          style={styles.select}
                          value={dependencyDraft.capabilityId}
                          onChange={(event) => setDependencyDraft((draft) => ({ ...draft, capabilityId: event.target.value }))}
                        >
                          <option value="">Whole product</option>
                          {selectedCapabilityOptions.map((option) => (
                            <option key={option.id} value={option.id}>{option.label}</option>
                          ))}
                        </select>
                      </div>
                      <div>
                        <label style={styles.label}>Depends On Product</label>
                        <select
                          style={styles.select}
                          value={dependencyDraft.dependsOnProductId}
                          onChange={(event) => setDependencyDraft((draft) => ({ ...draft, dependsOnProductId: event.target.value, dependsOnCapabilityId: "" }))}
                        >
                          <option value="">Select product</option>
                          {(products ?? []).filter((product) => product.id !== selectedProduct.id).map((product) => (
                            <option key={product.id} value={product.id}>{product.name}</option>
                          ))}
                        </select>
                      </div>
                    </div>
                    <div style={styles.formRow}>
                      <div>
                        <label style={styles.label}>Depends On Capability</label>
                        <select
                          style={styles.select}
                          value={dependencyDraft.dependsOnCapabilityId}
                          onChange={(event) => setDependencyDraft((draft) => ({ ...draft, dependsOnCapabilityId: event.target.value }))}
                          disabled={!dependencyDraft.dependsOnProductId}
                        >
                          <option value="">Whole product</option>
                          {dependencyTargetCapabilityOptions.map((option) => (
                            <option key={option.id} value={option.id}>{option.label}</option>
                          ))}
                        </select>
                      </div>
                      <div>
                        <label style={styles.label}>Kind</label>
                        <select
                          style={styles.select}
                          value={dependencyDraft.dependencyKind}
                          onChange={(event) => setDependencyDraft((draft) => ({ ...draft, dependencyKind: event.target.value as ProductDependencyKind }))}
                        >
                          {(["platform", "capability", "data", "integration", "operational", "other"] as ProductDependencyKind[]).map((kind) => (
                            <option key={kind} value={kind}>{kind}</option>
                          ))}
                        </select>
                      </div>
                    </div>
                    <label style={styles.label}>Description</label>
                    <textarea
                      style={styles.textarea}
                      value={dependencyDraft.description}
                      onChange={(event) => setDependencyDraft((draft) => ({ ...draft, description: event.target.value }))}
                    />
                    <button
                      style={styles.btn}
                      onClick={() => createProductDependencyMutation.mutate()}
                      disabled={!selectedProductId || !dependencyDraft.dependsOnProductId || createProductDependencyMutation.isPending}
                    >
                      {createProductDependencyMutation.isPending ? "Adding..." : "Add Dependency"}
                    </button>
                  </div>
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>Captured Dependencies</div>
                    {selectedProductDependencies.length > 0 ? (
                      selectedProductDependencies.map((dependency: ProductDependency) => (
                        <div key={dependency.id} style={styles.contextCard}>
                          <div style={styles.contextLabel}>{dependency.dependency_kind} · {dependency.status}</div>
                          <div style={styles.contextTitle}>
                            {dependency.capability_id ? capabilityLabelById.get(dependency.capability_id) ?? "Selected capability" : selectedProduct.name}
                          </div>
                          <div style={styles.contextText}>
                            depends on {productNameById.get(dependency.depends_on_product_id) ?? "Unknown product"}
                            {dependency.depends_on_capability_id ? ` / ${capabilityLabelById.get(dependency.depends_on_capability_id) ?? "selected capability"}` : ""}
                          </div>
                          {dependency.description ? <div style={{ ...styles.contextText, marginTop: 8 }}>{dependency.description}</div> : null}
                        </div>
                      ))
                    ) : (
                      <div style={styles.empty}>No dependencies captured for this product yet.</div>
                    )}
                  </div>
                </>
              ) : (
                <div style={styles.empty}>Select a product before editing dependencies.</div>
              )
            ) : selectedProduct ? (
              renderProductManagementConsole()
            ) : (
              <div style={styles.empty}>
                {isLoading
                  ? "Loading products..."
                  : products && products.length > 0
                    ? "Select a product from Product List to start refining the management tree."
                    : "No visible products yet. Use Add Product or disable Hide Example Products in Settings."}
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
          <div style={styles.formRow}>
            <div>
              <label style={styles.label}>Lifecycle</label>
              <select
                style={styles.select}
                value={productDialogMode === "create" ? productForm.lifecycle : productDraft.lifecycle}
                onChange={(e) => (productDialogMode === "create"
                  ? setProductForm({ ...productForm, lifecycle: e.target.value as Product["lifecycle"] })
                  : setProductDraft({ ...productDraft, lifecycle: e.target.value as Product["lifecycle"] }))}
              >
                {productLifecycleOptions.map((option) => <option key={option} value={option}>{option}</option>)}
              </select>
            </div>
            <div>
              <label style={styles.label}>Health</label>
              <select
                style={styles.select}
                value={productDialogMode === "create" ? productForm.health : productDraft.health}
                onChange={(e) => (productDialogMode === "create"
                  ? setProductForm({ ...productForm, health: e.target.value as Product["health"] })
                  : setProductDraft({ ...productDraft, health: e.target.value as Product["health"] }))}
              >
                {productHealthOptions.map((option) => <option key={option} value={option}>{option}</option>)}
              </select>
            </div>
          </div>
          <div style={styles.formRow}>
            <div>
              <label style={styles.label}>Owner / Hat</label>
              <input
                style={styles.input}
                value={productDialogMode === "create" ? productForm.ownerLabel : productDraft.ownerLabel}
                onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, ownerLabel: e.target.value }) : setProductDraft({ ...productDraft, ownerLabel: e.target.value }))}
              />
            </div>
            <div>
              <label style={styles.label}>Investment</label>
              <select
                style={styles.select}
                value={productDialogMode === "create" ? productForm.investmentStatus : productDraft.investmentStatus}
                onChange={(e) => (productDialogMode === "create"
                  ? setProductForm({ ...productForm, investmentStatus: e.target.value as Product["investment_status"] })
                  : setProductDraft({ ...productDraft, investmentStatus: e.target.value as Product["investment_status"] }))}
              >
                {productInvestmentOptions.map((option) => <option key={option} value={option}>{option}</option>)}
              </select>
            </div>
          </div>
          <label style={styles.label}>Roadmap</label>
          <textarea
            style={styles.textarea}
            value={productDialogMode === "create" ? productForm.roadmap : productDraft.roadmap}
            onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, roadmap: e.target.value }) : setProductDraft({ ...productDraft, roadmap: e.target.value }))}
          />
          <label style={styles.label}>Evidence</label>
          <textarea
            style={styles.textarea}
            value={productDialogMode === "create" ? productForm.evidence : productDraft.evidence}
            onChange={(e) => (productDialogMode === "create" ? setProductForm({ ...productForm, evidence: e.target.value }) : setProductDraft({ ...productDraft, evidence: e.target.value }))}
          />
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

      {deleteProductCandidate && (
        <ModalShell title={`Delete Product: ${deleteProductCandidate.name}`} onClose={() => setDeleteProductCandidate(null)}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Double Confirmation</div>
            <div style={styles.contextTitle}>This will archive the product and remove it from active product workflows.</div>
            <div style={styles.contextText}>
              The current backend exposes archive as the supported product removal operation. Type the product name and confirm the archive action to continue.
            </div>
          </div>
          <label style={styles.label}>Type product name</label>
          <input
            style={styles.input}
            value={deleteConfirmName}
            onChange={(event) => setDeleteConfirmName(event.target.value)}
            placeholder={deleteProductCandidate.name}
          />
          <label style={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={deleteConfirmArchive}
              onChange={(event) => setDeleteConfirmArchive(event.target.checked)}
            />
            I understand this product will be archived.
          </label>
          {formError && <div style={{ ...styles.errorText, marginTop: 10 }}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 16 }}>
            <button style={styles.ghostBtn} onClick={() => setDeleteProductCandidate(null)}>Cancel</button>
            <button
              style={styles.btnDanger}
              onClick={() => archiveMutation.mutate(deleteProductCandidate.id)}
              disabled={!deleteConfirmationReady || archiveMutation.isPending}
            >
              {archiveMutation.isPending ? "Archiving..." : "Delete Product"}
            </button>
          </div>
        </ModalShell>
      )}

      {resetPlanCandidate && (
        <ModalShell title={`Reset Product Plan: ${resetPlanCandidate.name}`} onClose={() => setResetPlanCandidate(null)}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Double Confirm</div>
            <div style={styles.contextTitle}>This removes the current product management tree.</div>
            <div style={styles.contextText}>
              Product areas, capabilities, and features will be deleted so this product can be replanned from a clean management tree.
              Delivery stories and tasks are preserved unless you explicitly include them below.
            </div>
          </div>
          <label style={styles.label}>Type the product name to confirm</label>
          <input
            style={styles.input}
            value={resetPlanConfirmName}
            onChange={(event) => setResetPlanConfirmName(event.target.value)}
            placeholder={resetPlanCandidate.name}
          />
          <label style={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={resetPlanConfirmTree}
              onChange={(event) => setResetPlanConfirmTree(event.target.checked)}
            />
            I understand the product areas, capabilities, and features will be deleted.
          </label>
          <label style={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={resetPlanDeleteDelivery}
              onChange={(event) => setResetPlanDeleteDelivery(event.target.checked)}
            />
            Also delete existing delivery stories and tasks for this product.
          </label>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setResetPlanCandidate(null)}>Cancel</button>
            <button
              style={styles.btnDanger}
              onClick={() => resetProductPlanMutation.mutate({ productId: resetPlanCandidate.id, deleteDelivery: resetPlanDeleteDelivery })}
              disabled={!resetPlanReady || resetProductPlanMutation.isPending}
            >
              {resetProductPlanMutation.isPending ? "Resetting..." : "Reset Plan"}
            </button>
          </div>
        </ModalShell>
      )}

      {deleteHierarchyCandidate && (
        <ModalShell title={`Delete ${getHierarchyDeleteLabel(deleteHierarchyCandidate.kind)}: ${deleteHierarchyCandidate.name}`} onClose={() => setDeleteHierarchyCandidate(null)}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Double Confirm</div>
            <div style={styles.contextTitle}>This deletes the selected {getHierarchyDeleteLabel(deleteHierarchyCandidate.kind).toLowerCase()}.</div>
            <div style={styles.contextText}>
              Child hierarchy under this node will also be removed. Related delivery stories may be detached by the database if they reference this scope.
            </div>
          </div>
          <label style={styles.label}>Type the name to confirm</label>
          <input
            style={styles.input}
            value={deleteHierarchyConfirmName}
            onChange={(event) => setDeleteHierarchyConfirmName(event.target.value)}
            placeholder={deleteHierarchyCandidate.name}
          />
          <label style={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={deleteHierarchyConfirmChecked}
              onChange={(event) => setDeleteHierarchyConfirmChecked(event.target.checked)}
            />
            I understand this hierarchy node and its child hierarchy will be deleted.
          </label>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setDeleteHierarchyCandidate(null)}>Cancel</button>
            <button
              style={styles.btnDanger}
              onClick={() => deleteHierarchyMutation.mutate(deleteHierarchyCandidate)}
              disabled={!deleteHierarchyReady || deleteHierarchyMutation.isPending}
            >
              {deleteHierarchyMutation.isPending ? "Deleting..." : `Delete ${getHierarchyDeleteLabel(deleteHierarchyCandidate.kind)}`}
            </button>
          </div>
        </ModalShell>
      )}

      {deleteWorkItemCandidate && (
        <ModalShell title={`Delete ${deleteWorkItemCandidate.kind}: ${deleteWorkItemCandidate.workItem.title}`} onClose={() => setDeleteWorkItemCandidate(null)}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Double Confirm</div>
            <div style={styles.contextTitle}>This deletes the selected {deleteWorkItemCandidate.kind}.</div>
            <div style={styles.contextText}>
              {deleteWorkItemCandidate.kind === "story"
                ? "Tasks under this story will also be deleted."
                : "This task will be removed from the selected story."}
            </div>
          </div>
          <label style={styles.label} htmlFor="delete-work-item-confirm-title">Type the title to confirm</label>
          <input
            id="delete-work-item-confirm-title"
            style={styles.input}
            value={deleteWorkItemConfirmName}
            onChange={(event) => setDeleteWorkItemConfirmName(event.target.value)}
            placeholder={deleteWorkItemCandidate.workItem.title}
          />
          <label style={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={deleteWorkItemConfirmChecked}
              onChange={(event) => setDeleteWorkItemConfirmChecked(event.target.checked)}
            />
            I understand this story/task will be deleted.
          </label>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setDeleteWorkItemCandidate(null)}>Cancel</button>
            <button
              style={styles.btnDanger}
              onClick={() => deleteManagementWorkItemMutation.mutate(deleteWorkItemCandidate)}
              disabled={!deleteManagementWorkItemReady || deleteManagementWorkItemMutation.isPending}
            >
              {deleteManagementWorkItemMutation.isPending ? "Deleting..." : `Delete ${deleteWorkItemCandidate.kind}`}
            </button>
          </div>
        </ModalShell>
      )}

      {storyDialogMode !== "closed" && (
        <ModalShell title={storyDialogMode === "edit" ? "Edit Story" : "Add Story"} onClose={() => setStoryDialogMode("closed")}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Feature</div>
            <div style={styles.contextTitle}>{selectedManagementFeature?.capabilityTree.capability.name ?? "No feature selected"}</div>
          </div>
          <label style={styles.label} htmlFor="management-story-title">Story title</label>
          <input
            id="management-story-title"
            style={styles.input}
            value={storyDraft.title}
            onChange={(event) => setStoryDraft((draft) => ({ ...draft, title: event.target.value }))}
          />
          <div style={styles.formRow}>
            <div>
              <label style={styles.label} htmlFor="management-story-status">Status</label>
              <select
                id="management-story-status"
                style={styles.input}
                value={storyDraft.status}
                onChange={(event) => setStoryDraft((draft) => ({ ...draft, status: event.target.value as WorkItem["status"] }))}
              >
                {workItemStatusOptions.map((status) => <option key={status} value={status}>{formatWorkItemMeta(status)}</option>)}
              </select>
            </div>
            <div>
              <label style={styles.label} htmlFor="management-story-priority">Priority</label>
              <select
                id="management-story-priority"
                style={styles.input}
                value={storyDraft.priority}
                onChange={(event) => setStoryDraft((draft) => ({ ...draft, priority: event.target.value as WorkItem["priority"] }))}
                disabled={storyDialogMode === "edit"}
              >
                {workItemPriorityOptions.map((priority) => <option key={priority} value={priority}>{priority}</option>)}
              </select>
            </div>
          </div>
          <label style={styles.label} htmlFor="management-story-problem">Problem Statement</label>
          <textarea
            id="management-story-problem"
            style={styles.textarea}
            value={storyDraft.problemStatement}
            onChange={(event) => setStoryDraft((draft) => ({ ...draft, problemStatement: event.target.value }))}
          />
          <label style={styles.label} htmlFor="management-story-description">Description</label>
          <textarea
            id="management-story-description"
            style={styles.textarea}
            value={storyDraft.description}
            onChange={(event) => setStoryDraft((draft) => ({ ...draft, description: event.target.value }))}
          />
          <label style={styles.label} htmlFor="management-story-acceptance-criteria">Acceptance Criteria</label>
          <textarea
            id="management-story-acceptance-criteria"
            style={styles.textarea}
            value={storyDraft.acceptanceCriteria}
            onChange={(event) => setStoryDraft((draft) => ({ ...draft, acceptanceCriteria: event.target.value }))}
          />
          <div style={styles.formRow}>
            <div>
              <label style={styles.label} htmlFor="management-story-constraints">Constraints</label>
              <textarea
                id="management-story-constraints"
                style={styles.textarea}
                value={storyDraft.constraints}
                onChange={(event) => setStoryDraft((draft) => ({ ...draft, constraints: event.target.value }))}
              />
            </div>
            <div>
              <label style={styles.label} htmlFor="management-story-complexity">Complexity</label>
              <select
                id="management-story-complexity"
                style={styles.input}
                value={storyDraft.complexity}
                onChange={(event) => setStoryDraft((draft) => ({ ...draft, complexity: event.target.value as WorkItem["complexity"] }))}
                disabled={storyDialogMode === "edit"}
              >
                {workItemComplexityOptions.map((complexity) => <option key={complexity} value={complexity}>{formatWorkItemMeta(complexity)}</option>)}
              </select>
              {storyDialogMode === "edit" && <div style={styles.contextText}>Priority and complexity are currently set when the story is created.</div>}
            </div>
          </div>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setStoryDialogMode("closed")}>Cancel</button>
            <button
              style={styles.btn}
              onClick={() => storyDialogMode === "edit" ? updateManagementStoryMutation.mutate() : createManagementStoryMutation.mutate()}
              disabled={!selectedManagementFeatureNode || !storyDraft.title.trim() || createManagementStoryMutation.isPending || updateManagementStoryMutation.isPending}
            >
              {createManagementStoryMutation.isPending || updateManagementStoryMutation.isPending
                ? "Saving..."
                : storyDialogMode === "edit" ? "Save Story" : "Add Story"}
            </button>
          </div>
        </ModalShell>
      )}

      {taskDialogMode !== "closed" && (
        <ModalShell title={taskDialogMode === "edit" ? "Edit Task" : "Add Task"} onClose={() => setTaskDialogMode("closed")}>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Story</div>
            <div style={styles.contextTitle}>{selectedManagementStory?.title ?? "No story selected"}</div>
          </div>
          <label style={styles.label} htmlFor="management-task-title">Task title</label>
          <input
            id="management-task-title"
            style={styles.input}
            value={taskDraft.title}
            onChange={(event) => setTaskDraft((draft) => ({ ...draft, title: event.target.value }))}
          />
          <div style={styles.formRow}>
            <div>
              <label style={styles.label} htmlFor="management-task-status">Status</label>
              <select
                id="management-task-status"
                style={styles.input}
                value={taskDraft.status}
                onChange={(event) => setTaskDraft((draft) => ({ ...draft, status: event.target.value as WorkItem["status"] }))}
              >
                {workItemStatusOptions.map((status) => <option key={status} value={status}>{formatWorkItemMeta(status)}</option>)}
              </select>
            </div>
            <div>
              <label style={styles.label} htmlFor="management-task-priority">Priority</label>
              <select
                id="management-task-priority"
                style={styles.input}
                value={taskDraft.priority}
                onChange={(event) => setTaskDraft((draft) => ({ ...draft, priority: event.target.value as WorkItem["priority"] }))}
                disabled={taskDialogMode === "edit"}
              >
                {workItemPriorityOptions.map((priority) => <option key={priority} value={priority}>{priority}</option>)}
              </select>
            </div>
          </div>
          <label style={styles.label} htmlFor="management-task-problem">Problem Statement</label>
          <textarea
            id="management-task-problem"
            style={styles.textarea}
            value={taskDraft.problemStatement}
            onChange={(event) => setTaskDraft((draft) => ({ ...draft, problemStatement: event.target.value }))}
          />
          <label style={styles.label} htmlFor="management-task-description">Description</label>
          <textarea
            id="management-task-description"
            style={styles.textarea}
            value={taskDraft.description}
            onChange={(event) => setTaskDraft((draft) => ({ ...draft, description: event.target.value }))}
          />
          <label style={styles.label} htmlFor="management-task-acceptance-criteria">Acceptance Criteria</label>
          <textarea
            id="management-task-acceptance-criteria"
            style={styles.textarea}
            value={taskDraft.acceptanceCriteria}
            onChange={(event) => setTaskDraft((draft) => ({ ...draft, acceptanceCriteria: event.target.value }))}
          />
          <div style={styles.formRow}>
            <div>
              <label style={styles.label} htmlFor="management-task-constraints">Constraints</label>
              <textarea
                id="management-task-constraints"
                style={styles.textarea}
                value={taskDraft.constraints}
                onChange={(event) => setTaskDraft((draft) => ({ ...draft, constraints: event.target.value }))}
              />
            </div>
            <div>
              <label style={styles.label} htmlFor="management-task-complexity">Complexity</label>
              <select
                id="management-task-complexity"
                style={styles.input}
                value={taskDraft.complexity}
                onChange={(event) => setTaskDraft((draft) => ({ ...draft, complexity: event.target.value as WorkItem["complexity"] }))}
                disabled={taskDialogMode === "edit"}
              >
                {workItemComplexityOptions.map((complexity) => <option key={complexity} value={complexity}>{formatWorkItemMeta(complexity)}</option>)}
              </select>
              {taskDialogMode === "edit" && <div style={styles.contextText}>Priority and complexity are currently set when the task is created.</div>}
            </div>
          </div>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setTaskDialogMode("closed")}>Cancel</button>
            <button
              style={styles.btn}
              onClick={() => taskDialogMode === "edit" ? updateManagementTaskMutation.mutate() : createManagementTaskMutation.mutate()}
              disabled={!selectedManagementStory || !taskDraft.title.trim() || createManagementTaskMutation.isPending || updateManagementTaskMutation.isPending}
            >
              {createManagementTaskMutation.isPending || updateManagementTaskMutation.isPending
                ? "Saving..."
                : taskDialogMode === "edit" ? "Save Task" : "Add Task"}
            </button>
          </div>
        </ModalShell>
      )}

      {moduleDialogMode !== "closed" && (
        <ModalShell
          title={moduleDialogMode === "create"
            ? `Create ${getHierarchyNodeKindLabel(moduleForm.nodeKind)}`
            : `Edit ${selectedModule ? getHierarchyNodeKindLabel(selectedModule.node_kind) : "Product Area"}: ${selectedModule?.name ?? ""}`}
          onClose={closeModuleDialog}
        >
          {moduleDialogMode === "create" ? (
            <>
              <label style={styles.label}>Product Area Kind</label>
              <select style={styles.input} value={moduleForm.nodeKind} onChange={(e) => setModuleForm({ ...moduleForm, nodeKind: e.target.value as HierarchyNodeKind })}>
                {(["area"] as HierarchyNodeKind[]).map((nodeKind) => (
                  <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                ))}
              </select>
              <div style={styles.contextText}>{getHierarchyNodeKindGuidance(moduleForm.nodeKind)}</div>
              <label style={styles.label}>{getHierarchyNodeKindLabel(moduleForm.nodeKind)} Name</label>
              <input style={styles.input} value={moduleForm.name} onChange={(e) => setModuleForm({ ...moduleForm, name: e.target.value })} />
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={moduleForm.description} onChange={(e) => setModuleForm({ ...moduleForm, description: e.target.value })} />
              <label style={styles.label}>Purpose</label>
              <input style={styles.input} value={moduleForm.purpose} onChange={(e) => setModuleForm({ ...moduleForm, purpose: e.target.value })} />
            </>
          ) : (
            <>
              <div style={styles.contextCard}>
                <div style={styles.contextLabel}>Product Area</div>
                <div style={styles.contextText}>{getHierarchyNodeKindGuidance("area")}</div>
              </div>
              <label style={styles.label}>Product Area Name</label>
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
                : updateModuleMutation.isPending ? "Saving..." : "Save Product Area"}
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
              <label style={styles.label}>Parent Product Area</label>
              <input style={styles.input} value={selectedModule?.name ?? ""} readOnly />
              <label style={styles.label}>Parent Node</label>
              <input
                style={styles.input}
                value={selectedCapability?.name ?? ""}
                readOnly
                placeholder={`Create a top-level child under ${selectedModule?.name ?? "the selected product area"}`}
              />
              <label style={styles.label}>Node Kind</label>
              <select style={styles.input} value={capabilityForm.nodeKind} onChange={(e) => setCapabilityForm({ ...capabilityForm, nodeKind: e.target.value as HierarchyNodeKind })}>
                {selectedCapabilityAllowedKindGroups.map((group) => (
                  <optgroup key={group.label} label={group.label}>
                    {group.kinds.map((nodeKind) => (
                      <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <div style={styles.contextText}>
                {getHierarchyNodeKindGuidance(capabilityForm.nodeKind)}
                {" "}
                Allowed here: {selectedCapabilityAllowedKindGroups.flatMap((group) => group.kinds).map((nodeKind) => getHierarchyNodeKindLabel(nodeKind)).join(", ")}.
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
                {editableCapabilityNodeKindGroups.map((group) => (
                  <optgroup key={group.label} label={group.label}>
                    {group.kinds.map((nodeKind) => (
                      <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <div style={styles.contextText}>{getHierarchyNodeKindGuidance(capabilityDraft.nodeKind)}</div>
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

function getHierarchyDeleteLabel(kind: "area" | "capability" | "feature") {
  switch (kind) {
    case "area":
      return "Product Area";
    case "capability":
      return "Capability";
    case "feature":
      return "Feature";
  }
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

function flattenCapabilityTreeList(nodes: CapabilityTree[]): CapabilityTree[] {
  return nodes.flatMap((node) => [node, ...flattenCapabilityTreeList(node.children)]);
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

function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

function isExampleProduct(product: Product) {
  return product.id.startsWith("example-") || product.tags.includes("example_product") || product.tags.includes("seeded_catalog");
}

function isActiveWorkItem(workItem: WorkItem) {
  return workItem.status !== "done" && workItem.status !== "cancelled";
}

function getProgressSummary(workItems: WorkItem[]) {
  const total = workItems.length;
  const done = workItems.filter((workItem) => workItem.status === "done").length;
  return {
    total,
    done,
    percent: total > 0 ? Math.round((done / total) * 100) : 0,
  };
}

interface ProductStatusSummary {
  productCount: number;
  nodeCount: number;
  leafCount: number;
  workItemCount: number;
  activeWorkItemCount: number;
  doneWorkItemCount: number;
  progress: ReturnType<typeof getProgressSummary>;
}

interface StatusRow {
  id: string;
  productId: string | null;
  nodeId?: string;
  nodeType?: HierarchyTreeNode["node_type"];
  moduleId?: string;
  capabilityId?: string | null;
  level: number;
  name: string;
  subtitle: string;
  kind: string;
  childCount: number;
  nodeCount: number;
  workItemCount: number;
  activeWorkItemCount: number;
  progress: ReturnType<typeof getProgressSummary>;
}

function buildProductStatusSummary(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  productTasksById: Map<string, WorkItem[]>,
): ProductStatusSummary {
  const allWorkItems = products.flatMap((product) => productTasksById.get(product.id) ?? []);
  return {
    productCount: products.length,
    nodeCount: products.reduce((total, product) => total + countHierarchyNodes(productTreeById.get(product.id)?.roots ?? []), 0),
    leafCount: products.reduce((total, product) => total + countLeafNodes(productTreeById.get(product.id)?.roots ?? []), 0),
    workItemCount: allWorkItems.length,
    activeWorkItemCount: allWorkItems.filter(isActiveWorkItem).length,
    doneWorkItemCount: allWorkItems.filter((workItem) => workItem.status === "done").length,
    progress: getProgressSummary(allWorkItems),
  };
}

function buildStatusRows(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  productTasksById: Map<string, WorkItem[]>,
  maxDepth: number,
  groupBy: "node" | "kind" | "work_status",
): StatusRow[] {
  if (groupBy === "kind") {
    return buildKindPivotRows(products, productTreeById, productTasksById, maxDepth);
  }
  if (groupBy === "work_status") {
    return buildWorkStatusPivotRows(products, productTasksById);
  }

  const rows: StatusRow[] = [];
  const includeProductRows = products.length !== 1;
  products.forEach((product) => {
    const tree = productTreeById.get(product.id);
    const workItems = productTasksById.get(product.id) ?? [];
    if (includeProductRows || !tree?.roots.length) {
      rows.push({
        id: `product:${product.id}`,
        productId: product.id,
        level: 0,
        name: product.name,
        subtitle: product.description || product.vision || "Product summary",
        kind: "Product",
        childCount: tree?.roots.length ?? 0,
        nodeCount: tree ? countHierarchyNodes(tree.roots) : 0,
        workItemCount: workItems.length,
        activeWorkItemCount: workItems.filter(isActiveWorkItem).length,
        progress: getProgressSummary(workItems),
      });
    }
    if (tree) {
      tree.roots.forEach((node) => pushNodeStatusRows(rows, product, node, workItems, maxDepth));
    }
  });
  return rows;
}

function pushNodeStatusRows(
  rows: StatusRow[],
  product: Product,
  node: HierarchyTreeNode,
  workItems: WorkItem[],
  maxDepth: number,
) {
  const level = node.depth + 1;
  if (level > maxDepth) {
    return;
  }
  const subtreeWorkItems = getSubtreeWorkItemsForNode(node, workItems);
  rows.push({
    id: `${product.id}:${node.node_type}:${node.id}`,
    productId: product.id,
    nodeId: node.id,
    nodeType: node.node_type,
    moduleId: node.module_id,
    capabilityId: node.capability_id,
    level,
    name: node.name,
    subtitle: node.path.join(" / ") || product.name,
    kind: getHierarchyNodeKindLabel(node.node_kind),
    childCount: node.children.length,
    nodeCount: countDescendantNodes(node) + 1,
    workItemCount: subtreeWorkItems.length,
    activeWorkItemCount: subtreeWorkItems.filter(isActiveWorkItem).length,
    progress: getProgressSummary(subtreeWorkItems),
  });
  node.children.forEach((child) => pushNodeStatusRows(rows, product, child, workItems, maxDepth));
}

function buildKindPivotRows(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  productTasksById: Map<string, WorkItem[]>,
  maxDepth: number,
): StatusRow[] {
  const groups = new Map<string, StatusRow>();
  products.forEach((product) => {
    const tree = productTreeById.get(product.id);
    const workItems = productTasksById.get(product.id) ?? [];
    (tree?.roots ?? []).forEach((node) => collectKindPivot(node, product, workItems, maxDepth, groups));
  });
  return Array.from(groups.values()).sort((a, b) => a.kind.localeCompare(b.kind));
}

function collectKindPivot(
  node: HierarchyTreeNode,
  product: Product,
  workItems: WorkItem[],
  maxDepth: number,
  groups: Map<string, StatusRow>,
) {
  const level = node.depth + 1;
  if (level > maxDepth) {
    return;
  }
  const kind = getHierarchyNodeKindLabel(node.node_kind);
  const directWorkItems = getDirectWorkItemsForNode(node, workItems);
  const existing = groups.get(kind) ?? {
    id: `kind:${kind}`,
    productId: product.id,
    level: 0,
    name: kind,
    subtitle: "Pivoted across matching node kinds",
    kind,
    childCount: 0,
    nodeCount: 0,
    workItemCount: 0,
    activeWorkItemCount: 0,
    progress: getProgressSummary([]),
  };
  const nextWorkItems = [
    ...Array.from({ length: existing.progress.done }, (_, index) => ({ id: `done-${index}`, status: "done" } as WorkItem)),
    ...Array.from({ length: existing.progress.total - existing.progress.done }, (_, index) => ({ id: `open-${index}`, status: "in_progress" } as WorkItem)),
    ...directWorkItems,
  ];
  groups.set(kind, {
    ...existing,
    childCount: existing.childCount + node.children.length,
    nodeCount: existing.nodeCount + 1,
    workItemCount: existing.workItemCount + directWorkItems.length,
    activeWorkItemCount: existing.activeWorkItemCount + directWorkItems.filter(isActiveWorkItem).length,
    progress: getProgressSummary(nextWorkItems),
  });
  node.children.forEach((child) => collectKindPivot(child, product, workItems, maxDepth, groups));
}

function buildWorkStatusPivotRows(
  products: Product[],
  productTasksById: Map<string, WorkItem[]>,
): StatusRow[] {
  const groups = new Map<string, WorkItem[]>();
  products.forEach((product) => {
    (productTasksById.get(product.id) ?? []).forEach((workItem) => {
      groups.set(workItem.status, [...(groups.get(workItem.status) ?? []), workItem]);
    });
  });
  return Array.from(groups.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([status, workItems]) => ({
      id: `work-status:${status}`,
      productId: products.length === 1 ? products[0].id : null,
      level: 0,
      name: status.replace(/_/g, " "),
      subtitle: "Pivoted across stories with this status",
      kind: "Work Status",
      childCount: 0,
      nodeCount: 0,
      workItemCount: workItems.length,
      activeWorkItemCount: workItems.filter(isActiveWorkItem).length,
      progress: getProgressSummary(workItems),
    }));
}
