import type { UseMutationResult } from "@tanstack/react-query";
import type { Dispatch, SetStateAction } from "react";
import { makeId, type PlannerMessage } from "../lib/plannerPageModel";

type PlannerComposerSubmitInput = {
  draft: string;
  isPlannerBusy: boolean;
  processMutation: UseMutationResult<unknown, Error, string>;
  selectedProductId: string | null;
  setDraft: Dispatch<SetStateAction<string>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
};

export function usePlannerComposerSubmit({
  draft,
  isPlannerBusy,
  processMutation,
  selectedProductId,
  setDraft,
  setMessages,
}: PlannerComposerSubmitInput) {
  const send = async () => {
    const content = draft.trim();
    if (!content || isPlannerBusy) {
      return;
    }
    if (!selectedProductId) {
      setMessages((current) => [
        ...current,
        {
          id: makeId(),
          role: "assistant",
          content: "Select a product before planning. Use Products to create one if needed.",
          meta: "Product required",
          kind: "error",
        },
      ]);
      return;
    }
    setDraft("");
    await processMutation.mutateAsync(content);
  };

  return {
    send,
  };
}
