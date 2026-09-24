// One "Shared doc" within a Table — DESIGN_BRIEF.md §4's fourth tab.
// A single fixed doc id per Table for now (no doc picker/creation UI
// yet — a real, named gap, not an oversight): every Table has exactly
// one shared doc, "notes".

import { useCallback, useEffect, useRef, useState } from "react";
import { api, type DocRevision } from "../api";

export const TABLE_DOC_ID = "notes";

interface TableDocState {
  loading: boolean;
  revisions: DocRevision[];
}

export function useTableDoc(tableId: string): TableDocState & { refresh: () => Promise<void> } {
  const [state, setState] = useState<TableDocState>({ loading: true, revisions: [] });

  // Same monotonic-request-id pattern as useTags.ts — lets the mount
  // effect just call refresh() (found duplicating the same fetch body
  // in a code-review pass) without losing the "a stale response can't
  // overwrite a fresher one" guarantee a boolean `cancelled` flag would
  // only have covered for the mount case, not a manual refresh() too.
  const requestId = useRef(0);
  const refresh = useCallback(async () => {
    const id = ++requestId.current;
    const revisions = await api.docHistory(tableId, TABLE_DOC_ID);
    if (id === requestId.current) setState({ loading: false, revisions });
  }, [tableId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { ...state, refresh };
}
