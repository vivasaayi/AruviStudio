# Product Book migration checkpoint

The desktop Product Book is a separate feature from the planner review packet.
Browser and MCP exports now cover Docs HTML, Book HTML, EPUB and PDF using the
shared renderer. Full parity and release readiness remain open because the
limitations and validation items below are not complete.

## Existing behavior to preserve

- Docs HTML renders product narrative, hierarchy, scoped references, delivery
  stories and tasks, metrics, navigation and status labels.
- Book HTML, EPUB and PDF share a book renderer with chapter numbering, nested
  hierarchy, rich text, reference atlas, index, table of contents and front/back
  matter. EPUB has its own navigation and archive layout; PDF uses a browser
  print step. Trim presets are trade 6×9, A5 and US Letter.
- Product and reference data, area/capability ordering, parent/child work items,
  status and empty-section behavior must remain product-scoped and escaped.
  Long text, malformed markup and untrusted URLs require explicit handling.
- Native exports render through the shared backend and use Rust commands to
  write files or produce PDF/EPUB. Browser downloads and MCP access use a
  server-side artifact interface without requiring a host path.

## Work remaining

- [x] Correct the desktop view/export source: fetch complete product work items
  through a shared Rust query; expose it via MCP. The page loads delivery items
  on request, while exports load them automatically. The query does not cap
  results, so very large products can still consume substantial memory.
- [x] Implement a product-scoped snapshot containing the product, hierarchy,
  reachable references and all work items. One SQLite read transaction prevents
  concurrent changes from mixing versions. Desktop exports and the authenticated
  MCP catalog action use it. Database tests exercise concurrent writes, reference
  scope and more than 2,000 work items; the snapshot is still unbounded in size.
- [x] Move EPUB archive packaging into a reusable Rust service and use it for
  desktop export. This no longer invokes the system `zip` utility or writes a
  temporary source tree; archive tests inspect the first uncompressed mimetype,
  book content, navigation and escaped metadata. The service currently packages
  caller-rendered XHTML in memory; real renderer output and EPUB reader
  compatibility still need end-to-end validation.
- [x] Capture an actual desktop-rendered 501-item EPUB fixture, package it via
  the Rust service, and check it with EPUBCheck 5.4.0: zero errors or warnings.
  Relative reference paths are retained as text in EPUB rather than links to
  files absent from the archive. Reproduce with `scripts/test-product-book-epub`
  and `EPUBCHECK_DIR` pointing to the unpacked [official EPUBCheck 5.4.0
  distribution](https://github.com/w3c/epubcheck/releases/tag/v5.4.0).
  This fixture does not cover image resources. Passing EPUBCheck does not prove
  reader compatibility.
- [ ] Port the Docs HTML and Book renderers to shared backend services, including
  all rich text, reference, index, numbering, layout and trim semantics. Compare
  output against desktop fixtures and inspect representative rendered pages.
  The existing TypeScript renderer is now bundled into an isolated QuickJS
  runtime inside the Rust service. An authenticated MCP `aruvi_catalog` action
  `render_product_book` returns Docs or Book HTML; Book output also includes
  table of contents and trim metadata. Book supports web, print and EPUB modes
  and all three trim presets; Docs supports web mode.
  A 501-item fixture compares all four desktop render modes byte-for-byte apart
  from their generation timestamps (`scripts/test-product-book-renderer`).
  Bundled output is checked against source in the frontend test suite. The
  runtime limits input to 64 MiB, JavaScript heap to 256 MiB, execution to 30
  seconds, and simultaneous snapshots/renders to two. Source snapshot creation
  and serialization are currently unbounded before the input-size check.
  Desktop HTML, EPUB and PDF exports call this same backend renderer and retain
  their native file writers. Browser and MCP HTML/EPUB/PDF exports use it as well;
  image-resource packaging and broad visual comparison remain open.
  For a local worktree, request a rendered book with:

  ```sh
  scripts/mcp call aruvi_catalog '{"action":"render_product_book","arguments":{"product_id":"PRODUCT_ID","format":"book","render_mode":"epub","trim_preset":"a5"}}'
  ```
- [ ] Provide authenticated browser and MCP export interfaces for each supported
  artifact. Docs HTML, Book HTML, EPUB and PDF are available through the product
  browser and `aruvi_catalog.export_product_book`; MCP returns a filename, media
  type and base64 bytes. Artifacts over 24 MiB fail explicitly; PDF source is
  limited to 12 MiB and PDF rendering to 60 seconds. EPUB and PDF can include
  embedded PNG and JPEG data images, subject to count, dimension and byte
  limits (32 references, 4 MiB per image, 12 MiB total unique image data, and
  2048 pixels per dimension). Repository-relative paths, remote URLs, animated
  PNG and other image types fail explicitly; safe scope-aware resolution and
  broader format support remain open. HTML may reference external resources.
  The real browser/MCP fixture includes PNG, JPEG and a duplicate PNG; the
  captured EPUB passed EPUBCheck 5.4.0 and the captured PDF contained both
  image types, with its first page visually inspected. The EPUB writer used by
  desktop shares the same image packaging code.
  The export action accepts `product_id`, `format` (`docs_html`, `book_html`,
  `epub`, or `pdf`) and `trim_preset` (`trade-6x9`, `a5`, or `us-letter`). For example:

  ```sh
  scripts/mcp call aruvi_catalog '{"action":"export_product_book","arguments":{"product_id":"PRODUCT_ID","format":"epub","trim_preset":"a5"}}'
  ```
  The isolated remote suite downloads both HTML variants and EPUB in a real
  browser, inspects the EPUB contents, tests MCP export and authentication, and
  checks that another product's data is excluded. A captured browser EPUB
  passed EPUBCheck 5.4.0 with zero findings. A 2,102-item backend HTML artifact
  test covers pagination and child items. Set `ARUVI_REMOTE_EPUB_OUTPUT` when
  running `scripts/test-container-remote.sh` to retain the test EPUB.
  The same suite downloads PDF in all three trim presets and exercises MCP PDF
  export. Captured PDFs had expected page dimensions, tagged structure and
  searchable product-scoped content; an A5 chapter was visually inspected. Set
  `ARUVI_REMOTE_PDF_OUTPUT_DIR` to retain those fixture files. Native real-browser
  tests also verify that scripts and external requests do not execute.
  PDF uses Chromium and Playwright with JavaScript disabled, external requests
  blocked, an empty secret environment and a private temporary directory.
  The worktree and isolated test use a separate non-root PDF container with no
  network, a read-only root filesystem, dropped capabilities, no-new-privileges,
  memory/PID limits, and only a private Unix socket shared with the app. It has
  no app database, repositories or API tokens. Chromium's own sandbox remains
  disabled there because this Mac's container runtime denies its namespace
  operation; the container is an additional isolation boundary, not a proof of
  Chromium sandboxing. The app fails closed if the renderer is unavailable.
  Staging and production adapters have not been updated or validated for this
  sidecar and must not be treated as release-ready on this evidence.
  Other servers can use `ARUVI_PDF_SOCKET` for an isolated service, or configure
  `ARUVI_PDF_NODE`, `ARUVI_PDF_BROWSER`, and `ARUVI_PDF_PLAYWRIGHT_PATH` for a
  local worker; local `ARUVI_PDF_SANDBOX` defaults to `1`.
  Desktop PDF files still use the older Playwright writer, which relies on a
  source-checkout script and does not yet apply the backend worker's request
  blocking. Unifying native and server-side PDF generation remains open.
- [x] Move desktop exports to the shared backend renderer and retain the native
  file writers and reveal flow. Tauri IPC is exercised with a real database and
  the export UI is tested with mocked native file writes. Actual native window,
  file reveal, and real PDF/EPUB reader checks remain open; the TypeScript
  renderer remains the shared source for the embedded bundle and is not removed.
- [ ] Validate multi-product isolation, nested stories/tasks, more than 2,000
  work items, empty products, malformed content, concurrent edits, unauthorized
  requests and export failures. Test all artifact formats, trim presets and the
  actual browser download, plus native desktop behavior where available.
