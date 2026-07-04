import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  applySemanticTemplate,
  convertCapabilityKind,
  createCapability,
  createProductArea,
  createWorkItem,
  deleteCapability,
  deleteProductArea,
  deleteWorkItem,
  getLatestWorkflowRunForWorkItem,
  handleWorkflowUserAction,
  rejectWorkItem,
  rejectWorkItemPlan,
  startWorkItemWorkflow,
  updateCapability,
  updateProduct,
  updateProductArea,
  updateWorkItem,
} from "../../../lib/tauri";
import {
  buildWorkItemTreeReport,
  findCapability,
  findProduct,
  findProductArea,
  findWorkItem,
  formatArrayField,
} from "./plannerCatalogResolvers";
import type { ExecutionResult, PlannerAction, PlannerPlan, ResolverContext } from "./plannerPageTypes";

export async function executePlannerAction(action: PlannerAction, context: ResolverContext): Promise<string[]> {
  switch (action.type) {
    case "update_product": {
      const product = findProduct(context, action.target?.productName);
      const updated = await updateProduct({
        id: product.id,
        name: action.fields.name,
        description: action.fields.description,
        vision: action.fields.vision,
        goals: action.fields.goals ? formatArrayField(action.fields.goals) : undefined,
        tags: action.fields.tags ? formatArrayField(action.fields.tags) : undefined,
      });
      return [`Updated product "${updated.name}".`];
    }
    case "create_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = await createProductArea({
        productId: product.id,
        name: action.name,
        description: action.description ?? "",
        purpose: action.purpose ?? "",
        nodeKind: (action as { nodeKind?: string }).nodeKind,
        explanation: (action as { explanation?: string }).explanation,
        examples: (action as { examples?: string }).examples,
        implementationNotes: (action as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action as { testGuidance?: string }).testGuidance,
      });
      return [`Created product area "${product_area.name}" in "${product.name}".`];
    }
    case "update_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const updated = await updateProductArea({
        id: product_area.id,
        name: action.fields.name,
        description: action.fields.description,
        purpose: action.fields.purpose,
        nodeKind: (action.fields as { nodeKind?: string }).nodeKind,
        explanation: (action.fields as { explanation?: string }).explanation,
        examples: (action.fields as { examples?: string }).examples,
        implementationNotes: (action.fields as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action.fields as { testGuidance?: string }).testGuidance,
      });
      return [`Updated capability "${updated.name}" in "${product.name}".`];
    }
    case "delete_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      await deleteProductArea(product_area.id);
      return [`Deleted product area "${product_area.name}" from "${product.name}".`];
    }
    case "create_capability": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const parentCapability = action.target?.capabilityName
        ? findCapability(context, product, product_area.name, action.target.capabilityName)
        : null;
      const capability = await createCapability({
        productAreaId: product_area.id,
        parentCapabilityId: parentCapability?.id,
        name: action.name,
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        priority: action.priority ?? "medium",
        risk: action.risk ?? "medium",
        technicalNotes: action.technicalNotes ?? "",
        nodeKind: (action as { nodeKind?: string }).nodeKind,
        explanation: (action as { explanation?: string }).explanation,
        examples: (action as { examples?: string }).examples,
        implementationNotes: (action as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action as { testGuidance?: string }).testGuidance,
      });
      return [`Created capability "${capability.name}" in "${product_area.name}".`];
    }
    case "apply_capability_template": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const parentCapability = action.target?.capabilityName
        ? findCapability(context, product, product_area.name, action.target.capabilityName)
        : null;
      const result = await applySemanticTemplate({
        productAreaId: product_area.id,
        parentCapabilityId: parentCapability?.id,
        templateKind: action.templateKind,
        name: action.name,
        description: action.description,
        priority: action.priority,
        risk: action.risk,
        explanation: action.explanation,
        examples: action.examples,
        implementationNotes: action.implementationNotes,
        testGuidance: action.testGuidance,
      });
      return [`Applied template ${result.template_kind} to "${result.topic_node.name}".`];
    }
    case "convert_capability_kind": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      const result = await convertCapabilityKind({
        id: capability.id,
        nodeKind: action.nodeKind,
        childStrategy: action.childStrategy,
      });
      return [`Converted capability "${result.capability.name}" to ${result.capability.node_kind}.`];
    }
    case "update_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      const updated = await updateCapability({
        id: capability.id,
        name: action.fields.name,
        description: action.fields.description,
        acceptanceCriteria: action.fields.acceptanceCriteria,
        technicalNotes: action.fields.technicalNotes,
        priority: action.fields.priority,
        risk: action.fields.risk,
        nodeKind: (action.fields as { nodeKind?: string }).nodeKind,
        explanation: (action.fields as { explanation?: string }).explanation,
        examples: (action.fields as { examples?: string }).examples,
        implementationNotes: (action.fields as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action.fields as { testGuidance?: string }).testGuidance,
      });
      return [`Updated capability "${updated.name}".`];
    }
    case "delete_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      await deleteCapability(capability.id);
      return [`Deleted capability "${capability.name}".`];
    }
    case "create_work_item": {
      const product = findProduct(context, action.target?.productName);
      const product_area = action.target?.productAreaName ? findProductArea(context, product, action.target.productAreaName) : context.activeProductAreaId ? findProductArea(context, product, undefined) : null;
      const capability = action.target?.capabilityName ? findCapability(context, product, action.target?.productAreaName, action.target.capabilityName) : context.activeCapabilityId ? findCapability(context, product, product_area?.name, undefined) : null;
      const workItem = await createWorkItem({
        productId: product.id,
        productAreaId: product_area?.id,
        capabilityId: capability?.id,
        title: action.title,
        problemStatement: action.problemStatement ?? action.description ?? "",
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        constraints: action.constraints ?? "",
        workItemType: action.workItemType ?? "story",
        priority: action.priority ?? "medium",
        complexity: action.complexity ?? "medium",
      });
      return [`Created story/task "${workItem.title}" in "${product.name}".`];
    }
    case "update_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      const updated = await updateWorkItem({
        id: workItem.id,
        title: action.fields.title,
        description: action.fields.description,
        problemStatement: action.fields.problemStatement,
        acceptanceCriteria: action.fields.acceptanceCriteria,
        constraints: action.fields.constraints,
        status: action.fields.status,
      });
      return [`Updated story/task "${updated.title}".`];
    }
    case "delete_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await deleteWorkItem(workItem.id);
      return [`Deleted story/task "${workItem.title}".`];
    }
    case "approve_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItem(workItem.id, action.notes);
      return [`Approved story/task "${workItem.title}".`];
    }
    case "reject_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await rejectWorkItem(workItem.id, action.notes ?? "Rejected from interactive planner.");
      return [`Rejected story/task "${workItem.title}".`];
    }
    case "approve_work_item_plan": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItemPlan(workItem.id, action.notes);
      return [`Approved plan for "${workItem.title}".`];
    }
    case "reject_work_item_plan": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await rejectWorkItemPlan(workItem.id, action.notes ?? "Rejected from interactive planner.");
      return [`Rejected plan for "${workItem.title}".`];
    }
    case "approve_work_item_test_review": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItemTestReview(workItem.id, action.notes);
      return [`Approved test review for "${workItem.title}".`];
    }
    case "start_workflow": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await startWorkItemWorkflow(workItem.id);
      return [`Started workflow for "${workItem.title}".`];
    }
    case "workflow_action": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      const run = await getLatestWorkflowRunForWorkItem(workItem.id);
      if (!run) {
        throw new Error(`No workflow run exists for "${workItem.title}".`);
      }
      await handleWorkflowUserAction({
        workflowRunId: run.id,
        action: action.action,
        notes: action.notes,
      });
      return [`Applied workflow action "${action.action}" to "${workItem.title}".`];
    }
    case "report_status": {
      const workItem = action.target?.workItemTitle || context.activeWorkItemId
        ? findWorkItem(context, action.target?.workItemTitle, action.target?.productName)
        : null;
      if (workItem) {
        const run = await getLatestWorkflowRunForWorkItem(workItem.id);
        const product = context.products.find((entry) => entry.id === workItem.product_id);
        return [
          `Status for "${workItem.title}": ${workItem.status}.`,
          `Product: ${product?.name ?? "unknown"}.`,
          run ? `Workflow: ${run.status} at ${run.current_stage}.` : "Workflow: not started.",
        ];
      }
      const product = action.target?.productName ? findProduct(context, action.target.productName) : findProduct(context, undefined);
      const scopedItems = context.workItems.filter((item) => item.product_id === product.id);
      const statusCounts = scopedItems.reduce<Record<string, number>>((acc, item) => {
        acc[item.status] = (acc[item.status] ?? 0) + 1;
        return acc;
      }, {});
      return [
        `Status for "${product.name}".`,
        ...Object.entries(statusCounts).map(([status, count]) => `${status}: ${count}`),
      ];
    }
    case "report_tree": {
      return [buildWorkItemTreeReport(context, action.target?.productName)];
    }
    default:
      return ["No executable action."];
  }
}

export async function executePlannerPlan(plan: PlannerPlan, context: ResolverContext): Promise<ExecutionResult> {
  const lines: string[] = [];
  const errors: string[] = [];
  for (const action of plan.actions) {
    try {
      const resultLines = await executePlannerAction(action, context);
      lines.push(...resultLines);
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error));
    }
  }
  return { lines, errors };
}
