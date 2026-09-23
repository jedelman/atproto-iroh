// Everything one Table detail screen needs — DESIGN_BRIEF.md §4:
// "Inside a Table: Members-first landing... led by that Table's pinned
// welcome message if one exists, then Messages, Decisions, Photos,
// Shared docs."

import { useEffect, useState } from "react";
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

export function useTable(tableId: string): TableState {
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

  useEffect(() => {
    let cancelled = false;

    async function load() {
      const [tables, members, pins, messages, images, proposals, governance] = await Promise.all([
        api.listTables(),
        api.listProfiles(tableId),
        api.pins(tableId),
        api.listMessages(tableId),
        api.listImages(tableId),
        api.listProposals(tableId),
        api.governanceState(tableId),
      ]);
      if (cancelled) return;
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

    load();
    return () => {
      cancelled = true;
    };
  }, [tableId]);

  return state;
}

export function profileFor(members: ProfileView[], authorHex: string): NodeProfile | undefined {
  return members.find((m) => m.author_hex === authorHex)?.profile;
}
