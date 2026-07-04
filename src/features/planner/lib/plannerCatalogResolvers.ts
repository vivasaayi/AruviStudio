import type {
  CapabilityTree,
  Product,
  ProductArea,
  ProductTree,
  WorkItem,
} from "../../../lib/types";
import { normalize } from "./plannerPageCore";
import type { PlannerTreeNode, ResolverContext } from "./plannerPageTypes";

export function findProduct(context: ResolverContext, productName?: string) {
  if (productName) {
    const normalized = normalize(productName);
    const exact = context.products.find((product) => normalize(product.name) === normalized);
    if (exact) {
      return exact;
    }
    const partial = context.products.filter((product) => normalize(product.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0];
    }
    if (partial.length > 1) {
      throw new Error(`Multiple products match "${productName}".`);
    }
    throw new Error(`No product matches "${productName}".`);
  }
  if (context.activeProductId) {
    const active = context.products.find((product) => product.id === context.activeProductId);
    if (active) {
      return active;
    }
  }
  if (context.products.length === 1) {
    return context.products[0];
  }
  throw new Error("Product is required.");
}

export function buildProductAreaOnlyTree(product: Product, productAreas: ProductArea[]): ProductTree {
  const sortedProductAreas = [...productAreas].sort((left, right) => left.sort_order - right.sort_order || left.name.localeCompare(right.name));
  return {
    product,
    product_areas: sortedProductAreas.map((product_area) => ({
      product_area,
      features: [],
    })),
    roots: sortedProductAreas.map((product_area) => ({
      id: product_area.id,
      node_type: "product_area",
      node_kind: product_area.node_kind,
      product_area_id: product_area.id,
      capability_id: null,
      parent_node_id: null,
      parent_node_type: null,
      depth: 0,
      name: product_area.name,
      description: product_area.description,
      summary: product_area.purpose,
      path: [product_area.name],
      allowed_child_kinds: ["capability"],
      children: [],
    })),
  };
}

export function findTree(context: ResolverContext, product: Product) {
  const tree = context.productTrees.find((entry) => entry.product.id === product.id);
  if (!tree) {
    throw new Error(`Product tree for "${product.name}" is not loaded.`);
  }
  return tree;
}

export function findProductArea(context: ResolverContext, product: Product, productAreaName?: string) {
  const tree = findTree(context, product);
  if (productAreaName) {
    const normalized = normalize(productAreaName);
    const exact = tree.product_areas.find((entry) => normalize(entry.product_area.name) === normalized);
    if (exact) {
      return exact.product_area;
    }
    const partial = tree.product_areas.filter((entry) => normalize(entry.product_area.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0].product_area;
    }
    if (partial.length > 1) {
      throw new Error(`Multiple product_areas match "${productAreaName}" in "${product.name}".`);
    }
    throw new Error(`No product_area matches "${productAreaName}" in "${product.name}".`);
  }
  if (context.activeProductAreaId) {
    const active = tree.product_areas.find((entry) => entry.product_area.id === context.activeProductAreaId);
    if (active) {
      return active.product_area;
    }
  }
  if (tree.product_areas.length === 1) {
    return tree.product_areas[0].product_area;
  }
  throw new Error("Product Area is required.");
}

export function flattenCapabilities(tree: CapabilityTree[], bucket: CapabilityTree[] = []) {
  tree.forEach((node) => {
    bucket.push(node);
    flattenCapabilities(node.children, bucket);
  });
  return bucket;
}

export function findCapability(context: ResolverContext, product: Product, productAreaName?: string, capabilityName?: string) {
  const product_area = findProductArea(context, product, productAreaName);
  const tree = findTree(context, product);
  const productAreaTree = tree.product_areas.find((entry) => entry.product_area.id === product_area.id);
  if (!productAreaTree) {
    throw new Error(`Product Area "${product_area.name}" has no capability tree.`);
  }
  const capabilities = flattenCapabilities(productAreaTree.features);
  if (capabilityName) {
    const normalized = normalize(capabilityName);
    const exact = capabilities.find((entry) => normalize(entry.capability.name) === normalized);
    if (exact) {
      return exact.capability;
    }
    const partial = capabilities.filter((entry) => normalize(entry.capability.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0].capability;
    }
    if (partial.length > 1) {
      throw new Error(`Multiple capabilities match "${capabilityName}" in "${product_area.name}".`);
    }
    throw new Error(`No capability matches "${capabilityName}" in "${product_area.name}".`);
  }
  if (context.activeCapabilityId) {
    const active = capabilities.find((entry) => entry.capability.id === context.activeCapabilityId);
    if (active) {
      return active.capability;
    }
  }
  throw new Error("Capability is required.");
}

export function findWorkItem(context: ResolverContext, workItemTitle?: string, productName?: string) {
  const inScope = productName
    ? context.workItems.filter((item) => {
        const product = context.products.find((entry) => entry.id === item.product_id);
        return product && normalize(product.name) === normalize(productName);
      })
    : context.workItems;
  if (workItemTitle) {
    const normalized = normalize(workItemTitle);
    const exact = inScope.find((item) => normalize(item.title) === normalized);
    if (exact) {
      return exact;
    }
    const partial = inScope.filter((item) => normalize(item.title).includes(normalized));
    if (partial.length === 1) {
      return partial[0];
    }
    if (partial.length > 1) {
      throw new Error(`Multiple work items match "${workItemTitle}".`);
    }
    throw new Error(`No work item matches "${workItemTitle}".`);
  }
  if (context.activeWorkItemId) {
    const active = context.workItems.find((item) => item.id === context.activeWorkItemId);
    if (active) {
      return active;
    }
  }
  throw new Error("Work item is required.");
}

export function formatArrayField(values?: string[]) {
  return values?.join(", ") ?? "";
}

export function formatWorkItemLine(workItem: WorkItem, indent: string) {
  return `${indent}- ${workItem.title} [${workItem.status}]`;
}

export function appendWorkItemHierarchy(lines: string[], items: WorkItem[], parentId: string | null, indent: string) {
  const children = items
    .filter((item) => (item.parent_work_item_id ?? null) === parentId)
    .sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title));
  children.forEach((child) => {
    lines.push(formatWorkItemLine(child, indent));
    appendWorkItemHierarchy(lines, items, child.id, `${indent}  `);
  });
}

export function buildWorkItemTreeReport(context: ResolverContext, productName?: string) {
  const lines: string[] = [];
  const products = productName ? [findProduct(context, productName)] : context.products;

  products.forEach((product) => {
    lines.push(product.name);
    const tree = context.productTrees.find((entry) => entry.product.id === product.id);
    const productItems = context.workItems.filter((item) => item.product_id === product.id);

    if (!tree) {
      appendWorkItemHierarchy(lines, productItems, null, "  ");
      lines.push("");
      return;
    }

    const includedWorkItemIds = new Set<string>();

    tree.product_areas.forEach((productAreaTree) => {
      lines.push(`  ${productAreaTree.product_area.name}`);

      const productAreaDirectItems = productItems.filter(
        (item) => item.product_area_id === productAreaTree.product_area.id && !item.capability_id,
      );
      if (productAreaDirectItems.length > 0) {
        lines.push("    direct stories/tasks");
        appendWorkItemHierarchy(lines, productAreaDirectItems, null, "      ");
        productAreaDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
      }

      const flattenedCapabilities = flattenCapabilities(productAreaTree.features);
      flattenedCapabilities.forEach((capabilityTree) => {
        const capabilityItems = productItems.filter((item) => item.capability_id === capabilityTree.capability.id);
        if (capabilityItems.length === 0) {
          return;
        }
        lines.push(`    ${capabilityTree.capability.name}`);
        appendWorkItemHierarchy(lines, capabilityItems, null, "      ");
        capabilityItems.forEach((item) => includedWorkItemIds.add(item.id));
      });
    });

    const unscopedItems = productItems.filter(
      (item) => !includedWorkItemIds.has(item.id) && !item.parent_work_item_id,
    );
    if (unscopedItems.length > 0) {
      lines.push("  unscoped");
      appendWorkItemHierarchy(lines, unscopedItems, null, "    ");
      unscopedItems.forEach((item) => includedWorkItemIds.add(item.id));
    }

    if (productItems.length === 0) {
      lines.push("  no stories/tasks");
    }
    lines.push("");
  });

  return lines.join("\n").trim();
}

export function buildWorkItemTreeNodes(context: ResolverContext, productName?: string): PlannerTreeNode[] {
  const products = productName ? [findProduct(context, productName)] : context.products;

  const buildWorkItemNodes = (items: WorkItem[], parentId: string | null): PlannerTreeNode[] =>
    items
      .filter((item) => (item.parent_work_item_id ?? null) === parentId)
      .sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title))
      .map((item) => ({
        id: item.id,
        label: item.title,
        meta: item.status,
        children: buildWorkItemNodes(items, item.id),
      }));

  return products.map((product) => {
    const tree = context.productTrees.find((entry) => entry.product.id === product.id);
    const productItems = context.workItems.filter((item) => item.product_id === product.id);
    const includedWorkItemIds = new Set<string>();
    const productAreaNodes: PlannerTreeNode[] = [];

    if (tree) {
      tree.product_areas.forEach((productAreaTree) => {
        const productAreaChildren: PlannerTreeNode[] = [];
        const productAreaDirectItems = productItems.filter(
          (item) => item.product_area_id === productAreaTree.product_area.id && !item.capability_id,
        );
        if (productAreaDirectItems.length > 0) {
          productAreaChildren.push({
            id: `${productAreaTree.product_area.id}-direct`,
            label: "Direct Delivery Items",
            children: buildWorkItemNodes(productAreaDirectItems, null),
          });
          productAreaDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
        }

        flattenCapabilities(productAreaTree.features).forEach((capabilityTree) => {
          const capabilityItems = productItems.filter((item) => item.capability_id === capabilityTree.capability.id);
          if (capabilityItems.length === 0) {
            return;
          }
          productAreaChildren.push({
            id: capabilityTree.capability.id,
            label: capabilityTree.capability.name,
            children: buildWorkItemNodes(capabilityItems, null),
          });
          capabilityItems.forEach((item) => includedWorkItemIds.add(item.id));
        });

        productAreaNodes.push({
          id: productAreaTree.product_area.id,
          label: productAreaTree.product_area.name,
          children: productAreaChildren,
        });
      });
    }

    const unscopedItems = productItems.filter(
      (item) => !includedWorkItemIds.has(item.id) && !item.parent_work_item_id,
    );
    if (unscopedItems.length > 0) {
      productAreaNodes.push({
        id: `${product.id}-unscoped`,
        label: "Unscoped",
        children: buildWorkItemNodes(unscopedItems, null),
      });
    }

    if (productAreaNodes.length === 0) {
      productAreaNodes.push({
        id: `${product.id}-empty`,
        label: "No stories/tasks",
        meta: "empty",
        children: [],
      });
    }

    return {
      id: product.id,
      label: product.name,
      children: productAreaNodes,
    };
  });
}
