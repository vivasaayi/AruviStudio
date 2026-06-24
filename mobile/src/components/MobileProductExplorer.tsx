import React from "react";
import { FlatList, Modal, Pressable, ScrollView, Text, TextInput, View } from "react-native";
import { ProductModeButton, ProductNodeRow, ProductPlannerPanel } from "./MobileProductComponents";
import type { HierarchyTreeNode, MobilePlannerToolTraceEntry, Product, ProductTree, ProductTreeSummary } from "../types";
import { formatNodeKind, getNodeSummary } from "../lib/productTree";
import { styles } from "../styles/appStyles";

export type ProductExploreTab = "map" | "work" | "search" | "overview";

type ProductStats = {
  productAreas: number;
  capabilities: number;
  totalNodes: number;
  leafNodes: number;
};

type ProductSearchResult = {
  node: HierarchyTreeNode;
  pathLabel: string;
};

type MobileProductExplorerProps = {
  selectedProduct: Product | null;
  selectedProductId: string | null;
  selectedProductNode: HierarchyTreeNode | null;
  selectedProductNodePath: HierarchyTreeNode[];
  productSummary: ProductTreeSummary | null;
  productTree: ProductTree | null;
  productStats: ProductStats;
  visibleProductChildren: HierarchyTreeNode[];
  filteredProductNodes: ProductSearchResult[];
  productExploreTab: ProductExploreTab;
  productSearchQuery: string;
  productError: string | null;
  isProductTreeLoading: boolean;
  isProductPickerOpen: boolean;
  products: Product[];
  productPlannerRecording: boolean;
  isRecorderRecording: boolean;
  isVoiceBusy: boolean;
  productPlannerStatus: string;
  productPlannerReply: string;
  productPlannerDraft: string;
  productPlannerTrace: MobilePlannerToolTraceEntry[];
  onLoadProducts: (preferredProductId?: string | null) => Promise<void>;
  onEnsureProductTree: (productId: string, force?: boolean) => Promise<unknown>;
  onProductError: (message: string) => void;
  onOpenNode: (nodeId: string) => void;
  onSelectParentNode: (nodeId: string | null) => void;
  onSwitchExploreTab: (mode: ProductExploreTab) => Promise<void>;
  onProductSearchQueryChange: (query: string) => void;
  onProductPickerOpenChange: (isOpen: boolean) => void;
  onSpeakPlannerReply: (reply: string) => void;
  onPlannerDraftChange: (draft: string) => void;
  onTogglePlannerRecording: () => Promise<void>;
  onSubmitPlannerPrompt: (prompt: string) => Promise<void>;
  describeError: (error: unknown) => string;
};

export function MobileProductExplorer({
  selectedProduct,
  selectedProductId,
  selectedProductNode,
  selectedProductNodePath,
  productSummary,
  productTree,
  productStats,
  visibleProductChildren,
  filteredProductNodes,
  productExploreTab,
  productSearchQuery,
  productError,
  isProductTreeLoading,
  isProductPickerOpen,
  products,
  productPlannerRecording,
  isRecorderRecording,
  isVoiceBusy,
  productPlannerStatus,
  productPlannerReply,
  productPlannerDraft,
  productPlannerTrace,
  onLoadProducts,
  onEnsureProductTree,
  onProductError,
  onOpenNode,
  onSelectParentNode,
  onSwitchExploreTab,
  onProductSearchQueryChange,
  onProductPickerOpenChange,
  onSpeakPlannerReply,
  onPlannerDraftChange,
  onTogglePlannerRecording,
  onSubmitPlannerPrompt,
  describeError,
}: MobileProductExplorerProps) {
  const currentContextTitle = selectedProductNode?.name ?? selectedProduct?.name ?? "Products";
  const currentContextSummary = selectedProductNode
    ? getNodeSummary(selectedProductNode)
    : selectedProduct?.description || "Select a product to inspect its structure.";
  const pathNodes = selectedProductNodePath;
  const hasProductContext = Boolean(productSummary || productTree);
  const productMeta = [
    `${productStats.productAreas} product areas`,
    `${productStats.totalNodes} nodes`,
    `${productStats.leafNodes} leaves`,
    selectedProduct?.status ?? productTree?.product.status ?? null,
  ].filter(Boolean).join(" · ");
  const parentPathLabel = pathNodes.length > 1
    ? pathNodes.slice(0, -1).map((node) => node.name).join(" / ")
    : selectedProduct?.name ?? "Product";

  if (productError && !productSummary && !productTree) {
    return (
      <View style={styles.productEmptyScreen}>
        <Text style={styles.productEmptyTitle}>Products unavailable</Text>
        <Text style={styles.productEmptyText}>{productError}</Text>
        <Pressable style={styles.productPrimaryAction} onPress={() => void onLoadProducts(selectedProductId)}>
          <Text style={styles.productPrimaryActionText}>Retry</Text>
        </Pressable>
      </View>
    );
  }

  const treeRequiredFallback = (
    <View style={styles.productEmptyScreen}>
      <Text style={styles.productEmptyTitle}>
        {isProductTreeLoading ? "Loading product map" : "Product map not loaded"}
      </Text>
      <Text style={styles.productEmptyText}>
        {isProductTreeLoading
          ? "Fetching the semantic tree for this product."
          : "Overview uses aggregate metrics. Load the map only when you need to browse product areas, capabilities, and features."}
      </Text>
      {!isProductTreeLoading && selectedProductId ? (
        <Pressable
          style={styles.productPrimaryAction}
          onPress={() => {
            void onEnsureProductTree(selectedProductId).catch((error) => {
              onProductError(describeError(error));
            });
          }}
        >
          <Text style={styles.productPrimaryActionText}>Load Product Map</Text>
        </Pressable>
      ) : null}
    </View>
  );

  return (
    <View style={styles.productScreen}>
      <View style={styles.productHeader}>
        <View style={styles.productHeaderTop}>
          <View style={styles.productHeaderCopy}>
            <Text style={styles.productHeaderTitle} numberOfLines={1}>{selectedProduct?.name ?? "No product"}</Text>
            <Text style={styles.productHeaderSummary} numberOfLines={1}>
              {hasProductContext ? productMeta : "Load a product to browse its summary."}
            </Text>
          </View>
          <Pressable
            style={styles.productChangeButton}
            onPress={() => onProductPickerOpenChange(true)}
            disabled={!products.length}
          >
            <Text style={styles.productChangeText}>Change</Text>
          </Pressable>
        </View>

        <View style={styles.productModeRow}>
          <ProductModeButton mode="map" label="Map" activeMode={productExploreTab} onPress={onSwitchExploreTab} />
          <ProductModeButton mode="work" label="Work" activeMode={productExploreTab} onPress={onSwitchExploreTab} />
          <ProductModeButton mode="search" label="Search" activeMode={productExploreTab} onPress={onSwitchExploreTab} />
          <ProductModeButton mode="overview" label="Overview" activeMode={productExploreTab} onPress={onSwitchExploreTab} />
        </View>
      </View>

      {productExploreTab === "map" ? (
        !productTree ? treeRequiredFallback : (
        <FlatList
          data={visibleProductChildren}
          keyExtractor={(node) => node.id}
          contentContainerStyle={styles.productListContent}
          ListHeaderComponent={(
            selectedProductNode ? (
              <View style={styles.productContextPanel}>
                <View style={styles.productBreadcrumbRow}>
                  <Pressable style={styles.productBackButton} onPress={() => onSelectParentNode(pathNodes[pathNodes.length - 2]?.id ?? null)}>
                    <Text style={styles.productBackText}>Back</Text>
                  </Pressable>
                  <Text style={styles.productPathLine} numberOfLines={1} ellipsizeMode="middle">
                    {parentPathLabel}
                  </Text>
                </View>
                <Text style={styles.productContextTitle} numberOfLines={2}>{currentContextTitle}</Text>
                <Text style={styles.productContextSummary} numberOfLines={3}>{currentContextSummary}</Text>
                <View style={styles.productNodeMetaRow}>
                  <Text style={styles.productKindBadge}>{formatNodeKind(selectedProductNode.node_kind)}</Text>
                  <Text style={styles.productNodeMeta}>
                    {visibleProductChildren.length === 1 ? "1 child" : `${visibleProductChildren.length} children`}
                  </Text>
                </View>
                <ProductPlannerPanel
                  isRecording={productPlannerRecording || isRecorderRecording}
                  isDisabled={isVoiceBusy && !(productPlannerRecording || isRecorderRecording)}
                  status={productPlannerStatus}
                  reply={productPlannerReply}
                  draft={productPlannerDraft}
                  trace={productPlannerTrace}
                  onSpeakReply={onSpeakPlannerReply}
                  onDraftChange={onPlannerDraftChange}
                  onToggleRecording={onTogglePlannerRecording}
                  onSubmitPrompt={onSubmitPlannerPrompt}
                />
              </View>
            ) : (
              <View style={styles.productRootHeader}>
                <Text style={styles.productRootTitle}>Root sections</Text>
                <Text style={styles.productRootMeta}>
                  {visibleProductChildren.length === 1 ? "1 top-level section" : `${visibleProductChildren.length} top-level sections`}
                </Text>
              </View>
            )
          )}
          renderItem={({ item }) => <ProductNodeRow node={item} onOpenNode={onOpenNode} />}
          ListEmptyComponent={(
            <View style={styles.productEmptyBlock}>
              <Text style={styles.productEmptyTitle}>No children here</Text>
              <Text style={styles.productEmptyText}>This node is a leaf. Use Search to jump elsewhere in the product map.</Text>
            </View>
          )}
        />
        )
      ) : productExploreTab === "search" ? (
        !productTree ? treeRequiredFallback : (
        <View style={styles.productSearchScreen}>
          <TextInput
            style={styles.productSearchInput}
            value={productSearchQuery}
            onChangeText={onProductSearchQueryChange}
            placeholder="Search nodes, kinds, summaries"
            placeholderTextColor="#7d8898"
            autoCapitalize="none"
            autoCorrect={false}
          />
          <FlatList
            data={filteredProductNodes}
            keyExtractor={(item) => item.node.id}
            contentContainerStyle={styles.productListContent}
            renderItem={({ item }) => (
              <ProductNodeRow node={item.node} pathLabel={item.pathLabel} onOpenNode={onOpenNode} />
            )}
            ListHeaderComponent={(
              <Text style={styles.productSearchCount}>
                {filteredProductNodes.length} {filteredProductNodes.length === 1 ? "match" : "matches"}
              </Text>
            )}
            ListEmptyComponent={(
              <View style={styles.productEmptyBlock}>
                <Text style={styles.productEmptyTitle}>No matches</Text>
                <Text style={styles.productEmptyText}>Try a product area, capability, node kind, or technical term.</Text>
              </View>
            )}
          />
        </View>
        )
      ) : productExploreTab === "overview" ? (
        <ScrollView style={styles.productOverviewScreen} contentContainerStyle={styles.productListContent}>
          <View style={styles.productOverviewCard}>
            <Text style={styles.productOverviewTitle}>{selectedProduct?.name ?? "Product"}</Text>
            <Text style={styles.productOverviewText}>{selectedProduct?.description || "No description."}</Text>
          </View>
          <View style={styles.productOverviewGrid}>
            <View style={styles.productOverviewMetric}>
              <Text style={styles.productStatValue}>{productStats.productAreas}</Text>
              <Text style={styles.productStatLabel}>Product Areas</Text>
            </View>
            <View style={styles.productOverviewMetric}>
              <Text style={styles.productStatValue}>{productStats.capabilities}</Text>
              <Text style={styles.productStatLabel}>Capabilities</Text>
            </View>
            <View style={styles.productOverviewMetric}>
              <Text style={styles.productStatValue}>{productStats.totalNodes}</Text>
              <Text style={styles.productStatLabel}>All nodes</Text>
            </View>
            <View style={styles.productOverviewMetric}>
              <Text style={styles.productStatValue}>{productStats.leafNodes}</Text>
              <Text style={styles.productStatLabel}>Leaf nodes</Text>
            </View>
          </View>
          {selectedProduct?.tags?.length ? (
            <View style={styles.productOverviewCard}>
              <Text style={styles.productOverviewTitle}>Tags</Text>
              <View style={styles.productTagRow}>
                {selectedProduct.tags.map((tag) => (
                  <Text key={tag} style={styles.productKindBadge}>{tag}</Text>
                ))}
              </View>
            </View>
          ) : null}
        </ScrollView>
      ) : (
        <View style={styles.productEmptyScreen}>
          <Text style={styles.productEmptyTitle}>Work view coming next</Text>
          <Text style={styles.productEmptyText}>
            The native product map is now separated from the desktop WebView. Next we should add a mobile work-item endpoint and show active delivery work by selected node.
          </Text>
        </View>
      )}
      <Modal
        visible={isProductPickerOpen}
        animationType="slide"
        transparent
        onRequestClose={() => onProductPickerOpenChange(false)}
      >
        <View style={styles.productModalOverlay}>
          <View style={styles.productModal}>
            <View style={styles.productModalHeader}>
              <View style={styles.productModalTitleBlock}>
                <Text style={styles.productModalTitle}>Choose Product</Text>
                <Text style={styles.productModalMeta}>{products.length} available</Text>
              </View>
              <Pressable style={styles.productModalClose} onPress={() => onProductPickerOpenChange(false)}>
                <Text style={styles.productModalCloseText}>Close</Text>
              </Pressable>
            </View>
            <FlatList
              data={products}
              keyExtractor={(product) => product.id}
              contentContainerStyle={styles.productModalList}
              renderItem={({ item }) => (
                <Pressable
                  style={[
                    styles.productModalRow,
                    item.id === selectedProduct?.id && styles.productModalRowActive,
                  ]}
                  onPress={() => {
                    onProductPickerOpenChange(false);
                    void onLoadProducts(item.id);
                  }}
                >
                  <View style={styles.productModalRowCopy}>
                    <Text style={styles.productModalRowTitle} numberOfLines={1}>{item.name}</Text>
                    <Text style={styles.productModalRowSummary} numberOfLines={2}>
                      {item.description || item.status}
                    </Text>
                  </View>
                  <Text style={styles.productModalRowStatus}>{item.status}</Text>
                </Pressable>
              )}
            />
          </View>
        </View>
      </Modal>
    </View>
  );
}
