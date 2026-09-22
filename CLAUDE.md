# CLAUDE.md — atproto-iroh

## What this is

A capability-scoped, audit-by-construction protocol combining atproto's
data model (lexicon-typed, signed, individually-owned repos) with iroh's
transport (direct peer-to-peer, no relay, no server anyone has to trust).
Read `README.md` for the one-paragraph pitch and `SPEC.md` in full before
making any architectural decision — it's the authoritative design record,
it changed shape substantially several times during design (visible in
this repo's git log from the moment of extraction forward, and in
`jedelman/street-smarts`'s git log before that), and the reasoning behind
each choice matters more than the choice itself for anyone extending it.
Don't skim it and start coding from a half-remembered summary.

## Status and what to do first

No longer a design sketch with no code. The three `iroh-docs` questions
this section used to list as the first task are answered — live, against
the real crate, in `crates/atproto-iroh-core/examples/iroh_docs_probe.rs`
— and every design fork SPEC.md needed resolved before real code was
buildable (repo/namespace relationship, flat vs. N-of-M governance,
rotation vs. mute) is resolved, in `SPEC.md` itself, reasoning kept
visible per this doc's own practice below. `crates/atproto-iroh-core` has
real modules now (`identity`, `namespace`, `records`, `governance`,
`mute`), not a stub — 14/14 tests green as of the last scaffold, unit
tests for the pure governance/ratification logic plus one live two-node
integration test doing real QUIC sync.

Governance is wired now (Tauri's "Client" section below), persistence and
at-rest encryption are real, and a version-preserving document primitive
(`namespace::save_document_revision`/`list_document_revisions`/
`load_document`) exists to fix a real, confirmed CRDT data-loss case —
see SPEC.md's 2026-09-22 note right before §6 for the finding and
`crates/atproto-iroh-core/tests/document_revisions.rs` for the live
proof. There's also a second client now: `crates/atproto-iroh-cli`, a
one-shot-process-per-command Linux CLI plus a `serve` mode for anything
that needs to stay reachable — see its own README, including a real bug
it found and fixed live (a `share` ticket naming a port nothing was still
listening on, once the process that issued it had already exited).

What's still genuinely unbuilt: the Tauri "Shared doc" UI hasn't been
switched from the lossy `put_text` primitive to the new revision-based
one yet (Tauri README's own note), there's no real founding-record
mechanism for governance (still resting on a placeholder), and mobile
(Android/iOS, CLAUDE.md's platform-priority section) hasn't been started
at all. If a future `iroh-docs` upgrade or new finding turns any of the
resolved SPEC.md forks out to be wrong, don't patch around it quietly —
revise the relevant section the same way every previous revision is
recorded, old reasoning kept visible, not deleted. That's the established
practice in this document; keep it.

## Language and structure

Rust (`.gitignore` already assumes it; iroh's canonical SDK is Rust).
`street-smarts` (this design's origin repo, also Jason's) uses a Cargo
workspace under `crates/` with a `[workspace.package]` block for shared
metadata — this repo now has a minimal one-crate version of that same
shape (`crates/atproto-iroh-core`, currently a stub). Don't invent the
full crate breakdown before the `iroh-docs` experiments above answer
enough of §6 to know where the real module boundaries are — a premature
multi-crate split encodes uncertainty as if it were architecture.

## Client

Decided, not yet built: **Tauri**, Rust backend embedding
`atproto-iroh-core` directly as a library — no IPC/FFI boundary between
UI and protocol logic the way an Electron+Node frontend would need
talking to a Rust core, small binary, genuinely offline (nothing here
has a server to reach even if the UI wanted one). Real cost accepted
knowingly, not overlooked: Tauri renders through the OS's native webview
(WebKit on macOS/Linux, WebView2 on Windows) rather than bundling
Chromium, so cross-platform rendering isn't as uniform as Electron's.
Jason's call, explicitly: worth it for the smaller footprint, and the
visual layer doesn't need to be load-bearing from day one — this repo is
MIT-licensed specifically so a rougher-but-functional reference UI is a
fine place to start; anyone who wants it prettier can make it prettier.
Mobile exists in Tauri 2.0 but is newer than its desktop story — untested
here, worth pressure-testing before assuming it if a phone client ever
matters.

Scaffolded now: `crates/atproto-iroh-tauri` — a separate crate/app
(Tauri's own `src-tauri` shape) depending on `atproto-iroh-core` as an
ordinary path dependency, not code added to the core crate itself (the
existing one-crate caution above is about premature protocol-side
splitting, never about mixing UI and protocol logic into one crate,
which shouldn't happen regardless of how many crates the protocol side
ends up as). Fifteen commands now — sharing/joining, a generic state
inspector, QR ticket rendering, freeform text, and governance included
(`spawn_node`/`node_did`/`create_namespace_with_profile`/
`list_namespaces`/`share_namespace`/`join_namespace`/`dump_namespace`/
`ticket_to_qr`/`write_text`/`read_text`/`submit_to_inbox`/
`list_proposals`/`governance_state`/`create_proposal`/`create_signal`),
a plain HTML/JS/CSS frontend with no bundler and no framework. The inspector
(`namespace::dump_all`) is genuinely per-record-type-agnostic — every
entry's bytes get tried as JSON, then UTF-8 text, then a hex fallback, so
it already covers every record type this crate defines and every
freeform write, without having been told about any of them individually.
`namespace::put_text`/`submit_text` (SPEC.md §5's "confirmed in code"
note) are the concrete answer to "a namespace doesn't have to hold typed
records at all" — shared mutable text at any key, no lexicon required;
`submit_text` specializes that for uncoordinated public submission (a
public inbox), safe against forgery by construction
(`RecordIdentifier`'s `(namespace, author, key)` shape, §3.4) even when
the write ticket granting access is deliberately posted somewhere public.
`cargo build -p atproto-iroh-tauri` verified clean, not just written —
see its own `README.md` for exactly what that check did and didn't
cover, and for everything real still missing (a real founding-record
mechanism, mute UI, QR scanning) before this is a usable app rather
than a proof the layers connect. Governance is wired
(`list_proposals`/`governance_state`/`create_proposal`/`create_signal`)
— see the README's note on what it's still resting on a placeholder for.

**Persistence is real now too** (`identity::Identity::load_or_generate`,
`namespace::Node::spawn_persistent`) — data under
`$XDG_DATA_HOME/atproto-iroh`, proven live in
`crates/atproto-iroh-core/tests/persistence.rs` (spawn, create a
namespace, write a record, a genuine shutdown, spawn a new `Node`
against the same files, check the DID/namespace/record all survived).
Found and fixed a real bug wiring it, not something cosmetic:
`Node::spawn()` had always generated a random `SecretKey` internally,
completely disconnected from whatever `Identity` a caller displayed as
a `did:iroh` — the shown DID and the node's actual network identity
were two unrelated keys. `spawn_persistent` threads the identity's real
key into the `Endpoint`, so persisting the identity file now persists
the thing it claims to.

**At-rest encryption, decided: an env var pointing at an
already-encrypted volume, not app-level crypto.** `paths::data_dir()`
(shared by `identity`/`namespace`/`mute`, so it's one setting) checks
`$ATPROTO_IROH_DATA_DIR` before falling back to the XDG convention —
point it at a FileVault/BitLocker/LUKS/encrypted-container mount and
that's the whole feature. Explicit call, not an oversight: "nothing can
protect someone from themselves" — if the volume's unlocked, so is
everything on it, same as any other file there, and this crate doesn't
need to own that security model when the OS already solves it.

**Extension model, decided: PRs to this repo, not a plugin system.**
Building governance's UI surfaced a real question — should new content
types (a calendar, a photo board, a chat) be lexicon-based plugins
inside this one app, or should this stay a shared library other apps
depend on separately? Jason's answer, and the plan going in: this repo
is MIT-licensed specifically so people extend it by contributing code
back, not by loading third-party plugins into a runtime extension point.
Concretely: `atproto-iroh-tauri` stays a thin reference shell (identity,
namespace/share/join, the generic inspector, freeform text as the
fallback for anything without dedicated UI); a genuinely different app
either grows inside this repo via a real PR (a new content type gets a
real Rust module and its own UI section, same shape governance just got,
not a dynamically-loaded schema) or lives as its own separate crate/app
depending on `atproto-iroh-core` directly. No formal plugin
architecture — don't build one speculatively; if a shared-UI-toolkit
seam becomes obvious once a few such PRs or separate apps actually
exist, extract it then, from evidence, the same way this repo's own
crate boundaries got decided.

## Federation model, platform priority, and background execution

Jason's framing (2026-09-22): this is for **tight autonomous orgs to
federate**, not a consumer app aiming for always-on personal devices as
its baseline. Some orgs are expected to stand up their own lightweight
node — self-hosted now, possibly a Jason-hosted option later — that stays
up and acts as a durable peer for that org's namespaces. That single fact
resolves what would otherwise be a hard problem below.

**Platform priority, for the record: Android, then iOS, then Linux, then
Windows.** Phones first because that's where non-technical members
actually are; desktop Linux/Windows matter more for the org-run
lightweight-node case than for member-facing UX.

**Background execution — fleshed out, not relied on.** The naive
assumption ("the app just stays running and syncs") breaks hardest on
Android: Doze/App Standby aggressively suspends processes and kills
sockets for anything not foregrounded or allowlisted, so a bare iroh
`Endpoint` sitting in a background service will get its QUIC connections
cut, unpredictably, the moment the OS decides the app is idle. The real
options, in the order an Android build would reach for them:
1. A **foreground service** with a persistent notification — the only
   way to get a real background socket on modern Android, and it costs
   the user a permanent "atproto-iroh is running" notification, which is
   a real UX tax for a tech-hostile audience, not a footnote.
2. **Battery-optimization allowlisting** (`REQUEST_IGNORE_BATTERY_
   OPTIMIZATIONS` or user-driven exemption) — needed even with a
   foreground service on some OEM skins (Samsung/Xiaomi/etc. layer their
   own aggressive killers on top of stock Android), and it's a
   permission prompt most users will decline or not understand.
3. `WorkManager`-scheduled periodic sync as a fallback for when neither
   of the above holds — bounded, OS-throttled (minimum ~15 minute
   intervals), so it's a "catch up eventually" mechanism, not a
   real-time one.

None of this is being built now, and — per Jason's explicit call —
**the architecture is not allowed to depend on it working.** The
federation model above is exactly why that's safe to defer: a phone that
opens the app, syncs against the org's always-on node (or any other peer
it can reach), and then closes is a complete, correct participant. The
org's lightweight node is what actually needs to be reachable
continuously; individual members' phones don't, and pretending otherwise
would mean building real complexity (steps 1–3) to solve a problem the
federation topology already solves at the org layer. iroh's own relay
fallback (`presets::N0`, not the `presets::Minimal` used in this repo's
tests so far) still matters here independent of background execution —
CGNAT and network-switching on mobile mean direct QUIC isn't always
reachable even in the foreground, so a relay-capable preset is the right
default for any Android build, separate from the backgrounding question.

**Android storage — wired now, not just noted.** The sandboxed-storage
gap this section used to just flag is closed:
`paths::set_data_dir_override` (a process-wide `OnceLock`, checked
before the env-var convention in `paths::data_dir()`) lets a host with
no shell environment hand this crate an explicit path instead. The
Tauri shell's `main.rs` calls it from a `.setup()` hook, behind
`#[cfg(any(target_os = "android", target_os = "ios"))]`, using
`app.path().app_data_dir()` — Tauri's own resolver, itself backed by
`Context.getFilesDir()`/`getExternalFilesDir()` on Android — so identity,
namespace storage, and the mute list all land in the real per-app
private directory instead of whatever `$HOME` happens to mean inside an
Android process (usually nothing usable). `spawn_node` also now binds
with `NetworkPreset::N0` (`namespace.rs`'s new enum, `presets::N0` under
the hood — relay fallback plus DNS address lookup) on Android/iOS
instead of the desktop-default `Minimal`, since CGNAT and network
switching mean direct QUIC often isn't reachable even in the foreground;
every existing test still uses `Minimal` unchanged. Separately, Android
storage mostly *simplifies* the at-rest-encryption story versus desktop:
Android encrypts app-private storage by default at the OS level (unlike
desktop, where `$ATPROTO_IROH_DATA_DIR` pointing at an encrypted volume
is opt-in) — so the "point it at an encrypted volume" feature stays a
desktop-specific concern, nothing Android needs replicated.

**What's still not done: an actual built, running APK.** Verified in
this sandbox: the `aarch64-linux-android`/`armv7-linux-androideabi`/
`i686-linux-android`/`x86_64-linux-android` Rust targets install cleanly
via `rustup target add`, `cargo-tauri` (the CLI `cargo tauri android
init`/`build` need) installs and runs, and `cargo tauri android init`
fails with a clean, expected error — `Android SDK not found at
/root/Android/Sdk` — because no Android SDK or NDK is installed here.
**Deliberately not installed in this sandbox**: the NDK alone is
multi-gigabyte unpacked, this environment's writable disk is a fixed,
non-refillable allowance (currently single-digit GB free), and a
partial or failed SDK/NDK install risks exhausting it for every repo
this session touches, not just this one — judged not worth that risk
for a step that can be done cleanly on a real dev machine or in CI
instead. To actually produce a running APK, from a machine or CI runner
with normal disk headroom:
```
# Android Studio's own SDK manager is the easiest path; the manual one:
sdkmanager --install "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;27.0.12077973"
export ANDROID_HOME=~/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973
cd crates/atproto-iroh-tauri
cargo tauri android init   # generates gen/android/ (gradle project) — not committed, generated
cargo tauri android build  # or `android dev` for a connected device/emulator
```
Nothing about the Rust-side code is expected to need changes for this —
the data-dir override and `NetworkPreset::N0` wiring above exist
specifically so `cargo tauri android init/build` has nothing left to
improvise once a real SDK/NDK is present. Worth pressure-testing for
real once that's run somewhere with the disk for it, not assumed clean
from the code alone.

## Batteries-included app list (Messaging built; rest still a recommendation)

Jason's ask (2026-09-22): for a non-technical/tech-hostile user base,
what should ship by default rather than being left to a future PR?
Grounded in the CRDT finding just above — every candidate below falls
into one of two patterns, and which one it falls into is the real design
question for each, not a UI detail:

- **Append-only / unique-key** (safe automatically, no extra work): each
  write gets its own key, so offline edits from different people never
  collide. Already the shape `NodeProfile`, `Proposal`/`Signal`, and
  `submit_text` use.
- **Shared mutable state** (needs the revision-history treatment just
  built, or an explicit "latest wins is fine here" call): multiple
  people can edit the *same* logical thing, so silent last-write-wins
  loss is a real risk unless it's deliberately either revisioned or
  accepted.

Recommended list, each tagged with its pattern:

- **Messaging — built (2026-09-22).** `crates/atproto-iroh-core/src/
  messaging.rs`: `Message` (`network.essmesh.chat.message`,
  `lexicons/network/essmesh/chat/message.json`), `send_message`/
  `list_messages`. A namespace is the channel — no separate room
  concept layered on top, same "the namespace already is the scoping
  unit" theme as everything else here. Append-only by nature (a chat is
  already a sequence of independent messages, one key per message, same
  shape `submit_text` already proved out), so it needed no new sync
  primitive — confirmed, not just claimed: proven live in
  `crates/atproto-iroh-core/tests/messaging.rs` (two nodes exchange a
  message and a reply, both see the full thread in order after sync).
  Wired into both clients: Tauri's `send_message`/`list_messages`
  commands and `dist`'s "Messages" section, and the CLI's `send`/
  `messages` subcommands (`send` prints the new message's own
  `{author_hex}/{rkey}` ref, copy-pasteable straight into a later
  `send --reply-to`).
- **Documents** — shared mutable state; this is exactly what
  `save_document_revision`/`list_document_revisions`/`load_document`
  (just built) are for. Needs real UI on top: showing "someone else
  edited this while you were offline" and a merge/pick step, not just
  silently picking the latest revision the way `load_document`'s default
  does.
- **Images** — append-only (each upload is its own blob + one metadata
  entry referencing it via `iroh-blobs`' content addressing); no new
  sync primitive needed, but real work in surfacing/thumbnailing that
  this repo hasn't touched.
- **Tagging — built (2026-09-22), and cross-lexicon by construction**
  (Jason's explicit requirement). `crates/atproto-iroh-core/src/
  tagging.rs`: `Tag` (`network.essmesh.tag`,
  `lexicons/network/essmesh/tag.json`), `add_tag`/`list_all_tags`/
  `tags_for`. Append-only, same shape as messaging — a tag is its own
  small record, not a mutation of the tagged item's own entry, so
  multiple people tagging the same thing concurrently just accumulates.
  The cross-lexicon part needed a new shared primitive:
  `records::record_ref`/`parse_record_ref`
  (`"{author_hex}/{collection}/{rkey}"`) — unlike `governance::
  subject_ref`/`messaging::reply_ref`, which only ever point within
  their own collection (a `Signal` always targets a `Proposal`, a reply
  always targets a `Message`), a tag can point at *any* record in *any*
  collection, so it's the one reference format that actually has to
  carry the collection name. Proven live in
  `crates/atproto-iroh-core/tests/tagging.rs`: the same `Tag` type tags
  a real `network.essmesh.chat.message` and a plain freeform document
  revision (no lexicon at all) in one test, both sync, both are found
  correctly by `tags_for` — no per-record-type tagging code anywhere.
  Wired into both clients: Tauri's `add_tag`/`tags_for` commands (a
  "Tag" button on each message in `dist`'s Messages section fills in the
  Tags panel's subject field) and the CLI's `tag`/`tags` subcommands.
- **Search** — not a sync-pattern question at all; a local index over
  whatever's already synced (`dump_all`'s reflective read is the
  existing hook a naive version could build on). No offline-conflict
  story because it's derived, read-only state, never itself synced.
- **Calendar** — the one candidate whose right pattern isn't obvious
  either way: an *event* (append-only, like a message) versus an *edit to
  an existing event* (shared-mutable, same doc-revision problem as
  documents — two people rescheduling the same meeting offline is a real
  case). Recommend modeling events as append-only creation plus
  revisioned edits, same shape as documents, rather than a single
  mutable event record.
- **Suggested additions**, not asked for but falling naturally out of
  what's already built:
  - **A member directory** — mostly already exists as a byproduct of
    `NodeProfile` plus `list_records`; needs UI, not new sync design.
  - **Presence/"who's around"** — deliberately *not* worth building on
    this substrate as a persisted record type; presence is inherently
    ephemeral and this system has no server to hold "currently online"
    state cheaply. If wanted at all, it belongs on `iroh-gossip` (already
    a dependency, used for docs sync) as a live broadcast, not a synced
    document.
  - **Polls** — this is governance's `Proposal`/`Signal` machinery,
    already built and wired, just without calling it "polls" in the UI.
    Worth surfacing as its own app-facing label rather than building a
    second, separate mechanism.

None of these are built beyond what's named above as already existing
(messaging's and images' primitives, governance-as-polls, the directory
byproduct). This is a recommendation and a pattern classification, not a
commitment to build the remaining UI this session.

## Linux CLI

`crates/atproto-iroh-cli` (binary `atproto-iroh`) — a second, non-GUI
client for driving a node from a script or local agent, separate from
Tauri's identity/data directory on purpose (`cli-agent/` subdirectory, so
the two can't file-lock-collide running side by side). One-shot process
per command, plus a `serve` mode for anything that needs to actually be
reachable — see its own README for the real bug this found (a `share`
ticket naming a stale port once its issuing process exits) and the fix
(`serve --share`), verified as a genuine two-process QUIC sync, not just
compiled.

## Lexicons

`lexicons/network/essmesh/` holds four drafted-but-unvalidated atproto
lexicon schemas (`node.profile`, `node.event`, `governance.proposal`,
`governance.signal`) — the concrete ESS (cooperative/solidarity-economy)
case this protocol was designed against. See `lexicons/README.md`,
including why there's deliberately no `Ratification` lexicon. Whether to
rename these to a generic namespace, keep them as-is as a reference
implementation, or add a second generic lexicon set is an open decision
(`SPEC.md` §6) — don't rename them unilaterally while implementing
something else; that's a design decision on its own.

## Relationship to `jedelman/street-smarts`

Cross-repo now, not in-repo. `street-smarts`'s `tools/sbci/sbci/ess_source.py`
defines `CATEGORY_WEIGHTS` whose keys this repo's `node.profile.category`
lexicon copies verbatim (`SPEC.md` §3.3, `lexicons/README.md`). No
automated check keeps them in sync — if one changes, watch for the other
silently drifting. Worth an actual CI check once both sides have real
code, not before.

## Git conventions

Not established yet for this repo specifically. This is a personal
project of Jason's, not (yet) a reviewed or collaborative codebase — when
in doubt, ask rather than assume a policy like `street-smarts`'s
`claude/**`-scoped merge authority applies here; it doesn't, unless and
until Jason says so for this repo.
