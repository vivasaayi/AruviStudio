import type { RepositoryTreeNode } from "../../../lib/types";
import { formatBytes } from "../lib/idePageModel";
import { styles } from "../lib/idePageStyles";

type IDEFileTreeNodeProps = {
  node: RepositoryTreeNode;
  expandedDirs: Record<string, boolean>;
  activeFilePath: string | null;
  onToggleDirectory: (relativePath: string) => void;
  onOpenFile: (relativePath: string) => void;
};

export function IDEFileTreeNode({
  node,
  expandedDirs,
  activeFilePath,
  onToggleDirectory,
  onOpenFile,
}: IDEFileTreeNodeProps) {
  const isDirectory = node.node_type === "directory";
  const isExpanded = expandedDirs[node.relative_path] ?? false;
  const isActiveFile = !isDirectory && activeFilePath === node.relative_path;
  const rowStyle = isActiveFile ? styles.treeNodeActive : styles.treeNode;

  return (
    <div>
      <div
        style={rowStyle}
        onClick={() => {
          if (isDirectory) {
            onToggleDirectory(node.relative_path);
            return;
          }
          onOpenFile(node.relative_path);
        }}
      >
        {isDirectory ? (
          <div style={styles.treeDirRow}>
            <span>{isExpanded ? "▾" : "▸"}</span>
            <span>{node.name}</span>
          </div>
        ) : (
          <div style={styles.treeFileRow}>
            <span>•</span>
            <span>{node.name}</span>
          </div>
        )}
        {!isDirectory && node.size_bytes != null && (
          <div style={styles.treeMeta}>{formatBytes(node.size_bytes)}</div>
        )}
      </div>
      {isDirectory && isExpanded && node.children.length > 0 && (
        <div style={styles.treeChildren}>
          {node.children.map((child) => (
            <IDEFileTreeNode
              key={child.relative_path}
              node={child}
              expandedDirs={expandedDirs}
              activeFilePath={activeFilePath}
              onToggleDirectory={onToggleDirectory}
              onOpenFile={onOpenFile}
            />
          ))}
        </div>
      )}
    </div>
  );
}
