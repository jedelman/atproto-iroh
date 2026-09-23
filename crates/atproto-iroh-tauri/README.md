# atproto-iroh-tauri

Reference client for `atproto-iroh-core` — see the root `CLAUDE.md`'s
"Client" section for why Tauri, and `../../SPEC.md` for the protocol
this is a UI on top of.

## Frontend rebuild (2026-09-23) — React + Vite, in progress

The plain HTML/JS/CSS frontend (`dist/index.html`/`main.js`/`styles.css`,
the vendored `jsQR`) is **gone**, replaced by a real React + Vite app
under `src/` per the confirmed `/shape` design brief
(`DESIGN_BRIEF.md`) — a deliberate departure from the "no bundler, no
framework" rule this crate held to until now, not a regression from it.
`dist/` is now a *build output* directory (`.gitignore`'d), regenerated
by `npm run build` / Tauri's own `beforeBuildCommand`, not source.

**Built and working**: the design-token system (warm-dark palette, Young
Serif/Hanken Grotesk, from the confirmed mockup artifact), the `Sticker`
component (built-in blob avatars), the **Feed** screen (the unified
cross-Table timeline — messages/photos/decisions merged, Table filter
chips, the pinned-message excerpt), the **Onboarding** screen (sticker +
name picker), the **Table detail** screen (Members-first landing, the
Table's pinned message(s) in full, tabbed Messages/Decisions/Photos —
DESIGN_BRIEF.md §4, plus a message **Composer** in the Messages tab,
optimistic — the sent message appears immediately, resolved through the
real self author hex via `nodeDid()` rather than a placeholder, so it
renders exactly the way the synced-back copy eventually will), and the
**Join** screen (camera QR scan, re-ported
from the old plain-JS app onto the real `jsqr` npm package, with a
paste-a-ticket fallback). All wired to a real API abstraction
(`src/api/`) that talks to the actual Tauri backend when running inside
Tauri and falls back to an in-memory mock backend
(`src/api/mockClient.ts`) otherwise — see "Mock dataset and tests," below.
Tapping a Table's sticker in the Feed navigates to its detail screen;
the filter chips below it filter the Feed in place instead; the dashed
"+ Join" circle at the end of that strip (and the Feed's own empty
state) opens Join.

**Screenshot-verified, not just test-verified** — a real Playwright +
headless-Chromium pipeline against `npm run dev`'s mock-backed browser
session caught three real bugs the type-checker and the test suite both
missed: table names truncating illegibly, a pinned excerpt silently
falling back to placeholder text (a dangling fixture reference — a real
message never existed at that pinned subject), and Table detail showing
*every* mock member on *every* table instead of each table's real
members (`mockClient.ts` was seeding all tables from the same full
profile set). All three fixed, and turned into real test assertions
afterward so they'd be caught by the suite next time, not just eyeballs.

**Not yet ported — a real, current gap, not an oversight**: the old
plain-JS app had working UI for every one of its 28 commands (Messaging
thread view *outside* a Table's own tab — reply-to isn't wired into the
Composer yet, Images upload, Documents with
conflict-visibility, Governance/Polls creation forms, Tagging, Mute,
Members/Profile editing, QR generate *and* scan, the raw inspector,
relay/control). Functionally, this rebuild still covers *less* than the
app it replaced; a deliberate trade (a real design direction on four
screens, over a complete-but-undesigned UI on eight) that needs the
remaining screens built to reach parity.

## What's real here

- `src-tauri/` is a genuine Tauri 2 app: `atproto-iroh-core` is a normal
  path dependency, no IPC or FFI boundary between UI-adjacent code and
  protocol logic.
- Twenty-nine commands (28 plus `pins`, added 2026-09-23 alongside
  `tagging::pins()` in the core crate), each a direct call into the core
  crate:
  `spawn_node` (loads or generates a persistent `did:iroh` identity and
  starts a real `iroh` node against real on-disk storage — deliberately
  not done at app launch; see `main.rs`'s comment on why), `node_did`,
  `create_namespace_with_profile` (creates a namespace, publishes a
  `NodeProfile` into it, **and posts this author's real `Founding` claim**
  — `fold::found_namespace`, replacing what used to be a placeholder
  heuristic; see the Governance bullet below), `list_namespaces`,
  `share_namespace` (`Node::share`, unwrapped to a plain paste-able
  ticket string), `join_namespace` (`Node::join`, which SPEC.md §6 item
  10 confirmed backfills full history, not just future writes — a real
  join), `dump_namespace` (the inspector, below), `ticket_to_qr` (an SVG
  QR code of a ticket string, nothing more), `doc_save`/`doc_load`/
  `doc_history`/`submit_to_inbox` (the freeform layer, below),
  `send_message`/`list_messages` (Messaging, below),
  `upload_image`/`list_images`/`load_image_bytes` (Images, below),
  `update_profile`/`list_profiles` (Members/Profile, below),
  `mute_author`/`unmute_author`/`list_muted` (Mute, below),
  `add_tag`/`tags_for` (Tags, below), and
  `list_proposals`/`governance_state`/`create_proposal`/`create_signal`
  (governance, below). `AppState.docs` holds every namespace this node
  currently has open — repopulated from disk on every `spawn_node` call
  now, not just built up in memory during one session — and
  `AppState.author` this node's persistent default record-signing author
  (`DocsApi::author_default`, not minted fresh), so edits from one
  person land under one consistent author across restarts, not a new
  stranger each launch.
- **The inspector** (`namespace::dump_all`, `dist`'s "Inspector"
  section): every raw entry currently synced into a namespace — author,
  key, timestamp, content — with no per-record-type code anywhere in the
  chain. Rust has no runtime reflection, so this isn't literally
  automatic over arbitrary types, but every record this crate writes is
  JSON or plain text, so "try JSON, then UTF-8 text, then fall back to
  hex" gets the same practical result: one generic view that already
  covers `NodeProfile`, governance `Proposal`/`Signal`/`Founding`
  records, and freeform text writes without having been told about any
  of them, and covers whatever gets added next the same way. A debugging
  view for building against, not a feature end users need.
- **Shared doc** (`namespace::save_document_revision`/
  `list_document_revisions`/`load_document`, `dist`'s "Shared doc"
  section): a namespace doesn't have to hold typed records at all —
  nothing about `iroh-docs` or this crate requires it. **No longer backed
  by `put_text` at a fixed key** — that was confirmed live (SPEC.md's
  2026-09-22 CRDT note) to be last-write-wins on concurrent offline
  edits, silently dropping one side's edit on reconnect. `doc_save` now
  writes an immutable, uniquely-keyed revision instead; `doc_load` shows
  the latest by save order, `doc_history` shows every revision so the UI
  can surface "someone else edited this while you were offline" instead
  of quietly discarding it. `namespace::put_text`/`get_text` still exist
  in the core crate (right primitive for genuinely single-writer text)
  but nothing in this UI calls them anymore. **Conflict visibility, not
  just history, built 2026-09-22**: "Load" now always refreshes history
  alongside it, and `main.js` flags a real concurrent-edit signal — the
  two most recent revisions from *different authors* within 5 minutes of
  each other (not just "more than one revision exists," which a single
  person saving twice in a row also produces) — with a visible
  `⚠ possible conflict` banner and a "Use this" button per revision that
  loads it into the edit box for review. Still no automatic merge —
  resolving a conflict is a human reading both revisions and re-saving,
  same as it always would be; this closes the *visibility* gap (silently
  picking a side), which is what was actually missing, not a text-diff
  problem this session never claimed to solve. The banner logic is
  pure/no-DOM and unit-tested standalone (five assertions including the
  exact window-boundary case) since nothing else in this reference app's
  JS has test coverage otherwise.
- **Inbox** (`namespace::submit_text`, `dist`'s "Inbox" section): the
  freeform layer's other half — `submit_text` mints a fresh key per call
  (`new_entry_key`, shared with `fold.rs`'s `propose`/`signal`), so any
  number of strangers can write without colliding or needing to agree on
  anything first. Unlike `doc_save`, this one's already collision-free by
  construction (nobody's overwriting anybody else's submission), which is
  exactly why it didn't need the same CRDT fix the Shared doc primitive
  did. This is the actual mechanism behind "a public inbox" — see the QR
  note below for why that's safe.
- **Messaging** (`messaging::send_message`/`list_messages`, `dist`'s
  "Messages" section): CLAUDE.md's batteries-included app list's first
  build, and the one it predicted would need no new primitive — a
  namespace is the channel, each message mints its own key
  (`new_entry_key`, same generator `submit_text`/`fold::propose`/
  `fold::signal` already share), so there's no collision or
  last-write-wins risk the way the old `put_text`-backed Shared doc had.
  `list_messages` sorts by key (the same "sort by the sortable key, not
  a separately tracked index" approach `list_document_revisions` uses)
  since `list_records` itself makes no ordering promise. `reply_to` is a
  plain `"{author_hex}/{rkey}"` string, same convention
  `governance::subject_ref` uses for `Signal.subject` — restated locally
  in `messaging.rs` rather than sharing one function across two
  unrelated modules for a one-line format.
- **Tags** (`tagging::add_tag`/`tags_for`, `dist`'s "Tags" section):
  cross-lexicon by construction (Jason's explicit ask) — one `Tag` type
  can tag a message, a Shared doc revision, a `NodeProfile`, anything,
  via `records::record_ref`'s `"{author_hex}/{collection}/{rkey}"`
  (the one reference format in this crate that actually carries the
  collection name, since `governance::subject_ref`/`messaging::reply_ref`
  only ever point within their own collection). Each message in the
  Messages section gets a "Tag" button that fills in this panel's
  subject field with that exact message's `record_ref` — no manual
  copy-pasting required for the common case.
- **Images** (`images::upload_image`/`list_images`/`load_image_bytes`,
  `dist`'s "Images" section): a file `<input>`, `file.arrayBuffer()` →
  `Array.from(new Uint8Array(...))` → `upload_image`, bytes crossing the
  Tauri IPC boundary as a plain JSON array (fine at reference-app scale,
  not a streaming upload path). No separate blob-fetch step on the read
  side either — `images::load_image_bytes` returns the same bytes
  `iroh-docs` already synced as the entry's content, rendered via
  `URL.createObjectURL(new Blob([...]))` rather than a base64 data URL,
  so no manual encoding on either side of the wire.
- **Members / Profile** (`update_profile`/`list_profiles`, `dist`'s
  "Members" section): `NodeProfile` editing separated from namespace
  creation — `create_namespace_with_profile` still sets an initial
  profile, but this lets an author update their own profile (name,
  category, neighborhood) any time afterward, an overwrite of their own
  fixed `NodeProfile::SELF_KEY` slot, safe by the same per-author-key
  construction as everything else (SPEC.md §3.4). `list_profiles` is the
  "who's here" view — every synced `NodeProfile`, unfiltered.
- **Mute, wired now** (`mute_author`/`unmute_author`/`list_muted`, `dist`'s
  "Mute" section): purely local, no sync, no lexicon (`mute::MuteList`'s
  own doc comment). The one place it actually changes anything yet —
  `list_messages` filters out muted authors before returning; every
  other read path (the inspector, `list_profiles`) is untouched, since
  filtering everything preemptively wasn't what was asked for and mute
  is deliberately scoped per-feature, not global. A muted author's
  messages still sync and still count for everyone else — this only
  changes what this one reader's client shows them.
- **QR codes** (`ticket_to_qr`, generation; camera scanning — built
  2026-09-23, see below): renders a ticket as an SVG QR. No native OS
  share sheet integration (unverified whether Tauri 2 has one; not
  checked). No restriction on which access mode gets turned into a code: a `Write`
  ticket posted publicly is a real, intended pattern here, not a mistake
  to guard against — a public inbox, a dead drop, a graffiti wall.
  Mechanically safe by construction, not by policy: `RecordIdentifier`
  is `(namespace, author, key)` (SPEC.md §3.4's validated finding), so a
  stranger holding a Write ticket can only ever write under an author
  *they* generated — they can't forge entries as you or overwrite anyone
  else's. What a public Write ticket actually needs is volume/moderation
  handling on the read side (see `mute.rs`, already built, not yet wired
  to inbox reading) — the real risk is flooding, which §6 item 12 already
  named as the live case rotation was weighed against, not forgery.
  Genuinely relevant to SPEC.md §3.6's discovery story either way: a QR
  code physically posted somewhere is the same trust shape as a ticket
  shared in a DM — nothing discoverable until someone already has the
  capability — just analog instead of digital.
- **QR scanning** (`dist/main.js`'s "Scan QR" button, next to Join):
  reads a ticket back out of a QR code via `getUserMedia` + a decode
  loop, no Tauri command involved — pure client-side JS, the same shape
  on desktop and mobile. The obvious approach — a Tauri plugin — turned
  out not to be an option: the only maintained one
  (`tauri-plugin-barcode-scanner`) depends on `tauri = "3.0.0-alpha.2"`,
  and this app is on stable Tauri 2 (checked live against the crate's
  own `Cargo.toml` before writing any code, not assumed). Went with a
  vendored decoder instead (`dist/vendor/jsQR.min.js`, jsQR 1.4.0,
  Apache-2.0) — fits this reference client's own "no bundler, no
  framework" rule better than a plugin would have anyway. Verified live,
  not just wired: a Node script round-trips a real QR-encoded string
  through the exact vendored file and confirms an exact match (the risk
  worth checking was the vendored/minified build itself, not the
  well-established jsQR algorithm). **Camera permission on Android**
  goes through wry's own `WebChromeClient.onPermissionRequest`
  (confirmed against the vendored wry 0.55.1 source: it already handles
  `getUserMedia`'s video-capture request, prompting for
  `android.permission.CAMERA` at runtime) — this app only needed to
  declare that permission in the manifest, done via
  `scripts/patch-android-manifest.sh` (idempotent, since `gen/android`
  regenerates from scratch and isn't committed — run it again after any
  fresh `cargo tauri android init`). **Not verified live**: an actual
  camera scan on a real device or emulator (none available in this
  sandbox) — the decode logic and the Android permission path are each
  independently confirmed, but the full path (open camera → point at a
  real QR → fill the field) hasn't been clicked through by a human yet.
  Desktop camera support depends on the OS webview actually exposing
  `getUserMedia` in a secure context; the code degrades to a visible
  error message (not a crash) if it's unavailable, but that path is
  also unverified without a desktop machine with a webcam to try it on.
- **Governance** (`fold::fold_namespace`/`propose`/`signal`, `dist`'s
  "Governance" section): `list_proposals` re-runs the whole fold on
  every call and returns each `Proposal` with its live `Ratification`
  status; `create_proposal`/`create_signal` post new records. **The
  founding-record gap this bullet used to describe is closed** (SPEC.md
  §6 item 16): `create_namespace_with_profile` now calls
  `fold::found_namespace` to post a real `Founding` claim at namespace
  creation (this app's own default starting policy —
  `default_founding_policy`, still explicitly *this app's* choice, not a
  protocol constant SPEC.md §3.7.2 says stays namespace-owned), and
  `list_proposals`/`governance_state` now call `fold::fold_namespace`,
  which resolves real genesis state from whatever `Founding` claims are
  synced instead of the old heuristic (self-asserted
  `governance_eligible: true` in a `NodeProfile`, which that lexicon's
  own field description already called non-authoritative). What's still
  a reference-app shortcut, not a protocol gap: the founder is always
  the namespace's sole genesis member here — this UI never prompts for
  co-founders at creation time, even though `found_namespace`/
  `resolve_founding` support a real multi-founder bootstrap. Anyone else
  has to be admitted afterward through a real `AdmitCoSigner` Proposal.
  **Deliberate, not a gap**: discussed with Jason (2026-09-22) and
  decided not to build a multi-founder prompt — silence-ratifies-by-
  default means inviting co-founders after solo creation costs about as
  much friction as founding together would, and toxic-founder removal
  already exists via governance; the mechanism (`resolve_founding`'s
  acceptance-window design) still supports real multi-founder bootstrap
  for anyone who builds a client that prompts for it, this one just
  won't. `list_proposals`/`governance_state` are also O(every record in
  the namespace) per call — a caching or incremental-fold question once
  a namespace has more than a handful of proposals, not addressed here.
- **Polls** (`dist`'s "Polls" section, above Governance): a
  `GovernanceClass::General` Proposal, relabeled — `create-poll` and the
  poll list call the *exact same* `create_proposal`/`create_signal`/
  `list_proposals` commands as the full Governance section, hardcoded to
  `class: "general"` and filtered/relabeled client-side only (Support/
  Object instead of Consent/Block). No new Rust. Said outright in the
  hint text, not left implied: **this is not a majority vote** — same
  objection-window mechanic as everything else (passes by default unless
  enough people explicitly object; silence counts as support) — calling
  it "Polls" without that caveat would have implied ordinary
  vote-counting behavior the mechanism doesn't have.
- **Persistence** (`identity::Identity::load_or_generate`,
  `namespace::Node::spawn_persistent`, both new): real, not cosmetic.
  Found and fixed a genuine bug while wiring this, not something
  cosmetic: before this, `Node::spawn()` always generated a random
  `SecretKey` internally, completely disconnected from whatever
  `Identity` a caller displayed as a `did:iroh` — the DID shown to a
  person and the node's actual network identity were two unrelated
  keys. `spawn_persistent` threads `Identity`'s own key through to the
  `Endpoint`, so persisting the identity file actually persists the
  network identity, not just a label next to a different one each
  restart. Data lives under `atproto_iroh_core::paths::data_dir()`
  (`$ATPROTO_IROH_DATA_DIR` if set, else `$XDG_DATA_HOME/atproto-iroh` or
  `~/.local/share/atproto-iroh`) — an `identity` file (raw 32-byte
  secret, same protection any other private key file needs, not
  enforced by this code) and a `node/` directory holding the persistent
  docs and blobs stores. **At-rest encryption is the env var, not app
  code**: point `ATPROTO_IROH_DATA_DIR` at a volume the OS already
  encrypts (FileVault/BitLocker/LUKS/a mounted encrypted container) —
  deliberately not this crate's own crypto layer, since it doesn't need
  to own that security model, and "if the volume's unlocked, everything
  on it is" was the explicit call made choosing this over app-level
  encryption. `mute::MuteList::default_path` and
  `identity::Identity::default_path` honor the same override
  (`paths.rs`), so one setting covers all of it, not three separately.
  `spawn_node` also repopulates `AppState.docs`
  from `Node::list_local_namespaces` on every launch, so a restarted app
  can immediately act on namespaces from a previous session. Proven
  live, not just by compiling: `tests/persistence.rs` spawns a node,
  creates a namespace, writes a record, does a *real* shutdown, spawns a
  brand-new `Node` against the same identity file and data directory,
  and checks that the DID, the namespace, and the record all survived
  with nothing handed to the second node directly.
- `dist/` is plain HTML/CSS/JS — no bundler, no `node_modules`, no
  framework. `tauri.conf.json` sets `withGlobalTauri: true` so
  `window.__TAURI__` is injected directly; `main.js` calls `invoke()`
  against it with zero build step. Deliberate, not a placeholder waiting
  for a real frontend stack — see the styling philosophy note below.

## What's not built yet

This proves the wiring, not a usable app. Missing, in roughly the order
a real client would need them:

- ~~QR scanning.~~ **Re-ported (2026-09-23)** — `src/hooks/
  useQrScanner.ts`, the same `getUserMedia` + decode-loop approach as
  the old plain-JS app, now against the real `jsqr` npm package instead
  of a vendored file. Wired into the new **Join** screen
  (`src/screens/Join.tsx`, at `/join`) — camera-first with a real
  paste-a-ticket fallback, reachable from the Feed's empty state and
  a persistent "+ Join" entry in the "your people" strip. Camera
  access itself isn't testable in this sandbox (no real camera, and
  jsdom can't fake `getUserMedia`) or in headless Chromium screenshots
  — the paste-ticket path is fully tested instead
  (`Join.test.tsx`: valid ticket → lands on the new table, invalid
  ticket → real error, not a silent failure).
- **Every screen except Feed, Onboarding, Table detail, and Join** —
  see "Frontend rebuild," above, for the full list and why this is a
  deliberate, temporary step backward in feature coverage, not an
  oversight.
- **Any error/loading state beyond a basic "Settling in…"/empty-state
  message.** Better than the old app's raw `textContent = "error:
  ..."` but still not a real design pass on error states.
- **No automatic merge for concurrent doc revisions.** The Shared doc
  section flags the conflict and lets a person load either revision for
  review (see its own bullet above), but reconciling two edits into one
  is still on the person reading them and re-saving by hand — a real
  text-diff/merge tool was never in this session's scope.

## Building

**Verified, not just written.** `cargo build -p atproto-iroh-tauri`
compiles clean (zero warnings) as of this scaffold — Tauri's Linux
backend needs `webkit2gtk-4.1` and `gtk3` development headers available
to `pkg-config` at build time (`apt-get install libwebkit2gtk-4.1-dev
libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` on Debian/Ubuntu;
see [Tauri's prerequisites docs](https://tauri.app/start/prerequisites/)
for other platforms), which the container this was scaffolded in didn't
have until they were installed specifically to run this check — noted
because "it builds" should mean it was actually built, not assumed from
the source looking right. macOS/Windows ship their own webviews and
don't need this step. `icons/icon.png` is a flat placeholder color, not
real artwork — `tauri::generate_context!()` requires *some* icon to
exist at that path even with `bundle.active: false`, found by hitting
the error, not anticipated.

Actually running it (`cargo tauri dev`) needs `tauri-cli`
(`cargo install tauri-cli --version "^2"`) and, in a container/CI
context, a display — not attempted here, since a headless environment
has nothing to show a window on regardless of whether the binary itself
is correct.

**Frontend build, verified 2026-09-23**: `npm install && npm run build`
(from this directory) runs `tsc -b && vite build` and produces a real
`dist/` — confirmed by then running `cargo build -p atproto-iroh-tauri`
against it, clean. `npm test` runs the Vitest suite (10 tests as of this
writing) against the mock API and the canonical fixtures — see "Mock
dataset and tests," below. `npm run dev` starts a plain Vite dev server
on `localhost:1420` (Tauri's `beforeDevCommand`) — since it has no
`window.__TAURI__`, it automatically uses the mock backend, so the UI is
directly viewable in an ordinary browser without a real iroh node or a
GTK/WebKit build at all.

## Mock dataset and tests

`src/mocks/fixtures.ts` is the canonical mock dataset — Jason's explicit
ask alongside "go ahead and implement": one realistic, small multi-Table
world (three Tables, five members, messages, an image, a Decision, and
pins — including one author pinning twice, so the "latest wins" rule has
something real to prove itself against) used three ways: `src/api/
mockClient.ts` reads and mutates a copy of it so `npm run dev` in a
plain browser has real content to render with zero backend; every test
imports it directly for assertions; and it's the *same* content the
`/shape` mockup artifact used (Garden Table, Weekend Hikers, New Parents
Crew), so what a plain-browser `npm run dev` shows matches what was
designed, not a different placeholder set that happens to compile.

`src/lib/pins.ts` mirrors `atproto-iroh-core`'s `tagging::
latest_per_author_with_label`/`tagging::pins()` resolution in
TypeScript (the mock backend has no Rust process to call into) and is
unit-tested directly against the fixtures in `pins.test.ts` — the same
"one per author, most recent wins, newest first" property
`tests/tagging.rs`'s `tags_can_tag_tags_and_pins_resolve_one_per_
author_by_recency` proves on the Rust side, now proven independently on
the TS side too. `src/mocks/fixtures.test.ts` checks the dataset's own
internal consistency (every reference resolves to something real in it)
rather than trusting it stays coherent as it's edited. `src/screens/
Feed.test.tsx` is a real React Testing Library render test — proves the
Feed screen shows real dataset content and that clicking a Table filter
chip actually filters, not just that the component compiles.

**Known gap, not silently accepted**: `npm audit` reports 7 advisories
(5 moderate, 1 high, 1 critical) as of this writing, all in dev-only
tooling (`esbuild`'s dev-server request handling, `vitest`'s mocker,
`react-router`'s SSR/open-redirect surface) — none of which apply to
this app's actual runtime (a local Tauri desktop/mobile shell, no SSR,
no public dev server). `npm audit fix --force` would jump to Vite 8 /
Vitest 5 / React Router 7, major versions not checked for compatibility
here — deferred deliberately rather than force-upgraded blind mid an
already-large change.

## Styling philosophy

**Superseded 2026-09-23** — kept below for the historical record this
document's own practice calls for, not as current guidance. The old
`dist/styles.css` (deleted along with the rest of the plain-JS
frontend) really was intentionally undesigned; the confirmed `/shape`
design brief (`DESIGN_BRIEF.md`) is now this project's real design
direction — warm-dark, "a shared kitchen table, not a corporate
dashboard," specific typefaces and an OKLCH palette in `src/styles/
tokens.css`, not an absence of one.

Original framing, for the record: repo is MIT-licensed specifically so
a rougher-but-functional reference UI was a fine place to *start*
(Jason's framing, kept verbatim in spirit): *"I spent my time in the
trenches pushing pixels, I've earned some inconsistencies. If people
want pretty, they can make it pretty."* — which is exactly what this
rebuild is: someone (Jason, this session) deciding it was time to make
it pretty.
