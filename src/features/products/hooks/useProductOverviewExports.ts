import React from "react";
import {
  exportProductOverviewEpub,
  exportProductOverviewHtml,
  exportProductOverviewPdf,
  getProductTree,
} from "../../../lib/tauri";
import type { Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import {
  BOOK_EXPORT_TRIM_PRESETS,
  buildProductOverviewBookBundle,
  getBookExportTrimPreset,
  type BookExportTrimPresetId,
} from "../lib/bookExport";
import { buildProductOverviewHtml } from "../lib/productOverview";

type ProductOverviewExportArgs = {
  selectedProduct: Product | null;
  workItems: WorkItem[];
  productReferences: ProductReference[];
};

export function useProductOverviewExports({
  selectedProduct,
  workItems,
  productReferences,
}: ProductOverviewExportArgs) {
  const [exportPath, setExportPath] = React.useState<string | null>(null);
  const [exportError, setExportError] = React.useState<string | null>(null);
  const [isExporting, setIsExporting] = React.useState(false);
  const [bookTrimPresetId, setBookTrimPresetId] = React.useState<BookExportTrimPresetId>(
    BOOK_EXPORT_TRIM_PRESETS[0].id,
  );

  const runExport = async (
    variant: "overview" | "book",
    builder: (input: {
      product: Product;
      tree?: ProductTree;
      workItems?: WorkItem[];
      references?: ProductReference[];
    }) => string,
  ) => {
    if (!selectedProduct) {
      return;
    }

    try {
      setIsExporting(true);
      setExportError(null);
      const exportTree = await getProductTree(selectedProduct.id);
      const html = builder({
        product: selectedProduct,
        tree: exportTree,
        workItems,
        references: productReferences,
      });
      const path = await exportProductOverviewHtml({
        fileName: `${slugify(selectedProduct.name)}-${variant}.html`,
        html,
      });
      setExportPath(path);
    } catch (error) {
      setExportPath(null);
      setExportError(String(error));
    } finally {
      setIsExporting(false);
    }
  };

  const exportHtml = async () => {
    await runExport("overview", buildProductOverviewHtml);
  };

  const runBookArtifactExport = async (format: "html" | "pdf" | "epub") => {
    if (!selectedProduct) {
      return;
    }

    const trimPreset = getBookExportTrimPreset(bookTrimPresetId);

    try {
      setIsExporting(true);
      setExportError(null);
      const exportTree = await getProductTree(selectedProduct.id);
      const bundle = buildProductOverviewBookBundle(
        {
          product: selectedProduct,
          tree: exportTree,
          workItems,
          references: productReferences,
        },
        {
          trimPreset,
          renderMode: format === "html" ? "web" : format === "pdf" ? "print" : "epub",
        },
      );

      let path: string;
      if (format === "html") {
        path = await exportProductOverviewHtml({
          fileName: `${slugify(selectedProduct.name)}-book.html`,
          html: bundle.html,
        });
      } else if (format === "pdf") {
        path = await exportProductOverviewPdf({
          fileName: `${slugify(selectedProduct.name)}-book.pdf`,
          html: bundle.html,
          pageWidth: trimPreset.pageWidth,
          pageHeight: trimPreset.pageHeight,
          marginTop: trimPreset.marginTop,
          marginRight: trimPreset.marginRight,
          marginBottom: trimPreset.marginBottom,
          marginLeft: trimPreset.marginLeft,
          headerTitle: selectedProduct.name,
          headerRight: trimPreset.label,
        });
      } else {
        path = await exportProductOverviewEpub({
          fileName: `${slugify(selectedProduct.name)}-book.epub`,
          title: selectedProduct.name,
          html: bundle.html,
          tocItems: bundle.tocItems,
          author: "Aruvi Studio",
          language: "en",
        });
      }

      setExportPath(path);
    } catch (error) {
      setExportPath(null);
      setExportError(String(error));
    } finally {
      setIsExporting(false);
    }
  };

  return {
    bookTrimPresetId,
    exportError,
    exportHtml,
    exportPath,
    isExporting,
    runBookArtifactExport,
    setBookTrimPresetId,
  };
}

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "product";
}
