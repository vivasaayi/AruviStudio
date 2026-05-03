import React from "react";

const styles: Record<string, React.CSSProperties> = {
  wrap: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    flexWrap: "wrap",
    padding: "8px 10px",
    borderRadius: 10,
    border: "1px solid #32353d",
    backgroundColor: "#1c2027",
    marginBottom: 10,
  },
  label: {
    fontSize: 10,
    fontWeight: 800,
    letterSpacing: 0.8,
    textTransform: "uppercase",
    color: "#8f96a3",
    marginRight: 4,
  },
  crumb: {
    fontSize: 12,
    color: "#e8edf7",
    padding: "3px 8px",
    borderRadius: 999,
    backgroundColor: "#262c36",
    border: "1px solid #39404c",
  },
  crumbAccent: {
    fontSize: 12,
    color: "#ffffff",
    padding: "3px 8px",
    borderRadius: 999,
    backgroundColor: "#173247",
    border: "1px solid #0e639c",
  },
  sep: {
    fontSize: 11,
    color: "#6f7887",
  },
};

export function ScopeBreadcrumb({
  productName,
  moduleName,
  capabilityName,
  path,
  label = "Scope",
}: {
  productName: string | null | undefined;
  moduleName?: string | null | undefined;
  capabilityName?: string | null | undefined;
  path?: string[] | null | undefined;
  label?: string;
}) {
  const crumbs = (path && path.length > 0
    ? path
    : [productName, moduleName, capabilityName].filter(Boolean)) as string[];

  if (crumbs.length === 0) {
    return null;
  }

  return (
    <div style={styles.wrap}>
      <span style={styles.label}>{label}</span>
      {crumbs.map((crumb, index) => (
        <React.Fragment key={`${crumb}-${index}`}>
          <span style={index === crumbs.length - 1 ? styles.crumbAccent : styles.crumb}>{crumb}</span>
          {index < crumbs.length - 1 ? <span style={styles.sep}>/</span> : null}
        </React.Fragment>
      ))}
    </div>
  );
}
