// Internal-consistency checks on the canonical dataset itself — this is
// what a "canonical" mock dataset needs to keep being true as it grows:
// every reference in it actually resolves to something else in it.
// Catches the easy mistake (a typo'd table id, a message from an author
// with no profile) before it silently breaks a screen that trusts the
// data is coherent.

import { describe, expect, it } from "vitest";
import {
  AUTHORS,
  GOVERNANCE_STATE,
  IMAGES,
  MESSAGES,
  PROFILES,
  PROPOSALS,
  TABLES,
  TAGS,
} from "./fixtures";

const tableIds = new Set(TABLES.map((t) => t.id));
const authorHexes = new Set(Object.values(AUTHORS));

describe("canonical fixtures — internal consistency", () => {
  it("every Table has at least one member with a profile", () => {
    for (const table of TABLES) {
      expect(table.memberAuthorHexes.length).toBeGreaterThan(0);
      for (const author of table.memberAuthorHexes) {
        expect(PROFILES[author], `${author} (member of ${table.name}) has no profile`).toBeDefined();
      }
    }
  });

  it("every per-table content map is keyed by a real table id", () => {
    for (const map of [MESSAGES, IMAGES, PROPOSALS, GOVERNANCE_STATE, TAGS]) {
      for (const key of Object.keys(map)) {
        expect(tableIds.has(key), `${key} is not a real table id`).toBe(true);
      }
    }
  });

  it("every message/image/proposal/tag author is a known author", () => {
    for (const messages of Object.values(MESSAGES)) {
      for (const m of messages) expect(authorHexes.has(m.author_hex)).toBe(true);
    }
    for (const images of Object.values(IMAGES)) {
      for (const i of images) expect(authorHexes.has(i.author_hex)).toBe(true);
    }
    for (const proposals of Object.values(PROPOSALS)) {
      for (const p of proposals) expect(authorHexes.has(p.author_hex)).toBe(true);
    }
    for (const tags of Object.values(TAGS)) {
      for (const t of tags) expect(authorHexes.has(t.author_hex)).toBe(true);
    }
  });

  it("every author hex looks like a real record_ref-safe hex string", () => {
    for (const hex of authorHexes) {
      expect(hex).toMatch(/^[0-9a-f]+$/);
    }
  });
});
