// Ported from the old plain-JS "Shared doc" panel (dist/main.js, before
// the React rebuild) — a real concurrent-edit signal, not just "there's
// more than one revision" (a single person saving twice in a row is
// normal, not a conflict). The two most recent revisions came from
// *different authors* within 5 minutes of each other.

import type { DocRevision } from "../api/types";

// Revisions are keyed by new_entry_key()'s sortable, zero-padded
// microsecond-timestamp string (namespace.rs) — parseable back into a
// BigInt for exactly this: deciding whether two revisions were close
// enough in time to have plausibly been concurrent, offline edits
// rather than one person's own sequential saves.
const CONCURRENT_WINDOW_MICROS = 5n * 60n * 1_000_000n;

/** True if the two most recent revisions (oldest-first order, same as
 * `doc_history`) look like a concurrent offline edit rather than one
 * person's own sequential saves. */
export function hasPossibleConflict(revisions: DocRevision[]): boolean {
  if (revisions.length < 2) return false;
  const secondLast = revisions[revisions.length - 2];
  const last = revisions[revisions.length - 1];
  if (secondLast.author_hex === last.author_hex) return false;
  const gap = BigInt(last.rev) - BigInt(secondLast.rev);
  return gap >= 0n && gap <= CONCURRENT_WINDOW_MICROS;
}
