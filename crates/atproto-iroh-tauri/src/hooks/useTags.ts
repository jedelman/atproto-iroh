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

import { useCallback, useEffect, useState } from "react";
import { api, type TagView } from "../api";
import { PIN_LABEL } from "../api/types";

export function useTags(tableId: string) {
  const [loading, setLoading] = useState(true);
  const [tags, setTags] = useState<TagView[]>([]);

  const refresh = useCallback(async () => {
    const all = await api.listAllTags(tableId);
    setTags(all);
    setLoading(false);
  }, [tableId]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const all = await api.listAllTags(tableId);
      if (!cancelled) {
        setTags(all);
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [tableId]);

  // system:-prefixed labels (PIN_LABEL and anything else this crate
  // later reserves — tagging.rs's own top doc comment) are built-in
  // behavior, not user ontology — never surfaced as a browsable label.
  const userLabels = Array.from(
    new Set(tags.filter((t) => t.label !== PIN_LABEL && !t.label.startsWith("system:")).map((t) => t.label)),
  ).sort();

  const tagsBySubject = new Map<string, TagView[]>();
  for (const t of tags) {
    if (t.label === PIN_LABEL || t.label.startsWith("system:")) continue;
    const list = tagsBySubject.get(t.subject) ?? [];
    list.push(t);
    tagsBySubject.set(t.subject, list);
  }

  return { loading, tags, userLabels, tagsBySubject, refresh };
}
