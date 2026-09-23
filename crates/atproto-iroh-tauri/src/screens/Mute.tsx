// Mute — local-only, no sync, not scoped to any one Table (`mute::
// MuteList`'s own doc comment: a reader's own client-side filter, never
// a protocol-level block). Reached from the Feed header rather than a
// per-Table settings menu, matching that global scope.

import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";

export function Mute() {
  const [loading, setLoading] = useState(true);
  const [muted, setMuted] = useState<string[]>([]);
  const [authorHex, setAuthorHex] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    const list = await api.listMuted();
    setMuted(list);
    setLoading(false);
  }

  useEffect(() => {
    refresh();
  }, []);

  async function mute() {
    const trimmed = authorHex.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    try {
      await api.muteAuthor(trimmed);
      setAuthorHex("");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function unmute(hex: string) {
    if (busy) return;
    setBusy(true);
    try {
      await api.unmuteAuthor(hex);
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to="/feed" style={{ color: "var(--text-3)", fontSize: 13, display: "inline-flex", alignItems: "center", gap: 4, marginBottom: 14 }}>
          ← Feed
        </Link>
        <h1 style={{ margin: 0, fontFamily: "var(--font-display)", fontSize: 24, fontWeight: 400 }}>
          Muted
        </h1>
        <p style={{ margin: "8px 0 0", fontSize: 13.5, color: "var(--text-2)", lineHeight: 1.5, maxWidth: "36ch" }}>
          Hides someone's content in your own view only — nothing is sent
          anywhere, and they're never notified.
        </p>
      </div>

      <div style={{ padding: "0 var(--space-xl) var(--space-lg)", display: "flex", gap: 8 }}>
        <input
          value={authorHex}
          onChange={(e) => setAuthorHex(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") mute();
          }}
          placeholder="Author id (hex)"
          style={{
            flexGrow: 1,
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 10,
            color: "var(--text)",
            padding: "9px 12px",
            fontSize: 13.5,
            fontFamily: "monospace",
          }}
        />
        <button
          onClick={mute}
          disabled={!authorHex.trim() || busy}
          style={{
            background: "var(--accent)",
            color: "var(--ink)",
            border: "none",
            borderRadius: 10,
            padding: "9px 16px",
            fontSize: 13.5,
            fontWeight: 700,
            opacity: !authorHex.trim() || busy ? 0.5 : 1,
          }}
        >
          Mute
        </button>
      </div>

      <div style={{ padding: "0 var(--space-xl) var(--space-xl)", display: "flex", flexDirection: "column", gap: 8 }}>
        {loading ? (
          <p style={{ color: "var(--text-3)", fontSize: 13.5 }}>Loading…</p>
        ) : muted.length === 0 ? (
          <p style={{ color: "var(--text-3)", fontSize: 13.5, textAlign: "center", padding: "var(--space-2xl) 0" }}>
            Nobody muted.
          </p>
        ) : (
          muted.map((hex) => (
            <div
              key={hex}
              style={{ background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 12, padding: "10px 14px", display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 }}
            >
              <span style={{ fontSize: 12.5, fontFamily: "monospace", color: "var(--text-2)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {hex}
              </span>
              <button
                onClick={() => unmute(hex)}
                disabled={busy}
                style={{ flexShrink: 0, background: "none", border: "1px solid var(--border)", borderRadius: 10, color: "var(--text-2)", padding: "5px 12px", fontSize: 12.5 }}
              >
                Unmute
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
