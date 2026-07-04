import { describe, expect, it } from "vitest";
import type { Product, WorkItem } from "../../../lib/types";
import { getProductModalReadiness } from "./productModalReadiness";

const product = { name: "Core Product" } as Product;
const workItem = { title: "Build workflow" } as WorkItem;
const hierarchyNode = { name: "Checkout" };

describe("getProductModalReadiness", () => {
  it("requires exact names and acknowledgements for destructive actions", () => {
    expect(getProductModalReadiness({
      deleteProductCandidate: product,
      deleteConfirmName: "Core Product",
      deleteConfirmArchive: true,
      resetPlanCandidate: product,
      resetPlanConfirmName: "Core Product",
      resetPlanConfirmTree: true,
      deleteHierarchyCandidate: hierarchyNode,
      deleteHierarchyConfirmName: "Checkout",
      deleteHierarchyConfirmChecked: true,
      deleteWorkItemCandidate: { workItem },
      deleteWorkItemConfirmName: "Build workflow",
      deleteWorkItemConfirmChecked: true,
    })).toEqual({
      deleteConfirmationReady: true,
      resetPlanReady: true,
      deleteHierarchyReady: true,
      deleteManagementWorkItemReady: true,
    });
  });

  it("rejects mismatched names or missing acknowledgements", () => {
    expect(getProductModalReadiness({
      deleteProductCandidate: product,
      deleteConfirmName: "core product",
      deleteConfirmArchive: true,
      resetPlanCandidate: product,
      resetPlanConfirmName: "Core Product",
      resetPlanConfirmTree: false,
      deleteHierarchyCandidate: hierarchyNode,
      deleteHierarchyConfirmName: "Checkout",
      deleteHierarchyConfirmChecked: false,
      deleteWorkItemCandidate: { workItem },
      deleteWorkItemConfirmName: "Other workflow",
      deleteWorkItemConfirmChecked: true,
    })).toEqual({
      deleteConfirmationReady: false,
      resetPlanReady: false,
      deleteHierarchyReady: false,
      deleteManagementWorkItemReady: false,
    });
  });
});
