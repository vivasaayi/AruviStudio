import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

interface UIState {
  leftSidebarVisible: boolean;
  rightSidebarVisible: boolean;
  bottomPanelVisible: boolean;
  bottomPanelTab: "terminal" | "logs" | "tests";
  productDialogMode: "closed" | "create" | "edit";
  moduleDialogMode: "closed" | "create" | "edit";
  capabilityDialogMode: "closed" | "create" | "edit";
  workItemCreateDialogOpen: boolean;
  productWorkspaceTab: "dashboard" | "structure" | "work-items";
  workItemWorkspaceTab: "backlog" | "detail" | "review";
  expandedModules: Record<string, boolean>;
  expandedCapabilities: Record<string, boolean>;
  showHierarchyWorkItems: boolean;
  activeView: "products" | "work-items" | "chat" | "ide" | "repositories" | "agents" | "models" | "settings";
  toggleLeftSidebar: () => void;
  toggleRightSidebar: () => void;
  toggleBottomPanel: () => void;
  setBottomPanelTab: (tab: "terminal" | "logs" | "tests") => void;
  openProductDialog: (mode: "create" | "edit") => void;
  closeProductDialog: () => void;
  openModuleDialog: (mode: "create" | "edit") => void;
  closeModuleDialog: () => void;
  openCapabilityDialog: (mode: "create" | "edit") => void;
  closeCapabilityDialog: () => void;
  openWorkItemCreateDialog: () => void;
  closeWorkItemCreateDialog: () => void;
  setProductWorkspaceTab: (tab: "dashboard" | "structure" | "work-items") => void;
  setWorkItemWorkspaceTab: (tab: "backlog" | "detail" | "review") => void;
  toggleModuleExpanded: (id: string) => void;
  toggleCapabilityExpanded: (id: string) => void;
  setModuleExpanded: (id: string, expanded: boolean) => void;
  setCapabilityExpanded: (id: string, expanded: boolean) => void;
  toggleHierarchyWorkItems: () => void;
  setActiveView: (view: UIState["activeView"]) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      leftSidebarVisible: true,
      rightSidebarVisible: true,
      bottomPanelVisible: false,
      bottomPanelTab: "terminal",
      productDialogMode: "closed",
      moduleDialogMode: "closed",
      capabilityDialogMode: "closed",
      workItemCreateDialogOpen: false,
      productWorkspaceTab: "dashboard",
      workItemWorkspaceTab: "backlog",
      expandedModules: {},
      expandedCapabilities: {},
      showHierarchyWorkItems: false,
      activeView: "products",
      toggleLeftSidebar: () => set((s) => ({ leftSidebarVisible: !s.leftSidebarVisible })),
      toggleRightSidebar: () => set((s) => ({ rightSidebarVisible: !s.rightSidebarVisible })),
      toggleBottomPanel: () => set((s) => ({ bottomPanelVisible: !s.bottomPanelVisible })),
      setBottomPanelTab: (tab) => set({ bottomPanelTab: tab }),
      openProductDialog: (mode) => set({ productDialogMode: mode, activeView: "products" }),
      closeProductDialog: () => set({ productDialogMode: "closed" }),
      openModuleDialog: (mode) => set({ moduleDialogMode: mode, activeView: "products", productWorkspaceTab: "structure" }),
      closeModuleDialog: () => set({ moduleDialogMode: "closed" }),
      openCapabilityDialog: (mode) => set({ capabilityDialogMode: mode, activeView: "products", productWorkspaceTab: "structure" }),
      closeCapabilityDialog: () => set({ capabilityDialogMode: "closed" }),
      openWorkItemCreateDialog: () => set({ workItemCreateDialogOpen: true, activeView: "work-items", workItemWorkspaceTab: "backlog" }),
      closeWorkItemCreateDialog: () => set({ workItemCreateDialogOpen: false }),
      setProductWorkspaceTab: (tab) => set({ productWorkspaceTab: tab }),
      setWorkItemWorkspaceTab: (tab) => set({ workItemWorkspaceTab: tab }),
      toggleModuleExpanded: (id) => set((s) => ({ expandedModules: { ...s.expandedModules, [id]: !(s.expandedModules[id] ?? true) } })),
      toggleCapabilityExpanded: (id) => set((s) => ({ expandedCapabilities: { ...s.expandedCapabilities, [id]: !(s.expandedCapabilities[id] ?? true) } })),
      setModuleExpanded: (id, expanded) => set((s) => ({ expandedModules: { ...s.expandedModules, [id]: expanded } })),
      setCapabilityExpanded: (id, expanded) => set((s) => ({ expandedCapabilities: { ...s.expandedCapabilities, [id]: expanded } })),
      toggleHierarchyWorkItems: () => set((s) => ({ showHierarchyWorkItems: !s.showHierarchyWorkItems })),
      setActiveView: (view) => set({ activeView: view }),
    }),
    {
      name: "aruvi-ui",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        leftSidebarVisible: state.leftSidebarVisible,
        rightSidebarVisible: state.rightSidebarVisible,
        bottomPanelVisible: state.bottomPanelVisible,
        bottomPanelTab: state.bottomPanelTab,
        productWorkspaceTab: state.productWorkspaceTab,
        workItemWorkspaceTab: state.workItemWorkspaceTab,
        expandedModules: state.expandedModules,
        expandedCapabilities: state.expandedCapabilities,
        showHierarchyWorkItems: state.showHierarchyWorkItems,
        activeView: state.activeView,
      }),
    },
  ),
);
