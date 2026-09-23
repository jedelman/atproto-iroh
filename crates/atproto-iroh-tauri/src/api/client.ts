// The command surface this frontend actually uses, as an interface —
// implemented once for real (tauriClient.ts, thin invoke() wrappers)
// and once against the canonical mock dataset (mockClient.ts, for
// `npm run dev` in a plain browser and for tests). Deliberately a
// subset of the 29 real Tauri commands (28 in CLAUDE.md's count plus
// `pins`, added this session) — this is the "Feed/Messages/Decisions
// first" slice DESIGN_BRIEF.md's Open Questions flagged, not full
// parity with every command yet (mute, doc-save/load/history, image
// upload/download, share/join/ticket_to_qr aren't wired into this
// layer in this pass).

import type {
  DocRevision,
  GovernanceStateView,
  ImageView,
  MessageView,
  NodeCategory,
  NodeProfile,
  ProfileView,
  ProposalView,
  SignalType,
  TagView,
} from "./types";

export interface TableSummary {
  id: string;
  /** Best-effort display name. Real backend has no "Table name" record
   * yet (DESIGN_BRIEF.md's Open Questions) — tauriClient's
   * implementation is an honest fallback (a truncated id), not a real
   * name; mockClient returns the canonical fixture names. */
  name: string;
}

export interface Client {
  spawnNode(): Promise<string>;
  nodeDid(): Promise<string | null>;
  listNamespaces(): Promise<string[]>;
  /** Every Table this node holds a capability into, with a display name. */
  listTables(): Promise<TableSummary[]>;
  /** Imports a Table from a shared ticket string (scanned or pasted).
   * Returns the new Table's id. */
  joinNamespace(ticket: string): Promise<string>;
  createNamespaceWithProfile(
    name: string,
    category: NodeCategory,
    avatar: string | null,
  ): Promise<string>;
  updateProfile(
    namespaceId: string,
    profile: Omit<NodeProfile, "created_at">,
  ): Promise<void>;
  listProfiles(namespaceId: string): Promise<ProfileView[]>;
  listMessages(namespaceId: string): Promise<MessageView[]>;
  /** Posts a message; no reply-to yet (threading isn't built in this
   * frontend pass — real gap, not silently dropped, see README). */
  sendMessage(namespaceId: string, text: string): Promise<string>;
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
  /** Posts a new decision/poll — hardcoded to `GovernanceClass::General`,
   * same relabeling the old plain-JS app's Polls section used
   * (CLAUDE.md's batteries-included list): "not a majority vote" is real,
   * not just copy — this ratifies by default absent enough Block
   * signals, same objection-window mechanic as every other decision
   * here. No subject-member/policy-change UI — that's real cosigner-
   * governance complexity out of scope for this pass. */
  createDecision(namespaceId: string, title: string, deadlineHours: number): Promise<string>;
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
}
