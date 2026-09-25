// Everything one Table detail screen needs — DESIGN_BRIEF.md §4:
// "Inside a Table: Members-first landing... led by that Table's pinned
// welcome message if one exists, then Messages, Decisions, Photos,
// Shared docs."

import { useCallback, useEffect, useRef, useState } from "react";
import { usePoll } from "./usePoll";
import {
  api,
  type ImageView,
  type MessageView,
  type NodeProfile,
  type ProfileView,
  type ProposalView,
  type TagView,
} from "../api";
import type { TableSummary } from "../api/client";

interface TableState {
  loading: boolean;
  table: TableSummary | null;
  members: ProfileView[];
  pins: TagView[];
  messages: MessageView[];
  images: ImageView[];
  proposals: ProposalView[];
  eligibleHex: string[];
}

export function useTable(
  tableId: string,
): TableState & {
  refreshProposals: () => Promise<void>;
  refreshPins: () => Promise<void>;
  reload: () => Promise<void>;
} {
  const [state, setState] = useState<TableState>({
    loading: true,
    table: null,
    members: [],
    pins: [],
    messages: [],
    images: [],
    proposals: [],
    eligibleHex: [],
  });

  // Reloads everything while the Table is open (usePoll below) — before
  // this, posts from other devices only appeared after leaving and
  // coming back. Monotonic request id so a slow reload can't overwrite a
  // fresher one; `loading` only ever flips false, never back to true, so
  // a background refresh doesn't blank the screen.
  const requestId = useRef(0);
  const reload = useCallback(async () => {
    const id = ++requestId.current;
    {
      const [tables, members, pins, messages, images, proposals, governance] = await Promise.all([
        api.listTables(),
        api.listProfiles(tableId),
        api.pins(tableId),
        api.listMessages(tableId),
        api.listImages(tableId),
        api.listProposals(tableId),
        api.governanceState(tableId),
      ]);
      if (id !== requestId.current) return;
      setState({
        loading: false,
        table: tables.find((t) => t.id === tableId) ?? null,
        members,
        pins,
        messages: [...messages].reverse(), // newest first, same convention as the Feed
        images: [...images].reverse(),
        proposals,
        eligibleHex: governance.eligible_hex,
      });
    }
  }, [tableId]);

  useEffect(() => {
    reload();
  }, [reload]);
  usePoll(reload, 5_000);

  // Re-fetches just proposals + governance state — used after creating a
  // decision/poll or signaling on one, so the Decisions tab reflects the
  // real fold result rather than a locally-guessed optimistic status
  // (ratification depends on eligibility/threshold math this hook
  // shouldn't duplicate — see fold::fold_namespace).
  const refreshProposals = useCallback(async () => {
    const [proposals, governance] = await Promise.all([
      api.listProposals(tableId),
      api.governanceState(tableId),
    ]);
    setState((prev) => ({ ...prev, proposals, eligibleHex: governance.eligible_hex }));
  }, [tableId]);

  // Same shape as refreshProposals — used after pinning a message so the
  // Table's pinned-welcome strip reflects the new pin without a full
  // reload of everything else (messages, images, members).
  const refreshPins = useCallback(async () => {
    const pins = await api.pins(tableId);
    setState((prev) => ({ ...prev, pins }));
  }, [tableId]);

  return { ...state, refreshProposals, refreshPins, reload };
}

export function profileFor(members: ProfileView[], authorHex: string): NodeProfile | undefined {
  return members.find((m) => m.author_hex === authorHex)?.profile;
}
