import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { HierarchyNodeType } from "../lib/types";

interface WorkspaceState {
  activeProductId: string | null;
  activeModuleId: string | null;
  activeCapabilityId: string | null;
  activeNodeId: string | null;
  activeNodeType: HierarchyNodeType | null;
  activeWorkItemId: string | null;
  activeRepoId: string | null;
  activeWorkspacePath: string | null;
  setActiveProduct: (id: string | null) => void;
  setActiveModule: (id: string | null) => void;
  setActiveCapability: (id: string | null) => void;
  setActiveHierarchyNode: (selection: { nodeId: string | null; nodeType: HierarchyNodeType | null; moduleId?: string | null; capabilityId?: string | null }) => void;
  setActiveWorkItem: (id: string | null) => void;
  setActiveRepo: (id: string | null) => void;
  setActiveWorkspace: (path: string | null) => void;
}

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    (set) => ({
      activeProductId: null,
      activeModuleId: null,
      activeCapabilityId: null,
      activeNodeId: null,
      activeNodeType: null,
      activeWorkItemId: null,
      activeRepoId: null,
      activeWorkspacePath: null,
      setActiveProduct: (id) => set({ activeProductId: id, activeModuleId: null, activeCapabilityId: null, activeNodeId: null, activeNodeType: null, activeWorkItemId: null }),
      setActiveModule: (id) => set({ activeModuleId: id, activeCapabilityId: null, activeNodeId: id, activeNodeType: id ? "module" : null, activeWorkItemId: null }),
      setActiveCapability: (id) => set((state) => ({ activeModuleId: id ? state.activeModuleId : state.activeModuleId, activeCapabilityId: id, activeNodeId: id, activeNodeType: id ? "capability" : null, activeWorkItemId: null })),
      setActiveHierarchyNode: ({ nodeId, nodeType, moduleId, capabilityId }) => set({
        activeModuleId: moduleId ?? (nodeType === "module" ? nodeId : null),
        activeCapabilityId: capabilityId ?? (nodeType === "capability" ? nodeId : null),
        activeNodeId: nodeId,
        activeNodeType: nodeType,
        activeWorkItemId: null,
      }),
      setActiveWorkItem: (id) => set({ activeWorkItemId: id }),
      setActiveRepo: (id) => set({ activeRepoId: id }),
      setActiveWorkspace: (path) => set({ activeWorkspacePath: path }),
    }),
    {
      name: "aruvi-workspace",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        activeProductId: state.activeProductId,
        activeModuleId: state.activeModuleId,
        activeCapabilityId: state.activeCapabilityId,
        activeNodeId: state.activeNodeId,
        activeNodeType: state.activeNodeType,
        activeWorkItemId: state.activeWorkItemId,
        activeRepoId: state.activeRepoId,
        activeWorkspacePath: state.activeWorkspacePath,
      }),
    },
  ),
);
