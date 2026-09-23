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
  GovernanceStateView,
  ImageView,
  MessageView,
  NodeCategory,
  NodeProfile,
  ProfileView,
  ProposalView,
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
  listImages(namespaceId: string): Promise<ImageView[]>;
  listProposals(namespaceId: string): Promise<ProposalView[]>;
  governanceState(namespaceId: string): Promise<GovernanceStateView>;
  pins(namespaceId: string): Promise<TagView[]>;
  tagsFor(namespaceId: string, subject: string): Promise<TagView[]>;
  addTag(namespaceId: string, subject: string, label: string): Promise<string>;
}
