import { useEffect, useState } from "react";
import { Navigate, Route, HashRouter, Routes } from "react-router-dom";
import { api } from "./api";
import { CreateTable } from "./screens/CreateTable";
import { Invite } from "./screens/Invite";
import { Onboarding } from "./screens/Onboarding";
import { Feed } from "./screens/Feed";
import { TableDetail } from "./screens/TableDetail";
import { Join } from "./screens/Join";
import { ProfileEdit } from "./screens/ProfileEdit";
import { Mute } from "./screens/Mute";
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
        <NodeGate>
          <Routes>
            <Route path="/" element={<Root />} />
            <Route path="/onboarding" element={<Onboarding />} />
            <Route path="/feed" element={<Feed />} />
            <Route path="/new" element={<CreateTable />} />
            <Route path="/table/:id" element={<TableDetail />} />
            <Route path="/table/:id/profile" element={<ProfileEdit />} />
            <Route path="/table/:id/invite" element={<Invite />} />
            <Route path="/join" element={<Join />} />
            <Route path="/mute" element={<Mute />} />
          </Routes>
        </NodeGate>
      </div>
    </HashRouter>
  );
}

// Nothing below renders until this device's node is up. Found on the
// first real install: nothing ever called spawn_node, so every backend
// command failed with "call spawn_node first" — the mock never needed
// it, so no test or screenshot could show it. spawn_node is idempotent
// and reloads every Table already on disk, so calling it once here
// covers first run and every restart alike.
export function NodeGate({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<{ status: "starting" } | { status: "ready" } | { status: "error"; message: string }>(
    { status: "starting" },
  );
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setState({ status: "starting" });
    api.spawnNode().then(
      () => !cancelled && setState({ status: "ready" }),
      (err) => !cancelled && setState({ status: "error", message: err instanceof Error ? err.message : String(err) }),
    );
    return () => {
      cancelled = true;
    };
  }, [attempt]);

  if (state.status === "ready") return <>{children}</>;
  return (
    <div style={{ padding: "var(--space-4xl) var(--space-xl)", color: "var(--text-2)" }}>
      {state.status === "starting" ? (
        <p style={{ margin: 0 }}>Settling in…</p>
      ) : (
        <>
          <p style={{ margin: "0 0 12px", color: "var(--text)" }}>Couldn't start up on this device.</p>
          <p style={{ margin: "0 0 16px", fontSize: 13, color: "var(--text-3)", fontFamily: "ui-monospace, monospace" }}>
            {state.message}
          </p>
          <button
            onClick={() => setAttempt((n) => n + 1)}
            style={{ background: "var(--accent)", color: "var(--ink)", border: "none", borderRadius: 12, padding: "10px 18px", fontWeight: 700 }}
          >
            Try again
          </button>
        </>
      )}
    </div>
  );
}

function Root() {
  const { draft } = useProfileDraft();
  return <Navigate to={draft.name ? "/feed" : "/onboarding"} replace />;
}
