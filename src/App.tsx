import React, { Suspense, lazy } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "./app/layout/AppShell";

const ProductListPage = lazy(() => import("./features/products/pages/ProductListPage").then((product_area) => ({ default: product_area.ProductListPage })));
const ProductDetailPage = lazy(() => import("./features/products/pages/ProductDetailPage").then((product_area) => ({ default: product_area.ProductDetailPage })));
const ProductOverviewPage = lazy(() => import("./features/products/pages/ProductOverviewPage").then((product_area) => ({ default: product_area.ProductOverviewPage })));
const PortfolioPage = lazy(() => import("./features/portfolio/pages/PortfolioPage").then((product_area) => ({ default: product_area.PortfolioPage })));
const WorkItemListPage = lazy(() => import("./features/work-items/pages/WorkItemListPage").then((product_area) => ({ default: product_area.WorkItemListPage })));
const WorkItemDetailPage = lazy(() => import("./features/work-items/pages/WorkItemDetailPage").then((product_area) => ({ default: product_area.WorkItemDetailPage })));
const RepositoryListPage = lazy(() => import("./features/repositories/pages/RepositoryListPage").then((product_area) => ({ default: product_area.RepositoryListPage })));
const AgentRegistryPage = lazy(() => import("./features/agents/pages/AgentRegistryPage").then((product_area) => ({ default: product_area.AgentRegistryPage })));
const ModelProviderListPage = lazy(() => import("./features/models/pages/ModelProviderListPage").then((product_area) => ({ default: product_area.ModelProviderListPage })));
const SettingsPage = lazy(() => import("./features/settings/pages/SettingsPage").then((product_area) => ({ default: product_area.SettingsPage })));
const IDEPage = lazy(() => import("./features/ide/pages/IDEPage").then((product_area) => ({ default: product_area.IDEPage })));
const ChatPage = lazy(() => import("./features/chat/pages/ChatPage").then((product_area) => ({ default: product_area.ChatPage })));
const VoiceChatPage = lazy(() => import("./features/chat/pages/VoiceChatPage").then((product_area) => ({ default: product_area.VoiceChatPage })));
const PlannerPage = lazy(() => import("./features/planner/pages/PlannerPage").then((product_area) => ({ default: product_area.PlannerPage })));
const ModelCallsPage = lazy(() => import("./features/calls/pages/ModelCallsPage").then((product_area) => ({ default: product_area.ModelCallsPage })));

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
