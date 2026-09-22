# atproto-iroh-tauri

Reference client for `atproto-iroh-core` — see the root `CLAUDE.md`'s
"Client" section for why Tauri, and `../../SPEC.md` for the protocol
this is a UI on top of.

## What's real here

- `src-tauri/` is a genuine Tauri 2 app: `atproto-iroh-core` is a normal
  path dependency, no IPC or FFI boundary between UI-adjacent code and
  protocol logic.
- Eleven commands, each a direct call into the core crate: `spawn_node`
  (generates a `did:iroh` identity and starts a real `iroh` node —
  deliberately not done at app launch; see `main.rs`'s comment on why),
  `node_did`, `create_namespace_with_profile` (creates a namespace and
  publishes a `NodeProfile` into it), `list_namespaces`, `share_namespace`
  (`Node::share`, unwrapped to a plain paste-able ticket string),
  `join_namespace` (`Node::join`, which SPEC.md §6 item 10 confirmed
  backfills full history, not just future writes — a real join),
  `dump_namespace` (the inspector, below), `ticket_to_qr` (an SVG QR
  code of a ticket string, nothing more), and `write_text`/`read_text`/
  `submit_to_inbox` (the freeform layer, below). `AppState.docs` holds
  every namespace this node currently has open, `AppState.author` one
  record-signing identity reused across every write this session makes
  (so edits from one person land under one consistent author rather than
  a new stranger each call).
- **The inspector** (`namespace::dump_all`, `dist`'s "Inspector"
  section): every raw entry currently synced into a namespace — author,
  key, timestamp, content — with no per-record-type code anywhere in the
  chain. Rust has no runtime reflection, so this isn't literally
  automatic over arbitrary types, but every record this crate writes is
  JSON or plain text, so "try JSON, then UTF-8 text, then fall back to
  hex" gets the same practical result: one generic view that already
  covers `NodeProfile`, governance `Proposal`/`Signal` records, and
  freeform `put_text` writes without having been told about any of them,
  and covers whatever gets added next the same way. A debugging view for
  building against, not a feature end users need.
- **Freeform text** (`namespace::put_text`/`get_text`/`submit_text`,
  `dist`'s "Shared doc" and "Inbox" sections): a namespace doesn't have
  to hold typed records at all — nothing about `iroh-docs` or this crate
  requires it. `put_text` writes plain UTF-8 at any key, no lexicon, no
  `Record` impl — the "Google doc without Google" primitive: shared,
  synced, capability-scoped text anyone with write access can read and
  edit. `submit_text` is the same idea specialized for uncoordinated
  submitters — it mints a fresh key per call (`new_entry_key`, shared
  with `fold.rs`'s `propose`/`signal`), so any number of strangers can
  write without colliding or needing to agree on anything first. This is
  the actual mechanism behind "a public inbox" — see the QR note below
  for why that's safe.
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
- **Governance** (`fold::fold`/`propose`/`signal`, `dist`'s "Governance"
  section): `list_proposals` re-runs the whole fold on every call and
  returns each `Proposal` with its live `Ratification` status;
  `create_proposal`/`create_signal` post new records. Real, but built on
  top of a gap that's still open, not silently papered over:
  SPEC.md §6 item 12 already named the missing piece (no `founding`
  record type naming a namespace's actual founder or its starting
  policy), so this UI supplies a placeholder starting policy
  (`placeholder_founding_policy`, explicitly documented as not a
  protocol default) and a heuristic eligible-member set (anyone who's
  self-asserted `governance_eligible: true` in their own `NodeProfile` —
  which that lexicon's own field description already calls
  non-authoritative). Fine for proving the ratification math works
  against real sync; not fine as the actual membership check a real
  deployment should trust. `list_proposals`/`governance_state` are also
  O(every record in the namespace) per call — a caching or incremental-
  fold question once a namespace has more than a handful of proposals,
  not addressed here.
- `dist/` is plain HTML/CSS/JS — no bundler, no `node_modules`, no
  framework. `tauri.conf.json` sets `withGlobalTauri: true` so
  `window.__TAURI__` is injected directly; `main.js` calls `invoke()`
  against it with zero build step. Deliberate, not a placeholder waiting
  for a real frontend stack — see the styling philosophy note below.

## What's not built yet

This proves the wiring, not a usable app. Missing, in roughly the order
a real client would need them:

- **A real founding-record mechanism.** Governance is wired now, but
  resting on the placeholder/heuristic bootstrap described above — the
  most consequential remaining gap, since it's load-bearing for whether
  the ratification math means anything in a real deployment, not just a
  reference client.
- **Persistence.** `Node::spawn` uses `Docs::memory()` — nothing survives
  a restart. Needs `Docs::persistent` plus somewhere sensible to put the
  data directory (see `mute.rs`'s `default_path` for the XDG-ish
  convention already established elsewhere in the core crate; this
  should probably follow the same one).
- **Mute UI.** `mute::MuteList` exists and is tested; nothing here reads
  or writes it.
- **QR scanning.** Generation only. Decoding would need webview camera
  access (`getUserMedia` + a JS decoder, e.g. `jsQR`) — plausible on
  desktop since Tauri's webview is a real browser engine, but camera
  permission behavior across WebKit/WebView2/webkit2gtk specifically
  wasn't checked in this session, and Tauri 2's mobile story (where a
  camera matters most) is already flagged elsewhere as less mature than
  desktop. Worth verifying before building, not assuming.
- **Any error/loading state beyond `textContent = "error: ..."`.** Fine
  for proving the wiring, not fine for anyone else to use.

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
