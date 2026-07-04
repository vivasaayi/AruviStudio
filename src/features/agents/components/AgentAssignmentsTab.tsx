import type React from "react";

import type { AgentTeam, Product, ProductArea, TeamAssignment } from "../../../lib/types";
import { resolveScopeLabel } from "../lib/agentRegistryPageModel";
import { styles } from "../lib/agentRegistryPageStyles";

type AssignmentScopeType = "product" | "product_area" | "capability";

type AgentAssignmentsTabProps = {
  teams: AgentTeam[];
  products: Product[];
  selectedTeamId: string | null;
  assignmentProductId: string | null;
  assignmentScopeType: AssignmentScopeType;
  assignmentProductAreaId: string;
  assignmentCapabilityId: string;
  currentProductAreaOptions: ProductArea[];
  currentCapabilityOptions: Array<{ id: string; name: string }>;
  selectedTeamAssignments: TeamAssignment[];
  assignmentError: string | null;
  onSelectedTeamChange: (teamId: string | null) => void;
  onAssignmentProductChange: (productId: string | null) => void;
  onAssignmentScopeTypeChange: React.Dispatch<React.SetStateAction<AssignmentScopeType>>;
  onAssignmentProductAreaChange: (productAreaId: string) => void;
  onAssignmentCapabilityChange: (capabilityId: string) => void;
  onAssignScope: () => void;
  onRemoveAssignment: (assignmentId: string) => void;
};

export function AgentAssignmentsTab({
  teams,
  products,
  selectedTeamId,
  assignmentProductId,
  assignmentScopeType,
  assignmentProductAreaId,
  assignmentCapabilityId,
  currentProductAreaOptions,
  currentCapabilityOptions,
  selectedTeamAssignments,
  assignmentError,
  onSelectedTeamChange,
  onAssignmentProductChange,
  onAssignmentScopeTypeChange,
  onAssignmentProductAreaChange,
  onAssignmentCapabilityChange,
  onAssignScope,
  onRemoveAssignment,
}: AgentAssignmentsTabProps) {
  return (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.sectionTitle}>Teams</div>
        <div style={styles.list}>
          {teams.length === 0 ? (
            <div style={styles.empty}>Create a team before assigning delivery scope.</div>
          ) : (
            teams.map((team) => (
              <button
                key={team.id}
                type="button"
                style={{
                  ...styles.listItem,
                  ...(team.id === selectedTeamId ? styles.listItemActive : {}),
                  textAlign: "left",
                }}
                onClick={() => onSelectedTeamChange(team.id)}
              >
                <div style={styles.itemTitle}>{team.name}</div>
                <div style={styles.itemMeta}>{team.department}</div>
              </button>
            ))
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>Scope Assignments</h2>
            <div style={styles.subtitle}>Resolve work items to a team first: capability or feature beats product area, product area beats product.</div>
          </div>
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Team</label>
            <select style={styles.select} value={selectedTeamId ?? ""} onChange={(event) => onSelectedTeamChange(event.target.value || null)}>
              <option value="">Select a team</option>
              {teams.map((team) => (
                <option key={team.id} value={team.id}>
                  {team.name}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Product</label>
            <select
              style={styles.select}
              value={assignmentProductId ?? ""}
              onChange={(event) => onAssignmentProductChange(event.target.value || null)}
            >
              <option value="">Select a product</option>
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Scope Type</label>
            <select
              style={styles.select}
              value={assignmentScopeType}
              onChange={(event) => onAssignmentScopeTypeChange(event.target.value as AssignmentScopeType)}
            >
              <option value="product">Product</option>
              <option value="product_area">Product Area</option>
              <option value="capability">Capability / Slice</option>
            </select>
          </div>
          {assignmentScopeType === "product_area" ? (
            <div style={styles.field}>
              <label style={styles.label}>Product Area</label>
              <select style={styles.select} value={assignmentProductAreaId} onChange={(event) => onAssignmentProductAreaChange(event.target.value)}>
                <option value="">Select a product area</option>
                {currentProductAreaOptions.map((productArea) => (
                  <option key={productArea.id} value={productArea.id}>
                    {productArea.name}
                  </option>
                ))}
              </select>
            </div>
          ) : null}
          {assignmentScopeType === "capability" ? (
            <div style={styles.field}>
              <label style={styles.label}>Capability / Slice</label>
              <select style={styles.select} value={assignmentCapabilityId} onChange={(event) => onAssignmentCapabilityChange(event.target.value)}>
                <option value="">Select a capability or rollout</option>
                {currentCapabilityOptions.map((capability) => (
                  <option key={capability.id} value={capability.id}>
                    {capability.name}
                  </option>
                ))}
              </select>
            </div>
          ) : null}
        </div>
        {assignmentError ? <div style={styles.error}>{assignmentError}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={onAssignScope}>
            Assign Scope
          </button>
        </div>
        <div style={styles.divider} />
        <div style={styles.sectionTitle}>Current Assignments</div>
        {selectedTeamAssignments.length === 0 ? (
          <div style={styles.empty}>No scopes assigned to the selected team yet.</div>
        ) : (
          <div style={styles.treeTable}>
            <div style={{ ...styles.treeHeader, gridTemplateColumns: "minmax(0, 1.5fr) 100px 160px 140px" }}>
              <div>Scope</div>
              <div>Type</div>
              <div>Resolved Name</div>
              <div style={{ textAlign: "right" }}>Actions</div>
            </div>
            {selectedTeamAssignments.map((assignment) => (
              <div
                key={assignment.id}
                style={{ ...styles.treeRow, gridTemplateColumns: "minmax(0, 1.5fr) 100px 160px 140px" }}
              >
                <div style={styles.treeNameCell}>
                  <span style={styles.treeCaret}>{assignment.scope_type === "product" ? "▣" : assignment.scope_type === "product_area" ? "▸" : "•"}</span>
                  <span style={styles.treeSubName}>{assignment.scope_type === "capability" ? "capability" : assignment.scope_type}</span>
                </div>
                <div style={styles.treeCell}>{assignment.scope_type === "capability" ? "capability" : assignment.scope_type}</div>
                <div style={styles.treeCell}>
                  {resolveScopeLabel(
                    assignment,
                    products,
                    currentProductAreaOptions.map((productArea) => ({ id: productArea.id, name: productArea.name })),
                    currentCapabilityOptions.map((capability) => ({ id: capability.id, name: capability.name })),
                  )}
                </div>
                <div style={styles.treeActions}>
                  <button type="button" style={styles.treeActionBtn} onClick={() => onRemoveAssignment(assignment.id)}>
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
