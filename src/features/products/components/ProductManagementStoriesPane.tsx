import type React from "react";

import type { WorkItem } from "../../../lib/types";
import { formatWorkItemMeta } from "../lib/workItemDisplay";

type ProductManagementStoriesPaneStyles = {
  managementPane: React.CSSProperties;
  sectionTitle: React.CSSProperties;
  managementActions: React.CSSProperties;
  btn: React.CSSProperties;
  ghostBtn: React.CSSProperties;
  managementList: React.CSSProperties;
  managementListButton: React.CSSProperties;
  managementListButtonActive: React.CSSProperties;
  managementItemSelect: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  rowSecondary: React.CSSProperties;
  inlineActionRow: React.CSSProperties;
  outlineActionBtn: React.CSSProperties;
  compactDangerBtn: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductManagementStoriesPane({
  stories,
  selectedStory,
  canCreateStory,
  onCreateStory,
  onOpenBuilder,
  onSelectStory,
  onEditStory,
  onDeleteStory,
  storyPageIndex,
  hasNextStoryPage,
  onPreviousStoryPage,
  onNextStoryPage,
  styles,
}: {
  stories: WorkItem[];
  selectedStory: WorkItem | null;
  canCreateStory: boolean;
  onCreateStory: () => void;
  onOpenBuilder: () => void;
  onSelectStory: (story: WorkItem) => void;
  onEditStory: (story: WorkItem) => void;
  onDeleteStory: (story: WorkItem) => void;
  storyPageIndex: number;
  hasNextStoryPage: boolean;
  onPreviousStoryPage: () => void;
  onNextStoryPage: () => void;
  styles: ProductManagementStoriesPaneStyles;
}) {
  return (
    <div style={styles.managementPane}>
      <div style={styles.sectionTitle}>
        <span>Stories</span>
        <div style={styles.managementActions}>
          <button
            style={styles.btn}
            onClick={onCreateStory}
            disabled={!canCreateStory}
          >
            + Story
          </button>
          <button style={styles.ghostBtn} onClick={onOpenBuilder}>Open Builder</button>
        </div>
      </div>
      <div style={styles.managementList}>
        {stories.length > 0 ? stories.map((story) => (
          <div key={story.id} style={selectedStory?.id === story.id ? styles.managementListButtonActive : styles.managementListButton}>
            <button
              style={styles.managementItemSelect}
              onClick={() => onSelectStory(story)}
            >
              <div style={styles.rowPrimary}>{story.title}</div>
              <div style={styles.rowSecondary}>{formatWorkItemMeta(story.status)} · {story.priority} · {story.complexity}</div>
            </button>
            <div style={styles.inlineActionRow}>
              <button style={styles.outlineActionBtn} onClick={() => onEditStory(story)}>Edit</button>
              <button style={styles.compactDangerBtn} onClick={() => onDeleteStory(story)}>Delete</button>
            </div>
          </div>
        )) : (
          <div style={styles.empty}>No stories for this feature yet.</div>
        )}
      </div>
      <div style={{ ...styles.managementActions, justifyContent: "space-between", marginTop: 10 }}>
        <span style={styles.rowSecondary}>Page {storyPageIndex + 1} · {stories.length} stories shown</span>
        <div style={styles.managementActions}>
          <button
            style={styles.ghostBtn}
            onClick={onPreviousStoryPage}
            disabled={storyPageIndex === 0}
          >
            Previous
          </button>
          <button
            style={styles.ghostBtn}
            onClick={onNextStoryPage}
            disabled={!hasNextStoryPage}
          >
            Next
          </button>
        </div>
      </div>
    </div>
  );
}
