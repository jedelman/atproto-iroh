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
  PROFILES,
  PROPOSALS,
  SELF_AUTHOR_HEX,
  TABLES,
  TAGS,
} from "../mocks/fixtures";
import { latestPerAuthorWithLabel } from "../lib/pins";
import { PIN_LABEL } from "./types";
import type { NodeProfile } from "./types";

const profiles: Record<string, Record<string, NodeProfile>> = {};
for (const table of TABLES) {
  profiles[table.id] = { ...PROFILES };
}
const tags: Record<string, typeof TAGS[string]> = structuredClone(TAGS);
const messages: Record<string, typeof MESSAGES[string]> = structuredClone(MESSAGES);

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
    return TABLES.map((t) => t.id);
  },

  async listTables() {
    return TABLES.map(({ id, name }) => ({ id, name }));
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
