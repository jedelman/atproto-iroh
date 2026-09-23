// The muted-author set, for screens that render other people's content
// and need to filter it out — global, not per-Table (mute::MuteList's
// own doc comment).

import { useEffect, useState } from "react";
import { api } from "../api";

export function useMutedAuthors(): Set<string> {
  const [muted, setMuted] = useState<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const list = await api.listMuted();
      if (!cancelled) setMuted(new Set(list));
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return muted;
}
