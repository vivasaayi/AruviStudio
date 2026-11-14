// Node types are intentionally absent from the browser tsconfig.
// @ts-expect-error This file executes only in Vitest's Node environment.
import { readFileSync } from "node:fs";
import { buildSync } from "esbuild";
import { expect, it } from "vitest";

it("keeps the embedded backend renderer in sync with the desktop source", () => {
  const result = buildSync({
    entryPoints: ["src/features/products/lib/productBookBackend.ts"],
    bundle: true,
    platform: "neutral",
    format: "iife",
    globalName: "AruviProductBook",
    target: "es2020",
    outfile: "src-tauri/resources/product_book_renderer.js",
    write: false,
  });
  expect(result.outputFiles[0].text).toBe(readFileSync("src-tauri/resources/product_book_renderer.js", "utf8"));
});
