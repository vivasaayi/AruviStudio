import React from "react";
import { createRoot } from "react-dom/client";
import { WorkItemArtifactModal } from "../../src/features/work-items/components/WorkItemArtifactModal";
import type { Artifact } from "../../src/lib/types";

let version = "v1";
(window as unknown as { changeArtifactVersion: () => void }).changeArtifactVersion = () => { version = "v2"; };
window.__ARUVI_E2E__ = {
  invoke: (async (command: string, args?: Record<string, unknown>) => {
    if (command !== "read_artifact_page") throw new Error("Unexpected command");
    const offset = Number(args?.offset ?? 0);
    if (args?.version && args.version !== version) throw new Error("Artifact changed; restart reading from the first page");
    return { content: offset === 0 ? "first page" : "second page", offset, next_offset: offset === 0 ? 10 : null, total_bytes: 21, version };
  }) as NonNullable<NonNullable<Window["__ARUVI_E2E__"]>["invoke"]>,
};
const artifact = { id: "fixture", artifact_type: "coding_output", storage_path: "/artifacts/large.txt", summary: "Large fixture" } as Artifact;
createRoot(document.getElementById("root")!).render(<WorkItemArtifactModal artifact={artifact} onClose={() => {}} />);
