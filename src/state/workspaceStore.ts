import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

interface WorkspaceState {
  activeProductId: string | null;
  activeModuleId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
  activeRepoId: string | null;
  activeWorkspacePath: string | null;
  setActiveProduct: (id: string | null) => void;
  setActiveModule: (id: string | null) => void;
  setActiveCapability: (id: string | null) => void;
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
      activeWorkItemId: null,
      activeRepoId: null,
      activeWorkspacePath: null,
      setActiveProduct: (id) => set({ activeProductId: id, activeModuleId: null, activeCapabilityId: null, activeWorkItemId: null }),
      setActiveModule: (id) => set({ activeModuleId: id, activeCapabilityId: null, activeWorkItemId: null }),
      setActiveCapability: (id) => set({ activeCapabilityId: id, activeWorkItemId: null }),
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
        activeWorkItemId: state.activeWorkItemId,
        activeRepoId: state.activeRepoId,
        activeWorkspacePath: state.activeWorkspacePath,
      }),
    },
  ),
);
