// Mirrors the property atproto-iroh-core's tests/tagging.rs proves on
// the Rust side (`tags_can_tag_tags_and_pins_resolve_one_per_author_by_
// recency`): one pin per author, most recent wins, newest first. This
// runs against the same canonical fixture content (Marisol pins twice,
// Devon tags something unrelated), so a change to either side's
// resolution logic that breaks the property shows up here without a
// live iroh node.

import { describe, expect, it } from "vitest";
import { latestPerAuthorWithLabel } from "./pins";
import { AUTHORS, TAGS } from "../mocks/fixtures";
import { PIN_LABEL } from "../api/types";

const GARDEN_TABLE_TAGS = TAGS[Object.keys(TAGS)[0]];

describe("latestPerAuthorWithLabel", () => {
  it("resolves one pin per author, the most recent, newest first", () => {
    const resolved = latestPerAuthorWithLabel(GARDEN_TABLE_TAGS, PIN_LABEL);

    expect(resolved).toHaveLength(1);
    expect(resolved[0].author_hex).toBe(AUTHORS.marisol);
    // Marisol's fixture pins are tag-0001 then tag-0002 — the later one
    // (rkey tag-0002) must be the one that resolves.
    expect(resolved[0].rkey).toBe("tag-0002");
  });

  it("ignores tags with a different label entirely", () => {
    const resolved = latestPerAuthorWithLabel(GARDEN_TABLE_TAGS, "harvest");
    expect(resolved).toHaveLength(1);
    expect(resolved[0].author_hex).toBe(AUTHORS.devon);
  });

  it("returns nothing for a label that was never used", () => {
    expect(latestPerAuthorWithLabel(GARDEN_TABLE_TAGS, "system:does-not-exist")).toEqual([]);
  });

  it("orders multiple authors' pins newest-rkey-first", () => {
    const tags = [
      { author_hex: "a", rkey: "0000000000000000001", subject: "s1", label: PIN_LABEL },
      { author_hex: "b", rkey: "0000000000000000003", subject: "s2", label: PIN_LABEL },
      { author_hex: "c", rkey: "0000000000000000002", subject: "s3", label: PIN_LABEL },
    ];
    const resolved = latestPerAuthorWithLabel(tags, PIN_LABEL);
    expect(resolved.map((t) => t.author_hex)).toEqual(["b", "c", "a"]);
  });
});
