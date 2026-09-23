// Mirrors atproto-iroh-core's tagging::latest_per_author_with_label /
// tagging::pins() exactly — "one per author, most recent wins, newest
// first" (DESIGN_BRIEF.md's Layout Strategy). The real (Tauri) backend
// path calls the `pins` command, which does this resolution in Rust
// already; this TS copy exists for the mock client (no Rust process to
// call) and is unit-tested directly against the canonical fixtures in
// pins.test.ts, the same property `tests/tagging.rs`'s
// `tags_can_tag_tags_and_pins_resolve_one_per_author_by_recency` proves
// on the Rust side. Keep the two in sync by hand — same caveat as the
// rest of api/types.ts.

import type { TagView } from "../api/types";

export function latestPerAuthorWithLabel(tags: TagView[], label: string): TagView[] {
  const latest = new Map<string, TagView>();
  for (const tag of tags) {
    if (tag.label !== label) continue;
    const existing = latest.get(tag.author_hex);
    if (!existing || tag.rkey > existing.rkey) {
      // rkey is a monotonic new_entry_key (namespace.rs) — string
      // comparison orders the same as creation order for same-length
      // keys, which is all `new_entry_key` ever produces. The real
      // Rust resolution compares `created_at` directly; this mock-only
      // stand-in doesn't carry timestamps on TagView (the real command
      // doesn't return them either — see TagView in types.ts), so rkey
      // recency is the best signal available here.
      latest.set(tag.author_hex, tag);
    }
  }
  return [...latest.values()].sort((a, b) => (a.rkey < b.rkey ? 1 : -1));
}
