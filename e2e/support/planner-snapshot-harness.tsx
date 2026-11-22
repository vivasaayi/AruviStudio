import React from "react";
import { createRoot } from "react-dom/client";
import { QueryClient } from "@tanstack/react-query";
import { usePlannerDesignPacketExport } from "../../src/features/planner/hooks/usePlannerDesignPacketExport";
import type { DraftValidationSummary } from "../../src/features/planner/lib/plannerDraftTree";

export function mountPlannerSnapshotHarness() {
  const mount = document.createElement("div");
  document.body.appendChild(mount);
  function Harness() {
    const packet = usePlannerDesignPacketExport({
      queryClient: new QueryClient(), sessionId: null, selectedProductId: null, productTrees: [],
      activeProductName: "<Inventory>", products: [], workItems: [], plannerWorkItemsHasMore: false,
      draftTreeNodes: [], latestDraftPlan: null, draftValidation: { score: 0, issues: [] } as DraftValidationSummary,
      selectedDraftNode: null, latestAssistantMessage: null, onAppendMessage: () => {},
    });
    return <><button onClick={() => void packet.exportDesignReviewPacket()}>Export snapshot fixture</button>
      <span data-testid="snapshot-path">{packet.designPacketPath}</span>
      <span data-testid="snapshot-error">{packet.designPacketError}</span></>;
  }
  createRoot(mount).render(<Harness />);
}
