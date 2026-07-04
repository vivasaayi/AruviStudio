import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type MockStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

type UIStoreProductArea = typeof import("./uiStore");

let uiStoreProductArea: UIStoreProductArea;
let initialState: ReturnType<UIStoreProductArea["useUIStore"]["getInitialState"]>;

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
  uiStoreProductArea = await import("./uiStore");
  initialState = uiStoreProductArea.useUIStore.getInitialState();
});

afterEach(() => {
  uiStoreProductArea.useUIStore.setState(initialState, true);
  globalThis.localStorage.clear();
});

describe("uiStore", () => {
  it("opens dialogs into the expected workspace views", () => {
    const store = uiStoreProductArea.useUIStore.getState();

    store.openProductDialog("edit");
    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      productDialogMode: "edit",
      activeView: "products",
    });

    store.openProductAreaDialog("create");
    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      productAreaDialogMode: "create",
      activeView: "products",
      productWorkspaceTab: "structure",
    });

    store.openCapabilityDialog("edit");
    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      capabilityDialogMode: "edit",
      activeView: "products",
      productWorkspaceTab: "structure",
    });

    store.openWorkItemCreateDialog();
    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      workItemCreateDialogOpen: true,
      activeView: "work-items",
      workItemWorkspaceTab: "backlog",
    });

    store.closeProductDialog();
    store.closeProductAreaDialog();
    store.closeCapabilityDialog();
    store.closeWorkItemCreateDialog();
    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      productDialogMode: "closed",
      productAreaDialogMode: "closed",
      capabilityDialogMode: "closed",
      workItemCreateDialogOpen: false,
    });
  });

  it("toggles expansion and presentation preferences predictably", () => {
    const store = uiStoreProductArea.useUIStore.getState();

    store.toggleProductAreaExpanded("product_area-1");
    store.toggleCapabilityExpanded("capability-1");
    store.toggleHierarchyWorkItems();
    store.toggleProductPickerCollapsed();

    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      expandedProductAreas: { "product_area-1": false },
      expandedCapabilities: { "capability-1": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
    });

    store.setProductAreaExpanded("product_area-1", true);
    store.setCapabilityExpanded("capability-1", true);
    store.setProductWorkspaceTab("delivery");
    store.setWorkItemWorkspaceTab("review");
    store.setActiveView("settings");

    expect(uiStoreProductArea.useUIStore.getState()).toMatchObject({
      expandedProductAreas: { "product_area-1": true },
      expandedCapabilities: { "capability-1": true },
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "review",
      activeView: "settings",
    });
  });

  it("migrates legacy persisted workspace tabs and partializes persisted state", async () => {
    const persistApi = (uiStoreProductArea.useUIStore as typeof uiStoreProductArea.useUIStore & {
      persist: { getOptions: () => { migrate?: (state: unknown, version: number) => unknown; partialize?: (state: ReturnType<UIStoreProductArea["useUIStore"]["getState"]>) => unknown } };
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

    uiStoreProductArea.useUIStore.setState({
      ...uiStoreProductArea.useUIStore.getState(),
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "detail",
      expandedProductAreas: { "product_area-2": true },
      expandedCapabilities: { "cap-2": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
      activeView: "ide",
      productDialogMode: "edit",
    });

    expect(options.partialize?.(uiStoreProductArea.useUIStore.getState())).toEqual({
      productWorkspaceTab: "delivery",
      workItemWorkspaceTab: "detail",
      expandedProductAreas: { "product_area-2": true },
      expandedCapabilities: { "cap-2": false },
      showHierarchyWorkItems: true,
      productPickerCollapsed: true,
      activeView: "ide",
    });
  });
});
