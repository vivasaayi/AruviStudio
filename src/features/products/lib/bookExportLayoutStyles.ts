import type { ProductOverviewBookOptions, BookExportTrimPreset } from "./bookExportOptions";

type BookExportRenderMode = NonNullable<ProductOverviewBookOptions["renderMode"]>;

type BookExportLayoutStylesInput = {
  trimPreset: BookExportTrimPreset;
  renderMode: BookExportRenderMode;
};

export function renderBookExportLayoutStyles({
  trimPreset,
  renderMode,
}: BookExportLayoutStylesInput) {
  return `      :root {
        color-scheme: light;
        --paper: #f8f4ec;
        --ink: #1f2733;
        --muted: #596574;
        --rule: #d8ccbc;
        --accent: #8e4c2d;
        --accent-soft: #f0e1d4;
        --chapter: #2f3c4d;
        --done: #2d7d57;
        --wip: #a06c00;
        --tbd: #486aa0;
        --blocked: #a44545;
        --page-width: ${trimPreset.pageWidth};
        --page-height: ${trimPreset.pageHeight};
        --content-width: ${trimPreset.contentWidth};
        --page-margin-top: ${trimPreset.marginTop};
        --page-margin-right: ${trimPreset.marginRight};
        --page-margin-bottom: ${trimPreset.marginBottom};
        --page-margin-left: ${trimPreset.marginLeft};
      }

      * { box-sizing: border-box; }
      html { scroll-behavior: smooth; }

      body {
        margin: 0;
        background: ${renderMode === "web"
          ? 'radial-gradient(circle at top, rgba(255,255,255,0.7), transparent 36%), linear-gradient(180deg, #efe8db 0%, var(--paper) 100%)'
          : "#ffffff"};
        color: var(--ink);
        font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      }

      .book-shell {
        width: 100%;
        max-width: ${renderMode === "web" ? "1680px" : "none"};
        margin: ${renderMode === "web" ? "0 auto" : "0"};
        display: ${renderMode === "web" ? "grid" : "block"};
        grid-template-columns: ${renderMode === "web" ? "320px minmax(0, 1fr)" : "1fr"};
        min-height: ${renderMode === "web" ? "100vh" : "auto"};
      }

      .book-sidebar {
        display: ${renderMode === "web" ? "block" : "none"};
        position: ${renderMode === "web" ? "sticky" : "static"};
        top: 0;
        align-self: start;
        height: 100vh;
        overflow-y: auto;
        padding: 28px 20px 28px 24px;
        border-right: 1px solid rgba(120, 96, 72, 0.12);
        background: rgba(250, 246, 239, 0.92);
        backdrop-filter: blur(10px);
      }

      .book-sidebar-inner {
        display: flex;
        flex-direction: column;
        gap: 16px;
      }

      .book-sidebar-kicker {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.14em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .book-sidebar-title {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 23px;
        font-weight: 800;
        line-height: 1.18;
        color: var(--chapter);
      }

      .book-sidebar-note {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        line-height: 1.6;
        color: var(--muted);
      }

      .book-sidebar-nav,
      .book-sidebar-node {
        display: flex;
        flex-direction: column;
      }

      .book-sidebar-nav { gap: 8px; }
      .book-sidebar-node { gap: 8px; }

      .book-sidebar-link {
        display: block;
        padding: 8px 10px;
        border-radius: 10px;
        color: var(--chapter);
        text-decoration: none;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 13px;
        font-weight: 700;
        line-height: 1.45;
      }

      .book-sidebar-link:hover {
        background: rgba(240, 225, 212, 0.58);
        color: var(--accent);
      }

      .book-sidebar-children {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-left: 14px;
        padding-left: 12px;
        border-left: 1px solid rgba(120, 96, 72, 0.18);
      }

      .book-main {
        min-width: 0;
        padding: ${renderMode === "web" ? "30px 34px 64px" : "0"};
      }

      .book {
        width: 100%;
        max-width: ${renderMode === "print" ? "var(--content-width)" : "none"};
        margin: ${renderMode === "web" ? "0" : "0 auto"};
        background: ${renderMode === "web" ? "rgba(255,255,255,0.78)" : "#ffffff"};
        border: ${renderMode === "web" ? "1px solid rgba(120, 96, 72, 0.14)" : "none"};
        box-shadow: ${renderMode === "web" ? "0 24px 60px rgba(64, 46, 31, 0.12)" : "none"};
      }

      .page {
        padding: ${renderMode === "epub" ? "2rem 1.4rem" : "58px 72px"};
        border-top: 1px solid rgba(120, 96, 72, 0.08);
        page-break-after: always;
        break-after: page;
      }

      .page:last-child {
        page-break-after: auto;
        break-after: auto;
      }

      .page:first-child { border-top: none; }`;
}
