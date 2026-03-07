import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "./app/layout/AppShell";
import { ProductListPage } from "./features/products/pages/ProductListPage";
import { ProductDetailPage } from "./features/products/pages/ProductDetailPage";
import { WorkItemListPage } from "./features/work-items/pages/WorkItemListPage";
import { WorkItemDetailPage } from "./features/work-items/pages/WorkItemDetailPage";
import { RepositoryListPage } from "./features/repositories/pages/RepositoryListPage";
import { AgentRegistryPage } from "./features/agents/pages/AgentRegistryPage";
import { ModelProviderListPage } from "./features/models/pages/ModelProviderListPage";
import { SettingsPage } from "./features/settings/pages/SettingsPage";
import { IDEPage } from "./features/ide/pages/IDEPage";
import { ChatPage } from "./features/chat/pages/ChatPage";

function App() {
  return (
    <BrowserRouter>
      <AppShell>
        <Routes>
          <Route path="/" element={<Navigate to="/products" replace />} />
          <Route path="/products" element={<ProductListPage />} />
          <Route path="/products/:productId" element={<ProductDetailPage />} />
          <Route path="/work-items" element={<WorkItemListPage />} />
          <Route path="/work-items/:workItemId" element={<WorkItemDetailPage />} />
          <Route path="/repositories" element={<RepositoryListPage />} />
          <Route path="/agents" element={<AgentRegistryPage />} />
          <Route path="/models" element={<ModelProviderListPage />} />
          <Route path="/chat" element={<ChatPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/ide" element={<IDEPage />} />
        </Routes>
      </AppShell>
    </BrowserRouter>
  );
}

export default App;
