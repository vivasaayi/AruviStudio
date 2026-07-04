import type { ChatMessagePayload, RepositoryTreeNode } from "../../../lib/types";

export type LocalChatMessage = ChatMessagePayload & { id: string };
export type CopilotMode = "chat" | "patch";

export interface PatchProposalItem {
  path: string;
  patch: string;
  base_sha256?: string | null;
  description?: string;
}

export interface CopilotPatchProposal {
  summary: string;
  patches: PatchProposalItem[];
  raw: string;
}

export function filterTreeNodes(nodes: RepositoryTreeNode[], rawFilter: string): RepositoryTreeNode[] {
  const filter = rawFilter.trim().toLowerCase();
  if (!filter) {
    return nodes;
  }

  const filtered: RepositoryTreeNode[] = [];
  for (const node of nodes) {
    if (node.node_type === "file") {
      if (
        node.name.toLowerCase().includes(filter) ||
        node.relative_path.toLowerCase().includes(filter)
      ) {
        filtered.push(node);
      }
      continue;
    }

    const children = filterTreeNodes(node.children, filter);
    if (
      node.name.toLowerCase().includes(filter) ||
      node.relative_path.toLowerCase().includes(filter) ||
      children.length > 0
    ) {
      filtered.push({
        ...node,
        children,
      });
    }
  }
  return filtered;
}

export function normalizePath(path: string): string {
  return path.replace(/\/+$/, "");
}

export function truncateForContext(content: string, maxChars: number): string {
  if (content.length <= maxChars) {
    return content;
  }
  const headLength = Math.floor(maxChars * 0.7);
  const tailLength = Math.max(0, maxChars - headLength);
  return `${content.slice(0, headLength)}\n\n...<truncated for context budget>...\n\n${content.slice(
    Math.max(content.length - tailLength, 0),
  )}`;
}

export function formatBytes(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

export function detectLanguage(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    json: "json",
    rs: "rust",
    py: "python",
    md: "markdown",
    html: "html",
    css: "css",
    scss: "scss",
    yml: "yaml",
    yaml: "yaml",
    toml: "toml",
    sql: "sql",
    sh: "shell",
    zsh: "shell",
    go: "go",
    java: "java",
    kt: "kotlin",
    swift: "swift",
  };
  return map[extension] ?? "plaintext";
}

export function parsePatchProposal(text: string): CopilotPatchProposal | null {
  const payload = extractJsonObject(text);
  if (!payload) {
    return null;
  }
  try {
    const parsed = JSON.parse(payload) as {
      type?: string;
      summary?: string;
      patches?: Array<{
        path?: string;
        patch?: string;
        base_sha256?: string | null;
        baseSha256?: string | null;
        description?: string;
      }>;
    };
    if (parsed.type !== "patch_proposal" || !Array.isArray(parsed.patches) || parsed.patches.length === 0) {
      return null;
    }
    const patches = parsed.patches
      .map((entry) => ({
        path: (entry.path ?? "").trim(),
        patch: entry.patch ?? "",
        base_sha256: entry.base_sha256 ?? entry.baseSha256 ?? null,
        description: entry.description,
      }))
      .filter((entry) => entry.path.length > 0 && entry.patch.trim().length > 0);
    if (patches.length === 0) {
      return null;
    }
    return {
      summary: parsed.summary?.trim() || "Patch proposal generated.",
      patches,
      raw: payload,
    };
  } catch {
    return null;
  }
}

function extractJsonObject(text: string): string | null {
  const fencedMatch = text.match(/```json\s*([\s\S]*?)```/i);
  if (fencedMatch?.[1]) {
    return fencedMatch[1].trim();
  }
  const trimmed = text.trim();
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    return trimmed;
  }
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start >= 0 && end > start) {
    return trimmed.slice(start, end + 1);
  }
  return null;
}
