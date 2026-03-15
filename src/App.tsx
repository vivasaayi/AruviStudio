import React, { Suspense, lazy } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "./app/layout/AppShell";

const ProductListPage = lazy(() => import("./features/products/pages/ProductListPage").then((module) => ({ default: module.ProductListPage })));
const ProductDetailPage = lazy(() => import("./features/products/pages/ProductDetailPage").then((module) => ({ default: module.ProductDetailPage })));
const WorkItemListPage = lazy(() => import("./features/work-items/pages/WorkItemListPage").then((module) => ({ default: module.WorkItemListPage })));
const WorkItemDetailPage = lazy(() => import("./features/work-items/pages/WorkItemDetailPage").then((module) => ({ default: module.WorkItemDetailPage })));
const RepositoryListPage = lazy(() => import("./features/repositories/pages/RepositoryListPage").then((module) => ({ default: module.RepositoryListPage })));
const AgentRegistryPage = lazy(() => import("./features/agents/pages/AgentRegistryPage").then((module) => ({ default: module.AgentRegistryPage })));
const ModelProviderListPage = lazy(() => import("./features/models/pages/ModelProviderListPage").then((module) => ({ default: module.ModelProviderListPage })));
const SettingsPage = lazy(() => import("./features/settings/pages/SettingsPage").then((module) => ({ default: module.SettingsPage })));
const IDEPage = lazy(() => import("./features/ide/pages/IDEPage").then((module) => ({ default: module.IDEPage })));
const ChatPage = lazy(() => import("./features/chat/pages/ChatPage").then((module) => ({ default: module.ChatPage })));
const PlannerPage = lazy(() => import("./features/planner/pages/PlannerPage").then((module) => ({ default: module.PlannerPage })));

const fallback = <div style={{ padding: 16, color: "#8f96a3" }}>Loading workspace…</div>;

function App() {
  return (
    <BrowserRouter>
      <AppShell>
        <Suspense fallback={fallback}>
          <Routes>
            <Route path="/" element={<Navigate to="/products" replace />} />
            <Route path="/products" element={<ProductListPage />} />
            <Route path="/products/:productId" element={<ProductDetailPage />} />
            <Route path="/work-items" element={<WorkItemListPage />} />
            <Route path="/work-items/:workItemId" element={<WorkItemDetailPage />} />
            <Route path="/planner" element={<PlannerPage />} />
            <Route path="/repositories" element={<RepositoryListPage />} />
            <Route path="/agents" element={<AgentRegistryPage />} />
            <Route path="/models" element={<ModelProviderListPage />} />
            <Route path="/chat" element={<ChatPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/ide" element={<IDEPage />} />
          </Routes>
        </Suspense>
      </AppShell>
    </BrowserRouter>
  );
}

export default App;
