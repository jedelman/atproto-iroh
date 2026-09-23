// The canonical mock dataset for this frontend — one realistic, small
// multi-Table world, used three ways: (1) `api/mockClient.ts` reads and
// mutates a copy of it so `npm run dev` (a plain browser, no Tauri
// runtime) has something real to render without a live iroh node; (2)
// component/screen tests import it directly for assertions; (3) it's the
// same content the /shape mockup artifact used (Garden Table, Weekend
// Hikers, New Parents Crew), so what you see in `npm run dev` matches
// what was designed, not a different placeholder set.
//
// Author hexes here are NOT real did:iroh keys — 64 lowercase hex
// characters shaped like a real AuthorId's hex encoding, but
// deterministic and human-legible (a repeating mnemonic) on purpose, so
// a test failure's diff is readable instead of forty random hex chars.
// Never treat these as real identities or use them outside mocks/tests.

import type {
  GovernanceStateView,
  ImageView,
  MessageView,
  NodeProfile,
  ProposalView,
  TagView,
} from "../api/types";
import { PIN_LABEL } from "../api/types";

export const SELF_AUTHOR_HEX = "5e1f".repeat(16);

const GARDEN_TABLE_ID = "table-garden0000000000000000000000000000000000000000000000001";
const HIKERS_TABLE_ID = "table-hikers0000000000000000000000000000000000000000000001";
const PARENTS_TABLE_ID = "table-parents000000000000000000000000000000000000000000001";
const BOOKCLUB_TABLE_ID = "table-bookclub00000000000000000000000000000000000000000001";

export const AUTHORS = {
  you: SELF_AUTHOR_HEX,
  marisol: "a11c".repeat(16),
  sequoia: "90fd".repeat(16),
  priya: "c3b2".repeat(16),
  devon: "7e40".repeat(16),
} as const;

/** Table identity itself has no real backend record yet — DESIGN_BRIEF.md's
 * Open Questions has the reasoning. This mock-only type stands in for the
 * `put_text`-based "table/name" this could become. */
export interface MockTable {
  id: string;
  name: string;
  memberAuthorHexes: string[];
}

export const TABLES: MockTable[] = [
  {
    id: GARDEN_TABLE_ID,
    name: "The Garden Table",
    memberAuthorHexes: [AUTHORS.you, AUTHORS.marisol, AUTHORS.devon],
  },
  {
    id: HIKERS_TABLE_ID,
    name: "Weekend Hikers",
    memberAuthorHexes: [AUTHORS.you, AUTHORS.sequoia],
  },
  {
    id: PARENTS_TABLE_ID,
    name: "New Parents Crew",
    memberAuthorHexes: [AUTHORS.you, AUTHORS.priya, AUTHORS.devon],
  },
  {
    // Deliberately doesn't include AUTHORS.you — this is the Table the
    // mock QR-scan/join demo flow adds you to, so joining actually
    // changes something observable (a new Table appears) rather than
    // re-joining one already in TABLES_YOU_ARE_IN.
    id: BOOKCLUB_TABLE_ID,
    name: "Thursday Book Club",
    memberAuthorHexes: [AUTHORS.priya, AUTHORS.sequoia],
  },
];

/** Which Tables "you" (SELF_AUTHOR_HEX) already hold a capability into
 * before any mock join happens — everything in TABLES except the one
 * reserved for the join demo. `mockClient.ts`'s `listTables`/
 * `listNamespaces` are scoped to this, not all of `TABLES`. */
export const TABLES_YOU_ARE_IN = TABLES.filter((t) => t.id !== BOOKCLUB_TABLE_ID).map((t) => t.id);

/** Fake "tickets" the mock Join/QR-scan flow accepts — stand-ins for a
 * real `DocTicket` string, which the mock backend has no way to parse
 * (there's no real iroh-docs engine behind it). Scanning/pasting
 * anything else should fail the same way a malformed real ticket would.
 */
export const MOCK_TICKETS: Record<string, string> = {
  "mock-ticket-bookclub": BOOKCLUB_TABLE_ID,
};

export const PROFILES: Record<string, NodeProfile> = {
  [AUTHORS.you]: {
    name: "Theo",
    category: "individual",
    avatar: "accent",
    created_at: "2026-08-01T12:00:00Z",
  },
  [AUTHORS.marisol]: {
    name: "Marisol",
    category: "informal_group",
    avatar: "gold",
    created_at: "2026-06-14T09:30:00Z",
  },
  [AUTHORS.sequoia]: {
    name: "Sequoia",
    category: "informal_group",
    avatar: "sage",
    created_at: "2026-07-02T16:00:00Z",
  },
  [AUTHORS.priya]: {
    name: "Priya",
    category: "individual",
    avatar: "rose",
    created_at: "2026-05-20T08:00:00Z",
  },
  [AUTHORS.devon]: {
    name: "Devon",
    category: "individual",
    avatar: "plum",
    created_at: "2026-05-21T08:00:00Z",
  },
};

/** Messages, per Table — oldest first, matching `messaging::list_messages`'s
 * own ordering guarantee. */
export const MESSAGES: Record<string, MessageView[]> = {
  [GARDEN_TABLE_ID]: [
    {
      author_hex: AUTHORS.marisol,
      rkey: "0001",
      subject: `${AUTHORS.marisol}/network.essmesh.chat.message/0001`,
      text: "Welcome! Water the beds Tue/Thu evenings, and grab whatever's ripe on your way out — that's what it's here for.",
      created_at: "2026-09-20T18:00:00Z",
    },
    {
      author_hex: AUTHORS.devon,
      rkey: "0002",
      subject: `${AUTHORS.devon}/network.essmesh.chat.message/0002`,
      text: "Tomatoes are going wild this week, come take some before the raccoons do.",
      created_at: "2026-09-22T14:10:00Z",
    },
  ],
  [HIKERS_TABLE_ID]: [
    {
      author_hex: AUTHORS.sequoia,
      rkey: "0001",
      subject: `${AUTHORS.sequoia}/network.essmesh.chat.message/0001`,
      text: "Made it to the overlook before the fog rolled in. Worth the 6am start.",
      created_at: "2026-09-23T10:05:00Z",
    },
  ],
  [PARENTS_TABLE_ID]: [
    {
      author_hex: AUTHORS.priya,
      rkey: "0001",
      subject: `${AUTHORS.priya}/network.essmesh.chat.message/0001`,
      text: "Does anyone have a spare pack-n-play for next weekend? Ours is still on a truck somewhere.",
      created_at: "2026-09-22T09:00:00Z",
    },
  ],
};

export const IMAGES: Record<string, ImageView[]> = {
  [HIKERS_TABLE_ID]: [
    {
      author_hex: AUTHORS.sequoia,
      rkey: "img-0001",
      content_type: "image/jpeg",
      len: 842_113,
      caption: "The overlook, 7:12am",
      created_at: "2026-09-23T10:06:00Z",
    },
  ],
};

export const PROPOSALS: Record<string, ProposalView[]> = {
  [GARDEN_TABLE_ID]: [
    {
      author_hex: AUTHORS.marisol,
      rkey: "prop-0001",
      proposal: {
        title: "Move tool-shed hours to weekends",
        description:
          "Nobody's objected, so this quietly takes effect Sunday unless someone speaks up before then.",
        class: "general",
        deadline: "2026-09-28T00:00:00Z",
        created_at: "2026-09-21T12:00:00Z",
      },
      status: { state: "open", blockers: [] },
    },
  ],
};

export const GOVERNANCE_STATE: Record<string, GovernanceStateView> = {
  [GARDEN_TABLE_ID]: { eligible_hex: [AUTHORS.marisol, AUTHORS.you, AUTHORS.devon] },
  [HIKERS_TABLE_ID]: { eligible_hex: [AUTHORS.sequoia, AUTHORS.you] },
  [PARENTS_TABLE_ID]: { eligible_hex: [AUTHORS.priya, AUTHORS.you, AUTHORS.devon] },
};

/** Tags, including pins — `label: "system:pin"` is `tagging::PIN_LABEL`.
 * Marisol pinned twice (only the later one should resolve as current). */
export const TAGS: Record<string, TagView[]> = {
  [GARDEN_TABLE_ID]: [
    {
      author_hex: AUTHORS.marisol,
      rkey: "tag-0001",
      subject: `${AUTHORS.marisol}/network.essmesh.chat.message/0001`,
      label: PIN_LABEL,
    },
    {
      author_hex: AUTHORS.marisol,
      rkey: "tag-0002",
      // Marisol's second, later pin — points at Devon's real message
      // (she only ever authored the one message herself, rkey 0001;
      // this was a dangling reference to a nonexistent
      // marisol/…/0002 before, found by actually looking at the
      // rendered screenshot, not just reading the code).
      subject: `${AUTHORS.devon}/network.essmesh.chat.message/0002`,
      label: PIN_LABEL,
    },
    {
      author_hex: AUTHORS.devon,
      rkey: "tag-0003",
      subject: `${AUTHORS.devon}/network.essmesh.chat.message/0002`,
      label: "harvest",
    },
  ],
};

/** Doc revisions, per Table → per doc id, oldest first (matches
 * `list_document_revisions`' ordering). */
export const DOC_REVISIONS: Record<string, Record<string, { author_hex: string; rev: string; text: string }[]>> = {
  [PARENTS_TABLE_ID]: {
    "meal-train": [
      {
        author_hex: AUTHORS.devon,
        rev: "0001",
        text: "Week of the 28th:\nMon — Priya\nWed — Devon\nFri — open",
      },
    ],
  },
};
