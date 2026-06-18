import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type MockStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

type UIStoreModule = typeof import("./uiStore");

let uiStoreModule: UIStoreModule;
let initialState: ReturnType<UIStoreModule["useUIStore"]["getInitialState"]>;

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
  uiStoreModule = await import("./uiStore");
  initialState = uiStoreModule.useUIStore.getInitialState();
});

afterEach(() => {
  uiStoreModule.useUIStore.setState(initialState, true);
  globalThis.localStorage.clear();
});

describe("uiStore", () => {
  it("opens dialogs into the expected workspace views", () => {
    const store = uiStoreModule.useUIStore.getState();

    store.openProductDialog("edit");
    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      productDialogMode: "edit",
      activeView: "products",
    });

    store.openModuleDialog("create");
    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      moduleDialogMode: "create",
      activeView: "products",
      productWorkspaceTab: "structure",
    });

    store.openCapabilityDialog("edit");
    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      capabilityDialogMode: "edit",
      activeView: "products",
      productWorkspaceTab: "structure",
    });

    store.openWorkItemCreateDialog();
    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      workItemCreateDialogOpen: true,
      activeView: "work-items",
      workItemWorkspaceTab: "backlog",
    });

    store.closeProductDialog();
    store.closeModuleDialog();
    store.closeCapabilityDialog();
    store.closeWorkItemCreateDialog();
    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      productDialogMode: "closed",
      moduleDialogMode: "closed",
      capabilityDialogMode: "closed",
      workItemCreateDialogOpen: false,
    });
  });

  it("toggles expansion and presentation preferences predictably", () => {
    const store = uiStoreModule.useUIStore.getState();

    store.toggleModuleExpanded("module-1");
    store.toggleCapabilityExpanded("capability-1");
    store.toggleHierarchyWorkItems();
    store.toggleProductPickerCollapsed();

    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      expandedModules: { "module-1": false },
      expandedCapabilities: { "capability-1": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
    });

    store.setModuleExpanded("module-1", true);
    store.setCapabilityExpanded("capability-1", true);
    store.setProductWorkspaceTab("delivery");
    store.setWorkItemWorkspaceTab("review");
    store.setActiveView("settings");

    expect(uiStoreModule.useUIStore.getState()).toMatchObject({
      expandedModules: { "module-1": true },
      expandedCapabilities: { "capability-1": true },
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "review",
      activeView: "settings",
    });
  });

  it("migrates legacy persisted workspace tabs and partializes persisted state", async () => {
    const persistApi = (uiStoreModule.useUIStore as typeof uiStoreModule.useUIStore & {
      persist: { getOptions: () => { migrate?: (state: unknown, version: number) => unknown; partialize?: (state: ReturnType<UIStoreModule["useUIStore"]["getState"]>) => unknown } };
    }).persist;
    const options = persistApi.getOptions();

    const migratedFromDashboard = await options.migrate?.({
      productWorkspaceTab: "dashboard",
      activeView: "planner",
    }, 0);
    const migratedFromWorkItems = await options.migrate?.({
      productWorkspaceTab: "work-items",
      activeView: "products",
    }, 0);
    const migratedFromOverview = await options.migrate?.({
      productWorkspaceTab: "overview",
      activeView: "products",
    }, 0);

    expect(migratedFromDashboard).toMatchObject({ productWorkspaceTab: "book" });
    expect(migratedFromWorkItems).toMatchObject({ productWorkspaceTab: "delivery" });
    expect(migratedFromOverview).toMatchObject({ productWorkspaceTab: "book" });

    uiStoreModule.useUIStore.setState({
      ...uiStoreModule.useUIStore.getState(),
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "detail",
      expandedModules: { "module-2": true },
      expandedCapabilities: { "cap-2": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
      activeView: "ide",
      productDialogMode: "edit",
    });

    expect(options.partialize?.(uiStoreModule.useUIStore.getState())).toEqual({
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "detail",
      expandedModules: { "module-2": true },
      expandedCapabilities: { "cap-2": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
      activeView: "ide",
    });
  });
});
