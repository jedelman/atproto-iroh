// Every tag in a Table, across every subject — the data a Tagging UI
// needs both to show "what's this message already tagged with" and to
// build a "browse by tag" filter, neither of which `tagsFor` alone (one
// subject at a time) can answer.
//
// A real bug found building this, not hypothetical: mockClient's list
// methods originally returned the live mutable array reference itself
// rather than a copy, so a later push() mutated the exact array this
// hook had already stored in state — React's Object.is bailout then
// silently skipped the re-render even though the underlying tag list
// had genuinely grown. Fixed in mockClient.ts (every list* method now
// returns a fresh array); this hook doesn't need to work around it, but
// the failure mode is worth knowing if a future client implementation
// reintroduces it: a refresh() that "does nothing" visibly despite the
// backend call succeeding is the symptom.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type TagView } from "../api";
import { usePoll } from "./usePoll";
import { PIN_LABEL } from "../api/types";

function isSystemLabel(label: string): boolean {
  return label === PIN_LABEL || label.startsWith("system:");
}

export function useTags(tableId: string) {
  const [loading, setLoading] = useState(true);
  const [tags, setTags] = useState<TagView[]>([]);

  // A monotonic request id rather than a boolean `cancelled` flag: it
  // covers both the auto-fetch-on-mount/tableId-change case *and* a
  // manually-triggered refresh() racing a still-in-flight one, so the
  // effect can just call refresh() (found duplicating the same fetch
  // body in a code-review pass) without losing the "don't let a stale
  // response overwrite a fresher one" guarantee the original mount
  // effect had.
  const requestId = useRef(0);
  const refresh = useCallback(async () => {
    const id = ++requestId.current;
    const all = await api.listAllTags(tableId);
    if (id === requestId.current) {
      setTags(all);
      setLoading(false);
    }
  }, [tableId]);

  useEffect(() => {
    refresh();
  }, [refresh]);
  // Same cadence as useTable — other members' tags and pins arrive
  // while the Table is open, not only on the next visit.
  usePoll(refresh, 5_000);

  // system:-prefixed labels (PIN_LABEL and anything else this crate
  // later reserves — tagging.rs's own top doc comment) are built-in
  // behavior, not user ontology — never surfaced as a browsable label.
  const userLabels = useMemo(
    () => Array.from(new Set(tags.filter((t) => !isSystemLabel(t.label)).map((t) => t.label))).sort(),
    [tags],
  );

  const tagsBySubject = useMemo(() => {
    const map = new Map<string, TagView[]>();
    for (const t of tags) {
      if (isSystemLabel(t.label)) continue;
      const list = map.get(t.subject) ?? [];
      list.push(t);
      map.set(t.subject, list);
    }
    return map;
  }, [tags]);

  return { loading, tags, userLabels, tagsBySubject, refresh };
}
