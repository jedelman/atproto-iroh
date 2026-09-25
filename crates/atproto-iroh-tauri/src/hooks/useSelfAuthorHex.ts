// The "hold self author hex in state, resolved once on mount, with
// unmount-cancellation" pattern — Feed's header avatar and
// DecisionsPanel's eligibility check each reimplemented this exact
// 8-10 line effect separately (found in a code-review pass), even
// though the plain resolve call itself (resolveSelfAuthorHex) was
// already centralized once for the same reason.

import { useEffect, useState } from "react";
import { api } from "../api";
import { resolveSelfAuthorHex } from "../lib/identity";

export function useSelfAuthorHex(): string | null {
  const [selfAuthorHex, setSelfAuthorHex] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const hex = await resolveSelfAuthorHex(api);
      if (!cancelled) setSelfAuthorHex(hex);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return selfAuthorHex;
}
