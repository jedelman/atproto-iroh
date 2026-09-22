# atproto-iroh-tauri

Reference client for `atproto-iroh-core` — see the root `CLAUDE.md`'s
"Client" section for why Tauri, and `../../SPEC.md` for the protocol
this is a UI on top of.

## What's real here

- `src-tauri/` is a genuine Tauri 2 app: `atproto-iroh-core` is a normal
  path dependency, no IPC or FFI boundary between UI-adjacent code and
  protocol logic.
- Twenty-eight commands, each a direct call into the core crate:
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
- **QR codes** (`ticket_to_qr`): renders a ticket as an SVG QR, nothing
  else — no scanning/camera decode built (see below), no native OS share
  sheet integration (unverified whether Tauri 2 has one; not checked).
  No restriction on which access mode gets turned into a code: a `Write`
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
  has to be admitted afterward through a real `AdmitCoSigner` Proposal,
  which is correct, just not the only valid founding shape the protocol
  allows. `list_proposals`/`governance_state` are also O(every record in
  the namespace) per call — a caching or incremental-fold question once
  a namespace has more than a handful of proposals, not addressed here.
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

- **QR scanning.** Generation only. Decoding would need webview camera
  access (`getUserMedia` + a JS decoder, e.g. `jsQR`) — plausible on
  desktop since Tauri's webview is a real browser engine, but camera
  permission behavior across WebKit/WebView2/webkit2gtk specifically
  wasn't checked in this session, and Tauri 2's mobile story (where a
  camera matters most) is already flagged elsewhere as less mature than
  desktop. Worth verifying before building, not assuming.
- **Any error/loading state beyond `textContent = "error: ..."`.** Fine
  for proving the wiring, not fine for anyone else to use.
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

## Styling philosophy

Repo is MIT-licensed specifically so a rougher-but-functional reference
UI is a fine place to start (Jason's framing, kept verbatim in spirit):
*"I spent my time in the trenches pushing pixels, I've earned some
inconsistencies. If people want pretty, they can make it pretty."*
`dist/styles.css` is intentionally undesigned — system fonts, no visual
identity, nothing here should be read as this project's design
direction, because it doesn't have one yet on purpose.
