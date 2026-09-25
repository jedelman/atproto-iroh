// Inviting people into a Table — the other missing half of USER_FLOW.md's
// minimum loop. share_namespace mints a Write ticket (what a member needs
// to post), ticket_to_qr renders it; the Join screen on another phone
// scans it. The ticket *is* the access (SPEC.md §3.4's bearer-secret
// model), so the copy says so plainly instead of hiding it.

import { useEffect, useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";
import { api } from "../api";

export function Invite() {
  const { id } = useParams<{ id: string }>();
  const tableId = id ?? "";
  const location = useLocation();
  const justCreated = Boolean((location.state as { justCreated?: boolean } | null)?.justCreated);

  const [ticket, setTicket] = useState<string | null>(null);
  const [qrSrc, setQrSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const t = await api.shareTable(tableId, "Write");
        const svg = await api.ticketToQr(t);
        if (cancelled) return;
        setTicket(t);
        // As an <img> data URL rather than injected markup: the SVG comes
        // from our own Rust qrcode crate, but an image can't run anything
        // either way, so there's nothing to reason about.
        setQrSrc(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [tableId]);

  async function copy() {
    if (!ticket) return;
    try {
      await navigator.clipboard.writeText(ticket);
      setCopied(true);
    } catch {
      // Clipboard can be unavailable in some webviews — the code is
      // still selectable in the box below.
      setCopied(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to={`/table/${tableId}`} style={{ color: "var(--text-3)", fontSize: 13 }}>
          ← Table
        </Link>
        <h1 style={{ margin: "14px 0 8px", fontFamily: "var(--font-display)", fontSize: 26, fontWeight: 400 }}>
          {justCreated ? "Now invite your people" : "Invite someone"}
        </h1>
        <p style={{ margin: 0, fontSize: 14, color: "var(--text-2)", lineHeight: 1.5 }}>
          Have them open the app, tap <strong style={{ color: "var(--text)" }}>Join</strong>, and scan this.
        </p>
      </div>

      <div style={{ padding: "var(--space-md) var(--space-xl)", display: "flex", flexDirection: "column", alignItems: "center", gap: 14 }}>
        {error ? (
          <p style={{ color: "var(--rose)", fontSize: 13, margin: 0 }}>Couldn't make an invite: {error}</p>
        ) : qrSrc ? (
          <img
            className="fade-scale-in"
            src={qrSrc}
            alt="Invite code as a QR code"
            style={{ width: 240, height: 240, borderRadius: 16, background: "#fff", padding: 10 }}
          />
        ) : (
          <div style={{ width: 240, height: 240, borderRadius: 16, background: "var(--surface-2)" }} aria-label="Making an invite…" />
        )}

        <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-3)", lineHeight: 1.5, textAlign: "center", maxWidth: "34ch" }}>
          Anyone with this code can join and post here — treat it like a house key.
          Share it in person or somewhere only your people will see.
        </p>
      </div>

      {ticket && (
        <div style={{ padding: "0 var(--space-xl) var(--space-xl)", display: "flex", flexDirection: "column", gap: 10 }}>
          <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
            Or send the code
          </p>
          <textarea
            readOnly
            value={ticket}
            rows={3}
            aria-label="Invite code"
            onFocus={(e) => e.currentTarget.select()}
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 12,
              color: "var(--text-2)",
              padding: 12,
              fontSize: 12,
              fontFamily: "ui-monospace, monospace",
              resize: "none",
            }}
          />
          <div style={{ display: "flex", gap: 10 }}>
            <button
              onClick={copy}
              style={{
                flexGrow: 1,
                background: "transparent",
                color: "var(--text)",
                border: "1px solid var(--border)",
                borderRadius: 12,
                padding: "12px",
                fontSize: 14,
                fontWeight: 600,
              }}
            >
              {copied ? "Copied" : "Copy code"}
            </button>
            <Link
              to={`/table/${tableId}`}
              style={{
                flexGrow: 1,
                textAlign: "center",
                background: "var(--accent)",
                color: "var(--ink)",
                borderRadius: 12,
                padding: "12px",
                fontSize: 14,
                fontWeight: 700,
              }}
            >
              {justCreated ? "Go to the table" : "Done"}
            </Link>
          </div>
        </div>
      )}
    </div>
  );
}
