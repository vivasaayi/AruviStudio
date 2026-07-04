import type { ProductOverviewTocItem } from "./productOverview";

export type BookExportTrimPresetId = "trade-6x9" | "a5" | "us-letter";
export type BookExportRenderMode = "web" | "print" | "epub";

export type BookExportTrimPreset = {
  id: BookExportTrimPresetId;
  label: string;
  description: string;
  pageWidth: string;
  pageHeight: string;
  contentWidth: string;
  marginTop: string;
  marginRight: string;
  marginBottom: string;
  marginLeft: string;
};

export type ProductOverviewBookOptions = {
  trimPreset?: BookExportTrimPreset | BookExportTrimPresetId;
  renderMode?: BookExportRenderMode;
  includeFrontMatter?: boolean;
  includeBackMatter?: boolean;
};

export type ProductOverviewBookBundle = {
  html: string;
  tocItems: ProductOverviewTocItem[];
  trimPreset: BookExportTrimPreset;
};

export const BOOK_EXPORT_TRIM_PRESETS: BookExportTrimPreset[] = [
  {
    id: "trade-6x9",
    label: "Trade Paperback (6×9)",
    description: "KDP-friendly trade paperback trim with balanced margins.",
    pageWidth: "6in",
    pageHeight: "9in",
    contentWidth: "5.8in",
    marginTop: "0.75in",
    marginRight: "0.72in",
    marginBottom: "0.85in",
    marginLeft: "0.82in",
  },
  {
    id: "a5",
    label: "A5 Book",
    description: "Compact technical handbook format.",
    pageWidth: "148mm",
    pageHeight: "210mm",
    contentWidth: "146mm",
    marginTop: "18mm",
    marginRight: "18mm",
    marginBottom: "22mm",
    marginLeft: "20mm",
  },
  {
    id: "us-letter",
    label: "US Letter Review",
    description: "Reviewer-friendly PDF with larger page real estate.",
    pageWidth: "8.5in",
    pageHeight: "11in",
    contentWidth: "8.25in",
    marginTop: "0.85in",
    marginRight: "0.85in",
    marginBottom: "0.95in",
    marginLeft: "0.95in",
  },
];

export const DEFAULT_TRIM_PRESET_ID: BookExportTrimPresetId = "trade-6x9";
export const BOOK_PUBLISHING_DETAILS_ID = "book-publishing-details";
export const BOOK_CONTENTS_ID = "book-contents";
export const BOOK_REFERENCE_ATLAS_ID = "book-reference-atlas";
export const BOOK_NODE_INDEX_ID = "book-node-index";
export const BOOK_EXPORT_NOTES_ID = "book-export-notes";

export function getBookExportTrimPreset(
  preset: BookExportTrimPreset | BookExportTrimPresetId | undefined,
): BookExportTrimPreset {
  if (preset && typeof preset !== "string") {
    return preset;
  }
  return BOOK_EXPORT_TRIM_PRESETS.find((candidate) => candidate.id === (preset ?? DEFAULT_TRIM_PRESET_ID))
    ?? BOOK_EXPORT_TRIM_PRESETS[0];
}
