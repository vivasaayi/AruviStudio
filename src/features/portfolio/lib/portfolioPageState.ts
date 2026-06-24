import type { StrategyNodeKind } from "../../../lib/types";

export type PortfolioTab = "summary" | "manage";
export type StrategyDialogMode = "closed" | "create" | "edit";
export type StrategyFormState = {
  parentNodeId: string;
  nodeKind: StrategyNodeKind;
  name: string;
  description: string;
  ownerLabel: string;
};

export const emptyStrategyForm: StrategyFormState = {
  parentNodeId: "",
  nodeKind: "strategic_product_area",
  name: "",
  description: "",
  ownerLabel: "",
};
