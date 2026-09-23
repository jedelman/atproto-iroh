// A single Table — DESIGN_BRIEF.md §4: Members-first landing (the
// original primary user action, still true once someone deliberately
// opens a specific Table rather than living in the merged Feed), led by
// the Table's pinned message(s), then tabs for Messages/Decisions/
// Photos. Shared docs is a real, named gap here — no doc-related
// Client methods exist yet (README's "Frontend rebuild" section).

import { useEffect, useMemo, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { Sticker } from "../components/Sticker";
import { profileFor, useTable } from "../hooks/useTable";
import { TABLE_DOC_ID, useTableDoc } from "../hooks/useTableDoc";
import { hasPossibleConflict } from "../lib/docConflict";
import { api, type MessageView, type ProfileView } from "../api";

type Tab = "messages" | "decisions" | "photos" | "docs";

export function TableDetail() {
  const { id } = useParams<{ id: string }>();
  const { loading, table, members, pins, messages, images, proposals, eligibleHex } = useTable(
    id ?? "",
  );
  const [tab, setTab] = useState<Tab>("messages");

  // Optimistic sends layered on top of the hook's own fetch — DESIGN_
  // BRIEF.md §6: "should all feel immediate... the UI shouldn't
  // visibly wait on network round-trips for local actions." Prepended
  // (newest first, same convention useTable already reverses to).
  const [sentMessages, setSentMessages] = useState<MessageView[]>([]);
  const allMessages = useMemo(() => [...sentMessages, ...messages], [sentMessages, messages]);

  const messageBySubject = useMemo(() => {
    const map = new Map<string, MessageView>();
    for (const m of allMessages) map.set(m.subject, m);
    return map;
  }, [allMessages]);

  if (loading) {
    return (
      <div style={{ padding: "var(--space-4xl) var(--space-xl)", color: "var(--text-3)" }}>
        Settling in…
      </div>
    );
  }

  if (!table) {
    return (
      <div style={{ padding: "var(--space-4xl) var(--space-xl)" }}>
        <Link to="/feed" style={{ color: "var(--accent)", fontSize: 14 }}>
          ← Back to feed
        </Link>
        <p style={{ color: "var(--text-2)", marginTop: 16 }}>
          Can't find that table — it may not have finished syncing yet.
        </p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      {/* Header */}
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to="/feed" style={{ color: "var(--text-3)", fontSize: 13, display: "inline-flex", alignItems: "center", gap: 4, marginBottom: 14 }}>
          ← Feed
        </Link>
        <h1 style={{ margin: 0, fontFamily: "var(--font-display)", fontSize: 24, fontWeight: 400 }}>
          {table.name}
        </h1>
      </div>

      {/* Members-first landing — the primary action */}
      <div style={{ padding: "0 var(--space-xl) var(--space-lg)" }}>
        <p style={{ margin: "0 0 10px", fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
          Who's here — just us
        </p>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 14 }}>
          {members.map((m) => (
            <div key={m.author_hex} style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Sticker id={m.profile.avatar} size={32} />
              <div>
                <div style={{ fontSize: 13.5, fontWeight: 600 }}>{m.profile.name}</div>
                {eligibleHex.includes(m.author_hex) && (
                  <div style={{ fontSize: 10.5, color: "var(--text-3)" }}>can weigh in on decisions</div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Pinned messages */}
      {pins.length > 0 && (
        <div style={{ padding: "0 var(--space-xl) var(--space-lg)", display: "flex", flexDirection: "column", gap: 8 }}>
          {pins.map((pin) => {
            const message = messageBySubject.get(pin.subject);
            const author = profileFor(members, pin.author_hex);
            return (
              <div
                key={pin.rkey}
                style={{ background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 16, padding: "12px 16px" }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <PinIcon />
                  <span style={{ fontSize: 11.5, color: "var(--accent)", fontWeight: 600 }}>
                    Pinned by {author?.name ?? "someone"}
                  </span>
                </div>
                <p style={{ margin: 0, fontSize: 13.5, color: "var(--text-2)" }}>
                  {message?.text ?? "(the pinned message hasn't synced yet)"}
                </p>
              </div>
            );
          })}
        </div>
      )}

      {/* Tabs */}
      <div style={{ display: "flex", gap: 4, padding: "0 var(--space-xl)", borderBottom: "1px solid var(--border)" }}>
        <TabButton active={tab === "messages"} onClick={() => setTab("messages")}>
          Messages
        </TabButton>
        <TabButton active={tab === "decisions"} onClick={() => setTab("decisions")}>
          Decisions
        </TabButton>
        <TabButton active={tab === "photos"} onClick={() => setTab("photos")}>
          Photos
        </TabButton>
        <TabButton active={tab === "docs"} onClick={() => setTab("docs")}>
          Shared doc
        </TabButton>
      </div>

      <div style={{ flexGrow: 1, padding: "var(--space-lg) var(--space-xl)", display: "flex", flexDirection: "column", gap: 12 }}>
        {tab === "messages" && (
          <>
            <Composer
              tableId={id ?? ""}
              onSent={(m) => setSentMessages((prev) => [m, ...prev])}
            />
            {allMessages.length === 0 ? (
              <EmptyTab text="No messages yet — be the first to say something." />
            ) : (
              allMessages.map((m) => {
                // No special-casing for "you" here: a member's own
                // profile in *this* Table is looked up the same way as
                // anyone else's. If it resolves to "Someone", that's
                // real and honest — capability and profile are
                // different things (SPEC.md §3.4); joining a Table
                // grants the former, not the latter, so sending a
                // message right after joining (before ever publishing
                // a profile there) genuinely has no name to show yet.
                const author = profileFor(members, m.author_hex);
                return (
                  <div key={m.rkey} style={{ background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 16, padding: 14 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                      <Sticker id={author?.avatar} size={24} />
                      <span style={{ fontSize: 13, fontWeight: 600 }}>{author?.name ?? "Someone"}</span>
                    </div>
                    <p style={{ margin: 0, fontSize: 14, color: "var(--text)", lineHeight: 1.55 }}>{m.text}</p>
                  </div>
                );
              })
            )}
          </>
        )}

        {tab === "decisions" &&
          (proposals.length === 0 ? (
            <EmptyTab text="No decisions yet." />
          ) : (
            proposals.map((p) => (
              <div key={p.rkey} style={{ background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 16, padding: 14 }}>
                <p style={{ margin: "0 0 6px", fontSize: 14.5, fontWeight: 600 }}>{p.proposal.title}</p>
                {p.proposal.description && (
                  <p style={{ margin: 0, fontSize: 13, color: "var(--text-2)" }}>{p.proposal.description}</p>
                )}
                <span
                  style={{
                    display: "inline-block",
                    marginTop: 8,
                    background: "var(--surface-3)",
                    color: "var(--sage)",
                    fontSize: 11,
                    fontWeight: 600,
                    padding: "3px 9px",
                    borderRadius: 999,
                  }}
                >
                  {p.status.state === "open" ? "Open" : p.status.state === "ratified" ? "Passed" : "Blocked"}
                </span>
              </div>
            ))
          ))}

        {tab === "photos" &&
          (images.length === 0 ? (
            <EmptyTab text="No photos yet." />
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 10 }}>
              {images.map((img) => (
                <div key={img.rkey}>
                  <div style={{ aspectRatio: "1", borderRadius: 12, background: "linear-gradient(155deg, var(--gold), var(--accent-strong))" }} />
                  {img.caption && <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--text-2)" }}>{img.caption}</p>}
                </div>
              ))}
            </div>
          ))}

        {tab === "docs" && <SharedDoc tableId={id ?? ""} members={members} />}
      </div>
    </div>
  );
}

function TabButton({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      style={{
        background: "none",
        border: "none",
        borderBottom: active ? "2px solid var(--accent)" : "2px solid transparent",
        color: active ? "var(--text)" : "var(--text-3)",
        fontWeight: active ? 600 : 500,
        fontSize: 13.5,
        padding: "10px 6px",
        marginRight: 14,
      }}
    >
      {children}
    </button>
  );
}

function Composer({ tableId, onSent }: { tableId: string; onSent: (m: MessageView) => void }) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);

  async function send() {
    const trimmed = text.trim();
    if (!trimmed || sending) return;
    setSending(true);
    try {
      // nodeDid() is "did:iroh:<hex>" — the real self author hex is
      // needed here so the optimistic entry resolves through
      // profileFor() exactly the way the real synced-back entry will,
      // not a placeholder that would only coincidentally look right.
      const did = await api.nodeDid();
      const selfAuthorHex = did?.replace(/^did:iroh:/, "") ?? "";
      const rkey = await api.sendMessage(tableId, trimmed);
      onSent({
        author_hex: selfAuthorHex,
        rkey,
        subject: `${selfAuthorHex}/network.essmesh.chat.message/${rkey}`,
        text: trimmed,
        reply_to: null,
        created_at: new Date().toISOString(),
      });
      setText("");
    } finally {
      setSending(false);
    }
  }

  return (
    <div style={{ display: "flex", gap: 8 }}>
      <input
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") send();
        }}
        placeholder="Say something…"
        style={{
          flexGrow: 1,
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 12,
          color: "var(--text)",
          padding: "10px 14px",
          fontSize: 14,
        }}
      />
      <button
        onClick={send}
        disabled={!text.trim() || sending}
        style={{
          background: "var(--accent)",
          color: "var(--ink)",
          border: "none",
          borderRadius: 12,
          padding: "10px 18px",
          fontSize: 14,
          fontWeight: 700,
          opacity: !text.trim() || sending ? 0.5 : 1,
        }}
      >
        Send
      </button>
    </div>
  );
}

function SharedDoc({ tableId, members }: { tableId: string; members: ProfileView[] }) {
  const { loading, revisions, refresh } = useTableDoc(tableId);
  const [text, setText] = useState("");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const latest = revisions[revisions.length - 1] ?? null;

  // "Load" always refreshes full history alongside it — one click, not
  // two, since a stale history is exactly what would hide a real
  // conflict from someone who only ever clicks the button that shows
  // the text.
  useEffect(() => {
    if (!loading && latest) setText(latest.text);
  }, [loading, latest?.rev]);

  async function save() {
    if (!text.trim() || saving) return;
    setSaving(true);
    try {
      await api.docSave(tableId, TABLE_DOC_ID, text);
      setStatus("saved");
      await refresh();
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <EmptyTab text="Loading…" />;
  }

  const conflict = hasPossibleConflict(revisions);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <div>
        <p style={{ margin: "0 0 8px", fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
          Notes — shared by everyone here
        </p>
        <textarea
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setStatus(null);
          }}
          rows={6}
          style={{
            width: "100%",
            resize: "vertical",
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            color: "var(--text)",
            padding: "12px 14px",
            fontSize: 14,
            fontFamily: "inherit",
            boxSizing: "border-box",
          }}
        />
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 8 }}>
          <button
            onClick={save}
            disabled={!text.trim() || saving}
            style={{
              background: "var(--accent)",
              color: "var(--ink)",
              border: "none",
              borderRadius: 12,
              padding: "9px 18px",
              fontSize: 14,
              fontWeight: 700,
              opacity: !text.trim() || saving ? 0.5 : 1,
            }}
          >
            Save
          </button>
          {status && <span style={{ fontSize: 12.5, color: "var(--text-3)" }}>{status}</span>}
        </div>
      </div>

      {conflict && (
        <div
          role="alert"
          style={{ background: "var(--surface)", border: "1px solid var(--rose)", borderRadius: 12, padding: "10px 14px" }}
        >
          <p style={{ margin: 0, fontSize: 13, color: "var(--rose)", fontWeight: 600 }}>
            ⚠ possible conflict
          </p>
          <p style={{ margin: "4px 0 0", fontSize: 12.5, color: "var(--text-2)" }}>
            The two most recent revisions came from different people within 5
            minutes of each other. Review both below and pick one with "Use
            this" before saving again — nothing is merged automatically.
          </p>
        </div>
      )}

      {revisions.length === 0 ? (
        <EmptyTab text="No revisions yet — write something above." />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <p style={{ margin: 0, fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
            History
          </p>
          {[...revisions].reverse().map((rev) => {
            const author = profileFor(members, rev.author_hex);
            return (
              <div
                key={rev.rev}
                style={{ background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 12, padding: "10px 14px", display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}
              >
                <div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                    <Sticker id={author?.avatar} size={18} />
                    <span style={{ fontSize: 12.5, fontWeight: 600 }}>{author?.name ?? "Someone"}</span>
                  </div>
                  <p style={{ margin: 0, fontSize: 13, color: "var(--text-2)", whiteSpace: "pre-wrap" }}>{rev.text}</p>
                </div>
                <button
                  onClick={() => {
                    setText(rev.text);
                    setStatus(`loaded ${author?.name ?? "that"} revision into the box — edit and Save to resolve`);
                  }}
                  style={{
                    flexShrink: 0,
                    background: "none",
                    border: "1px solid var(--border)",
                    borderRadius: 10,
                    color: "var(--text-2)",
                    padding: "5px 10px",
                    fontSize: 12,
                  }}
                >
                  Use this
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function EmptyTab({ text }: { text: string }) {
  return <p style={{ color: "var(--text-3)", fontSize: 13.5, textAlign: "center", padding: "var(--space-2xl) 0" }}>{text}</p>;
}

function PinIcon() {
  return (
    <svg width="11" height="11" viewBox="0 0 24 24" fill="var(--accent)">
      <path d="M12 2l1.6 5.1L19 8l-4 3.6.9 5.4-3.9-2.6-3.9 2.6.9-5.4-4-3.6 5.4-.9z" />
    </svg>
  );
}
