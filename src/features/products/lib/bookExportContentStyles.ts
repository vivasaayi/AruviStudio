import type { ProductOverviewBookOptions } from "./bookExportOptions";

type BookExportRenderMode = NonNullable<ProductOverviewBookOptions["renderMode"]>;

type BookExportContentStylesInput = {
  renderMode: BookExportRenderMode;
};

export function renderBookExportContentStyles({ renderMode }: BookExportContentStylesInput) {
  return `
      .title-page {
        min-height: ${renderMode === "web" ? "86vh" : "calc(var(--page-height) - var(--page-margin-top) - var(--page-margin-bottom))"};
        display: flex;
        flex-direction: column;
        justify-content: center;
      }

      .kicker,
      .toc-title,
      .section-kicker,
      .chapter-kicker,
      .meta-label,
      .note-label {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.14em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .kicker { letter-spacing: 0.18em; font-size: 12px; }

      h1, h2, h3, h4 {
        margin: 0;
        font-weight: 700;
        color: var(--chapter);
      }

      h1 {
        margin-top: 18px;
        font-size: clamp(42px, 6vw, 64px);
        line-height: 0.96;
        letter-spacing: -0.03em;
      }

      .deck {
        margin-top: 18px;
        max-width: 620px;
        font-size: 23px;
        line-height: 1.6;
        color: var(--muted);
      }

      .book-meta {
        margin-top: 28px;
        padding-top: 20px;
        border-top: 1px solid var(--rule);
        display: flex;
        flex-wrap: wrap;
        gap: 22px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        color: var(--muted);
      }

      .meta-item strong {
        display: block;
        margin-bottom: 4px;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--chapter);
      }

      .front-grid,
      .back-grid,
      .section-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 18px;
        margin-top: 24px;
      }

      .panel {
        padding: 16px 18px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        background: rgba(255,255,255,0.55);
      }

      .panel h3,
      .panel h4 {
        font-size: 14px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin-bottom: 10px;
      }

      .goal-list {
        margin: 0;
        padding-left: 18px;
      }

      .goal-list li,
      .book-list li {
        margin-top: 10px;
        line-height: 1.7;
      }

      .body-copy,
      .lead,
      .note-copy,
      .work-copy,
      .index-copy {
        line-height: 1.85;
        color: var(--ink);
      }

      .lead {
        font-size: 19px;
        color: var(--muted);
      }

      .body-copy { font-size: 17px; }

      .toc-list {
        display: flex;
        flex-direction: column;
        gap: 18px;
        margin-top: 24px;
      }

      .toc-group {
        border-top: 1px solid var(--rule);
        padding-top: 14px;
      }

      .toc-product_area,
      .toc-capability {
        display: flex;
        justify-content: space-between;
        gap: 18px;
        align-items: baseline;
      }

      .toc-product_area {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 18px;
        font-weight: 700;
      }

      .toc-capability {
        font-size: 14px;
        line-height: 1.45;
        color: var(--muted);
      }

      .toc-product_area a,
      .toc-capability a,
      .inline-link {
        color: inherit;
        text-decoration: none;
      }

      .toc-product_area a:hover,
      .toc-capability a:hover,
      .inline-link:hover {
        color: var(--accent);
      }

      .toc-children {
        margin-top: 10px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-left: 14px;
        border-left: 1px solid rgba(120, 96, 72, 0.18);
      }

      .chapter-title {
        margin-top: 10px;
        font-size: 34px;
        line-height: 1.08;
      }

      .chapter-intro {
        margin-top: 12px;
        font-size: 19px;
        line-height: 1.75;
        color: var(--muted);
      }

      .chapter-stats,
      .capability-meta {
        margin-top: 18px;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .stat-pill,
      .meta-chip {
        display: inline-flex;
        align-items: center;
        border-radius: 999px;
        padding: 6px 10px;
        background: var(--accent-soft);
        color: var(--chapter);
        font-size: 12px;
        font-weight: 700;
      }

      .capability {
        margin-top: 32px;
        padding-top: 20px;
        border-top: 1px solid rgba(120, 96, 72, 0.18);
      }

      .capability-title {
        margin-top: 8px;
        font-size: 26px;
        line-height: 1.18;
      }

      .section-block {
        margin-top: 24px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
      }

      .note-block {
        margin-top: 18px;
        padding-left: 16px;
        border-left: 3px solid var(--rule);
      }

      .note-copy {
        margin-top: 8px;
        font-size: 15px;
        color: var(--muted);
      }

      .book-paragraph {
        margin: 0 0 14px;
        line-height: 1.8;
      }

      .book-list {
        margin: 0;
        padding-left: 20px;
      }

      .book-inline-code {
        font-family: "SFMono-Regular", "SFMono", "Cascadia Code", Menlo, Consolas, monospace;
        font-size: 0.9em;
        background: rgba(46, 61, 82, 0.08);
        border-radius: 4px;
        padding: 0.08rem 0.34rem;
      }

      .book-code,
      .book-figure,
      .book-table-wrap,
      .book-quote {
        margin: 18px 0;
      }

      .book-code pre {
        margin: 0;
        padding: 16px 18px;
        overflow-x: auto;
        background: #1d2430;
        color: #eef4ff;
        border-radius: 12px;
        font-size: 13px;
        line-height: 1.65;
      }

      .book-figure img {
        max-width: 100%;
        border-radius: 10px;
        border: 1px solid rgba(120, 96, 72, 0.18);
      }

      .book-figure figcaption,
      .book-code figcaption,
      .book-table-wrap figcaption {
        margin-top: 8px;
        color: var(--muted);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .book-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 14px;
        background: rgba(255,255,255,0.62);
      }

      .book-table th,
      .book-table td {
        padding: 10px 12px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        text-align: left;
        vertical-align: top;
      }

      .book-table th {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        background: rgba(240, 225, 212, 0.55);
      }

      .book-quote {
        padding: 12px 16px;
        border-left: 3px solid var(--accent);
        background: rgba(240, 225, 212, 0.32);
        color: var(--muted);
      }

      .work-list {
        margin-top: 18px;
        display: flex;
        flex-direction: column;
        gap: 12px;
      }

      .work-item {
        padding: 14px 16px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        background: rgba(255,255,255,0.55);
      }

      .work-head {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        align-items: baseline;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .work-title {
        font-size: 15px;
        font-weight: 700;
        color: var(--chapter);
      }

      .work-status {
        font-size: 11px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.06em;
      }

      .work-status.is-done { color: var(--done); }
      .work-status.is-wip { color: var(--wip); }
      .work-status.is-tbd { color: var(--tbd); }
      .work-status.is-blocked { color: var(--blocked); }

      .work-copy {
        margin-top: 6px;
        font-size: 14px;
        color: var(--muted);
      }

      .child-work {
        margin-top: 10px;
        padding-left: 14px;
        border-left: 2px solid rgba(120, 96, 72, 0.16);
      }

      .reference-list,
      .index-list {
        display: flex;
        flex-direction: column;
        gap: 14px;
        margin-top: 18px;
      }

      .reference-item,
      .index-item {
        padding-bottom: 14px;
        border-bottom: 1px solid rgba(120, 96, 72, 0.12);
      }

      .reference-path,
      .index-path {
        margin-top: 4px;
        color: var(--muted);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .reference-uri {
        display: block;
        margin-top: 8px;
        color: var(--accent);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        word-break: break-word;
      }

      .footer-note {
        margin-top: 34px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        color: var(--muted);
      }
  `;
}
