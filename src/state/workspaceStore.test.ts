import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type MockStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

type WorkspaceStoreModule = typeof import("./workspaceStore");

let workspaceStoreModule: WorkspaceStoreModule;
let initialState: ReturnType<WorkspaceStoreModule["useWorkspaceStore"]["getInitialState"]>;

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
  workspaceStoreModule = await import("./workspaceStore");
  initialState = workspaceStoreModule.useWorkspaceStore.getInitialState();
});

afterEach(() => {
  workspaceStoreModule.useWorkspaceStore.setState(initialState, true);
  globalThis.localStorage.clear();
});

describe("workspaceStore", () => {
  it("resets downstream selection when product, module, and capability change", () => {
    const store = workspaceStoreModule.useWorkspaceStore.getState();

    store.setActiveProduct("product-1");
    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeProductId: "product-1",
      activeModuleId: null,
      activeCapabilityId: null,
      activeNodeId: null,
      activeNodeType: null,
      activeWorkItemId: null,
    });

    store.setActiveModule("module-1");
    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeModuleId: "module-1",
      activeCapabilityId: null,
      activeNodeId: "module-1",
      activeNodeType: "module",
    });

    store.setActiveCapability("capability-1");
    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeModuleId: "module-1",
      activeCapabilityId: "capability-1",
      activeNodeId: "capability-1",
      activeNodeType: "capability",
    });
  });

  it("applies explicit hierarchy selections and clears work-item focus", () => {
    workspaceStoreModule.useWorkspaceStore.setState({
      ...workspaceStoreModule.useWorkspaceStore.getState(),
      activeWorkItemId: "work-item-1",
    });

    workspaceStoreModule.useWorkspaceStore.getState().setActiveHierarchyNode({
      nodeId: "capability-2",
      nodeType: "capability",
      moduleId: "module-2",
      capabilityId: "capability-2",
    });

    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeModuleId: "module-2",
      activeCapabilityId: "capability-2",
      activeNodeId: "capability-2",
      activeNodeType: "capability",
      activeWorkItemId: null,
    });

    workspaceStoreModule.useWorkspaceStore.getState().setActiveHierarchyNode({
      nodeId: "module-3",
      nodeType: "module",
    });

    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeModuleId: "module-3",
      activeCapabilityId: null,
      activeNodeId: "module-3",
      activeNodeType: "module",
    });
  });

  it("tracks work-item, repo, and workspace selections and partializes persisted state", () => {
    const store = workspaceStoreModule.useWorkspaceStore.getState();

    store.setActiveWorkItem("work-item-2");
    store.setActiveRepo("repo-1");
    store.setActiveWorkspace("/tmp/workspace");

    expect(workspaceStoreModule.useWorkspaceStore.getState()).toMatchObject({
      activeWorkItemId: "work-item-2",
      activeRepoId: "repo-1",
      activeWorkspacePath: "/tmp/workspace",
    });

    const persistApi = (workspaceStoreModule.useWorkspaceStore as typeof workspaceStoreModule.useWorkspaceStore & {
      persist: { getOptions: () => { partialize?: (state: ReturnType<WorkspaceStoreModule["useWorkspaceStore"]["getState"]>) => unknown } };
    }).persist;

    expect(persistApi.getOptions().partialize?.(workspaceStoreModule.useWorkspaceStore.getState())).toEqual({
      activeProductId: null,
      activeModuleId: null,
      activeCapabilityId: null,
      activeNodeId: null,
      activeNodeType: null,
      activeWorkItemId: "work-item-2",
      activeRepoId: "repo-1",
      activeWorkspacePath: "/tmp/workspace",
    });
  });
});
