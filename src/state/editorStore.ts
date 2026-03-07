import { create } from "zustand";

interface OpenFile {
  id: string;
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  language: string;
}

interface EditorState {
  openFiles: OpenFile[];
  activeFileId: string | null;
  openFile: (file: Omit<OpenFile, "isDirty">) => void;
  closeFile: (id: string) => void;
  setActiveFile: (id: string) => void;
  updateFileContent: (id: string, content: string) => void;
  replaceFileContent: (id: string, content: string) => void;
  markFileSaved: (id: string) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  openFiles: [],
  activeFileId: null,
  openFile: (file) =>
    set((s) => {
      const existing = s.openFiles.find((f) => f.id === file.id);
      if (existing) return { activeFileId: file.id };
      return {
        openFiles: [...s.openFiles, { ...file, isDirty: false }],
        activeFileId: file.id,
      };
    }),
  closeFile: (id) =>
    set((s) => {
      const files = s.openFiles.filter((f) => f.id !== id);
      const activeFileId =
        s.activeFileId === id
          ? files.length > 0
            ? files[files.length - 1].id
            : null
          : s.activeFileId;
      return { openFiles: files, activeFileId };
    }),
  setActiveFile: (id) => set({ activeFileId: id }),
  updateFileContent: (id, content) =>
    set((s) => ({
      openFiles: s.openFiles.map((f) =>
        f.id === id ? { ...f, content, isDirty: true } : f
      ),
    })),
  replaceFileContent: (id, content) =>
    set((s) => ({
      openFiles: s.openFiles.map((f) =>
        f.id === id ? { ...f, content, isDirty: false } : f
      ),
    })),
  markFileSaved: (id) =>
    set((s) => ({
      openFiles: s.openFiles.map((f) =>
        f.id === id ? { ...f, isDirty: false } : f
      ),
    })),
}));
