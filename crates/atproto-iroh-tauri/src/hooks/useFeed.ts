// Aggregates every Table's content into one merged timeline —
// DESIGN_BRIEF.md's Layout Strategy: "the fix... is one unified
// timeline across everything you're in, with Table and tag as filters
// on that timeline, not separate destinations." Simplest option first,
// per the brief's own Open Questions: N per-Table client-side calls
// merged in JS, not a new aggregating Tauri command — worth revisiting
// once real usage shows whether that matters.

import { useCallback, useEffect, useRef, useState } from "react";
import { usePoll } from "./usePoll";
import {
  api,
  type ImageView,
  type MessageView,
  type NodeProfile,
  type ProposalView,
  type TagView,
} from "../api";
import type { TableSummary } from "../api/client";

// No "pin" variant here on purpose: a pin isn't its own feed item, it's
// a label on an existing message (surfaced separately as the featured
// excerpt in Feed.tsx) — modeling it as a fourth FeedItem kind would
// mean the same message could appear twice in one timeline.
export type FeedItem =
  | { kind: "message"; id: string; tableId: string; createdAt: string; message: MessageView }
  | { kind: "photo"; id: string; tableId: string; createdAt: string; image: ImageView }
  | { kind: "decision"; id: string; tableId: string; createdAt: string; proposal: ProposalView };

interface FeedState {
  loading: boolean;
  tables: TableSummary[];
  items: FeedItem[];
  pinsByTable: Record<string, TagView[]>;
  /** Merged across every Table this node is in — last write wins if the
   * same author has different profiles in different Tables (an
   * approximation, not a real identity-resolution pass; fine at
   * reference-app scale). */
  profilesByAuthor: Record<string, NodeProfile>;
}

export function useFeed(): FeedState {
  const [state, setState] = useState<FeedState>({
    loading: true,
    tables: [],
    items: [],
    pinsByTable: {},
    profilesByAuthor: {},
  });

  // Monotonic request id (same pattern as useTags) so a slow poll can't
  // overwrite a fresher one.
  const requestId = useRef(0);
  const load = useCallback(async () => {
    const id = ++requestId.current;
    {
      const tables = await api.listTables();
      const items: FeedItem[] = [];
      const pinsByTable: Record<string, TagView[]> = {};
      const profilesByAuthor: Record<string, NodeProfile> = {};

      await Promise.all(
        tables.map(async (table) => {
          const [messages, images, proposals, tablePins, profiles] = await Promise.all([
            api.listMessages(table.id),
            api.listImages(table.id),
            api.listProposals(table.id),
            api.pins(table.id),
            api.listProfiles(table.id),
          ]);
          pinsByTable[table.id] = tablePins;
          for (const { author_hex, profile } of profiles) {
            profilesByAuthor[author_hex] = profile;
          }
          for (const message of messages) {
            items.push({
              kind: "message",
              id: `${table.id}/${message.author_hex}/${message.rkey}`,
              tableId: table.id,
              createdAt: message.created_at,
              message,
            });
          }
          for (const image of images) {
            items.push({
              kind: "photo",
              id: `${table.id}/${image.author_hex}/${image.rkey}`,
              tableId: table.id,
              createdAt: image.created_at,
              image,
            });
          }
          for (const proposal of proposals) {
            items.push({
              kind: "decision",
              id: `${table.id}/${proposal.author_hex}/${proposal.rkey}`,
              tableId: table.id,
              createdAt: proposal.proposal.created_at,
              proposal,
            });
          }
        }),
      );

      items.sort((a, b) => (a.createdAt < b.createdAt ? 1 : -1));

      if (id === requestId.current)
        setState({ loading: false, tables, items, pinsByTable, profilesByAuthor });
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);
  usePoll(load, 10_000);

  return state;
}
