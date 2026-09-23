import { Navigate, Route, HashRouter, Routes } from "react-router-dom";
import { Onboarding } from "./screens/Onboarding";
import { Feed } from "./screens/Feed";
import { TableDetail } from "./screens/TableDetail";
import { useProfileDraft } from "./hooks/useProfileDraft";

// HashRouter, not BrowserRouter: Tauri serves the frontend from a
// custom protocol (tauri://localhost / https://tauri.localhost, not a
// real HTTP server), where a plain path-based route on refresh has
// nothing to fall back to. Hash routing needs no server-side rewrite
// rule at all — the safer default for this shell, not a "for now" choice.
export function App() {
  return (
    <HashRouter>
      <div style={{ maxWidth: 430, margin: "0 auto", minHeight: "100vh", background: "var(--bg)" }}>
        <Routes>
          <Route path="/" element={<Root />} />
          <Route path="/onboarding" element={<Onboarding />} />
          <Route path="/feed" element={<Feed />} />
          <Route path="/table/:id" element={<TableDetail />} />
        </Routes>
      </div>
    </HashRouter>
  );
}

function Root() {
  const { draft } = useProfileDraft();
  return <Navigate to={draft.name ? "/feed" : "/onboarding"} replace />;
}
