// Home screen — DESIGN_BRIEF.md §4: "Feed is the home screen, a
// deliberate reaction to Discord's failure mode." One merged timeline
// across every Table, Table/tag as filters rather than destinations.
// Primary user action (§2): the top-of-screen strip answers "these are
// your people, this is just us" before the feed itself.

import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { Sticker } from "../components/Sticker";
import { PinIcon } from "../components/PinIcon";
import { useFeed, type FeedItem } from "../hooks/useFeed";
import { useMutedAuthors } from "../hooks/useMutedAuthors";
import { useSelfAuthorHex } from "../hooks/useSelfAuthorHex";
import { parseRecordRef } from "../lib/recordRef";
import { isPinVisible } from "../lib/mutedPins";
import { findMessageBySubject } from "../lib/feedMessages";
import { cardStyle } from "../lib/cardStyle";

function itemAuthorHex(item: FeedItem): string {
  switch (item.kind) {
    case "message":
      return item.message.author_hex;
    case "photo":
      return item.image.author_hex;
    case "decision":
      return item.proposal.author_hex;
  }
}

function timeAgo(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const min = Math.max(1, Math.round(ms / 60_000));
  if (min < 60) return `${min}m`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h`;
  return `${Math.round(hr / 24)}d`;
}

// Tag-based filtering (DESIGN_BRIEF.md: "Table and tag as filters on
// that timeline") isn't wired yet — it needs each item's own tags,
// which useFeed doesn't fetch per item in this first pass (only
// aggregate pins). Table filtering is real; tag filtering is a real
// gap, not silently faked with a lookalike chip.
type FeedFilter = { kind: "all" } | { kind: "table"; id: string };

export function Feed() {
  const { loading, tables, items, pinsByTable, profilesByAuthor } = useFeed();
  const [filter, setFilter] = useState<FeedFilter>({ kind: "all" });
  const mutedAuthors = useMutedAuthors();

  // Found in review: the header icon picked whichever author happened
  // to be first in profilesByAuthor's insertion order (async fetch
  // completion order across every Table) rather than the viewer's own
  // profile — a user in several Tables would typically see a random
  // other member's sticker as their own account icon.
  const selfAuthorHex = useSelfAuthorHex();

  const visibleItems = useMemo(() => {
    const unmuted = items.filter((i) => !mutedAuthors.has(itemAuthorHex(i)));
    if (filter.kind === "all") return unmuted;
    return unmuted.filter((i) => i.tableId === filter.id);
  }, [items, filter, mutedAuthors]);

  // One pinned excerpt to feature — the most recently pinned message
  // across every Table, matching "excerpted in the Feed the first time
  // a person sees a new Table's activity." Skips a pin if either the
  // pinner or the pinned message's own author is muted — found in
  // review: this used to read straight from the unfiltered
  // pinsByTable/items, so a muted author's message could still be
  // featured here even though it's correctly hidden from the feed list.
  const featuredPin = useMemo(() => {
    let best: { tableId: string; tag: (typeof pinsByTable)[string][number] } | null = null;
    for (const [tableId, tablePins] of Object.entries(pinsByTable)) {
      for (const tag of tablePins) {
        if (!isPinVisible(tag, (subject) => findMessageBySubject(items, subject)?.author_hex, mutedAuthors)) {
          continue;
        }
        if (!best || tag.rkey > best.tag.rkey) best = { tableId, tag };
      }
    }
    return best;
  }, [pinsByTable, items, mutedAuthors]);

  if (loading) {
    return (
      <div style={{ padding: "var(--space-4xl) var(--space-xl)", color: "var(--text-3)" }}>
        Settling in…
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      {/* Top bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--space-xl) var(--space-xl) var(--space-md)",
        }}
      >
        <div style={{ fontFamily: "var(--font-display)", fontSize: 19 }}>atproto-iroh</div>
        <Link to="/mute" aria-label="Muted authors">
          <Sticker id={selfAuthorHex ? profilesByAuthor[selfAuthorHex]?.avatar : undefined} size={34} />
        </Link>
      </div>

      {/* "These are your people" strip */}
      {tables.length === 0 ? (
        <EmptyTablesState />
      ) : (
        <>
          <div
            className="row-scroll"
            style={{ display: "flex", gap: 18, padding: "4px var(--space-xl) var(--space-lg)", overflowX: "auto" }}
          >
            {tables.map((table) => (
              <Link
                key={table.id}
                to={`/table/${table.id}`}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: 6,
                  flexShrink: 0,
                }}
              >
                <Sticker id={tableAvatar(table.id)} size={52} />
                <span
                  style={{
                    fontSize: 10.5,
                    color: "var(--text-2)",
                    fontWeight: 500,
                    width: 66,
                    lineHeight: 1.25,
                    textAlign: "center",
                    display: "-webkit-box",
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: "vertical",
                    overflow: "hidden",
                  }}
                >
                  {table.name}
                </span>
              </Link>
            ))}
            <Link
              to="/join"
              aria-label="Join a table"
              style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, flexShrink: 0 }}
            >
              <div
                style={{
                  width: 52,
                  height: 52,
                  borderRadius: "50%",
                  border: "1.5px dashed var(--border)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <svg width={18} height={18} viewBox="0 0 24 24" fill="none" stroke="var(--text-3)" strokeWidth={2} strokeLinecap="round">
                  <path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3" />
                </svg>
              </div>
              <span style={{ fontSize: 10.5, color: "var(--text-3)", fontWeight: 500 }}>Join</span>
            </Link>
                        <Link
              to="/new"
              aria-label="Start a table"
              style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, flexShrink: 0 }}
            >
              <div
                style={{
                  width: 52,
                  height: 52,
                  borderRadius: "50%",
                  border: "1.5px dashed var(--border)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <svg width={18} height={18} viewBox="0 0 24 24" fill="none" stroke="var(--text-3)" strokeWidth={2} strokeLinecap="round">
                  <path d="M12 5v14M5 12h14" />
                </svg>
              </div>
              <span style={{ fontSize: 10.5, color: "var(--text-3)", fontWeight: 500 }}>Start</span>
            </Link>
          </div>

          {/* Pinned excerpt */}
          {featuredPin && (
            <div style={{ padding: "0 var(--space-xl) var(--space-lg)" }}>
              <div style={{ ...cardStyle, padding: "14px 16px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
                  <PinIcon />
                  <span style={{ fontSize: 12, color: "var(--accent)", fontWeight: 600 }}>
                    Pinned in {tableName(tables, featuredPin.tableId)}
                  </span>
                </div>
                <p style={{ margin: 0, fontSize: 13.5, color: "var(--text-2)", lineHeight: 1.5 }}>
                  {profilesByAuthor[featuredPin.tag.author_hex]?.name ?? "Someone"}: {pinExcerpt(items, featuredPin.tag.subject)}
                </p>
              </div>
            </div>
          )}

          {/* Filter chips */}
          <div className="row-scroll" style={{ display: "flex", gap: 8, padding: "0 var(--space-xl) var(--space-lg)", overflowX: "auto" }}>
            <Chip active={filter.kind === "all"} onClick={() => setFilter({ kind: "all" })}>
              All
            </Chip>
            {tables.map((table) => (
              <Chip
                key={table.id}
                active={filter.kind === "table" && filter.id === table.id}
                onClick={() => setFilter({ kind: "table", id: table.id })}
              >
                {table.name}
              </Chip>
            ))}
          </div>
        </>
      )}

      {/* Feed */}
      <div style={{ flexGrow: 1, padding: "0 var(--space-xl) var(--space-md)", display: "flex", flexDirection: "column", gap: 12 }}>
        {visibleItems.length === 0 && tables.length > 0 && (
          <p style={{ color: "var(--text-3)", fontSize: 13.5, padding: "var(--space-2xl) 0", textAlign: "center" }}>
            Nothing here yet — it'll fill in as your tables do.
          </p>
        )}
        {visibleItems.map((item) => (
          <FeedItemCard
            key={item.id}
            item={item}
            tableName={tableName(tables, item.tableId)}
            profile={profilesByAuthor[itemAuthorHex(item)]}
          />
        ))}
      </div>
    </div>
  );
}

function tableName(tables: { id: string; name: string }[], id: string): string {
  return tables.find((t) => t.id === id)?.name ?? "Unknown table";
}

/** Deterministic per-Table sticker so the "your people" strip has visual
 * variety without needing a real Table-avatar concept (which doesn't
 * exist yet — same gap as the Table-name one). */
function tableAvatar(tableId: string): string {
  const ids = ["gold", "sage", "rose", "plum", "sky", "accent-strong"] as const;
  let hash = 0;
  for (let i = 0; i < tableId.length; i++) hash = (hash * 31 + tableId.charCodeAt(i)) | 0;
  return ids[Math.abs(hash) % ids.length];
}

function pinExcerpt(items: FeedItem[], subject: string): string {
  const parsed = parseRecordRef(subject);
  if (!parsed) return subject;
  return findMessageBySubject(items, subject)?.text ?? "(pinned)";
}

function Chip({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      style={{
        background: active ? "var(--accent)" : "transparent",
        color: active ? "var(--ink)" : "var(--text-2)",
        border: active ? "none" : "1px solid var(--border)",
        borderRadius: 999,
        padding: "7px 16px",
        fontSize: 13,
        fontWeight: active ? 600 : 500,
        flexShrink: 0,
        whiteSpace: "nowrap",
      }}
    >
      {children}
    </button>
  );
}

function EmptyTablesState() {
  return (
    <div style={{ padding: "var(--space-3xl) var(--space-xl)", textAlign: "center" }}>
      <p style={{ fontFamily: "var(--font-display)", fontSize: 20, margin: "0 0 8px" }}>
        No tables yet
      </p>
      <p style={{ fontSize: 14, color: "var(--text-2)", margin: 0 }}>
        <Link to="/join" style={{ color: "var(--accent)", fontWeight: 600 }}>
          Scan a code
        </Link>{" "}
        to join one, or{" "}
        <Link to="/new" style={{ color: "var(--accent)", fontWeight: 600 }}>
          start your own
        </Link>
        .
      </p>
    </div>
  );
}

function FeedItemCard({
  item,
  tableName,
  profile,
}: {
  item: FeedItem;
  tableName: string;
  profile: { name: string; avatar?: string | null } | undefined;
}) {
  const itemCardStyle: React.CSSProperties = { ...cardStyle, overflow: "hidden" };

  if (item.kind === "decision") {
    const { proposal, status } = item.proposal;
    const isOpen = status.state === "open";
    return (
      <div style={{ ...itemCardStyle, padding: 16 }}>
        <TypeHeader icon={<DecisionIcon />} label={`Decision · ${tableName}`} color="var(--sage)" time={timeAgo(item.createdAt)} />
        <p style={{ margin: "10px 0", fontSize: 15, fontWeight: 600 }}>{proposal.title}</p>
        {proposal.description && (
          <p style={{ margin: "0 0 12px", fontSize: 13.5, color: "var(--text-2)", lineHeight: 1.5 }}>{proposal.description}</p>
        )}
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <span
            style={{
              background: "var(--surface-3)",
              color: "var(--sage)",
              fontSize: 11,
              fontWeight: 600,
              padding: "4px 10px",
              borderRadius: 999,
            }}
          >
            {isOpen ? "Open" : status.state === "ratified" ? "Passed" : "Blocked"}
          </span>
        </div>
      </div>
    );
  }

  if (item.kind === "photo") {
    return (
      <div style={itemCardStyle}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "14px 16px 10px" }}>
          <Sticker id={profile?.avatar} size={28} />
          <div style={{ flexGrow: 1, fontSize: 13.5, fontWeight: 600 }}>
            {profile?.name ?? "Someone"} <span style={{ fontWeight: 400, color: "var(--text-3)" }}>in {tableName}</span>
          </div>
          <span style={{ fontSize: 11, color: "var(--text-3)" }}>{timeAgo(item.createdAt)}</span>
        </div>
        <div style={{ height: 170, background: "linear-gradient(155deg, var(--gold), var(--accent-strong))" }} />
        {item.image.caption && (
          <p style={{ margin: 0, padding: "10px 16px 14px", fontSize: 13.5, color: "var(--text-2)" }}>{item.image.caption}</p>
        )}
      </div>
    );
  }

  // message
  const { message } = item;
  return (
    <div style={{ ...itemCardStyle, padding: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
        <Sticker id={profile?.avatar} size={28} />
        <div style={{ flexGrow: 1, fontSize: 13.5, fontWeight: 600 }}>
          {profile?.name ?? "Someone"} <span style={{ fontWeight: 400, color: "var(--text-3)" }}>in {tableName}</span>
        </div>
        <span style={{ fontSize: 11, color: "var(--text-3)" }}>{timeAgo(item.createdAt)}</span>
      </div>
      <p style={{ margin: 0, fontSize: 14.5, color: "var(--text)", lineHeight: 1.6 }}>{message.text}</p>
    </div>
  );
}

function TypeHeader({ icon, label, color, time }: { icon: React.ReactNode; label: string; color: string; time: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {icon}
        <span style={{ fontSize: 12, fontWeight: 600, color }}>{label}</span>
      </div>
      <span style={{ fontSize: 11, color: "var(--text-3)" }}>{time}</span>
    </div>
  );
}

function DecisionIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--sage)" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M9 12l2 2 4-4" />
      <circle cx="12" cy="12" r="9" />
    </svg>
  );
}
