// Mock backend for `npm run dev` in a plain browser (no Tauri runtime —
// `window.__TAURI__` is absent, see index.ts) and for tests. Reads and
// mutates in-memory copies of the canonical fixtures
// (mocks/fixtures.ts) rather than the fixtures themselves, so repeated
// interactions during a dev session don't corrupt the module the tests
// also import.

import type { Client } from "./client";
import {
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
import type { NodeProfile } from "./types";

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

let nextRkeySeq = 9000;
function mockRkey() {
  // Same fixed-width, chronologically-sortable shape as the real
  // `namespace::new_entry_key` — see lib/pins.ts's comment on why the
  // width has to stay constant.
  return String(Date.now() * 1000 + nextRkeySeq++).padStart(19, "0");
}

export const mockClient: Client = {
  async spawnNode() {
    return `did:iroh:${SELF_AUTHOR_HEX}`;
  },

  async nodeDid() {
    return `did:iroh:${SELF_AUTHOR_HEX}`;
  },

  async listNamespaces() {
    return [...joinedTableIds];
  },

  async listTables() {
    return TABLES.filter((t) => joinedTableIds.has(t.id)).map(({ id, name }) => ({ id, name }));
  },

  async joinNamespace(ticket) {
    const tableId = MOCK_TICKETS[ticket.trim()];
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

  async createNamespaceWithProfile(name, category, avatar) {
    const id = `table-mock-${mockRkey()}`;
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
    return messages[namespaceId] ?? [];
  },

  async sendMessage(namespaceId, text) {
    const rkey = mockRkey();
    const subject = `${SELF_AUTHOR_HEX}/network.essmesh.chat.message/${rkey}`;
    messages[namespaceId] ??= [];
    messages[namespaceId].push({
      author_hex: SELF_AUTHOR_HEX,
      rkey,
      subject,
      text,
      reply_to: null,
      created_at: new Date().toISOString(),
    });
    return rkey;
  },

  async listImages(namespaceId) {
    return IMAGES[namespaceId] ?? [];
  },

  async listProposals(namespaceId) {
    return PROPOSALS[namespaceId] ?? [];
  },

  async governanceState(namespaceId) {
    return GOVERNANCE_STATE[namespaceId] ?? { eligible_hex: [] };
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
};
