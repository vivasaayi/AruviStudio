# Product Book migration checkpoint

The desktop Product Book is a separate feature from the planner review packet.
The browser and MCP do not yet have equivalent Product Book exports. Do not
remove the desktop renderer or claim export parity until these steps pass.

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
- Native exports currently render HTML in TypeScript and use Rust commands to
  write files or produce PDF/EPUB. Browser downloads and MCP access need an
  appropriate server-side artifact interface, without requiring a host path.

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
  This fixture does not cover embedded or remote images; the renderer currently
  emits image sources without packaging them, so those remain an open parity and
  EPUB validity issue. Passing EPUBCheck does not prove reader compatibility.
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
  Desktop, browser downloads and MCP EPUB/PDF artifacts do not yet use this
  renderer, and image-resource packaging and visual comparison remain open.
  For a local worktree, request a rendered book with:

  ```sh
  scripts/mcp call aruvi_catalog '{"action":"render_product_book","arguments":{"product_id":"PRODUCT_ID","format":"book","render_mode":"epub","trim_preset":"a5"}}'
  ```
- [ ] Provide authenticated browser and MCP export interfaces for each supported
  artifact. Implement EPUB and PDF with actual runtime dependencies or document
  explicit platform limitations; do not return an incomplete artifact as success.
- [ ] Move desktop exports to the same backend renderer and retain native file
  save/reveal behavior. Remove the old renderer only after parity is demonstrated.
- [ ] Validate multi-product isolation, nested stories/tasks, more than 2,000
  work items, empty products, malformed content, concurrent edits, unauthorized
  requests and export failures. Test all artifact formats, trim presets and the
  actual browser download, plus native desktop behavior where available.
