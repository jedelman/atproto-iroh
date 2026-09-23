// Holds the onboarding pick (name + sticker) between the Onboarding
// screen and whichever comes next — creating or joining the first
// Table, since NodeProfile is written per-Table (there's no single
// global profile to write it to immediately). Backed by localStorage:
// a genuine per-device convenience (the draft, not synced state), same
// category of storage this codebase's own artifact-authoring guidance
// reserves it for — never treated as the source of truth once a real
// Table exists.

import { useCallback, useState } from "react";
import type { StickerId } from "../components/Sticker";

export interface ProfileDraft {
  name: string;
  avatar: StickerId | null;
}

const KEY = "atproto-iroh:profile-draft";

function load(): ProfileDraft {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return JSON.parse(raw) as ProfileDraft;
  } catch {
    // ignore — falls through to default
  }
  return { name: "", avatar: null };
}

export function useProfileDraft() {
  const [draft, setDraftState] = useState<ProfileDraft>(load);

  const setDraft = useCallback((next: ProfileDraft) => {
    setDraftState(next);
    try {
      localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      // best-effort only — see this file's top comment
    }
  }, []);

  return { draft, setDraft };
}
