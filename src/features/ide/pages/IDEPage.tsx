import React from "react";
import Editor from "@monaco-editor/react";
import { TabBar } from "../../../app/layout/TabBar";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { IDECopilotPanel } from "../components/IDECopilotPanel";
import { IDEFileTreeNode } from "../components/IDEFileTreeNode";
import { revealInFinder } from "../../../lib/tauri";
import { useIDEWorkspaceController } from "../hooks/useIDEWorkspaceController";
import { styles } from "../lib/idePageStyles";

export function IDEPage() {
  const {
    activeCapability,
    activeFile,
    activeFileRepositoryId,
    activeProduct,
    activeProductArea,
    modelOptions,
    copilotMode,
    setCopilotMode,
    copilotProviderId,
    setCopilotProviderId,
    copilotModelName,
    setCopilotModelName,
    copilotTemp,
    setCopilotTemp,
    copilotMaxTokens,
    setCopilotMaxTokens,
    copilotSystemPrompt,
    setCopilotSystemPrompt,
    includeActiveFileContext,
    setIncludeActiveFileContext,
    contextBudgetChars,
    setContextBudgetChars,
    copilotMessages,
    patchProposal,
    copilotDraft,
    setCopilotDraft,
    copilotError,
    isCopilotSending,
    isApplyingProposal,
    resetCopilot,
    sendCopilot,
    applyPatchProposal,
    createWorkspace,
    expandedDirs,
    fileError,
    filteredTree,
    handleSelectProduct,
    isSaving,
    isTreeRefreshing,
    openFolder,
    openRepositoryFile,
    products,
    providers,
    refetchTree,
    scopeResolvedRepo,
    saveActiveFile,
    selectedProductId,
    selectedRepoId,
    selectedRepository,
    setFileError,
    setTreeFilter,
    treeFilter,
    toggleDirectory,
    updateFileContent,
    workItemResolvedRepo,
  } = useIDEWorkspaceController();

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.headerTop}>
          <div>
            <h1 style={styles.title}>IDE Workspace</h1>
            <div style={styles.subtitle}>
              Product-scoped workspace browser, code editor, and Aruvi Copilot.
            </div>
          </div>
          <div style={styles.productPicker}>
            <label style={styles.label}>Product</label>
            <select
              style={styles.select}
              value={selectedProductId ?? ""}
              onChange={(event) => handleSelectProduct(event.target.value)}
            >
              <option value="">Select product</option>
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </select>
          </div>
        </div>
        <ScopeBreadcrumb
          label="Product Scope"
          productName={activeProduct?.name}
          productAreaName={activeProductArea?.name}
          capabilityName={activeCapability?.name}
        />
      </div>

      <div style={styles.workspace}>
        <div style={styles.panel}>
          <div style={styles.panelHeader}>
            <div style={styles.panelTitle}>Workspace</div>
            <div style={styles.controlRow}>
              <button style={styles.buttonGhost} onClick={() => void refetchTree()} disabled={!selectedRepoId}>
                Refresh
              </button>
              {selectedRepository && (
                <button
                  style={styles.buttonGhost}
                  onClick={() => void revealInFinder(selectedRepository.local_path).catch((error) => setFileError(String(error)))}
                >
                  Reveal in Finder
                </button>
              )}
              <button style={styles.button} onClick={() => void openFolder()} disabled={!selectedProductId}>
                Attach Folder
              </button>
            </div>
          </div>
          <div style={styles.leftBody}>
            <input
              style={styles.input}
              value={treeFilter}
              onChange={(event) => setTreeFilter(event.target.value)}
              placeholder="Filter files..."
              disabled={!selectedRepository}
            />
            {!selectedProductId ? (
              <div style={styles.status}>Select a product before opening an IDE workspace.</div>
            ) : !selectedRepository ? (
              <>
                <div style={styles.status}>
                  {workItemResolvedRepo || scopeResolvedRepo
                    ? "Resolving workspace for the selected product..."
                    : "No workspace is attached to this product yet. Create one here, or attach an existing folder."}
                </div>
                {!workItemResolvedRepo && !scopeResolvedRepo ? (
                  <button style={{ ...styles.button, alignSelf: "flex-start" }} onClick={() => void createWorkspace()}>
                    Create Workspace
                  </button>
                ) : null}
              </>
            ) : (
              <div style={styles.status}>
                <strong>{selectedRepository.name}</strong>
                <br />
                {selectedRepository.local_path}
              </div>
            )}
            {selectedRepository && isTreeRefreshing && <div style={styles.status}>Refreshing file tree...</div>}
            {selectedRepository && filteredTree.length === 0 ? (
              <div style={styles.status}>No files match the current filter.</div>
            ) : selectedRepository ? (
              filteredTree.map((node) => (
                <IDEFileTreeNode
                  key={node.relative_path}
                  node={node}
                  expandedDirs={expandedDirs}
                  activeFilePath={activeFile?.path ?? null}
                  onToggleDirectory={toggleDirectory}
                  onOpenFile={(relativePath) => void openRepositoryFile(relativePath)}
                />
              ))
            ) : (
              null
            )}
            {fileError && <div style={styles.error}>{fileError}</div>}
          </div>
        </div>

        <div style={{ ...styles.panel, ...styles.editorPanel }}>
          <div style={styles.editorHeader}>
            <div style={styles.editorPath}>
              {activeFile ? `${activeFile.path}${activeFile.isDirty ? " • unsaved" : ""}` : "No file selected"}
            </div>
            <div style={styles.controlRow}>
              <button style={styles.buttonGhost} onClick={() => void refetchTree()} disabled={!selectedRepoId}>
                Reload Tree
              </button>
              <button
                style={styles.button}
                onClick={() => void saveActiveFile()}
                disabled={!activeFile || isSaving || !activeFileRepositoryId}
              >
                {isSaving ? "Saving..." : "Save (⌘/Ctrl+S)"}
              </button>
            </div>
          </div>
          <TabBar />
          <div style={styles.editorBody}>
            {activeFile ? (
              <Editor
                path={activeFile.path}
                language={activeFile.language}
                value={activeFile.content}
                theme="vs-light"
                onChange={(value) => updateFileContent(activeFile.id, value ?? "")}
                options={{
                  fontSize: 13,
                  fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
                  minimap: { enabled: false },
                  smoothScrolling: true,
                  automaticLayout: true,
                  tabSize: 2,
                  wordWrap: "on",
                }}
              />
            ) : (
              <div style={styles.placeholder}>
                <div>Open a file from the workspace tree.</div>
                <div>Use Aruvi Copilot on the right to validate prompts and responses while editing.</div>
              </div>
            )}
          </div>
        </div>

        <IDECopilotPanel
          providers={providers}
          modelOptions={modelOptions}
          copilotMode={copilotMode}
          setCopilotMode={setCopilotMode}
          copilotProviderId={copilotProviderId}
          setCopilotProviderId={setCopilotProviderId}
          copilotModelName={copilotModelName}
          setCopilotModelName={setCopilotModelName}
          copilotTemp={copilotTemp}
          setCopilotTemp={setCopilotTemp}
          copilotMaxTokens={copilotMaxTokens}
          setCopilotMaxTokens={setCopilotMaxTokens}
          copilotSystemPrompt={copilotSystemPrompt}
          setCopilotSystemPrompt={setCopilotSystemPrompt}
          includeActiveFileContext={includeActiveFileContext}
          setIncludeActiveFileContext={setIncludeActiveFileContext}
          contextBudgetChars={contextBudgetChars}
          setContextBudgetChars={setContextBudgetChars}
          copilotMessages={copilotMessages}
          patchProposal={patchProposal}
          copilotDraft={copilotDraft}
          setCopilotDraft={setCopilotDraft}
          copilotError={copilotError}
          isCopilotSending={isCopilotSending}
          isApplyingProposal={isApplyingProposal}
          onClear={resetCopilot}
          onSend={() => void sendCopilot()}
          onApplyPatchProposal={(proposal) => void applyPatchProposal(proposal)}
        />
      </div>
    </div>
  );
}
