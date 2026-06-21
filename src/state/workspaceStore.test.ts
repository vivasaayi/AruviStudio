import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type MockStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

type WorkspaceStoreProductArea = typeof import("./workspaceStore");

let workspaceStoreProductArea: WorkspaceStoreProductArea;
let initialState: ReturnType<WorkspaceStoreProductArea["useWorkspaceStore"]["getInitialState"]>;

function createMockStorage(): MockStorage {
  const data = new Map<string, string>();
  return {
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => {
      data.set(key, value);
    },
    removeItem: (key) => {
      data.delete(key);
    },
    clear: () => {
      data.clear();
    },
  };
}

beforeEach(async () => {
  vi.resetModules();
  Object.defineProperty(globalThis, "localStorage", {
    value: createMockStorage(),
    configurable: true,
  });
  workspaceStoreProductArea = await import("./workspaceStore");
  initialState = workspaceStoreProductArea.useWorkspaceStore.getInitialState();
});

afterEach(() => {
  workspaceStoreProductArea.useWorkspaceStore.setState(initialState, true);
  globalThis.localStorage.clear();
});

describe("workspaceStore", () => {
  it("resets downstream selection when product, product_area, and capability change", () => {
    const store = workspaceStoreProductArea.useWorkspaceStore.getState();

    store.setActiveProduct("product-1");
    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeProductId: "product-1",
      activeProductAreaId: null,
      activeCapabilityId: null,
      activeNodeId: null,
      activeNodeType: null,
      activeWorkItemId: null,
    });

    store.setActiveProductArea("product_area-1");
    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeProductAreaId: "product_area-1",
      activeCapabilityId: null,
      activeNodeId: "product_area-1",
      activeNodeType: "product_area",
    });

    store.setActiveCapability("capability-1");
    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeProductAreaId: "product_area-1",
      activeCapabilityId: "capability-1",
      activeNodeId: "capability-1",
      activeNodeType: "capability",
    });
  });

  it("applies explicit hierarchy selections and clears work-item focus", () => {
    workspaceStoreProductArea.useWorkspaceStore.setState({
      ...workspaceStoreProductArea.useWorkspaceStore.getState(),
      activeWorkItemId: "work-item-1",
    });

    workspaceStoreProductArea.useWorkspaceStore.getState().setActiveHierarchyNode({
      nodeId: "capability-2",
      nodeType: "capability",
      productAreaId: "product_area-2",
      capabilityId: "capability-2",
    });

    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeProductAreaId: "product_area-2",
      activeCapabilityId: "capability-2",
      activeNodeId: "capability-2",
      activeNodeType: "capability",
      activeWorkItemId: null,
    });

    workspaceStoreProductArea.useWorkspaceStore.getState().setActiveHierarchyNode({
      nodeId: "product_area-3",
      nodeType: "product_area",
    });

    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeProductAreaId: "product_area-3",
      activeCapabilityId: null,
      activeNodeId: "product_area-3",
      activeNodeType: "product_area",
    });
  });

  it("tracks work-item, repo, and workspace selections and partializes persisted state", () => {
    const store = workspaceStoreProductArea.useWorkspaceStore.getState();

    store.setActiveWorkItem("work-item-2");
    store.setActiveRepo("repo-1");
    store.setActiveWorkspace("/tmp/workspace");

    expect(workspaceStoreProductArea.useWorkspaceStore.getState()).toMatchObject({
      activeWorkItemId: "work-item-2",
      activeRepoId: "repo-1",
      activeWorkspacePath: "/tmp/workspace",
    });

    const persistApi = (workspaceStoreProductArea.useWorkspaceStore as typeof workspaceStoreProductArea.useWorkspaceStore & {
      persist: { getOptions: () => { partialize?: (state: ReturnType<WorkspaceStoreProductArea["useWorkspaceStore"]["getState"]>) => unknown } };
    }).persist;

    expect(persistApi.getOptions().partialize?.(workspaceStoreProductArea.useWorkspaceStore.getState())).toEqual({
      activeProductId: null,
      activeProductAreaId: null,
      activeCapabilityId: null,
      activeNodeId: null,
      activeNodeType: null,
      activeWorkItemId: "work-item-2",
      activeRepoId: "repo-1",
      activeWorkspacePath: "/tmp/workspace",
    });
  });
});
