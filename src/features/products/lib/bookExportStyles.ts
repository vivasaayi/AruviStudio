import type { ProductOverviewBookOptions, BookExportTrimPreset } from "./bookExportOptions";
import { renderBookExportContentStyles } from "./bookExportContentStyles";
import { renderBookExportLayoutStyles } from "./bookExportLayoutStyles";
import { renderBookExportPageStyles } from "./bookExportPageStyles";

type BookExportRenderMode = NonNullable<ProductOverviewBookOptions["renderMode"]>;

type BookExportStylesInput = {
  trimPreset: BookExportTrimPreset;
  renderMode: BookExportRenderMode;
};

export function renderBookExportStyles({ trimPreset, renderMode }: BookExportStylesInput) {
  return `${renderBookExportLayoutStyles({ trimPreset, renderMode })}
      ${renderBookExportContentStyles({ renderMode })}
      ${renderBookExportPageStyles()}`;
}
