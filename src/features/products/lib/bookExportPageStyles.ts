export function renderBookExportPageStyles() {
  return `
      @page {
        size: var(--page-width) var(--page-height);
        margin: var(--page-margin-top) var(--page-margin-right) var(--page-margin-bottom) var(--page-margin-left);
      }

      @media (max-width: 900px) {
        .book-shell {
          display: block;
          max-width: none;
        }

        .book-sidebar {
          position: static;
          height: auto;
          border-right: none;
          border-bottom: 1px solid rgba(120, 96, 72, 0.12);
        }

        .book-main {
          padding: 16px 10px 30px;
        }

        .book {
          width: 100%;
          margin: 0;
        }

        .page {
          padding: 32px 24px;
        }

        .title-page {
          min-height: auto;
        }

        .front-grid,
        .back-grid,
        .section-grid {
          grid-template-columns: 1fr;
        }
      }

      @media print {
        body {
          background: #fff;
        }

        .book-sidebar {
          display: none;
        }

        .book-main {
          padding: 0;
        }

        .book {
          width: 100%;
          max-width: none;
          margin: 0;
          box-shadow: none;
          border: none;
        }

        .page {
          padding: 0;
        }
      }
  `;
}
