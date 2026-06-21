import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

interface UIState {
  productDialogMode: "closed" | "create" | "edit";
  productAreaDialogMode: "closed" | "create" | "edit";
  capabilityDialogMode: "closed" | "create" | "edit";
  workItemCreateDialogOpen: boolean;
  productWorkspaceTab: "book" | "structure" | "delivery";
  workItemWorkspaceTab: "backlog" | "detail" | "review" | "external_cli";
  expandedProductAreas: Record<string, boolean>;
  expandedCapabilities: Record<string, boolean>;
  showHierarchyWorkItems: boolean;
  productPickerCollapsed: boolean;
  activeView: "portfolio" | "products" | "product-overview" | "work-items" | "planner" | "chat" | "voice-chat" | "calls" | "ide" | "repositories" | "agents" | "models" | "settings";
  openProductDialog: (mode: "create" | "edit") => void;
  closeProductDialog: () => void;
  openProductAreaDialog: (mode: "create" | "edit") => void;
  closeProductAreaDialog: () => void;
  openCapabilityDialog: (mode: "create" | "edit") => void;
  closeCapabilityDialog: () => void;
  openWorkItemCreateDialog: () => void;
  closeWorkItemCreateDialog: () => void;
  setProductWorkspaceTab: (tab: "book" | "structure" | "delivery") => void;
  setWorkItemWorkspaceTab: (tab: "backlog" | "detail" | "review" | "external_cli") => void;
  toggleProductAreaExpanded: (id: string) => void;
  toggleCapabilityExpanded: (id: string) => void;
  setProductAreaExpanded: (id: string, expanded: boolean) => void;
  setCapabilityExpanded: (id: string, expanded: boolean) => void;
  toggleHierarchyWorkItems: () => void;
  toggleProductPickerCollapsed: () => void;
  setActiveView: (view: UIState["activeView"]) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      productDialogMode: "closed",
      productAreaDialogMode: "closed",
      capabilityDialogMode: "closed",
      workItemCreateDialogOpen: false,
      productWorkspaceTab: "book",
      workItemWorkspaceTab: "backlog",
      expandedProductAreas: {},
      expandedCapabilities: {},
      showHierarchyWorkItems: false,
      productPickerCollapsed: false,
      activeView: "planner",
      openProductDialog: (mode) => set({ productDialogMode: mode, activeView: "products" }),
      closeProductDialog: () => set({ productDialogMode: "closed" }),
      openProductAreaDialog: (mode) => set({ productAreaDialogMode: mode, activeView: "products", productWorkspaceTab: "structure" }),
      closeProductAreaDialog: () => set({ productAreaDialogMode: "closed" }),
      openCapabilityDialog: (mode) => set({ capabilityDialogMode: mode, activeView: "products", productWorkspaceTab: "structure" }),
      closeCapabilityDialog: () => set({ capabilityDialogMode: "closed" }),
      openWorkItemCreateDialog: () => set({ workItemCreateDialogOpen: true, activeView: "work-items", workItemWorkspaceTab: "backlog" }),
      closeWorkItemCreateDialog: () => set({ workItemCreateDialogOpen: false }),
      setProductWorkspaceTab: (tab) => set({ productWorkspaceTab: tab }),
      setWorkItemWorkspaceTab: (tab) => set({ workItemWorkspaceTab: tab }),
      toggleProductAreaExpanded: (id) => set((s) => ({ expandedProductAreas: { ...s.expandedProductAreas, [id]: !(s.expandedProductAreas[id] ?? true) } })),
      toggleCapabilityExpanded: (id) => set((s) => ({ expandedCapabilities: { ...s.expandedCapabilities, [id]: !(s.expandedCapabilities[id] ?? true) } })),
      setProductAreaExpanded: (id, expanded) => set((s) => ({ expandedProductAreas: { ...s.expandedProductAreas, [id]: expanded } })),
      setCapabilityExpanded: (id, expanded) => set((s) => ({ expandedCapabilities: { ...s.expandedCapabilities, [id]: expanded } })),
      toggleHierarchyWorkItems: () => set((s) => ({ showHierarchyWorkItems: !s.showHierarchyWorkItems })),
      toggleProductPickerCollapsed: () => set((s) => ({ productPickerCollapsed: !s.productPickerCollapsed })),
      setActiveView: (view) => set({ activeView: view }),
    }),
    {
      name: "aruvi-ui",
      storage: createJSONStorage(() => localStorage),
      migrate: (persistedState) => {
        const state = persistedState as Partial<UIState> & { productWorkspaceTab?: string };
        const rawTab = state.productWorkspaceTab as string | undefined;
        const nextTab = rawTab === "dashboard"
          ? "book"
          : rawTab === "work-items"
            ? "delivery"
            : rawTab === "overview"
              ? "book"
              : rawTab;
        return {
          ...state,
          productWorkspaceTab: nextTab,
        } as UIState;
      },
      partialize: (state) => ({
        productWorkspaceTab: state.productWorkspaceTab,
        workItemWorkspaceTab: state.workItemWorkspaceTab,
        expandedProductAreas: state.expandedProductAreas,
        expandedCapabilities: state.expandedCapabilities,
        showHierarchyWorkItems: state.showHierarchyWorkItems,
        productPickerCollapsed: state.productPickerCollapsed,
        activeView: state.activeView,
      }),
    },
  ),
);
