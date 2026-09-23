// Mirrors the Rust shapes this frontend actually consumes —
// atproto-iroh-core's records.rs/governance.rs/tagging.rs/messaging.rs/
// images.rs and the Tauri command *View structs in src-tauri/src/lib.rs.
// Kept hand-written rather than codegen'd (no ts-rs/specta wiring exists
// yet) — a real drift risk, same as the CATEGORY_WEIGHTS/NodeCategory
// sync-by-hand note in records.rs; worth a real typegen pass once this
// frontend is past its first slice.

export type NodeCategory =
  | "cooperative"
  | "collective"
  | "mutual_aid"
  | "nonprofit"
  | "informal_group"
  | "individual"
  | "other";

export interface NodeProfile {
  name: string;
  category: NodeCategory;
  neighborhood?: string | null;
  description?: string | null;
  /** An id into the client's built-in sticker set — see components/Sticker.tsx. */
  avatar?: string | null;
  governance_eligible?: boolean | null;
  created_at: string; // RFC 3339
}

export interface ProfileView {
  author_hex: string;
  profile: NodeProfile;
}

/** `list_messages`'s exact Tauri return shape (`MessageView` in lib.rs) —
 * flat, not nested under a `message` key; `subject` is a ready-made
 * `record_ref` the frontend can pass straight to `add_tag`/`tags_for`/`pins`
 * without knowing `Message::COLLECTION` itself. */
export interface MessageView {
  author_hex: string;
  rkey: string;
  subject: string;
  text: string;
  reply_to?: string | null;
  created_at: string;
}

/** `list_images`'s exact Tauri return shape (`ImageView` in lib.rs) — flat. */
export interface ImageView {
  author_hex: string;
  rkey: string;
  content_type: string;
  len: number;
  caption?: string | null;
  created_at: string;
}

export type GovernanceClass =
  | "admitCoSigner"
  | "removeCoSigner"
  | "changePolicy"
  | "general";

export type SignalType = "consent" | "stand_aside" | "block" | "abstain" | "exit";

export interface PolicyValue {
  window_seconds: number;
  block_threshold: number;
}

export interface PolicyChange {
  admit_co_signer?: PolicyValue | null;
  remove_co_signer?: PolicyValue | null;
  change_policy?: PolicyValue | null;
}

export interface Proposal {
  title: string;
  description?: string | null;
  class: GovernanceClass;
  block_threshold?: number | null;
  deadline: string;
  policy_change?: PolicyChange | null;
  subject_member?: string | null;
  created_at: string;
}

export type RatificationView =
  | { state: "open"; blockers: string[] }
  | { state: "ratified" }
  | { state: "blocked"; blockers: string[] };

export interface ProposalView {
  author_hex: string;
  rkey: string;
  proposal: Proposal;
  status: RatificationView;
}

export interface GovernanceStateView {
  eligible_hex: string[];
}

/** `tagging::Tag` unwrapped by both `tags_for` and `pins` — same TagView shape. */
export interface TagView {
  author_hex: string;
  rkey: string;
  subject: string;
  label: string;
}

/** Reserved label this crate gives built-in meaning to — mirrors
 * `tagging::PIN_LABEL` in atproto-iroh-core. Keep these in sync by hand,
 * same caveat as the rest of this file. */
export const PIN_LABEL = "system:pin";

export interface DocRevision {
  author_hex: string;
  rev: string;
  text: string;
}
