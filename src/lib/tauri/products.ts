import { invoke, toJsonArrayString } from "./core";
import type {
  Capability,
  NodeKindConversionResult,
  Product,
  ProductArea,
  ProductTree,
  ProductTreeSummary,
  SemanticTemplateApplicationResult,
} from "../types";

// Product commands
export const createProduct = (data: {
  name: string;
  description: string;
  vision: string;
  goals: string;
  tags: string;
  lifecycle?: Product["lifecycle"];
  health?: Product["health"];
  ownerLabel?: string;
  investmentStatus?: Product["investment_status"];
  roadmap?: string;
  evidence?: string;
}) =>
  invoke<Product>("create_product", {
    request: {
      name: data.name,
      description: data.description,
      vision: data.vision,
      goals: toJsonArrayString(data.goals),
      tags: toJsonArrayString(data.tags),
      lifecycle: data.lifecycle,
      health: data.health,
      owner_label: data.ownerLabel,
      investment_status: data.investmentStatus,
      roadmap: data.roadmap,
      evidence: data.evidence,
    },
  });

export const getProduct = (id: string) => invoke<Product>("get_product", { id });
export const listProducts = () => invoke<Product[]>("list_products");
export const seedExampleProducts = () => invoke<void>("seed_example_products");
export const updateProduct = (data: {
  id: string;
  name?: string;
  description?: string;
  vision?: string;
  goals?: string;
  tags?: string;
  lifecycle?: Product["lifecycle"];
  health?: Product["health"];
  ownerLabel?: string;
  investmentStatus?: Product["investment_status"];
  roadmap?: string;
  evidence?: string;
}) =>
  invoke<Product>("update_product", {
    request: {
      id: data.id,
      name: data.name,
      description: data.description,
      vision: data.vision,
      goals: toJsonArrayString(data.goals),
      tags: toJsonArrayString(data.tags),
      lifecycle: data.lifecycle,
      health: data.health,
      owner_label: data.ownerLabel,
      investment_status: data.investmentStatus,
      roadmap: data.roadmap,
      evidence: data.evidence,
    },
  });
export const archiveProduct = (id: string) => invoke<Product>("archive_product", { id });
export const resetProductPlan = (data: { productId: string; deleteDelivery: boolean }) =>
  invoke<{
    productId: string;
    productAreasDeleted: number;
    capabilitiesDeleted: number;
    workItemsDeleted: number;
    agentWorkRunsDeleted: number;
    agentWorkItemsDeleted: number;
    agentWorkEventsDeleted: number;
    agentWorkEvidenceDeleted: number;
    agentWorkDependenciesDeleted: number;
    agentWorkLocksDeleted: number;
    agentWorkBatchesDeleted: number;
  }>("reset_product_plan", {
    product_id: data.productId,
    productId: data.productId,
    delete_delivery: data.deleteDelivery,
    deleteDelivery: data.deleteDelivery,
  });

// ProductArea commands
export const createProductArea = (data: {
  productId: string;
  name: string;
  description: string;
  purpose: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<ProductArea>("create_product_area", {
    request: {
      product_id: data.productId,
      name: data.name,
      description: data.description,
      purpose: data.purpose,
      node_kind: data.nodeKind,
      explanation: data.explanation,
      examples: data.examples,
      implementation_notes: data.implementationNotes,
      test_guidance: data.testGuidance,
    },
  });
export const listProductAreas = (productId: string) =>
  invoke<ProductArea[]>("list_product_areas", { productId, product_id: productId });
export const updateProductArea = (data: {
  id: string;
  name?: string;
  description?: string;
  purpose?: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<ProductArea>("update_product_area", {
    request: {
      id: data.id,
      name: data.name,
      description: data.description,
      purpose: data.purpose,
      node_kind: data.nodeKind,
      explanation: data.explanation,
      examples: data.examples,
      implementation_notes: data.implementationNotes,
      test_guidance: data.testGuidance,
    },
  });
export const deleteProductArea = (id: string) => invoke("delete_product_area", { id });
export const reorderProductAreas = (productId: string, orderedIds: string[]) =>
  invoke("reorder_product_areas", {
    productId,
    product_id: productId,
    orderedIds,
    ordered_ids: orderedIds,
  });

// Capability commands
export const createCapability = (data: {
  productAreaId: string;
  parentCapabilityId?: string;
  name: string;
  description: string;
  acceptanceCriteria: string;
  priority: string;
  risk: string;
  technicalNotes: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Capability>("create_capability", {
    request: {
      product_area_id: data.productAreaId,
      parent_capability_id: data.parentCapabilityId,
      name: data.name,
      description: data.description,
      acceptance_criteria: data.acceptanceCriteria,
      priority: data.priority,
      risk: data.risk,
      technical_notes: data.technicalNotes,
      node_kind: data.nodeKind,
      explanation: data.explanation,
      examples: data.examples,
      implementation_notes: data.implementationNotes,
      test_guidance: data.testGuidance,
    },
  });
export const listCapabilities = (productAreaId: string) =>
  invoke<Capability[]>("list_capabilities", { productAreaId, product_area_id: productAreaId });
export const listProductCapabilities = (productId: string) =>
  invoke<Capability[]>("list_product_capabilities", { productId, product_id: productId });
export const getCapability = (id: string) => invoke<Capability>("get_capability", { id });
export const updateCapability = (data: {
  id: string;
  name?: string;
  description?: string;
  acceptanceCriteria?: string;
  priority?: string;
  risk?: string;
  technicalNotes?: string;
  nodeKind?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<Capability>("update_capability", {
    request: {
      id: data.id,
      name: data.name,
      description: data.description,
      acceptance_criteria: data.acceptanceCriteria,
      priority: data.priority,
      risk: data.risk,
      technical_notes: data.technicalNotes,
      node_kind: data.nodeKind,
      explanation: data.explanation,
      examples: data.examples,
      implementation_notes: data.implementationNotes,
      test_guidance: data.testGuidance,
    },
  });
export const deleteCapability = (id: string) => invoke("delete_capability", { id });
export const reorderCapabilities = (data: { productAreaId: string; parentCapabilityId?: string; orderedIds: string[] }) =>
  invoke("reorder_capabilities", {
    product_area_id: data.productAreaId,
    parent_capability_id: data.parentCapabilityId,
    ordered_ids: data.orderedIds,
  });
export const applySemanticTemplate = (data: {
  productAreaId: string;
  parentCapabilityId?: string;
  templateKind: "operator_chapter" | "technical_topic_book";
  name: string;
  description?: string;
  priority?: string;
  risk?: string;
  explanation?: string;
  examples?: string;
  implementationNotes?: string;
  testGuidance?: string;
}) =>
  invoke<SemanticTemplateApplicationResult>("apply_semantic_template", {
    request: {
      product_area_id: data.productAreaId,
      parent_capability_id: data.parentCapabilityId,
      template_kind: data.templateKind,
      name: data.name,
      description: data.description,
      priority: data.priority,
      risk: data.risk,
      explanation: data.explanation,
      examples: data.examples,
      implementation_notes: data.implementationNotes,
      test_guidance: data.testGuidance,
    },
  });
export const convertCapabilityKind = (data: {
  id: string;
  nodeKind: string;
  childStrategy?: "reject" | "reparent_to_parent";
}) =>
  invoke<NodeKindConversionResult>("convert_capability_kind", {
    id: data.id,
    nodeKind: data.nodeKind,
    node_kind: data.nodeKind,
    childStrategy: data.childStrategy,
    child_strategy: data.childStrategy,
  });

// Product tree
export const getProductTree = (productId: string) =>
  invoke<ProductTree>("get_product_tree", { productId, product_id: productId });
export const summarizeProductTree = (productId: string) =>
  invoke<ProductTreeSummary>("summarize_product_tree", { productId, product_id: productId });
