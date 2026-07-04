import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createStrategyNode,
  deleteStrategyNode,
  linkProductToStrategy,
  listProductDependencies,
  listProducts,
  listProductStrategyLinks,
  listStrategyNodes,
  unlinkProductFromStrategy,
  updateStrategyNode,
} from "../../../lib/tauri";
import type { Product, StrategyNode } from "../../../lib/types";
import {
  buildStrategyTree,
  collectStrategySubtreeIds,
  countProductsForStrategy,
  findTreeNode,
  formatPortfolioError,
  getChildKind,
} from "../lib/portfolioStrategyTree";
import { emptyStrategyForm, type PortfolioTab, type StrategyDialogMode, type StrategyFormState } from "../lib/portfolioPageState";

export function usePortfolioPageController() {
  const queryClient = useQueryClient();
  const { data: products = [], isLoading: productsLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: strategyNodes = [], isLoading: strategyLoading } = useQuery({ queryKey: ["strategy-nodes"], queryFn: listStrategyNodes });
  const { data: strategyLinks = [] } = useQuery({ queryKey: ["product-strategy-links"], queryFn: listProductStrategyLinks });
  const { data: dependencies = [] } = useQuery({ queryKey: ["product-dependencies"], queryFn: listProductDependencies });

  const [activeTab, setActiveTab] = useState<PortfolioTab>("summary");
  const [selectedStrategyNodeId, setSelectedStrategyNodeId] = useState<string | null>(null);
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(() => new Set());
  const [strategyDialogMode, setStrategyDialogMode] = useState<StrategyDialogMode>("closed");
  const [strategyForm, setStrategyForm] = useState<StrategyFormState>(emptyStrategyForm);
  const [editingNodeId, setEditingNodeId] = useState<string | null>(null);
  const [deleteCandidate, setDeleteCandidate] = useState<StrategyNode | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [deleteAcknowledge, setDeleteAcknowledge] = useState(false);
  const [linkDraft, setLinkDraft] = useState({ productId: "", isPrimary: true });
  const [formError, setFormError] = useState<string | null>(null);

  const strategyTree = useMemo(() => buildStrategyTree(strategyNodes), [strategyNodes]);
  const selectedStrategyNode = strategyNodes.find((node) => node.id === selectedStrategyNodeId) ?? null;
  const strategyNodeById = useMemo(() => new Map(strategyNodes.map((node) => [node.id, node])), [strategyNodes]);
  const unlinkedProducts = products.filter((product) => !strategyLinks.some((link) => link.product_id === product.id));
  const visibleProductIds = useMemo(() => {
    if (!selectedStrategyNodeId) {
      return null;
    }
    const descendantIds = new Set(collectStrategySubtreeIds(strategyTree, selectedStrategyNodeId));
    return new Set(strategyLinks.filter((link) => descendantIds.has(link.strategy_node_id)).map((link) => link.product_id));
  }, [selectedStrategyNodeId, strategyLinks, strategyTree]);
  const visibleProducts = visibleProductIds ? products.filter((product) => visibleProductIds.has(product.id)) : products;
  const selectedNodeProductCount = selectedStrategyNode
    ? countProductsForStrategy(findTreeNode(strategyTree, selectedStrategyNode.id), strategyLinks)
    : products.length;
  const selectedNodeLinks = selectedStrategyNode
    ? strategyLinks.filter((link) => link.strategy_node_id === selectedStrategyNode.id)
    : [];
  const selectedNodeLinkedProductIds = new Set(selectedNodeLinks.map((link) => link.product_id));
  const linkableProducts = products.filter((product) => !selectedNodeLinkedProductIds.has(product.id));

  const invalidateStrategy = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["strategy-nodes"] }),
      queryClient.invalidateQueries({ queryKey: ["product-strategy-links"] }),
    ]);
  };

  const closeStrategyDialog = () => {
    setStrategyDialogMode("closed");
    setStrategyForm(emptyStrategyForm);
    setEditingNodeId(null);
    setFormError(null);
  };

  const createStrategyMutation = useMutation({
    mutationFn: () => createStrategyNode({
      parentNodeId: strategyForm.parentNodeId || null,
      nodeKind: strategyForm.nodeKind,
      name: strategyForm.name.trim(),
      description: strategyForm.description.trim(),
      ownerLabel: strategyForm.ownerLabel.trim(),
    }),
    onSuccess: async (created) => {
      setSelectedStrategyNodeId(created.id);
      setExpandedNodeIds((current) => new Set([...current, created.parent_node_id ?? created.id]));
      closeStrategyDialog();
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const updateStrategyMutation = useMutation({
    mutationFn: () => updateStrategyNode({
      id: editingNodeId!,
      parentNodeId: strategyForm.parentNodeId || null,
      clearParent: !strategyForm.parentNodeId,
      nodeKind: strategyForm.nodeKind,
      name: strategyForm.name.trim(),
      description: strategyForm.description.trim(),
      ownerLabel: strategyForm.ownerLabel.trim(),
    }),
    onSuccess: async (updated) => {
      setSelectedStrategyNodeId(updated.id);
      closeStrategyDialog();
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const deleteStrategyMutation = useMutation({
    mutationFn: (id: string) => deleteStrategyNode(id),
    onSuccess: async (_, deletedId) => {
      if (selectedStrategyNodeId === deletedId) {
        setSelectedStrategyNodeId(null);
      }
      setDeleteCandidate(null);
      setDeleteConfirmName("");
      setDeleteAcknowledge(false);
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const linkProductMutation = useMutation({
    mutationFn: () => linkProductToStrategy({
      productId: linkDraft.productId,
      strategyNodeId: selectedStrategyNodeId!,
      isPrimary: linkDraft.isPrimary,
    }),
    onSuccess: async () => {
      setLinkDraft({ productId: "", isPrimary: true });
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const unlinkProductMutation = useMutation({
    mutationFn: (productId: string) => unlinkProductFromStrategy({
      productId,
      strategyNodeId: selectedStrategyNodeId!,
    }),
    onSuccess: async () => {
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const productLabel = (productId: string) => products.find((product) => product.id === productId)?.name ?? "Unknown product";
  const strategyLabel = (strategyNodeId: string) => strategyNodeById.get(strategyNodeId)?.name ?? "Unknown strategy";
  const selectedProductLinks = (product: Product) => strategyLinks.filter((link) => link.product_id === product.id);

  const openCreateRootDialog = () => {
    setStrategyForm({ ...emptyStrategyForm, nodeKind: "strategic_product_area" });
    setEditingNodeId(null);
    setFormError(null);
    setStrategyDialogMode("create");
  };

  const openCreateChildDialog = (parent: StrategyNode) => {
    const childKind = getChildKind(parent.node_kind);
    if (!childKind) {
      return;
    }
    setStrategyForm({
      ...emptyStrategyForm,
      parentNodeId: parent.id,
      nodeKind: childKind,
    });
    setEditingNodeId(null);
    setFormError(null);
    setStrategyDialogMode("create");
    setExpandedNodeIds((current) => new Set([...current, parent.id]));
  };

  const openEditDialog = (node: StrategyNode) => {
    setStrategyForm({
      parentNodeId: node.parent_node_id ?? "",
      nodeKind: node.node_kind,
      name: node.name,
      description: node.description,
      ownerLabel: node.owner_label,
    });
    setEditingNodeId(node.id);
    setFormError(null);
    setStrategyDialogMode("edit");
  };

  const toggleExpanded = (nodeId: string) => {
    setExpandedNodeIds((current) => {
      const next = new Set(current);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };

  const expandAll = () => setExpandedNodeIds(new Set(strategyNodes.map((node) => node.id)));
  const collapseAll = () => setExpandedNodeIds(new Set());

  return {
    activeTab,
    collapseAll,
    createStrategyMutation,
    deleteAcknowledge,
    deleteCandidate,
    deleteConfirmName,
    deleteStrategyMutation,
    dependencies,
    editingNodeId,
    expandAll,
    expandedNodeIds,
    formError,
    linkDraft,
    linkProductMutation,
    linkableProducts,
    openCreateChildDialog,
    openCreateRootDialog,
    openEditDialog,
    productLabel,
    products,
    productsLoading,
    selectedNodeLinks,
    selectedNodeProductCount,
    selectedProductLinks,
    selectedStrategyNode,
    selectedStrategyNodeId,
    setActiveTab,
    setDeleteAcknowledge,
    setDeleteCandidate,
    setDeleteConfirmName,
    setFormError,
    setLinkDraft,
    setSelectedStrategyNodeId,
    setStrategyForm,
    strategyDialogMode,
    strategyForm,
    strategyLabel,
    strategyLinks,
    strategyLoading,
    strategyNodes,
    strategyTree,
    toggleExpanded,
    unlinkProductMutation,
    unlinkedProducts,
    updateStrategyMutation,
    visibleProducts,
    closeStrategyDialog,
  };
}

export type PortfolioPageController = ReturnType<typeof usePortfolioPageController>;
