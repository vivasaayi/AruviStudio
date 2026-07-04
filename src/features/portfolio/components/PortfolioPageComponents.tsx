import type React from "react";

import type { Product, ProductDependency, ProductStrategyLink, StrategyNode, StrategyNodeKind } from "../../../lib/types";
import {
  collectDescendantIds,
  countProductsForStrategy,
  getChildKind,
  strategyKindLabels,
  type StrategyTreeNode,
} from "../lib/portfolioStrategyTree";
import type { StrategyFormState } from "../lib/portfolioPageState";
import { styles } from "../lib/portfolioPageStyles";

export function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div style={styles.statCard}>
      <div style={styles.statLabel}>{label}</div>
      <div style={styles.statValue}>{value}</div>
    </div>
  );
}

export function ProductCard({ product, links, strategyLabel }: { product: Product; links: ProductStrategyLink[]; strategyLabel: (strategyNodeId: string) => string }) {
  return (
    <div style={styles.productCard}>
      <div style={styles.productTitle}>{product.name}</div>
      <div style={styles.productText}>{product.description || product.vision || "No product description yet."}</div>
      <div style={styles.badgeRow}>
        <span style={styles.badgeGreen}>{product.status}</span>
        <span style={styles.badgeMuted}>{product.lifecycle}</span>
        <span style={styles.badgeMuted}>{product.health}</span>
        {links.map((link) => (
          <span key={link.id} style={styles.badge}>{link.is_primary ? "Primary: " : ""}{strategyLabel(link.strategy_node_id)}</span>
        ))}
      </div>
    </div>
  );
}

export function CompactStrategyList(props: {
  nodes: StrategyTreeNode[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  strategyLinks: ProductStrategyLink[];
}) {
  return (
    <div style={styles.tree}>
      {props.nodes.map((node) => (
        <button
          key={node.id}
          style={props.selectedId === node.id ? styles.treeNodeActive : styles.treeNode}
          onClick={() => props.onSelect(node.id)}
        >
          <div style={styles.treeTitle}>{node.name}</div>
          <div style={styles.treeMeta}>{strategyKindLabels[node.node_kind]} / {countProductsForStrategy(node, props.strategyLinks)} products</div>
        </button>
      ))}
    </div>
  );
}

export function StrategyNodeAccordion(props: {
  node: StrategyTreeNode;
  selectedId: string | null;
  expandedNodeIds: Set<string>;
  strategyLinks: ProductStrategyLink[];
  onSelect: (id: string) => void;
  onToggle: (id: string) => void;
  onAddChild: (node: StrategyNode) => void;
  onEdit: (node: StrategyNode) => void;
  onDelete: (node: StrategyNode) => void;
}) {
  const { node, selectedId, expandedNodeIds, strategyLinks, onSelect, onToggle, onAddChild, onEdit, onDelete } = props;
  const isExpanded = expandedNodeIds.has(node.id);
  const productCount = countProductsForStrategy(node, strategyLinks);
  const canAddChild = Boolean(getChildKind(node.node_kind));
  return (
    <div>
      <div style={selectedId === node.id ? styles.treeNodeActive : styles.treeNode}>
        <div style={styles.treeHeader}>
          {node.children.length > 0 ? (
            <button style={styles.toggle} aria-label={`${isExpanded ? "Collapse" : "Expand"} ${node.name}`} onClick={() => onToggle(node.id)}>{isExpanded ? "-" : "+"}</button>
          ) : (
            <div style={styles.togglePlaceholder} />
          )}
          <div onClick={() => onSelect(node.id)} style={{ cursor: "pointer", minWidth: 0 }}>
            <div style={styles.treeTitle}>{node.name}</div>
            <div style={styles.treeMeta}>
              {strategyKindLabels[node.node_kind]}{node.owner_label ? ` / ${node.owner_label}` : ""} / {productCount} products
            </div>
          </div>
          <div style={styles.treeActions}>
            {canAddChild ? <button style={styles.ghostButton} aria-label={`Add child to ${node.name}`} onClick={() => onAddChild(node)}>Add</button> : null}
            <button style={styles.ghostButton} aria-label={`Edit ${node.name}`} onClick={() => onEdit(node)}>Edit</button>
            <button style={styles.dangerButton} aria-label={`Delete ${node.name}`} onClick={() => onDelete(node)}>Delete</button>
          </div>
        </div>
      </div>
      {isExpanded && node.children.length > 0 ? (
        <div style={styles.treeChildren}>
          {node.children.map((child) => (
            <StrategyNodeAccordion key={child.id} {...props} node={child} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function StrategyNodeForm(props: {
  form: StrategyFormState;
  nodes: StrategyNode[];
  editingNodeId: string | null;
  onChange: (form: StrategyFormState) => void;
}) {
  const { form, nodes, editingNodeId, onChange } = props;
  const parentNode = nodes.find((node) => node.id === form.parentNodeId) ?? null;
  const allowedKind = parentNode ? getChildKind(parentNode.node_kind) : "strategic_product_area";
  const blockedParentIds = new Set(editingNodeId ? [editingNodeId, ...collectDescendantIds(nodes, editingNodeId)] : []);
  const parentOptions = nodes.filter((node) => !blockedParentIds.has(node.id) && Boolean(getChildKind(node.node_kind)));
  const setParent = (parentNodeId: string) => {
    const nextParent = nodes.find((node) => node.id === parentNodeId) ?? null;
    onChange({
      ...form,
      parentNodeId,
      nodeKind: nextParent ? getChildKind(nextParent.node_kind) ?? form.nodeKind : "strategic_product_area",
    });
  };
  return (
    <>
      <div style={styles.label}>Parent</div>
      <select style={styles.input} value={form.parentNodeId} onChange={(event) => setParent(event.target.value)}>
        <option value="">No parent / Strategic Product Area</option>
        {parentOptions.map((node) => <option key={node.id} value={node.id}>{strategyKindLabels[node.node_kind]} / {node.name}</option>)}
      </select>
      <div style={styles.label}>Kind</div>
      <select
        style={styles.input}
        value={form.nodeKind}
        onChange={(event) => onChange({ ...form, nodeKind: event.target.value as StrategyNodeKind })}
        disabled
      >
        <option value={allowedKind ?? form.nodeKind}>{strategyKindLabels[allowedKind ?? form.nodeKind]}</option>
      </select>
      <div style={styles.label}>Name</div>
      <input aria-label="Strategy node name" style={styles.input} value={form.name} onChange={(event) => onChange({ ...form, name: event.target.value })} />
      <div style={styles.label}>Owner / Hat</div>
      <input aria-label="Owner or hat" style={styles.input} value={form.ownerLabel} onChange={(event) => onChange({ ...form, ownerLabel: event.target.value })} placeholder="CEO, Head of Devices, Founder" />
      <div style={styles.label}>Description</div>
      <textarea aria-label="Strategy node description" style={styles.textarea} value={form.description} onChange={(event) => onChange({ ...form, description: event.target.value })} />
    </>
  );
}

export function DependencyRow({ dependency, productLabel }: { dependency: ProductDependency; productLabel: (productId: string) => string }) {
  return (
    <div style={{ borderTop: "1px solid #d9e0ea", paddingTop: 8, marginTop: 8 }}>
      <div style={styles.productTitle}>{productLabel(dependency.product_id)}</div>
      <div style={styles.productText}>depends on {productLabel(dependency.depends_on_product_id)}</div>
      <div style={styles.badgeRow}>
        <span style={styles.badge}>{dependency.dependency_kind}</span>
        <span style={styles.badgeGreen}>{dependency.status}</span>
      </div>
      {dependency.description ? <div style={styles.productText}>{dependency.description}</div> : null}
    </div>
  );
}

export function ModalShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div style={styles.modalBackdrop} onClick={onClose}>
      <div style={styles.modal} onClick={(event) => event.stopPropagation()}>
        <div style={styles.modalHeader}>
          <div style={styles.modalTitle}>{title}</div>
          <button style={styles.ghostButton} onClick={onClose}>Close</button>
        </div>
        <div style={styles.modalBody}>{children}</div>
      </div>
    </div>
  );
}
