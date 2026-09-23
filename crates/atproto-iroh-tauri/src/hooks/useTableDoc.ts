// One "Shared doc" within a Table — DESIGN_BRIEF.md §4's fourth tab.
// A single fixed doc id per Table for now (no doc picker/creation UI
// yet — a real, named gap, not an oversight): every Table has exactly
// one shared doc, "notes".

import { useCallback, useEffect, useState } from "react";
import { api, type DocRevision } from "../api";

export const TABLE_DOC_ID = "notes";

interface TableDocState {
  loading: boolean;
  revisions: DocRevision[];
}

export function useTableDoc(tableId: string): TableDocState & { refresh: () => Promise<void> } {
  const [state, setState] = useState<TableDocState>({ loading: true, revisions: [] });

  const refresh = useCallback(async () => {
    const revisions = await api.docHistory(tableId, TABLE_DOC_ID);
    setState({ loading: false, revisions });
  }, [tableId]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const revisions = await api.docHistory(tableId, TABLE_DOC_ID);
      if (!cancelled) setState({ loading: false, revisions });
    })();
    return () => {
      cancelled = true;
    };
  }, [tableId]);

  return { ...state, refresh };
}
