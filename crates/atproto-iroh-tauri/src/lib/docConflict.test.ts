import { describe, expect, it } from "vitest";
import { hasPossibleConflict } from "./docConflict";
import type { DocRevision } from "../api/types";

const BASE = 1_700_000_000_000_000;
function rev(offsetMicros: number, author: string, text = "x"): DocRevision {
  return { author_hex: author, rev: String(BASE + offsetMicros).padStart(19, "0"), text };
}

describe("hasPossibleConflict", () => {
  it("is false with fewer than two revisions", () => {
    expect(hasPossibleConflict([])).toBe(false);
    expect(hasPossibleConflict([rev(0, "alice")])).toBe(false);
  });

  it("is false when the same author saves twice in a row, however close", () => {
    const revisions = [rev(0, "alice"), rev(1000, "alice")];
    expect(hasPossibleConflict(revisions)).toBe(false);
  });

  it("is true when different authors save within the 5-minute window", () => {
    const revisions = [rev(0, "alice"), rev(120_000_000, "bob")]; // 120s apart
    expect(hasPossibleConflict(revisions)).toBe(true);
  });

  it("is false when different authors save more than 5 minutes apart", () => {
    const revisions = [rev(0, "alice"), rev(600_000_000, "bob")]; // 600s apart
    expect(hasPossibleConflict(revisions)).toBe(false);
  });

  it("only looks at the two most recent revisions, not the whole history", () => {
    const revisions = [
      rev(0, "alice"),
      rev(120_000_000, "bob"), // would conflict with alice, but isn't the tail anymore
      rev(240_000_000, "bob"), // same author as the one before it — no conflict
    ];
    expect(hasPossibleConflict(revisions)).toBe(false);
  });
});
