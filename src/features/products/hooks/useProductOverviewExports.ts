import React from "react";
import {
  exportProductOverviewEpub,
  exportProductOverviewHtml,
  exportProductOverviewPdf,
  renderProductBook,
} from "../../../lib/tauri";
import type { Product } from "../../../lib/types";
import {
  BOOK_EXPORT_TRIM_PRESETS,
  type BookExportTrimPresetId,
} from "../lib/bookExportOptions";

type ProductOverviewExportArgs = {
  selectedProduct: Product | null;
};

export function useProductOverviewExports({
  selectedProduct,
}: ProductOverviewExportArgs) {
  const [exportPath, setExportPath] = React.useState<string | null>(null);
  const [exportError, setExportError] = React.useState<string | null>(null);
  const [isExporting, setIsExporting] = React.useState(false);
  const [bookTrimPresetId, setBookTrimPresetId] = React.useState<BookExportTrimPresetId>(
    BOOK_EXPORT_TRIM_PRESETS[0].id,
  );

  const exportHtml = async () => {
    if (!selectedProduct) {
      return;
    }

    try {
      setIsExporting(true);
      setExportError(null);
      const rendered = await renderProductBook(selectedProduct.id, "docs", "web", bookTrimPresetId);
      const path = await exportProductOverviewHtml({
        fileName: `${slugify(rendered.productName)}-overview.html`,
        html: rendered.html,
      });
      setExportPath(path);
    } catch (error) {
      setExportPath(null);
      setExportError(String(error));
    } finally {
      setIsExporting(false);
    }
  };

  const runBookArtifactExport = async (format: "html" | "pdf" | "epub") => {
    if (!selectedProduct) {
      return;
    }

    try {
      setIsExporting(true);
      setExportError(null);
      const bundle = await renderProductBook(selectedProduct.id, "book", format === "html" ? "web" : format === "pdf" ? "print" : "epub", bookTrimPresetId);
      if (!bundle.trimPreset || !bundle.tocItems) throw new Error("The product book renderer returned incomplete output");
      const productName = bundle.productName;
      const trimPreset = bundle.trimPreset;

      let path: string;
      if (format === "html") {
        path = await exportProductOverviewHtml({
          fileName: `${slugify(productName)}-book.html`,
          html: bundle.html,
        });
      } else if (format === "pdf") {
        path = await exportProductOverviewPdf({
          fileName: `${slugify(productName)}-book.pdf`,
          html: bundle.html,
          pageWidth: trimPreset.pageWidth,
          pageHeight: trimPreset.pageHeight,
          marginTop: trimPreset.marginTop,
          marginRight: trimPreset.marginRight,
          marginBottom: trimPreset.marginBottom,
          marginLeft: trimPreset.marginLeft,
          headerTitle: productName,
          headerRight: trimPreset.label,
        });
      } else {
        path = await exportProductOverviewEpub({
          fileName: `${slugify(productName)}-book.epub`,
          title: productName,
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
