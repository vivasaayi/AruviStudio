import { describe, expect, it } from "vitest";
import { renderReferenceUri } from "./bookExportReferences";

describe("book reference links", () => {
  it("keeps repository-relative references visible without dangling EPUB links", () => {
    expect(renderReferenceUri("docs/architecture.md", "epub")).toBe('<span class="reference-uri">docs/architecture.md</span>');
    expect(renderReferenceUri("docs/architecture.md", "web")).toContain('href="docs/architecture.md"');
    expect(renderReferenceUri("https://example.org/docs", "epub")).toContain('href="https://example.org/docs"');
  });

  it("does not turn unsafe or malformed URLs into links", () => {
    for (const uri of ["javascript:alert(1)", "data:text/html,evil", "//example.org/a", "https://example.org/a\n<script>"]) {
      const rendered = renderReferenceUri(uri, "web");
      expect(rendered).not.toContain("href=");
      expect(rendered).not.toContain("<script>");
    }
  });

  it("replaces characters forbidden in XHTML", () => {
    expect(renderReferenceUri("bad\u0000path", "epub")).toBe('<span class="reference-uri">bad\uFFFDpath</span>');
  });
});
