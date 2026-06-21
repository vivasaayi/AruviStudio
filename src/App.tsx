import React, { Suspense, lazy } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "./app/layout/AppShell";

const ProductListPage = lazy(() => import("./features/products/pages/ProductListPage").then((page) => ({ default: page.ProductListPage })));
const ProductDetailPage = lazy(() => import("./features/products/pages/ProductDetailPage").then((page) => ({ default: page.ProductDetailPage })));
const ProductOverviewPage = lazy(() => import("./features/products/pages/ProductOverviewPage").then((page) => ({ default: page.ProductOverviewPage })));
const PortfolioPage = lazy(() => import("./features/portfolio/pages/PortfolioPage").then((page) => ({ default: page.PortfolioPage })));
const WorkItemListPage = lazy(() => import("./features/work-items/pages/WorkItemListPage").then((page) => ({ default: page.WorkItemListPage })));
const WorkItemDetailPage = lazy(() => import("./features/work-items/pages/WorkItemDetailPage").then((page) => ({ default: page.WorkItemDetailPage })));
const RepositoryListPage = lazy(() => import("./features/repositories/pages/RepositoryListPage").then((page) => ({ default: page.RepositoryListPage })));
const AgentRegistryPage = lazy(() => import("./features/agents/pages/AgentRegistryPage").then((page) => ({ default: page.AgentRegistryPage })));
const ModelProviderListPage = lazy(() => import("./features/models/pages/ModelProviderListPage").then((page) => ({ default: page.ModelProviderListPage })));
const SettingsPage = lazy(() => import("./features/settings/pages/SettingsPage").then((page) => ({ default: page.SettingsPage })));
const IDEPage = lazy(() => import("./features/ide/pages/IDEPage").then((page) => ({ default: page.IDEPage })));
const ChatPage = lazy(() => import("./features/chat/pages/ChatPage").then((page) => ({ default: page.ChatPage })));
const VoiceChatPage = lazy(() => import("./features/chat/pages/VoiceChatPage").then((page) => ({ default: page.VoiceChatPage })));
const PlannerPage = lazy(() => import("./features/planner/pages/PlannerPage").then((page) => ({ default: page.PlannerPage })));
const ModelCallsPage = lazy(() => import("./features/calls/pages/ModelCallsPage").then((page) => ({ default: page.ModelCallsPage })));

const fallback = <div style={{ padding: 16, color: "#8f96a3" }}>Loading workspace…</div>;

function App() {
  return (
    <BrowserRouter>
      <AppShell>
        <Suspense fallback={fallback}>
          <Routes>
            <Route path="/" element={<Navigate to="/planner" replace />} />
            <Route path="/portfolio" element={<PortfolioPage />} />
            <Route path="/product-overview" element={<ProductOverviewPage />} />
            <Route path="/products" element={<ProductListPage />} />
            <Route path="/products/:productId" element={<ProductDetailPage />} />
            <Route path="/work-items" element={<WorkItemListPage />} />
            <Route path="/work-items/:workItemId" element={<WorkItemDetailPage />} />
            <Route path="/planner" element={<PlannerPage />} />
            <Route path="/repositories" element={<RepositoryListPage />} />
            <Route path="/agents" element={<AgentRegistryPage />} />
            <Route path="/models" element={<ModelProviderListPage />} />
            <Route path="/chat" element={<ChatPage />} />
            <Route path="/voice-chat" element={<VoiceChatPage />} />
            <Route path="/calls" element={<ModelCallsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/ide" element={<IDEPage />} />
          </Routes>
        </Suspense>
      </AppShell>
    </BrowserRouter>
  );
}

export default App;
