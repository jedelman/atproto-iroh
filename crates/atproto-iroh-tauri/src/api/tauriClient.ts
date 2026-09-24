// Real backend: thin wrappers over `invoke()`. Command names are the
// literal Rust fn names (src-tauri/src/lib.rs); argument keys are
// camelCase — Tauri v2's command macro defaults to `ArgumentCase::Camel`
// and converts to each command's real snake_case parameter name
// (confirmed against the vendored tauri-macros source, not assumed).

import { invoke } from "@tauri-apps/api/core";
import type { Client } from "./client";
import type {
  DocRevision,
  GovernanceStateView,
  ImageView,
  MessageView,
  NodeProfile,
  ProfileView,
  ProposalView,
  TagView,
} from "./types";

// doc_load/doc_history return (rev, author_hex, text) tuples on the
// Rust side (src-tauri/src/lib.rs) — reshaped to DocRevision here so
// the rest of the frontend never has to know the wire shape.
function tupleToRevision([rev, author_hex, text]: [string, string, string]): DocRevision {
  return { rev, author_hex, text };
}

export const tauriClient: Client = {
  spawnNode: () => invoke("spawn_node"),
  nodeDid: () => invoke("node_did"),
  listNamespaces: () => invoke("list_namespaces"),
  listTables: async () => {
    const ids = await invoke<string[]>("list_namespaces");
    // Honest fallback — see TableSummary's doc comment: no real
    // "Table name" record exists in the backend yet.
    return ids.map((id) => ({ id, name: `Table ${id.slice(0, 8)}…` }));
  },
  joinNamespace: (ticket) => invoke<string>("join_namespace", { ticket }),
  createNamespaceWithProfile: (name, category, avatar) =>
    invoke("create_namespace_with_profile", { name, category, avatar }),
  updateProfile: (namespaceId, profile: Omit<NodeProfile, "created_at">) =>
    invoke("update_profile", {
      namespaceId,
      name: profile.name,
      category: profile.category,
      neighborhood: profile.neighborhood ?? null,
      description: profile.description ?? null,
      avatar: profile.avatar ?? null,
      governanceEligible: profile.governance_eligible ?? null,
    }),
  listProfiles: (namespaceId) =>
    invoke<ProfileView[]>("list_profiles", { namespaceId }),
  listMessages: (namespaceId) =>
    invoke<MessageView[]>("list_messages", { namespaceId }),
  sendMessage: (namespaceId, text, replyTo) =>
    invoke<string>("send_message", {
      namespaceId,
      text,
      replyToAuthorHex: replyTo?.authorHex ?? null,
      replyToRkey: replyTo?.rkey ?? null,
    }),
  listImages: (namespaceId) => invoke<ImageView[]>("list_images", { namespaceId }),
  uploadImage: (namespaceId, bytes, contentType, caption) =>
    invoke<string>("upload_image", { namespaceId, bytes, contentType, caption }),
  loadImageBytes: (namespaceId, authorHex, rkey) =>
    invoke<number[] | null>("load_image_bytes", { namespaceId, authorHex, rkey }),
  listProposals: (namespaceId) =>
    invoke<ProposalView[]>("list_proposals", { namespaceId }),
  governanceState: (namespaceId) =>
    invoke<GovernanceStateView>("governance_state", { namespaceId }),
  createDecision: (namespaceId, title, deadlineHours, extra) =>
    invoke<string>("create_proposal", {
      namespaceId,
      title,
      description: null,
      class: extra?.class ?? "general",
      deadlineHours,
      subjectMemberHex: extra?.subjectMemberHex ?? null,
      policyChange: extra?.policyChange ?? null,
    }),
  signalDecision: (namespaceId, proposalAuthorHex, proposalRkey, signalType) =>
    invoke<string>("create_signal", {
      namespaceId,
      proposalAuthorHex,
      proposalRkey,
      signalType,
      text: null,
    }),
  pins: (namespaceId) => invoke<TagView[]>("pins", { namespaceId }),
  tagsFor: (namespaceId, subject) =>
    invoke<TagView[]>("tags_for", { namespaceId, subject }),
  addTag: (namespaceId, subject, label) =>
    invoke<string>("add_tag", { namespaceId, subject, label }),
  listAllTags: (namespaceId) => invoke<TagView[]>("list_all_tags", { namespaceId }),
  docSave: (namespaceId, docId, text) =>
    invoke<string>("doc_save", { namespaceId, docId, text }),
  docLoad: async (namespaceId, docId) => {
    const result = await invoke<[string, string, string] | null>("doc_load", {
      namespaceId,
      docId,
    });
    return result ? tupleToRevision(result) : null;
  },
  docHistory: async (namespaceId, docId) => {
    const revisions = await invoke<[string, string, string][]>("doc_history", {
      namespaceId,
      docId,
    });
    return revisions.map(tupleToRevision);
  },
  muteAuthor: (authorHex) => invoke("mute_author", { authorHex }),
  unmuteAuthor: (authorHex) => invoke("unmute_author", { authorHex }),
  listMuted: () => invoke<string[]>("list_muted"),
};
