import { afterEach, describe, expect, it } from "vitest";

import { useEditorStore } from "./editorStore";

const initialState = useEditorStore.getInitialState();

afterEach(() => {
  useEditorStore.setState(initialState, true);
});

describe("editorStore", () => {
  it("opens files once and focuses the active file", () => {
    useEditorStore.getState().openFile({
      id: "file-1",
      path: "src/file-1.ts",
      name: "file-1.ts",
      content: "one",
      language: "ts",
    });
    useEditorStore.getState().openFile({
      id: "file-1",
      path: "src/file-1.ts",
      name: "file-1.ts",
      content: "updated",
      language: "ts",
    });

    const state = useEditorStore.getState();
    expect(state.openFiles).toHaveLength(1);
    expect(state.activeFileId).toBe("file-1");
    expect(state.openFiles[0].content).toBe("one");
    expect(state.openFiles[0].isDirty).toBe(false);
  });

  it("tracks dirty state and saved replacements", () => {
    useEditorStore.getState().openFile({
      id: "file-1",
      path: "src/file-1.ts",
      name: "file-1.ts",
      content: "one",
      language: "ts",
    });
    useEditorStore.getState().updateFileContent("file-1", "two");
    expect(useEditorStore.getState().openFiles[0]).toMatchObject({
      content: "two",
      isDirty: true,
    });

    useEditorStore.getState().markFileSaved("file-1");
    expect(useEditorStore.getState().openFiles[0].isDirty).toBe(false);

    useEditorStore.getState().replaceFileContent("file-1", "three");
    expect(useEditorStore.getState().openFiles[0]).toMatchObject({
      content: "three",
      isDirty: false,
    });
  });

  it("reassigns the active file when the current tab closes", () => {
    const store = useEditorStore.getState();
    store.openFile({
      id: "file-1",
      path: "src/file-1.ts",
      name: "file-1.ts",
      content: "one",
      language: "ts",
    });
    store.openFile({
      id: "file-2",
      path: "src/file-2.ts",
      name: "file-2.ts",
      content: "two",
      language: "ts",
    });

    store.closeFile("file-2");
    expect(useEditorStore.getState().activeFileId).toBe("file-1");

    store.closeFile("file-1");
    expect(useEditorStore.getState()).toMatchObject({
      activeFileId: null,
      openFiles: [],
    });
  });
});
