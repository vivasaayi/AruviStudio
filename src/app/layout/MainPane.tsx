import React from "react";

export function MainPane({ children }: { children: React.ReactNode }) {
  return <div style={{ height: "100%", overflow: "auto" }}>{children}</div>;
}
