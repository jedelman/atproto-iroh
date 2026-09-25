// Re-fetch while a screen is open — the simplest honest answer to "posts
// from the other phone never show up until I leave and come back." Runs
// `callback` every `intervalMs` while the app is visible, and once right
// away whenever it comes back to the foreground (Android WebViews fire
// visibilitychange on app switch). Pauses while hidden rather than
// polling a backgrounded app. Push-on-arrival (subscribing to iroh-docs'
// LiveEvents) can replace this later — see CLAUDE.md's connectivity
// design, which needs the same event stream.

import { useEffect, useRef } from "react";

export function usePoll(callback: () => void, intervalMs: number) {
  const saved = useRef(callback);
  useEffect(() => {
    saved.current = callback;
  }, [callback]);

  useEffect(() => {
    const tick = () => {
      if (!document.hidden) saved.current();
    };
    const id = setInterval(tick, intervalMs);
    document.addEventListener("visibilitychange", tick);
    return () => {
      clearInterval(id);
      document.removeEventListener("visibilitychange", tick);
    };
  }, [intervalMs]);
}
