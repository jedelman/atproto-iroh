// Starting a Table — the missing half of USER_FLOW.md's minimum loop
// (the Feed's "…or start your own." used to lead nowhere). Founding a
// Table writes three things at once (create_namespace_with_profile):
// your profile there, a Founding claim naming you its first member, and
// the Table's name. Then straight to Invite, since a Table with only
// you in it isn't a table yet.

import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api } from "../api";
import { Sticker } from "../components/Sticker";
import { useProfileDraft } from "../hooks/useProfileDraft";

export function CreateTable() {
  const navigate = useNavigate();
  const { draft } = useProfileDraft();
  const [tableName, setTableName] = useState("");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const yourName = draft.name || "Someone new";
  const canCreate = tableName.trim().length > 0 && !creating;

  async function create() {
    if (!canCreate) return;
    setCreating(true);
    setError(null);
    try {
      const id = await api.createNamespaceWithProfile(yourName, "individual", draft.avatar ?? null, tableName.trim());
      navigate(`/table/${id}/invite`, { state: { justCreated: true } });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setCreating(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to="/feed" style={{ color: "var(--text-3)", fontSize: 13 }}>
          ← Feed
        </Link>
        <h1 style={{ margin: "14px 0 8px", fontFamily: "var(--font-display)", fontSize: 26, fontWeight: 400 }}>
          Start a table
        </h1>
        <p style={{ margin: 0, fontSize: 14, color: "var(--text-2)" }}>
          A private place for your people. Nobody else can see it — not us, not a server.
        </p>
      </div>

      <div style={{ padding: "var(--space-md) var(--space-xl)", display: "flex", flexDirection: "column", gap: 10 }}>
        <label htmlFor="table-name" style={{ fontSize: 12.5, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
          What's it called?
        </label>
        <input
          id="table-name"
          value={tableName}
          onChange={(e) => setTableName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") create();
          }}
          placeholder="Sunday dinner crew"
          maxLength={60}
          autoFocus
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            color: "var(--text)",
            padding: "12px 14px",
            fontSize: 16,
          }}
        />

        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 8 }}>
          <Sticker id={draft.avatar} size={32} />
          <p style={{ margin: 0, fontSize: 13.5, color: "var(--text-2)" }}>
            You'll be the first one here, as <strong style={{ color: "var(--text)" }}>{yourName}</strong>.
          </p>
        </div>

        <button
          onClick={create}
          disabled={!canCreate}
          style={{
            marginTop: 14,
            background: "var(--accent)",
            color: "var(--ink)",
            border: "none",
            borderRadius: 16,
            padding: "16px",
            fontSize: 15.5,
            fontWeight: 700,
            opacity: canCreate ? 1 : 0.5,
          }}
        >
          {creating ? "Setting the table…" : "Start it"}
        </button>
        {error && (
          <p style={{ color: "var(--rose)", fontSize: 13, margin: 0 }}>Couldn't start it: {error}</p>
        )}
      </div>
    </div>
  );
}
