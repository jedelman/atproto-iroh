// Joining is the hero flow for most people — DESIGN_BRIEF.md §4/§6:
// "QR scan is the front door for most people — the obvious first
// action, not one option among many on a dense form." Camera first,
// manual paste as the real fallback for no-camera/permission-denied,
// not an afterthought.

import { useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { api } from "../api";
import { useQrScanner } from "../hooks/useQrScanner";
import { useProfileDraft } from "../hooks/useProfileDraft";
import { resolveSelfAuthorHex } from "../lib/identity";

export function Join() {
  const navigate = useNavigate();
  const { draft } = useProfileDraft();
  const [pastedTicket, setPastedTicket] = useState("");
  const [joinError, setJoinError] = useState<string | null>(null);
  const [joining, setJoining] = useState(false);

  async function doJoin(ticket: string) {
    setJoining(true);
    setJoinError(null);
    try {
      const tableId = await api.joinNamespace(ticket);
      // The Onboarding pick (name/sticker) never reaches a Table on its
      // own — NodeProfile is per-Table, so joining is the first point
      // one actually exists to write it to. Found in review: this call
      // was missing entirely, leaving every new member as "Someone"
      // with a default sticker until they separately visited Profile
      // edit and re-entered everything from scratch. No "create a
      // Table" screen exists in this frontend pass either, so Join is
      // the only place this handoff can happen right now. A failure
      // here shouldn't strand the person on an error screen for a join
      // that actually succeeded — they can always fix their profile
      // from the Table afterward — so it's a separate try/catch, not
      // part of the join's own error path.
      //
      // Found in a follow-up review: Join is a persistent, always-
      // reachable entry point (Feed's own "Join or start a table"
      // link), and node.join()/joinNamespace is safely re-callable —
      // so re-scanning or re-pasting a ticket for a Table you already
      // belong to is a real path, not a hypothetical. The first version
      // of this fix called updateProfile unconditionally with
      // hardcoded onboarding defaults, which would silently overwrite
      // an existing member's real name/avatar/neighborhood/description
      // /governance_eligible back to those defaults — exactly the data
      // -loss class ProfileEdit.tsx's own round-tripped-field comment
      // exists to avoid. Only write the draft profile if this author
      // has no profile in this Table yet.
      //
      // A later review pass caught a second, narrower issue: the
      // existence check and the write used to share one try/catch, so
      // a transient failure in nodeDid()/listProfiles() (not the write
      // itself) would silently skip the write too — reintroducing the
      // very "shows up as Someone" bug this whole fix exists for, now
      // on the common first-join path rather than the rarer re-join
      // one. The existence check gets its own try/catch that defaults
      // to "no profile yet" on failure, so a lookup error still errs
      // toward writing (safe: it can only cause an extra overwrite on
      // a genuine re-join, not a missing profile on a first join) —
      // only the write itself stays best-effort against the outer
      // catch below.
      let alreadyHasProfile = false;
      try {
        const [selfAuthorHex, existing] = await Promise.all([
          resolveSelfAuthorHex(api).then((hex) => hex ?? ""),
          api.listProfiles(tableId),
        ]);
        alreadyHasProfile = existing.some((p) => p.author_hex === selfAuthorHex);
      } catch {
        // Couldn't tell — proceed as if this is a first join.
      }
      if (!alreadyHasProfile) {
        try {
          await api.updateProfile(tableId, {
            name: draft.name || "Someone new",
            category: "individual",
            avatar: draft.avatar,
            neighborhood: null,
            description: null,
            governance_eligible: null,
          });
        } catch {
          // Best-effort — the join itself is what matters here.
        }
      }
      // DESIGN_BRIEF.md §6: "scan → see the Table's people appear" —
      // the justJoined flag lets the landing screen give that arrival a
      // real moment (TableDetail's members strip) rather than the
      // members list just being there, indistinguishable from opening a
      // Table you've belonged to for months.
      navigate(`/table/${tableId}`, { state: { justJoined: true } });
    } catch (err) {
      setJoinError(err instanceof Error ? err.message : String(err));
      setJoining(false);
    }
  }

  const { status, error: scanError, videoRef, start, stop } = useQrScanner((text) => {
    doJoin(text);
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to="/feed" style={{ color: "var(--text-3)", fontSize: 13 }}>
          ← Feed
        </Link>
        <h1 style={{ margin: "14px 0 8px", fontFamily: "var(--font-display)", fontSize: 26, fontWeight: 400 }}>
          Join a table
        </h1>
        <p style={{ margin: 0, fontSize: 14, color: "var(--text-2)" }}>
          Scan the code someone shared with you.
        </p>
      </div>

      <div style={{ padding: "0 var(--space-xl) var(--space-lg)" }}>
        {status === "scanning" ? (
          <div className="fade-scale-in" style={{ borderRadius: 20, overflow: "hidden", background: "black", position: "relative" }}>
            <video ref={videoRef} playsInline muted style={{ width: "100%", display: "block" }} />
            <button
              onClick={stop}
              style={{
                position: "absolute",
                bottom: 12,
                left: "50%",
                transform: "translateX(-50%)",
                background: "var(--surface)",
                color: "var(--text)",
                border: "1px solid var(--border)",
                borderRadius: 999,
                padding: "8px 18px",
                fontSize: 13,
              }}
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            onClick={start}
            disabled={status === "requesting" || joining}
            style={{
              width: "100%",
              background: "var(--accent)",
              color: "var(--ink)",
              border: "none",
              borderRadius: 16,
              padding: "18px",
              fontSize: 15.5,
              fontWeight: 700,
            }}
          >
            {status === "requesting" ? "Opening camera…" : joining ? "Joining…" : "Scan a code"}
          </button>
        )}
        {scanError && (
          <p style={{ color: "var(--rose)", fontSize: 13, marginTop: 10 }}>
            {scanError} — paste the ticket below instead.
          </p>
        )}
      </div>

      <div style={{ padding: "0 var(--space-xl) var(--space-xl)", display: "flex", flexDirection: "column", gap: 10 }}>
        <p style={{ margin: 0, fontSize: 12.5, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
          Or paste a ticket
        </p>
        <textarea
          value={pastedTicket}
          onChange={(e) => setPastedTicket(e.target.value)}
          rows={3}
          placeholder="Paste the ticket text here"
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            color: "var(--text)",
            padding: 12,
            fontSize: 13,
            fontFamily: "ui-monospace, monospace",
            resize: "vertical",
          }}
        />
        <button
          onClick={() => doJoin(pastedTicket)}
          disabled={!pastedTicket.trim() || joining}
          style={{
            background: "transparent",
            color: "var(--text)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            padding: "12px",
            fontSize: 14,
            fontWeight: 600,
            opacity: !pastedTicket.trim() || joining ? 0.5 : 1,
          }}
        >
          Join with this ticket
        </button>
        {joinError && (
          <p style={{ color: "var(--rose)", fontSize: 13, margin: 0 }}>
            Couldn't join: {joinError}
          </p>
        )}
      </div>
    </div>
  );
}
