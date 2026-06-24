import type { PlannerAction, PlannerPlan } from "./plannerPageTypes";

export function summarizeAction(action: PlannerAction | Record<string, unknown> | null | undefined) {
  if (!action || typeof action !== "object") {
    return {
      symbol: "?",
      tone: "warn" as const,
      title: "Unknown planner action",
      detail: "The planner returned an empty or invalid action payload.",
    };
  }
  const raw = action as Record<string, unknown>;
  const actionType = typeof (action as { type?: unknown }).type === "string"
    ? String((action as { type: string }).type)
    : "unknown_action";
  const target = raw.target as { productName?: string; productAreaName?: string; capabilityName?: string; workItemTitle?: string } | undefined;
  const name = typeof raw.name === "string" ? raw.name : undefined;
  const title = typeof raw.title === "string" ? raw.title : undefined;
  const description = typeof raw.description === "string" ? raw.description : undefined;
  const vision = typeof raw.vision === "string" ? raw.vision : undefined;
  const fields = raw.fields ?? undefined;
  switch (actionType) {
    case "create_product_area":
      return { symbol: "+", tone: "add", title: `Create product area ${name ?? target?.productAreaName ?? "unnamed product area"}`, detail: target?.productName ? `Product: ${target.productName}` : "Attach to selected product." };
    case "create_capability":
      return { symbol: "+", tone: "add", title: `Create capability ${name ?? target?.capabilityName ?? "unnamed capability"}`, detail: [target?.productName, target?.productAreaName].filter(Boolean).join(" / ") || "Attach to selected scope." };
    case "apply_capability_template":
      return {
        symbol: "+",
        tone: "add",
        title: `Apply template ${String(raw.templateKind ?? "chapter")} to ${name ?? "unnamed topic"}`,
        detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "Create a product-design scaffold.",
      };
    case "convert_capability_kind":
      return {
        symbol: "~",
        tone: "update",
        title: `Convert capability ${target?.capabilityName ?? ""}`.trim(),
        detail: `nodeKind=${String(raw.nodeKind ?? "unknown")} childStrategy=${String(raw.childStrategy ?? "reject")}`,
      };
    case "create_work_item":
      return { symbol: "+", tone: "add", title: `Create story/task ${title ?? target?.workItemTitle ?? "untitled work item"}`, detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "New delivery work proposal." };
    case "update_product":
      return { symbol: "~", tone: "update", title: `Update product ${target?.productName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_product_area":
      return { symbol: "~", tone: "update", title: `Update product area ${target?.productAreaName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_capability":
      return { symbol: "~", tone: "update", title: `Update capability ${target?.capabilityName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_work_item":
      return { symbol: "~", tone: "update", title: `Update story/task ${target?.workItemTitle ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "approve_work_item":
    case "approve_work_item_plan":
    case "approve_work_item_test_review":
    case "start_workflow":
    case "workflow_action":
    case "reject_work_item":
    case "reject_work_item_plan":
    case "archive_product":
    case "delete_product_area":
    case "delete_capability":
    case "delete_work_item":
      return { symbol: "!", tone: "warn", title: actionType.replace(/_/g, " "), detail: JSON.stringify(action, null, 2) };
    case "report_status":
      return { symbol: "i", tone: "update", title: "Status report", detail: target?.productName || target?.workItemTitle || "Current scope" };
    case "report_tree":
      return { symbol: "i", tone: "update", title: "Tree report", detail: target?.productName || "All products" };
    default:
      return {
        symbol: "?",
        tone: "warn",
        title: actionType.replace(/_/g, " "),
        detail: JSON.stringify(action, null, 2),
      };
  }
}

export function getReportTreeProductName(plan: PlannerPlan) {
  const treeAction = plan.actions.find((action): action is Extract<PlannerAction, { type: "report_tree" }> => action.type === "report_tree");
  return treeAction?.target?.productName;
}
