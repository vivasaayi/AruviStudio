import { useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";

export function useProductPageRuntimeContext() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const isProductDetailRoute = location.pathname.startsWith("/products/");
  const workspace = useWorkspaceStore();
  const ui = useUIStore();

  return {
    isProductDetailRoute,
    navigate,
    queryClient,
    ui,
    workspace,
  };
}
