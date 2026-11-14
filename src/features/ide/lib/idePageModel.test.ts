import { expect, it } from "vitest";
import { parsePatchProposal } from "./idePageModel";

it("keeps every valid target in a structured proposal", () => {
  const input = { type: "patch_proposal", summary: "Update", patches: [
    { path: " a.txt ", patch: "@@ -1 +1 @@\n-old\n+new" },
    { path: "b.txt", patch: "@@ -0,0 +1 @@\n+new", baseSha256: "abc" },
  ] };
  expect(parsePatchProposal(JSON.stringify(input))?.patches).toMatchObject([
    { path: "a.txt", patch: input.patches[0].patch },
    { path: "b.txt", patch: input.patches[1].patch, base_sha256: "abc" },
  ]);
});

it.each([null, {}, { path: "", patch: "change" }, { path: "b.txt", patch: "" },
  { path: "b.txt", patch: "change", base_sha256: 42 },
  { path: "b.txt", patch: "change", description: 42 }])(
  "rejects the entire proposal when one target is malformed: %j", (bad) => {
    expect(parsePatchProposal(JSON.stringify({ type: "patch_proposal", patches: [
      { path: "a.txt", patch: "change" }, bad,
    ] }))).toBeNull();
  },
);
