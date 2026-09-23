// A single Table — DESIGN_BRIEF.md §4: Members-first landing (the
// original primary user action, still true once someone deliberately
// opens a specific Table rather than living in the merged Feed), led by
// the Table's pinned message(s), then tabs for Messages/Decisions/
// Photos. Shared docs is a real, named gap here — no doc-related
// Client methods exist yet (README's "Frontend rebuild" section).

import { useMemo, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { Sticker } from "../components/Sticker";
import { profileFor, useTable } from "../hooks/useTable";

type Tab = "messages" | "decisions" | "photos";

export function TableDetail() {
  const { id } = useParams<{ id: string }>();
  const { loading, table, members, pins, messages, images, proposals, eligibleHex } = useTable(
    id ?? "",
  );
  const [tab, setTab] = useState<Tab>("messages");

  const messageBySubject = useMemo(() => {
    const map = new Map<string, (typeof messages)[number]>();
    for (const m of messages) map.set(m.subject, m);
    return map;
  }, [messages]);

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
      </div>

      <div style={{ flexGrow: 1, padding: "var(--space-lg) var(--space-xl)", display: "flex", flexDirection: "column", gap: 12 }}>
        {tab === "messages" &&
          (messages.length === 0 ? (
            <EmptyTab text="No messages yet — be the first to say something." />
          ) : (
            messages.map((m) => {
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
          ))}

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
