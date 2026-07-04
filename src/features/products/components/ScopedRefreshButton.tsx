import { useState } from "react";

const styles = {
  refreshWrap: { display: "flex", alignItems: "center", gap: 8, minHeight: 30 },
  refreshBtn: { padding: "6px 10px", minHeight: 30, fontSize: 12, fontWeight: 800, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3b4049", borderRadius: 8, cursor: "pointer", whiteSpace: "nowrap" as const },
  refreshBtnBusy: { padding: "6px 10px", minHeight: 30, fontSize: 12, fontWeight: 800, backgroundColor: "#173247", color: "#ffffff", border: "1px solid #0e639c", borderRadius: 8, cursor: "wait", whiteSpace: "nowrap" as const },
  refreshError: { maxWidth: 260, fontSize: 11, color: "#ff7b72", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" as const },
};

export function ScopedRefreshButton({
  label = "Refresh",
  disabled,
  onRefresh,
}: {
  label?: string;
  disabled?: boolean;
  onRefresh: () => Promise<void>;
}) {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const isDisabled = disabled || isRefreshing;

  const runRefresh = async () => {
    if (isDisabled) {
      return;
    }
    try {
      setIsRefreshing(true);
      setRefreshError(null);
      await onRefresh();
    } catch (error) {
      setRefreshError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <div style={styles.refreshWrap}>
      <button
        style={{
          ...(isRefreshing ? styles.refreshBtnBusy : styles.refreshBtn),
          ...(disabled ? { opacity: 0.55, cursor: "not-allowed" } : null),
        }}
        onClick={() => void runRefresh()}
        disabled={isDisabled}
        title={refreshError ?? label}
      >
        {isRefreshing ? "Refreshing..." : label}
      </button>
      {refreshError ? <span style={styles.refreshError}>{refreshError}</span> : null}
    </div>
  );
}
