// A single Table — DESIGN_BRIEF.md §4: Members-first landing (the
// original primary user action, still true once someone deliberately
// opens a specific Table rather than living in the merged Feed), led by
// the Table's pinned message(s), then tabs for Messages/Decisions/
// Photos/Shared doc.

import { useEffect, useMemo, useState } from "react";
import { useParams, useLocation, Link } from "react-router-dom";
import { Sticker } from "../components/Sticker";
import { PinIcon } from "../components/PinIcon";
import { profileFor, useTable } from "../hooks/useTable";
import { TABLE_DOC_ID, useTableDoc } from "../hooks/useTableDoc";
import { useTags } from "../hooks/useTags";
import { useMutedAuthors } from "../hooks/useMutedAuthors";
import { useSelfAuthorHex } from "../hooks/useSelfAuthorHex";
import { hasPossibleConflict } from "../lib/docConflict";
import { resolveSelfAuthorHex } from "../lib/identity";
import { isPinVisible } from "../lib/mutedPins";
import { cardStyle, rowCardStyle } from "../lib/cardStyle";
import {
  api,
  PIN_LABEL,
  type GovernanceClass,
  type ImageView,
  type MessageView,
  type PolicyChange,
  type PolicyValue,
  type Proposal,
  type ProfileView,
  type ProposalView,
  type TagView,
} from "../api";

type Tab = "messages" | "decisions" | "photos" | "docs";

export function TableDetail() {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  // Set by Join.tsx's navigate() right after a successful join — see its
  // own comment for why this is worth a real arrival moment rather than
  // the members list just silently being there.
  const justJoined = Boolean((location.state as { justJoined?: boolean } | null)?.justJoined);
  const { loading, table, members, pins, messages, images, proposals, eligibleHex, refreshProposals, refreshPins } =
    useTable(id ?? "");
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

  // Keyed by messaging::reply_ref's "{author_hex}/{rkey}" convention —
  // deliberately a separate map from messageBySubject above, since
  // Message.reply_to isn't a record_ref (no collection segment) the way
  // a tag's subject is; reusing messageBySubject here would silently
  // never match.
  const messageByReplyRef = useMemo(() => {
    const map = new Map<string, MessageView>();
    for (const m of allMessages) map.set(`${m.author_hex}/${m.rkey}`, m);
    return map;
  }, [allMessages]);
  const [replyTarget, setReplyTarget] = useState<MessageView | null>(null);

  const [uploadedImages, setUploadedImages] = useState<ImageView[]>([]);
  const mutedAuthors = useMutedAuthors();
  // Hides a pin if either the person who pinned it OR the pinned
  // message's own author is muted — found in review: this used to read
  // straight from the unfiltered `pins`/`messageBySubject`, so a muted
  // author's message still showed in full up here even though the same
  // content was correctly hidden from the Messages tab below.
  const visiblePins = useMemo(
    () =>
      pins.filter((pin) =>
        isPinVisible(pin, (subject) => messageBySubject.get(subject)?.author_hex, mutedAuthors),
      ),
    [pins, messageBySubject, mutedAuthors],
  );
  const allImages = useMemo(
    () => [...uploadedImages, ...images].filter((img) => !mutedAuthors.has(img.author_hex)),
    [uploadedImages, images, mutedAuthors],
  );

  const { userLabels, tagsBySubject, refresh: refreshTags } = useTags(id ?? "");
  const [activeTagFilter, setActiveTagFilter] = useState<string | null>(null);
  const visibleMessages = useMemo(
    () =>
      allMessages
        .filter((m) => !mutedAuthors.has(m.author_hex))
        .filter((m) =>
          activeTagFilter === null
            ? true
            : (tagsBySubject.get(m.subject) ?? []).some((t) => t.label === activeTagFilter),
        ),
    [allMessages, tagsBySubject, activeTagFilter, mutedAuthors],
  );

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
      <div className={justJoined ? "joined-pop" : undefined} style={{ padding: "0 var(--space-xl) var(--space-lg)" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 10 }}>
          <p style={{ margin: 0, fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
            Who's here — just us
          </p>
          <Link to={`/table/${id}/profile`} style={{ color: "var(--accent)", fontSize: 12.5, fontWeight: 600 }}>
            Edit your profile
          </Link>
        </div>
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

      {/* Pinned messages — DESIGN_BRIEF.md §5: "a brand-new Table
          without a pinned welcome shouldn't feel broken or
          half-finished." Anyone can pin (tagging.rs's open posture, not
          founder-only), so the prompt doesn't single anyone out — it's
          the same line for whoever opens a pin-less Table first. */}
      {visiblePins.length === 0 ? (
        <div style={{ padding: "0 var(--space-xl) var(--space-lg)" }}>
          <p style={{ margin: 0, fontSize: 13, color: "var(--text-3)", fontStyle: "italic" }}>
            Nothing pinned yet — pin a message below to say what this
            Table's about.
          </p>
        </div>
      ) : (
        <div style={{ padding: "0 var(--space-xl) var(--space-lg)", display: "flex", flexDirection: "column", gap: 8 }}>
          {visiblePins.map((pin) => {
            const message = messageBySubject.get(pin.subject);
            const author = profileFor(members, pin.author_hex);
            return (
              <div
                key={pin.rkey}
                style={{ ...cardStyle, padding: "12px 16px" }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <PinIcon size={11} />
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
              replyTarget={replyTarget}
              replyTargetAuthor={replyTarget ? profileFor(members, replyTarget.author_hex)?.name ?? "Someone" : null}
              onClearReply={() => setReplyTarget(null)}
            />
            {userLabels.length > 0 && (
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {activeTagFilter && (
                  <FilterChip label="All" active={false} onClick={() => setActiveTagFilter(null)} />
                )}
                {userLabels.map((label) => (
                  <FilterChip
                    key={label}
                    label={label}
                    active={activeTagFilter === label}
                    onClick={() => setActiveTagFilter(activeTagFilter === label ? null : label)}
                  />
                ))}
              </div>
            )}
            {visibleMessages.length === 0 ? (
              <EmptyTab
                text={
                  activeTagFilter
                    ? `No messages tagged "${activeTagFilter}".`
                    : "No messages yet — be the first to say something."
                }
              />
            ) : (
              visibleMessages.map((m) => {
                // No special-casing for "you" here: a member's own
                // profile in *this* Table is looked up the same way as
                // anyone else's. If it resolves to "Someone", that's
                // real and honest — capability and profile are
                // different things (SPEC.md §3.4); joining a Table
                // grants the former, not the latter, so sending a
                // message right after joining (before ever publishing
                // a profile there) genuinely has no name to show yet.
                const author = profileFor(members, m.author_hex);
                const repliedTo = m.reply_to ? messageByReplyRef.get(m.reply_to) : undefined;
                const repliedToAuthor = repliedTo ? profileFor(members, repliedTo.author_hex)?.name ?? "Someone" : null;
                return (
                  <div key={m.rkey} style={{ ...cardStyle, padding: 14 }}>
                    {m.reply_to && (
                      <p style={{ margin: "0 0 6px", fontSize: 12, color: "var(--text-3)", borderRadius: 8, background: "var(--surface-2)", padding: "5px 8px" }}>
                        ↳ Replying to {repliedToAuthor ?? "someone"}
                        {repliedTo ? `: "${truncate(repliedTo.text, 60)}"` : " (that message hasn't synced yet)"}
                      </p>
                    )}
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                      <Sticker id={author?.avatar} size={24} />
                      <span style={{ fontSize: 13, fontWeight: 600 }}>{author?.name ?? "Someone"}</span>
                    </div>
                    <p style={{ margin: "0 0 8px", fontSize: 14, color: "var(--text)", lineHeight: 1.55 }}>{m.text}</p>
                    <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
                      <MessageTags
                        tableId={id ?? ""}
                        subject={m.subject}
                        tags={tagsBySubject.get(m.subject) ?? []}
                        onTagged={refreshTags}
                        onPinned={refreshPins}
                      />
                      <button
                        onClick={() => setReplyTarget(m)}
                        style={{ background: "none", border: "none", color: "var(--text-3)", fontSize: 11.5, padding: "2px 4px" }}
                      >
                        Reply
                      </button>
                    </div>
                  </div>
                );
              })
            )}
          </>
        )}

        {tab === "decisions" && (
          <DecisionsPanel
            tableId={id ?? ""}
            proposals={proposals}
            eligibleHex={eligibleHex}
            members={members}
            onChanged={refreshProposals}
          />
        )}

        {tab === "photos" && (
          <PhotosPanel
            tableId={id ?? ""}
            images={allImages}
            onUploaded={(img) => setUploadedImages((prev) => [img, ...prev])}
          />
        )}

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

function FilterChip({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        background: active ? "var(--accent)" : "var(--surface)",
        color: active ? "var(--ink)" : "var(--text-2)",
        border: "1px solid " + (active ? "var(--accent)" : "var(--border)"),
        borderRadius: 999,
        padding: "4px 12px",
        fontSize: 12.5,
        fontWeight: 600,
      }}
    >
      {`#${label}`}
    </button>
  );
}

function MessageTags({
  tableId,
  subject,
  tags,
  onTagged,
  onPinned,
}: {
  tableId: string;
  subject: string;
  tags: TagView[];
  onTagged: () => void;
  // Pinning is itself just a tag (tagging.rs's PIN_LABEL — SPEC.md's
  // "tags are monads" note) but a separate callback from onTagged: a
  // pin also has to refresh useTable's own `pins` state (the top-of-
  // screen strip), not just the per-message tag list this component
  // already re-fetches via onTagged.
  onPinned: () => void;
}) {
  const [adding, setAdding] = useState(false);
  const [label, setLabel] = useState("");
  const [saving, setSaving] = useState(false);
  const [pinning, setPinning] = useState(false);

  async function submit() {
    const trimmed = label.trim();
    if (!trimmed || saving) return;
    setSaving(true);
    try {
      await api.addTag(tableId, subject, trimmed);
      setLabel("");
      setAdding(false);
      onTagged();
    } finally {
      setSaving(false);
    }
  }

  async function pin() {
    if (pinning) return;
    setPinning(true);
    try {
      await api.addTag(tableId, subject, PIN_LABEL);
      onPinned();
    } finally {
      setPinning(false);
    }
  }

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
      {tags.map((t) => (
        <span
          key={t.rkey}
          style={{ fontSize: 11.5, color: "var(--text-3)", background: "var(--surface-2)", borderRadius: 999, padding: "2px 9px" }}
        >
          {`#${t.label}`}
        </span>
      ))}
      <button
        onClick={pin}
        disabled={pinning}
        aria-label="Pin this message"
        style={{ background: "none", border: "none", color: "var(--text-3)", fontSize: 11.5, padding: "2px 4px", display: "inline-flex", alignItems: "center", gap: 3, opacity: pinning ? 0.5 : 1 }}
      >
        <PinIcon size={10} /> Pin
      </button>
      {adding ? (
        <span style={{ display: "inline-flex", gap: 4 }}>
          <input
            autoFocus
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
              if (e.key === "Escape") setAdding(false);
            }}
            placeholder="tag name"
            style={{
              width: 90,
              background: "var(--bg)",
              border: "1px solid var(--border)",
              borderRadius: 999,
              color: "var(--text)",
              padding: "2px 9px",
              fontSize: 11.5,
            }}
          />
        </span>
      ) : (
        <button
          onClick={() => setAdding(true)}
          style={{ background: "none", border: "none", color: "var(--text-3)", fontSize: 11.5, padding: "2px 4px" }}
        >
          + Tag
        </button>
      )}
    </div>
  );
}

function Composer({
  tableId,
  onSent,
  replyTarget,
  replyTargetAuthor,
  onClearReply,
}: {
  tableId: string;
  onSent: (m: MessageView) => void;
  replyTarget: MessageView | null;
  replyTargetAuthor: string | null;
  onClearReply: () => void;
}) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);

  async function send() {
    const trimmed = text.trim();
    if (!trimmed || sending) return;
    setSending(true);
    try {
      // The real self author hex is needed here so the optimistic
      // entry resolves through profileFor() exactly the way the real
      // synced-back entry will, not a placeholder that would only
      // coincidentally look right.
      const selfAuthorHex = (await resolveSelfAuthorHex(api)) ?? "";
      const replyTo = replyTarget ? { authorHex: replyTarget.author_hex, rkey: replyTarget.rkey } : undefined;
      const rkey = await api.sendMessage(tableId, trimmed, replyTo);
      onSent({
        author_hex: selfAuthorHex,
        rkey,
        subject: `${selfAuthorHex}/network.essmesh.chat.message/${rkey}`,
        text: trimmed,
        reply_to: replyTo ? `${replyTo.authorHex}/${replyTo.rkey}` : null,
        created_at: new Date().toISOString(),
      });
      setText("");
      onClearReply();
    } finally {
      setSending(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {replyTarget && (
        <div style={{ display: "flex", alignItems: "center", gap: 8, background: "var(--surface-2)", borderRadius: 10, padding: "6px 10px" }}>
          <p style={{ margin: 0, flexGrow: 1, fontSize: 12, color: "var(--text-3)" }}>
            Replying to {replyTargetAuthor ?? "someone"}: "{truncate(replyTarget.text, 50)}"
          </p>
          <button
            onClick={onClearReply}
            aria-label="Cancel reply"
            style={{ background: "none", border: "none", color: "var(--text-3)", fontSize: 13, padding: "0 4px" }}
          >
            ×
          </button>
        </div>
      )}
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
    </div>
  );
}

function PhotosPanel({
  tableId,
  images,
  onUploaded,
}: {
  tableId: string;
  images: ImageView[];
  onUploaded: (img: ImageView) => void;
}) {
  const [caption, setCaption] = useState("");
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleFile(file: File) {
    setUploading(true);
    setError(null);
    try {
      // Same reasoning as Composer's send(): the optimistic entry's
      // author_hex has to be the real self hex, not a placeholder — an
      // ImageThumb keyed off a wrong hex would fail its own
      // loadImageBytes lookup (mockClient stores bytes under the real
      // hex the upload actually used).
      const selfAuthorHex = (await resolveSelfAuthorHex(api)) ?? "";
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      const rkey = await api.uploadImage(
        tableId,
        bytes,
        file.type || "application/octet-stream",
        caption.trim() || null,
      );
      onUploaded({
        author_hex: selfAuthorHex,
        rkey,
        content_type: file.type || "application/octet-stream",
        len: bytes.length,
        caption: caption.trim() || null,
        created_at: new Date().toISOString(),
      });
      setCaption("");
    } catch (err) {
      setError(String(err));
    } finally {
      setUploading(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <div style={{ ...cardStyle, padding: 14, display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
        <input
          value={caption}
          onChange={(e) => setCaption(e.target.value)}
          placeholder="Caption (optional)"
          style={{
            flexGrow: 1,
            minWidth: 140,
            background: "var(--bg)",
            border: "1px solid var(--border)",
            borderRadius: 10,
            color: "var(--text)",
            padding: "9px 12px",
            fontSize: 13.5,
          }}
        />
        <label
          style={{
            background: "var(--accent)",
            color: "var(--ink)",
            border: "none",
            borderRadius: 10,
            padding: "9px 16px",
            fontSize: 13.5,
            fontWeight: 700,
            cursor: uploading ? "default" : "pointer",
            opacity: uploading ? 0.5 : 1,
          }}
        >
          {uploading ? "Uploading…" : "Add photo"}
          <input
            type="file"
            accept="image/*"
            disabled={uploading}
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file) handleFile(file);
            }}
            style={{ display: "none" }}
          />
        </label>
      </div>
      {error && <p style={{ margin: 0, fontSize: 12.5, color: "var(--rose)" }}>{error}</p>}

      {images.length === 0 ? (
        <EmptyTab text="No photos yet." />
      ) : (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 10 }}>
          {images.map((img) => (
            <div key={img.rkey}>
              <ImageThumb tableId={tableId} image={img} />
              {img.caption && <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--text-2)" }}>{img.caption}</p>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ImageThumb({ tableId, image }: { tableId: string; image: ImageView }) {
  const [src, setSrc] = useState<string | null>(null);
  const [notSynced, setNotSynced] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    (async () => {
      const bytes = await api.loadImageBytes(tableId, image.author_hex, image.rkey);
      if (cancelled) return;
      if (bytes === null) {
        setNotSynced(true);
        return;
      }
      const blob = new Blob([new Uint8Array(bytes)], { type: image.content_type });
      objectUrl = URL.createObjectURL(blob);
      setSrc(objectUrl);
    })();
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [tableId, image.author_hex, image.rkey, image.content_type]);

  if (notSynced) {
    return (
      <div
        style={{ aspectRatio: "1", borderRadius: 12, background: "var(--surface-2)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 11, color: "var(--text-3)", textAlign: "center", padding: 8 }}
      >
        not synced yet
      </div>
    );
  }

  if (!src) {
    return <div style={{ aspectRatio: "1", borderRadius: 12, background: "var(--surface-2)" }} />;
  }

  return (
    <img
      src={src}
      alt={image.caption ?? image.rkey}
      style={{ width: "100%", aspectRatio: "1", objectFit: "cover", borderRadius: 12, display: "block" }}
    />
  );
}

type DecisionType = "poll" | "admitCoSigner" | "removeCoSigner" | "changePolicy";

// GovernanceClass values a policy change can actually target —
// PolicyChange.for_class (governance.rs) has no "general" field, since
// General-class decisions were never policy-governed to begin with.
const POLICY_TARGET_CLASSES: { class: GovernanceClass; label: string }[] = [
  { class: "admitCoSigner", label: "Admitting co-signers" },
  { class: "removeCoSigner", label: "Removing co-signers" },
  { class: "changePolicy", label: "Changing policy" },
];

function policyChangeField(target: GovernanceClass, value: PolicyValue): PolicyChange {
  switch (target) {
    case "admitCoSigner":
      return { admit_co_signer: value };
    case "removeCoSigner":
      return { remove_co_signer: value };
    default:
      return { change_policy: value };
  }
}

function classLabel(p: Proposal): string | null {
  switch (p.class) {
    case "admitCoSigner":
      return "Admit co-signer";
    case "removeCoSigner":
      return "Remove co-signer";
    case "changePolicy":
      return "Change policy";
    default:
      return null;
  }
}

function DecisionsPanel({
  tableId,
  proposals,
  eligibleHex,
  members,
  onChanged,
}: {
  tableId: string;
  proposals: ProposalView[];
  eligibleHex: string[];
  members: ProfileView[];
  onChanged: () => void;
}) {
  const selfAuthorHex = useSelfAuthorHex();
  const [type, setType] = useState<DecisionType>("poll");
  const [title, setTitle] = useState("");
  const [deadlineHours, setDeadlineHours] = useState(72);
  const [creating, setCreating] = useState(false);
  const [voting, setVoting] = useState<string | null>(null);
  const [subjectMemberHex, setSubjectMemberHex] = useState("");
  const [policyTargetClass, setPolicyTargetClass] = useState<GovernanceClass>("admitCoSigner");
  const [windowDays, setWindowDays] = useState(3);
  const [blockThreshold, setBlockThreshold] = useState(2);

  const canWeighIn = selfAuthorHex !== null && eligibleHex.includes(selfAuthorHex);
  const admittableMembers = members.filter((m) => !eligibleHex.includes(m.author_hex));
  const removableMembers = members.filter((m) => eligibleHex.includes(m.author_hex));

  function nameFor(hex: string): string {
    return profileFor(members, hex)?.name ?? "Someone";
  }

  async function create() {
    if (creating) return;
    setCreating(true);
    try {
      if (type === "poll") {
        const trimmed = title.trim();
        if (!trimmed) return;
        await api.createDecision(tableId, trimmed, deadlineHours);
        setTitle("");
      } else if (type === "admitCoSigner" || type === "removeCoSigner") {
        if (!subjectMemberHex) return;
        const verb = type === "admitCoSigner" ? "Admit" : "Remove";
        await api.createDecision(tableId, `${verb} ${nameFor(subjectMemberHex)} as co-signer`, deadlineHours, {
          class: type,
          subjectMemberHex,
        });
        setSubjectMemberHex("");
      } else {
        const targetLabel = POLICY_TARGET_CLASSES.find((t) => t.class === policyTargetClass)?.label ?? policyTargetClass;
        await api.createDecision(tableId, `Change policy: ${targetLabel.toLowerCase()}`, deadlineHours, {
          class: "changePolicy",
          policyChange: policyChangeField(policyTargetClass, {
            window_seconds: windowDays * 86_400,
            block_threshold: blockThreshold,
          }),
        });
      }
      onChanged();
    } finally {
      setCreating(false);
    }
  }

  async function signal(p: ProposalView, signalType: "consent" | "block") {
    const key = `${p.rkey}:${signalType}`;
    setVoting(key);
    try {
      await api.signalDecision(tableId, p.author_hex, p.rkey, signalType);
      onChanged();
    } finally {
      setVoting(null);
    }
  }

  const canCreate =
    !creating &&
    (type === "poll"
      ? title.trim().length > 0
      : type === "admitCoSigner" || type === "removeCoSigner"
        ? subjectMemberHex.length > 0
        : true);

  const selectStyle: React.CSSProperties = {
    background: "var(--bg)",
    border: "1px solid var(--border)",
    borderRadius: 10,
    color: "var(--text)",
    padding: "9px 10px",
    fontSize: 13.5,
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <div style={{ ...cardStyle, padding: 14 }}>
        <p style={{ margin: "0 0 8px", fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
          New decision
        </p>
        <p style={{ margin: "0 0 10px", fontSize: 12, color: "var(--text-3)" }}>
          Not a majority vote — this passes automatically once the window
          closes, unless enough people object. Silence counts as support.
        </p>
        <select value={type} onChange={(e) => setType(e.target.value as DecisionType)} style={{ ...selectStyle, marginBottom: 8, width: "100%" }}>
          <option value="poll">Poll</option>
          <option value="admitCoSigner">Admit a co-signer</option>
          <option value="removeCoSigner">Remove a co-signer</option>
          <option value="changePolicy">Change policy</option>
        </select>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {type === "poll" && (
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="What are you deciding?"
              style={{
                flexGrow: 1,
                minWidth: 160,
                background: "var(--bg)",
                border: "1px solid var(--border)",
                borderRadius: 10,
                color: "var(--text)",
                padding: "9px 12px",
                fontSize: 13.5,
              }}
            />
          )}
          {(type === "admitCoSigner" || type === "removeCoSigner") && (
            <select
              value={subjectMemberHex}
              onChange={(e) => setSubjectMemberHex(e.target.value)}
              style={{ ...selectStyle, flexGrow: 1, minWidth: 160 }}
            >
              <option value="">
                {(type === "admitCoSigner" ? admittableMembers : removableMembers).length === 0
                  ? "No eligible members"
                  : "Choose a member…"}
              </option>
              {(type === "admitCoSigner" ? admittableMembers : removableMembers).map((m) => (
                <option key={m.author_hex} value={m.author_hex}>
                  {m.profile.name}
                </option>
              ))}
            </select>
          )}
          {type === "changePolicy" && (
            <>
              <select
                value={policyTargetClass}
                onChange={(e) => setPolicyTargetClass(e.target.value as GovernanceClass)}
                style={{ ...selectStyle, flexGrow: 1, minWidth: 160 }}
              >
                {POLICY_TARGET_CLASSES.map((t) => (
                  <option key={t.class} value={t.class}>
                    {t.label}
                  </option>
                ))}
              </select>
              <input
                type="number"
                min={1}
                value={windowDays}
                onChange={(e) => setWindowDays(Number(e.target.value))}
                aria-label="Objection window, in days"
                title="Objection window, in days"
                style={{ ...selectStyle, width: 70 }}
              />
              <input
                type="number"
                min={1}
                value={blockThreshold}
                onChange={(e) => setBlockThreshold(Number(e.target.value))}
                aria-label="Block threshold"
                title="How many objections it takes to block"
                style={{ ...selectStyle, width: 70 }}
              />
            </>
          )}
          <select
            value={deadlineHours}
            onChange={(e) => setDeadlineHours(Number(e.target.value))}
            style={selectStyle}
          >
            <option value={24}>1 day</option>
            <option value={72}>3 days</option>
            <option value={168}>1 week</option>
          </select>
          <button
            onClick={create}
            disabled={!canCreate}
            style={{
              background: "var(--accent)",
              color: "var(--ink)",
              border: "none",
              borderRadius: 10,
              padding: "9px 16px",
              fontSize: 13.5,
              fontWeight: 700,
              opacity: !canCreate ? 0.5 : 1,
            }}
          >
            Propose
          </button>
        </div>
      </div>

      {proposals.length === 0 ? (
        <EmptyTab text="No decisions yet." />
      ) : (
        proposals.map((p) => (
          <div key={p.rkey} style={{ ...cardStyle, padding: 14 }}>
            <p style={{ margin: "0 0 6px", fontSize: 14.5, fontWeight: 600 }}>{p.proposal.title}</p>
            {classLabel(p.proposal) && (
              <p style={{ margin: "0 0 6px", fontSize: 11, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
                {classLabel(p.proposal)}
                {p.proposal.subject_member ? ` · ${nameFor(p.proposal.subject_member)}` : ""}
              </p>
            )}
            {p.proposal.description && (
              <p style={{ margin: 0, fontSize: 13, color: "var(--text-2)" }}>{p.proposal.description}</p>
            )}
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 10 }}>
              <span
                className="status-pill"
                style={{
                  display: "inline-block",
                  background: "var(--surface-3)",
                  color: p.status.state === "blocked" ? "var(--rose)" : "var(--sage)",
                  fontSize: 11,
                  fontWeight: 600,
                  padding: "3px 9px",
                  borderRadius: 999,
                }}
              >
                {p.status.state === "open"
                  ? `Open${p.status.blockers.length ? `, ${p.status.blockers.length} objection(s)` : ""}`
                  : p.status.state === "ratified"
                    ? "Passed"
                    : `Did not pass (${p.status.blockers.length} objection(s))`}
              </span>
              {canWeighIn && p.status.state === "open" && (
                <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
                  <button
                    onClick={() => signal(p, "consent")}
                    disabled={voting !== null}
                    style={{ background: "none", border: "1px solid var(--border)", borderRadius: 10, color: "var(--sage)", padding: "5px 12px", fontSize: 12.5, fontWeight: 600 }}
                  >
                    Support
                  </button>
                  <button
                    onClick={() => signal(p, "block")}
                    disabled={voting !== null}
                    style={{ background: "none", border: "1px solid var(--border)", borderRadius: 10, color: "var(--rose)", padding: "5px 12px", fontSize: 12.5, fontWeight: 600 }}
                  >
                    Object
                  </button>
                </div>
              )}
            </div>
          </div>
        ))
      )}
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
          style={{ ...rowCardStyle, border: "1px solid var(--rose)", padding: "10px 14px" }}
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
                style={{ ...rowCardStyle, padding: "10px 14px", display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}
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

function truncate(text: string, maxLength: number): string {
  return text.length > maxLength ? `${text.slice(0, maxLength).trimEnd()}…` : text;
}

