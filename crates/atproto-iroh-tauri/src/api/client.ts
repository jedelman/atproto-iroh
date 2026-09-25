// The command surface this frontend actually uses, as an interface —
// implemented once for real (tauriClient.ts, thin invoke() wrappers)
// and once against the canonical mock dataset (mockClient.ts, for
// `npm run dev` in a plain browser and for tests). Deliberately a
// subset of the 30 real Tauri commands (28 in CLAUDE.md's count plus
// `pins`/`list_all_tags`, added this session) — this is the
// "Feed/Messages/Decisions first" slice DESIGN_BRIEF.md's Open
// Questions flagged, not full parity with every command yet
// (share/join/ticket_to_qr generation, the raw inspector, and
// relay/control aren't wired into this layer).

import type {
  DocRevision,
  GovernanceClass,
  GovernanceStateView,
  ImageView,
  MessageView,
  NodeCategory,
  NodeProfile,
  PolicyChange,
  ProfileView,
  ProposalView,
  SignalType,
  TagView,
} from "./types";

export interface TableSummary {
  id: string;
  /** A genesis member's `table/name` entry (`fold::read_table_name`),
   * or a truncated id when there isn't one — never an invented name. */
  name: string;
}

export interface Client {
  /** Starts (or, if already running, no-ops) this device's node and
   * reloads every Table it holds. Every other call that touches a Table
   * fails until this has resolved once — App.tsx gates the whole UI on
   * it. `nodeDid`/`listNamespaces`/`listTables` answer null/empty before
   * it instead of failing, same as the real backend. */
  spawnNode(): Promise<string>;
  nodeDid(): Promise<string | null>;
  listNamespaces(): Promise<string[]>;
  /** Every Table this node holds a capability into, with a display name. */
  listTables(): Promise<TableSummary[]>;
  /** Imports a Table from a shared ticket string (scanned or pasted).
   * Returns the new Table's id. */
  joinNamespace(ticket: string): Promise<string>;
  /** Founds a new Table: your profile, a `Founding` claim naming you as
   * its sole genesis member, and (optionally) its display name — see
   * `fold::read_table_name` for why only a genesis member's name counts. */
  createNamespaceWithProfile(
    name: string,
    category: NodeCategory,
    avatar: string | null,
    tableName?: string,
  ): Promise<string>;
  /** A join ticket for this Table. "Write" is what an invited member
   * needs to post; "Read" suits a relay. A ticket is a bearer secret
   * (SPEC.md §3.4): whoever holds it has that access. */
  shareTable(namespaceId: string, mode: "Read" | "Write"): Promise<string>;
  /** Renders a ticket as SVG markup (the Rust `qrcode` crate). */
  ticketToQr(ticket: string): Promise<string>;
  updateProfile(
    namespaceId: string,
    profile: Omit<NodeProfile, "created_at">,
  ): Promise<void>;
  listProfiles(namespaceId: string): Promise<ProfileView[]>;
  listMessages(namespaceId: string): Promise<MessageView[]>;
  /** Posts a message — `replyTo` is optional and, when given, becomes
   * `Message.reply_to` (`messaging::reply_ref`'s
   * `"{author_hex}/{rkey}"` convention, distinct from `record_ref`'s
   * tagging-oriented `"{author_hex}/{collection}/{rkey}"`). */
  sendMessage(
    namespaceId: string,
    text: string,
    replyTo?: { authorHex: string; rkey: string },
  ): Promise<string>;
  listImages(namespaceId: string): Promise<ImageView[]>;
  /** Uploads an image — bytes cross the Tauri IPC boundary as a plain
   * number array (no separate binary-transfer path in this reference
   * app; fine at reasonable sizes, not tuned for large files). Returns
   * the new image's rkey. */
  uploadImage(
    namespaceId: string,
    bytes: number[],
    contentType: string,
    caption: string | null,
  ): Promise<string>;
  /** One image's raw bytes, or null if this node hasn't synced them yet
   * — never a silent placeholder. */
  loadImageBytes(namespaceId: string, authorHex: string, rkey: string): Promise<number[] | null>;
  listProposals(namespaceId: string): Promise<ProposalView[]>;
  governanceState(namespaceId: string): Promise<GovernanceStateView>;
  /** Posts a new decision — General-class defaults to the "Poll"
   * relabeling the old plain-JS app used (CLAUDE.md's batteries-included
   * list): "not a majority vote" is real, not just copy — this ratifies
   * by default absent enough Block signals, same objection-window
   * mechanic as every other decision here. `subjectMemberHex` (required
   * on admitCoSigner/removeCoSigner) and `policyChange` (required on
   * changePolicy) are optional and ignored on General, matching
   * `Proposal`'s own optionality. */
  createDecision(
    namespaceId: string,
    title: string,
    deadlineHours: number,
    extra?: { class?: GovernanceClass; subjectMemberHex?: string; policyChange?: PolicyChange },
  ): Promise<string>;
  /** Signals on a decision — `proposalAuthorHex`/`proposalRkey` name the
   * target since `RecordIdentifier` is `(namespace, author, key)`,
   * SPEC.md §3.4, not just a key. */
  signalDecision(
    namespaceId: string,
    proposalAuthorHex: string,
    proposalRkey: string,
    signalType: SignalType,
  ): Promise<string>;
  pins(namespaceId: string): Promise<TagView[]>;
  tagsFor(namespaceId: string, subject: string): Promise<TagView[]>;
  addTag(namespaceId: string, subject: string, label: string): Promise<string>;
  /** Every tag in the namespace, across every subject — the "browse by
   * tag" hook `tagsFor` can't answer, since it only knows one subject
   * at a time. */
  listAllTags(namespaceId: string): Promise<TagView[]>;
  /** Writes a new, immutable revision — never overwrites a prior one.
   * See `namespace::save_document_revision`'s doc comment for the CRDT
   * data-loss case this replaced. Returns the new revision's rev key. */
  docSave(namespaceId: string, docId: string, text: string): Promise<string>;
  /** The latest revision by key order — a default for "what to show
   * right now," not a claim it's the semantically correct pick if two
   * edits landed close together. Pair with docHistory for that. */
  docLoad(namespaceId: string, docId: string): Promise<DocRevision | null>;
  /** Every revision, oldest first — the UI's hook for surfacing "someone
   * else edited this while you were offline" instead of silently
   * picking a side. */
  docHistory(namespaceId: string, docId: string): Promise<DocRevision[]>;
  /** Mutes an author's content in this reader's own client — local
   * only, no sync, not scoped to any one Table (`mute::MuteList`'s doc
   * comment). */
  muteAuthor(authorHex: string): Promise<void>;
  unmuteAuthor(authorHex: string): Promise<void>;
  listMuted(): Promise<string[]>;
}
