import { useState, type Dispatch, type SetStateAction } from "react";
import type { QueryClient } from "@tanstack/react-query";

import {
  exportProductOverviewHtml,
  getProductTree,
} from "../../../lib/tauri";
import type { Product, ProductTree, WorkItem } from "../../../lib/types";
import {
  buildDesignReviewPacketHtml,
  makeId,
  slugifyPacketName,
  type PlannerMessage,
  type PlannerPlan,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import type { DraftValidationSummary } from "../lib/plannerDraftTree";

type PlannerDesignPacketExportInput = {
  queryClient: QueryClient;
  selectedProductId: string | null;
  productTrees: ProductTree[];
  activeProductName: string | null;
  products: Product[];
  workItems: WorkItem[];
  plannerWorkItemsHasMore: boolean;
  draftTreeNodes: PlannerTreeNode[];
  latestDraftPlan: PlannerPlan | null;
  draftValidation: DraftValidationSummary;
  selectedDraftNode: PlannerTreeNode | null;
  latestAssistantMessage: PlannerMessage | null;
  onAppendMessage: Dispatch<SetStateAction<PlannerMessage[]>>;
};

export function usePlannerDesignPacketExport({
  queryClient,
  selectedProductId,
  productTrees,
  activeProductName,
  products,
  workItems,
  plannerWorkItemsHasMore,
  draftTreeNodes,
  latestDraftPlan,
  draftValidation,
  selectedDraftNode,
  latestAssistantMessage,
  onAppendMessage,
}: PlannerDesignPacketExportInput) {
  const [designPacketPath, setDesignPacketPath] = useState<string | null>(null);
  const [designPacketError, setDesignPacketError] = useState<string | null>(null);
  const [isExportingDesignPacket, setIsExportingDesignPacket] = useState(false);

  const exportDesignReviewPacket = async () => {
    if (isExportingDesignPacket) {
      return;
    }
    const packetRootName = draftTreeNodes[0]?.label ?? activeProductName ?? "Design Review Packet";
    try {
      setIsExportingDesignPacket(true);
      setDesignPacketError(null);
      const exportProductTrees = selectedProductId
        ? [
            await queryClient.fetchQuery({
              queryKey: ["plannerProductTree", selectedProductId],
              queryFn: () => getProductTree(selectedProductId),
            }),
          ]
        : productTrees;
      const html = buildDesignReviewPacketHtml({
        title: packetRootName,
        generatedAt: new Date().toLocaleString(),
        activeProductName,
        currentProducts: products,
        currentProductTrees: exportProductTrees,
        currentWorkItems: workItems,
        currentWorkItemsHasMore: plannerWorkItemsHasMore,
        draftTreeNodes,
        plan: latestDraftPlan,
        validation: draftValidation,
        selectedNode: selectedDraftNode,
        latestAssistantText: latestAssistantMessage?.content ?? null,
      });
      const path = await exportProductOverviewHtml({
        fileName: `${slugifyPacketName(packetRootName)}-design-review-packet.html`,
        html,
      });
      setDesignPacketPath(path);
      onAppendMessage((current) => [
        ...current,
        {
          id: makeId(),
          role: "assistant",
          content: `Generated a design review packet for "${packetRootName}".\n${path}`,
          meta: "Design packet exported",
          kind: "text",
        },
      ]);
    } catch (error) {
      setDesignPacketPath(null);
      setDesignPacketError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsExportingDesignPacket(false);
    }
  };

  return {
    designPacketPath,
    designPacketError,
    isExportingDesignPacket,
    exportDesignReviewPacket,
  };
}
