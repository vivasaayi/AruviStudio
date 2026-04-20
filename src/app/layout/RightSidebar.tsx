import React from "react";
import { useQuery } from "@tanstack/react-query";
import { getCapabilityChildLabel, getCapabilityHierarchyLabel } from "../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../state/workspaceStore";
import { getProductTree, getWorkItem, listProducts } from "../../lib/tauri";

const styles: Record<string, React.CSSProperties> = {
  container: { height: "100%", backgroundColor: "#1a1c21", overflow: "auto", padding: 10 },
  header: { fontSize: 11, fontWeight: 700, textTransform: "uppercase" as const, color: "#8f96a3", letterSpacing: 1, marginBottom: 10 },
  card: { padding: 10, borderRadius: 10, border: "1px solid #32353d", backgroundColor: "#22252b", marginBottom: 10 },
  title: { fontSize: 13, fontWeight: 700, color: "#f3f3f3", marginBottom: 6 },
  info: { fontSize: 12, color: "#aab2bf", lineHeight: 1.4 },
  meta: { fontSize: 11, color: "#8f96a3", marginTop: 4 },
};

export function RightSidebar() {
  const { activeWorkItemId, activeProductId, activeModuleId, activeCapabilityId } = useWorkspaceStore();
  const { data: products } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: tree } = useQuery({
    queryKey: ["inspectorProductTree", activeProductId],
    queryFn: () => getProductTree(activeProductId!),
    enabled: !!activeProductId,
  });
  const { data: workItem } = useQuery({ queryKey: ["sidebarWorkItem", activeWorkItemId], queryFn: () => getWorkItem(activeWorkItemId!), enabled: !!activeWorkItemId });
  const activeProduct = products?.find((product) => product.id === activeProductId) ?? null;
  const activeModule = tree?.modules.find((entry) => entry.module.id === activeModuleId)?.module ?? null;
  const activeCapabilityNode = tree ? findCapabilityNodeInTree(tree.modules, activeCapabilityId) : null;
  const scopeName = activeCapabilityNode
    ? `${getCapabilityHierarchyLabel(activeCapabilityNode.capability.level)} scope`
    : activeModuleId
      ? "Module scope"
      : activeProductId
        ? "Product scope"
        : "No scope selected";

  return (
    <div style={styles.container}>
      <div style={styles.header}>Inspector</div>
      <div style={styles.card}>
        <div style={styles.title}>Product</div>
        {activeProduct ? (
          <>
            <div style={styles.info}>{activeProduct.name}</div>
            <div style={styles.meta}>{activeProduct.status} · {activeProduct.tags.length > 0 ? activeProduct.tags.join(", ") : "No tags"}</div>
          </>
        ) : (
          <div style={styles.info}>Select a product to anchor planning and work item creation.</div>
        )}
      </div>
      <div style={styles.card}>
        <div style={styles.title}>Scope</div>
        <div style={styles.info}>{scopeName}</div>
        <div style={styles.meta}>
          {activeCapabilityNode
            ? `${activeCapabilityNode.capability.name} · ${activeCapabilityNode.children.length} ${getCapabilityChildLabel(activeCapabilityNode.capability.level, { plural: activeCapabilityNode.children.length !== 1, lowercase: true })}`
            : activeModule
              ? `${activeModule.name} · ${(tree?.modules.find((entry) => entry.module.id === activeModule.id)?.features.length ?? 0)} capabilities`
              : "Backlog and work item intake follow the active hierarchy."}
        </div>
      </div>
      {(activeModule || activeCapabilityNode) && (
        <div style={styles.card}>
          <div style={styles.title}>Detail</div>
          {activeCapabilityNode ? (
            <>
            <div style={styles.info}>{activeCapabilityNode.capability.name}</div>
            <div style={styles.meta}>{activeCapabilityNode.capability.status.replace(/_/g, " ")} · {activeCapabilityNode.capability.priority}</div>
            </>
          ) : activeModule ? (
            <>
              <div style={styles.info}>{activeModule.name}</div>
              <div style={styles.meta}>{activeModule.description || activeModule.purpose || "No module description yet."}</div>
            </>
          ) : null}
        </div>
      )}
      <div style={styles.card}>
        <div style={styles.title}>Work Item</div>
        {workItem ? (
          <>
            <div style={styles.info}>{workItem.title}</div>
            <div style={styles.meta}>{workItem.status.replace(/_/g, " ")} · {workItem.priority} · {workItem.work_item_type}</div>
          </>
        ) : (
          <div style={styles.info}>Select a work item to inspect delivery state and review signals.</div>
        )}
      </div>
    </div>
  );
}

function findCapabilityNodeInTree(modules: Array<{ features: Array<any> }>, capabilityId: string | null): any | null {
  if (!capabilityId) {
    return null;
  }
  for (const module of modules) {
    for (const capabilityNode of module.features) {
      const found = searchCapabilityNode(capabilityNode, capabilityId);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

function searchCapabilityNode(capabilityNode: any, capabilityId: string): any | null {
  if (capabilityNode.capability.id === capabilityId) {
    return capabilityNode;
  }
  for (const child of capabilityNode.children) {
    const found = searchCapabilityNode(child, capabilityId);
    if (found) {
      return found;
    }
  }
  return null;
}
