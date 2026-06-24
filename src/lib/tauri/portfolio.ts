import { invoke } from "./core";
import type {
  ProductDependency,
  ProductDependencyKind,
  ProductDependencyStatus,
  ProductReference,
  ProductStrategyLink,
  StrategyNode,
  StrategyNodeKind,
} from "../types";

// Strategy / portfolio commands
export const listStrategyNodes = () => invoke<StrategyNode[]>("list_strategy_nodes");
export const createStrategyNode = (data: {
  parentNodeId?: string | null;
  nodeKind: StrategyNodeKind;
  name: string;
  description: string;
  ownerLabel?: string;
}) =>
  invoke<StrategyNode>("create_strategy_node", {
    parent_node_id: data.parentNodeId ?? null,
    parentNodeId: data.parentNodeId ?? null,
    node_kind: data.nodeKind,
    nodeKind: data.nodeKind,
    name: data.name,
    description: data.description,
    owner_label: data.ownerLabel ?? "",
    ownerLabel: data.ownerLabel ?? "",
  });
export const updateStrategyNode = (data: {
  id: string;
  parentNodeId?: string | null;
  clearParent?: boolean;
  nodeKind?: StrategyNodeKind;
  name?: string;
  description?: string;
  ownerLabel?: string;
}) =>
  invoke<StrategyNode>("update_strategy_node", {
    request: {
      id: data.id,
      parent_node_id: data.parentNodeId ?? null,
      clear_parent: data.clearParent ?? false,
      node_kind: data.nodeKind ?? null,
      name: data.name ?? null,
      description: data.description ?? null,
      owner_label: data.ownerLabel ?? null,
    },
  });
export const deleteStrategyNode = (id: string) => invoke<void>("delete_strategy_node", { id });
export const listProductStrategyLinks = () => invoke<ProductStrategyLink[]>("list_product_strategy_links");
export const linkProductToStrategy = (data: { productId: string; strategyNodeId: string; isPrimary?: boolean }) =>
  invoke<ProductStrategyLink>("link_product_to_strategy", {
    product_id: data.productId,
    productId: data.productId,
    strategy_node_id: data.strategyNodeId,
    strategyNodeId: data.strategyNodeId,
    is_primary: data.isPrimary ?? false,
    isPrimary: data.isPrimary ?? false,
  });
export const unlinkProductFromStrategy = (data: { productId: string; strategyNodeId: string }) =>
  invoke<void>("unlink_product_from_strategy", {
    product_id: data.productId,
    productId: data.productId,
    strategy_node_id: data.strategyNodeId,
    strategyNodeId: data.strategyNodeId,
  });
export const listProductDependencies = () => invoke<ProductDependency[]>("list_product_dependencies");
export const createProductDependency = (data: {
  productId: string;
  capabilityId?: string | null;
  dependsOnProductId: string;
  dependsOnCapabilityId?: string | null;
  dependencyKind?: ProductDependencyKind;
  description: string;
  status?: ProductDependencyStatus;
}) =>
  invoke<ProductDependency>("create_product_dependency", {
    request: {
      product_id: data.productId,
      capability_id: data.capabilityId ?? null,
      depends_on_product_id: data.dependsOnProductId,
      depends_on_capability_id: data.dependsOnCapabilityId ?? null,
      dependency_kind: data.dependencyKind ?? "platform",
      description: data.description,
      status: data.status ?? "active",
    },
  });
export const deleteProductDependency = (id: string) => invoke<void>("delete_product_dependency", { id });

export const listProductReferences = (scope?: { scopeType: ProductReference["scope_type"]; scopeId: string }) =>
  invoke<ProductReference[]>("list_product_references", {
    scope_type: scope?.scopeType,
    scope_id: scope?.scopeId,
  });
export const createProductReference = (data: {
  scopeType: ProductReference["scope_type"];
  scopeId: string;
  title: string;
  referenceKind: ProductReference["reference_kind"];
  uri?: string;
  content?: string;
}) =>
  invoke<ProductReference>("create_product_reference", {
    request: {
      scope_type: data.scopeType,
      scope_id: data.scopeId,
      title: data.title,
      reference_kind: data.referenceKind,
      uri: data.uri ?? "",
      content: data.content ?? "",
    },
  });
export const deleteProductReference = (id: string) => invoke<void>("delete_product_reference", { id });
