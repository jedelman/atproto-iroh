// Mock backend for `npm run dev` in a plain browser (no Tauri runtime —
// `window.__TAURI__` is absent, see index.ts) and for tests. Reads and
// mutates in-memory copies of the canonical fixtures
// (mocks/fixtures.ts) rather than the fixtures themselves, so repeated
// interactions during a dev session don't corrupt the module the tests
// also import.

import type { Client } from "./client";
import {
  DOC_REVISIONS,
  GOVERNANCE_STATE,
  IMAGES,
  MESSAGES,
  MOCK_TICKETS,
  PROFILES,
  PROPOSALS,
  SELF_AUTHOR_HEX,
  TABLES,
  TABLES_YOU_ARE_IN,
  TAGS,
} from "../mocks/fixtures";
import { latestPerAuthorWithLabel } from "../lib/pins";
import { PIN_LABEL } from "./types";
import type { DocRevision, NodeProfile } from "./types";

const profiles: Record<string, Record<string, NodeProfile>> = {};
for (const table of TABLES) {
  // Only this Table's real members — found via screenshot, not the
  // (passing) test suite: every table was seeding *all* PROFILES
  // regardless of TABLES[].memberAuthorHexes, so a Table detail screen
  // showed people who'd never joined it.
  profiles[table.id] = Object.fromEntries(
    table.memberAuthorHexes
      .filter((hex) => hex in PROFILES)
      .map((hex) => [hex, PROFILES[hex]]),
  );
}
const tags: Record<string, typeof TAGS[string]> = structuredClone(TAGS);
const messages: Record<string, typeof MESSAGES[string]> = structuredClone(MESSAGES);
const joinedTableIds = new Set<string>(TABLES_YOU_ARE_IN);
// Per-Table map of docId → revisions, oldest first — same shape doc_history
// returns for real, so useDocConflict's logic never has to know it's mocked.
const docs: Record<string, Record<string, DocRevision[]>> = structuredClone(DOC_REVISIONS);
const proposals: Record<string, typeof PROPOSALS[string]> = structuredClone(PROPOSALS);
const images: Record<string, typeof IMAGES[string]> = structuredClone(IMAGES);
// Real byte storage, keyed by `${namespaceId}/${author_hex}/${rkey}` —
// separate from `images` (metadata only) the same way the real backend
// keeps them as two entries sharing one rkey (namespace::put_bytes),
// so listing a gallery never means holding every image's bytes at once.
// The pre-existing fixture image (Sequoia's overlook photo) deliberately
// has no entry here — it was never "uploaded" through this session, so
// loadImageBytes honestly returns null for it, same as a real node that
// hasn't synced those bytes yet.
const imageBytes = new Map<string, number[]>();
// Not per-Table — mute::MuteList's doc comment: "local only, no sync,
// no lexicon," a reader's own client-side filter, same in the mock.
const mutedAuthors = new Set<string>();
const governance: Record<string, typeof GOVERNANCE_STATE[string]> = structuredClone(GOVERNANCE_STATE);
// Mutable copy so shareTable can mint tickets for Tables created this
// session and joinNamespace can redeem them.
const tickets: Record<string, string> = { ...MOCK_TICKETS };
// Names for Tables created this session (fixture Tables carry their own).
const createdTableNames: Record<string, string> = {};

// Mirrors the real backend's "call spawn_node first": found on the first
// device install, where nothing ever spawned the node and every command
// failed — invisible here until now because the mock never cared.
let nodeSpawned = false;
/** Test-only: put the mock back in its pre-spawn state. */
export function resetMockNodeForTests() {
  nodeSpawned = false;
}

function imageBytesKey(namespaceId: string, authorHex: string, rkey: string) {
  return `${namespaceId}/${authorHex}/${rkey}`;
}

let nextRkeySeq = 9000;
function mockRkey() {
  // Same fixed-width, chronologically-sortable shape as the real
  // `namespace::new_entry_key` — see lib/pins.ts's comment on why the
  // width has to stay constant.
  return String(Date.now() * 1000 + nextRkeySeq++).padStart(19, "0");
}

const impl: Client = {
  async spawnNode() {
    nodeSpawned = true;
    return `did:iroh:${SELF_AUTHOR_HEX}`;
  },

  async nodeDid() {
    return nodeSpawned ? `did:iroh:${SELF_AUTHOR_HEX}` : null;
  },

  async listNamespaces() {
    return nodeSpawned ? [...joinedTableIds] : [];
  },

  async listTables() {
    if (!nodeSpawned) return [];
    return [...joinedTableIds].map((id) => ({
      id,
      name: TABLES.find((t) => t.id === id)?.name ?? createdTableNames[id] ?? `Table ${id.slice(0, 8)}…`,
    }));
  },

  async joinNamespace(ticket) {
    const tableId = tickets[ticket.trim()];
    if (!tableId) {
      throw new Error("invalid ticket");
    }
    joinedTableIds.add(tableId);
    // Real `Node::join` doesn't add the joiner as a *member* — capability
    // and membership are different things here (SPEC.md §3.4) — so this
    // deliberately doesn't add SELF_AUTHOR_HEX to the table's profiles;
    // matches what a real join actually does.
    return tableId;
  },

  async createNamespaceWithProfile(name, category, avatar, tableName) {
    const id = `table-mock-${mockRkey()}`;
    joinedTableIds.add(id);
    // Founder is the sole genesis member, same as fold::found_namespace.
    governance[id] = { eligible_hex: [SELF_AUTHOR_HEX] };
    const trimmed = tableName?.trim();
    if (trimmed) createdTableNames[id] = trimmed;
    profiles[id] = {
      [SELF_AUTHOR_HEX]: {
        name,
        category,
        avatar: avatar ?? undefined,
        created_at: new Date().toISOString(),
      },
    };
    messages[id] = [];
    return id;
  },

  async shareTable(namespaceId, mode) {
    if (!joinedTableIds.has(namespaceId)) throw new Error("unknown namespace — has this node opened it?");
    const ticket = `mock-ticket-${mode.toLowerCase()}-${namespaceId}`;
    tickets[ticket] = namespaceId;
    return ticket;
  },

  async ticketToQr(ticket) {
    // No QR encoder in the frontend on purpose (the real one is Rust's
    // qrcode crate) — so the mock renders a labeled stand-in rather than
    // a fake code a phone might try, and fail, to scan.
    const label = ticket.length > 28 ? `${ticket.slice(0, 28)}…` : ticket;
    return `<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256"><rect width="256" height="256" fill="#ffffff"/><rect x="16" y="16" width="224" height="224" fill="none" stroke="#000" stroke-width="4" stroke-dasharray="10 8"/><text x="128" y="120" font-family="sans-serif" font-size="16" text-anchor="middle">QR (dev mock)</text><text x="128" y="148" font-family="monospace" font-size="9" text-anchor="middle">${label.replace(/[<>&"]/g, "")}</text></svg>`;
  },

  async updateProfile(namespaceId, profile) {
    const table = profiles[namespaceId];
    if (!table) throw new Error("unknown namespace — has this node opened it?");
    table[SELF_AUTHOR_HEX] = { ...profile, created_at: new Date().toISOString() };
  },

  async listProfiles(namespaceId) {
    const table = profiles[namespaceId];
    if (!table) return [];
    return Object.entries(table).map(([author_hex, profile]) => ({ author_hex, profile }));
  },

  async listMessages(namespaceId) {
    // A copy, not the live array — found via a real React bug this
    // session: mockClient methods that returned the mutable array
    // reference directly meant a subsequent push() mutated the exact
    // array a caller had already stored in state, so React's
    // Object.is bailout silently dropped the re-render even though
    // the underlying data had genuinely changed (useTags.ts's own
    // comment has the full story).
    return [...(messages[namespaceId] ?? [])];
  },

  async sendMessage(namespaceId, text, replyTo) {
    const rkey = mockRkey();
    const subject = `${SELF_AUTHOR_HEX}/network.essmesh.chat.message/${rkey}`;
    messages[namespaceId] ??= [];
    messages[namespaceId].push({
      author_hex: SELF_AUTHOR_HEX,
      rkey,
      subject,
      text,
      reply_to: replyTo ? `${replyTo.authorHex}/${replyTo.rkey}` : null,
      created_at: new Date().toISOString(),
    });
    return rkey;
  },

  async listImages(namespaceId) {
    return [...(images[namespaceId] ?? [])];
  },

  async uploadImage(namespaceId, bytes, contentType, caption) {
    const rkey = mockRkey();
    images[namespaceId] ??= [];
    images[namespaceId].push({
      author_hex: SELF_AUTHOR_HEX,
      rkey,
      subject: `${SELF_AUTHOR_HEX}/network.essmesh.chat.image/${rkey}`,
      content_type: contentType,
      len: bytes.length,
      caption: caption ?? undefined,
      created_at: new Date().toISOString(),
    });
    imageBytes.set(imageBytesKey(namespaceId, SELF_AUTHOR_HEX, rkey), bytes);
    return rkey;
  },

  async loadImageBytes(namespaceId, authorHex, rkey) {
    return imageBytes.get(imageBytesKey(namespaceId, authorHex, rkey)) ?? null;
  },

  async listProposals(namespaceId) {
    return [...(proposals[namespaceId] ?? [])];
  },

  async governanceState(namespaceId) {
    return governance[namespaceId] ?? { eligible_hex: [] };
  },

  async createDecision(namespaceId, title, deadlineHours, extra) {
    const rkey = mockRkey();
    proposals[namespaceId] ??= [];
    proposals[namespaceId].push({
      author_hex: SELF_AUTHOR_HEX,
      rkey,
      proposal: {
        title,
        class: extra?.class ?? "general",
        deadline: new Date(Date.now() + deadlineHours * 3_600_000).toISOString(),
        policy_change: extra?.policyChange ?? null,
        subject_member: extra?.subjectMemberHex ?? null,
        created_at: new Date().toISOString(),
      },
      status: { state: "open", blockers: [] },
    });
    return rkey;
  },

  async signalDecision(namespaceId, proposalAuthorHex, proposalRkey, signalType) {
    const rkey = mockRkey();
    const target = (proposals[namespaceId] ?? []).find(
      (p) => p.author_hex === proposalAuthorHex && p.rkey === proposalRkey,
    );
    // A real ratification depends on the full fold (eligibility,
    // per-class threshold, and whether the deadline has passed —
    // fold::fold_namespace) — this mock only simulates the one signal
    // this UI actually acts on immediately: a Block puts the decision
    // in a visibly-objected state. Consent doesn't fast-forward
    // ratification here, same as the real thing (it only ratifies once
    // the deadline passes with too few blockers).
    if (target && signalType === "block" && target.status.state !== "ratified") {
      const blockers = target.status.state === "blocked" ? target.status.blockers : [];
      if (!blockers.includes(SELF_AUTHOR_HEX)) blockers.push(SELF_AUTHOR_HEX);
      target.status = { state: "blocked", blockers };
    }
    return rkey;
  },

  async pins(namespaceId) {
    return latestPerAuthorWithLabel(tags[namespaceId] ?? [], PIN_LABEL);
  },

  async tagsFor(namespaceId, subject) {
    return (tags[namespaceId] ?? []).filter((t) => t.subject === subject);
  },

  async addTag(namespaceId, subject, label) {
    const rkey = mockRkey();
    tags[namespaceId] ??= [];
    tags[namespaceId].push({ author_hex: SELF_AUTHOR_HEX, rkey, subject, label });
    return rkey;
  },

  async listAllTags(namespaceId) {
    return [...(tags[namespaceId] ?? [])];
  },

  async docSave(namespaceId, docId, text) {
    const rev = mockRkey();
    docs[namespaceId] ??= {};
    docs[namespaceId][docId] ??= [];
    docs[namespaceId][docId].push({
      author_hex: SELF_AUTHOR_HEX,
      rev,
      text,
      subject: `${SELF_AUTHOR_HEX}/network.essmesh.namespace.doc.${docId}.rev/${rev}`,
    });
    return rev;
  },

  async docLoad(namespaceId, docId) {
    const revisions = docs[namespaceId]?.[docId] ?? [];
    return revisions.length ? revisions[revisions.length - 1] : null;
  },

  async docHistory(namespaceId, docId) {
    return [...(docs[namespaceId]?.[docId] ?? [])];
  },

  async muteAuthor(authorHex) {
    mutedAuthors.add(authorHex);
  },

  async unmuteAuthor(authorHex) {
    mutedAuthors.delete(authorHex);
  },

  async listMuted() {
    return [...mutedAuthors];
  },
};

const UNGUARDED = new Set<string>([
  "spawnNode",
  "nodeDid",
  "listNamespaces",
  "listTables",
  "ticketToQr",
  "muteAuthor",
  "unmuteAuthor",
  "listMuted",
]);

export const mockClient: Client = new Proxy(impl, {
  get(target, prop, receiver) {
    const value = Reflect.get(target, prop, receiver);
    if (typeof value !== "function" || UNGUARDED.has(String(prop))) return value;
    return async (...args: unknown[]) => {
      if (!nodeSpawned) throw new Error("call spawn_node first");
      return (value as (...a: unknown[]) => unknown).apply(target, args);
    };
  },
});
