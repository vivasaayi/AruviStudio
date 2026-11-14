import { buildProductOverviewBookBundle, type BookExportRenderMode, type BookExportTrimPresetId } from "./bookExport";
import { buildProductOverviewHtml } from "./productOverview";
import type { ProductReference, ProductTree, WorkItem } from "../../../lib/types";

type RenderRequest = {
  source: { tree: ProductTree; work_items: WorkItem[]; references: ProductReference[] };
  format: "docs" | "book";
  renderMode: BookExportRenderMode;
  trimPreset: BookExportTrimPresetId;
};

// This entry point is bundled into the Rust service. It has no filesystem,
// network or DOM access and accepts only the server's product snapshot.
export function renderJson(requestJson: string): string {
  const request = JSON.parse(requestJson) as RenderRequest;
  const { tree, work_items: workItems, references } = request.source;
  const input = { product: tree.product, tree, workItems, references };
  if (request.format === "docs") return JSON.stringify({ html: buildProductOverviewHtml(input) });
  if (request.format === "book") return JSON.stringify(buildProductOverviewBookBundle(input, {
    renderMode: request.renderMode,
    trimPreset: request.trimPreset,
  }));
  throw new Error("Unsupported product book format");
}
