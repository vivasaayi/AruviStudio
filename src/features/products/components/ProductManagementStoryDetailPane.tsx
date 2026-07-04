import type React from "react";

import type { WorkItem } from "../../../lib/types";
import { formatWorkItemMeta } from "../lib/workItemDisplay";

type ProductManagementStoryDetailPaneStyles = {
  managementPane: React.CSSProperties;
  sectionTitle: React.CSSProperties;
  managementActions: React.CSSProperties;
  ghostBtn: React.CSSProperties;
  contextCard: React.CSSProperties;
  contextLabel: React.CSSProperties;
  contextTitle: React.CSSProperties;
  contextText: React.CSSProperties;
  workItemDetailGrid: React.CSSProperties;
  badgeMuted: React.CSSProperties;
  btn: React.CSSProperties;
  productAreaHeader: React.CSSProperties;
  compactActionBtn: React.CSSProperties;
  compactDangerBtn: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductManagementStoryDetailPane({
  selectedStory,
  tasks,
  onEditStory,
  onOpenStory,
  onCreateTask,
  onEditTask,
  onDeleteTask,
  styles,
}: {
  selectedStory: WorkItem | null;
  tasks: WorkItem[];
  onEditStory: (story: WorkItem) => void;
  onOpenStory: (story: WorkItem) => void;
  onCreateTask: () => void;
  onEditTask: (task: WorkItem) => void;
  onDeleteTask: (task: WorkItem) => void;
  styles: ProductManagementStoryDetailPaneStyles;
}) {
  return (
    <div style={styles.managementPane}>
      {selectedStory ? (
        <>
          <div style={styles.sectionTitle}>
            <span>Story Details</span>
            <div style={styles.managementActions}>
              <button style={styles.ghostBtn} onClick={() => onEditStory(selectedStory)}>Edit</button>
              <button style={styles.ghostBtn} onClick={() => onOpenStory(selectedStory)}>Open Story</button>
            </div>
          </div>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>
              {formatWorkItemMeta(selectedStory.status)} · {formatWorkItemMeta(selectedStory.work_item_type)} · {selectedStory.priority} priority · {formatWorkItemMeta(selectedStory.complexity)} complexity
            </div>
            <div style={styles.contextTitle}>{selectedStory.title}</div>
            <div style={styles.workItemDetailGrid}>
              <div>
                <div style={styles.contextLabel}>Problem</div>
                <div style={styles.contextText}>{selectedStory.problem_statement || "No problem statement captured yet."}</div>
              </div>
              <div>
                <div style={styles.contextLabel}>Description</div>
                <div style={styles.contextText}>{selectedStory.description || "No story description yet."}</div>
              </div>
              <div>
                <div style={styles.contextLabel}>Acceptance Criteria</div>
                <div style={styles.contextText}>{selectedStory.acceptance_criteria || "No acceptance criteria captured yet."}</div>
              </div>
              <div>
                <div style={styles.contextLabel}>Constraints</div>
                <div style={styles.contextText}>{selectedStory.constraints || "No constraints captured yet."}</div>
              </div>
            </div>
          </div>
          <div style={styles.sectionTitle}>
            <span>Tasks</span>
            <div style={styles.managementActions}>
              <span style={styles.badgeMuted}>{tasks.length}</span>
              <button
                style={styles.btn}
                onClick={onCreateTask}
              >
                + Task
              </button>
            </div>
          </div>
          {tasks.length > 0 ? tasks.map((task) => (
            <div key={task.id} style={styles.contextCard}>
              <div style={styles.productAreaHeader}>
                <div>
                  <div style={styles.contextTitle}>{task.title}</div>
                  <div style={styles.contextText}>{formatWorkItemMeta(task.status)} · {task.priority} · {formatWorkItemMeta(task.complexity)}</div>
                  {task.description && <div style={styles.contextText}>{task.description}</div>}
                </div>
                <div style={styles.managementActions}>
                  <button style={styles.compactActionBtn} onClick={() => onEditTask(task)}>Edit</button>
                  <button style={styles.compactDangerBtn} onClick={() => onDeleteTask(task)}>Delete</button>
                </div>
              </div>
            </div>
          )) : (
            <div style={styles.empty}>No tasks under this story yet.</div>
          )}
        </>
      ) : (
        <div style={styles.empty}>Select a story to inspect details and tasks.</div>
      )}
    </div>
  );
}
