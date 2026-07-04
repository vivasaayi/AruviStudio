import type { CSSProperties, Dispatch, SetStateAction } from "react";

import type { Product, ProductDependency, ProductDependencyKind } from "../../../lib/types";
import type { ProductDependencyDraft } from "../lib/productListPageState";

type CapabilityOption = {
  id: string;
  label: string;
};

type ProductDependenciesTabProps = {
  selectedProduct: Product;
  products: Product[];
  selectedProductId: string | null;
  dependencyDraft: ProductDependencyDraft;
  setDependencyDraft: Dispatch<SetStateAction<ProductDependencyDraft>>;
  selectedCapabilityOptions: CapabilityOption[];
  dependencyTargetCapabilityOptions: CapabilityOption[];
  selectedProductDependencies: ProductDependency[];
  productNameById: Map<string, string>;
  capabilityLabelById: Map<string, string>;
  isCreatingDependency: boolean;
  onCreateDependency: () => void;
  styles: Record<string, CSSProperties>;
};

const dependencyKindOptions: ProductDependencyKind[] = ["platform", "capability", "data", "integration", "operational", "other"];

export function ProductDependenciesTab({
  selectedProduct,
  products,
  selectedProductId,
  dependencyDraft,
  setDependencyDraft,
  selectedCapabilityOptions,
  dependencyTargetCapabilityOptions,
  selectedProductDependencies,
  productNameById,
  capabilityLabelById,
  isCreatingDependency,
  onCreateDependency,
  styles,
}: ProductDependenciesTabProps) {
  return (
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
              {products.filter((product) => product.id !== selectedProduct.id).map((product) => (
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
              {dependencyKindOptions.map((kind) => (
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
          onClick={onCreateDependency}
          disabled={!selectedProductId || !dependencyDraft.dependsOnProductId || isCreatingDependency}
        >
          {isCreatingDependency ? "Adding..." : "Add Dependency"}
        </button>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Captured Dependencies</div>
        {selectedProductDependencies.length > 0 ? (
          selectedProductDependencies.map((dependency) => (
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
  );
}
