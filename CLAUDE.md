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

No longer a design sketch with no code, and no longer just a scaffold
either — this section drifted badly behind the actual state of the repo
for a while (a standing risk this doc's own "keep it accurate" discipline
exists to catch; caught and fixed 2026-09-22, see git history for
exactly how stale it had gotten). Current, real state:

`crates/atproto-iroh-core` has eight real modules (`identity`,
`namespace`, `records`, `governance`, `fold`, `mute`, `paths`,
`messaging`, `images`, `tagging` — not a stub, not five), all
exercised live: unit tests for pure logic (governance/ratification,
`resolve_founding`, `record_ref`) plus a dozen live two-node integration
tests doing real QUIC sync — messaging, images, tagging, document
revisions, governance folding, founding claims, persistence, all proven
against the actual crate, not asserted. Governance is real end to end,
including a real founding-record mechanism (`governance::Founding`/
`resolve_founding`, `fold::found_namespace`/`fold_namespace` — SPEC.md §6
item 16), not the placeholder heuristic earlier versions of this section
described. Persistence and at-rest encryption are real. The CRDT
data-loss case in `put_text`-at-a-fixed-key is fixed (version-preserving
`save_document_revision`/`list_document_revisions`/`load_document`,
SPEC.md's 2026-09-22 CRDT note) and the Tauri "Shared doc" UI is wired to
it, including a conflict-visibility pass (Documents UX, batteries list
below) — not just the primitive existing unused.

Two clients now, both real: the Tauri reference app (33 commands — see
"Client" below) and `crates/atproto-iroh-cli`, a one-shot-process Linux
CLI plus a `serve` mode for anything that needs to stay reachable (own
README, including a real bug it found and fixed live: a `share` ticket
naming a port nothing was still listening on once its issuing process
had exited). Four of the batteries-included apps are built (Messaging,
Images, Tagging — cross-lexicon by construction — and Documents UX);
see that section for exactly what's still just a recommendation
(Search, Calendar, and the deliberately-not-building-it presence case).

What's still genuinely unbuilt: real APKs now exist for all four
Android ABIs (2026-09-23, built, size-optimized, and the arm64 one
debug-signed and sent to Jason for device testing — federation-model
section has the full account, including a real Rust-side fix the build
required despite an earlier claim that none would be needed) but none
have actually been installed and clicked through on a real device or
emulator yet (none available in this sandbox), and there's no real
release keystore, only a local throwaway debug one; QR scanning is now
built both ways (generate and read back, Client section below) but
unverified on a real camera; mute/profile UI has no CLI parity for
profile specifically; and no thumbnailing/format validation on images.
If a future `iroh-docs` upgrade or new finding turns any of the resolved
SPEC.md forks out to be wrong, don't patch around it quietly — revise
the relevant section the same way every previous revision is recorded,
old reasoning kept visible, not deleted. That's the established practice
in this document; keep it, and keep *this section* honest too — it's
the one a reader hits first, so letting it lag the rest of the document
defeats the whole point of writing the rest of it accurately.

## Language and structure

Rust (`.gitignore` already assumes it; iroh's canonical SDK is Rust), a
Cargo workspace under `crates/` with a `[workspace.package]` block for
shared metadata — same shape `street-smarts` (this design's origin repo,
also Jason's) uses. Three real crates now, each with a settled reason to
be separate, not a premature split: `atproto-iroh-core` (protocol logic,
no UI dependency of any kind), `atproto-iroh-tauri` (the reference GUI,
depends on `-core` as an ordinary path dependency), `atproto-iroh-cli`
(the Linux agent-driving client, same relationship). Within `-core`,
still one crate on purpose — the original caution here was against
splitting the *protocol* logic prematurely before its own module
boundaries were evidence-based, and that's still true: `identity`,
`namespace`, `governance`/`fold`, `mute`, `messaging`, `images`,
`tagging` are modules within one crate, not separate crates, because
nothing has yet forced them apart.

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

`crates/atproto-iroh-tauri` — a separate crate/app (Tauri's own
`src-tauri` shape) depending on `atproto-iroh-core` as an ordinary path
dependency, not code added to the core crate itself. 28 commands now,
across identity/namespace/sharing, the generic inspector, QR ticket
rendering — plus QR *scanning* now too (2026-09-23), read back via a
vendored `jsQR` and `getUserMedia`, no new Tauri command since it's pure
client-side JS; own README has the real Tauri-3-alpha plugin dead end
this worked around — Shared doc (with conflict-visibility UX), Messaging, Images,
Tagging (cross-lexicon), Members/Profile, Mute, and governance (real
`Founding`-based genesis state, not a placeholder) — a plain HTML/JS/CSS
frontend, no bundler, no framework. **Superseded 2026-09-23**: that
`dist/` frontend is deleted, replaced by a real React + Vite app under
`src/` per the confirmed `/shape` design brief
(`crates/atproto-iroh-tauri/DESIGN_BRIEF.md`, and this document's own
"Design context" section below) — a deliberate departure from "no
bundler, no framework," not a regression from it. Implementation
started the same day and kept going across the rest of it: **seven
screens now built and tested** (a canonical mock dataset + a Vitest
suite that grew to 33 tests as each screen landed — this crate's own
README has the detail) — Feed, Onboarding, Table detail (Members-first
landing, pinned messages, and five tabs: Messages with a real optimistic
Composer and per-message Tagging/browse-by-tag, Decisions with real
propose/Support/Object forms, Photos with real byte-verified upload,
and Shared doc with conflict-visibility UX), Join (QR scan + paste
fallback), Profile edit (per-Table, since `NodeProfile` is per-namespace),
and Mute (global, with real content filtering in the Feed and Table
detail, not just an inert list). What's left: Messaging thread view
*outside* a Table's own tab (no reply-to yet), the full Governance UI
beyond General-class decisions (no admit/remove-cosigner or
policy-change forms), Tagging on Images/Shared docs (Messages only),
QR *generation*, the raw inspector, and relay/control — a real,
narrower gap now, not the wholesale step back this section used to
describe; `crates/atproto-iroh-tauri/README.md`'s own "Not yet ported"
list is kept current there rather than duplicated and re-drifting here.
The 33 Tauri commands (28, plus `pins`/`list_all_tags` added alongside
their `tagging::` core-crate counterparts, plus `list_tables`/
`self_author_hex`/`set_table_name` from device testing) are unaffected
either way, since this is a frontend rebuild over a backend that only
gained three small additions (`pins`, `list_all_tags`,
`NodeProfile.avatar`) for it. Exact command names, what each does,
and what's still missing live in `crates/atproto-iroh-tauri/README.md`,
kept current there rather than duplicated and re-drifting here; this
section stays high-level on purpose. The inspector (`namespace::
dump_all`) is genuinely per-record-type-agnostic — every entry's bytes
get tried as JSON, then UTF-8 text, then a hex fallback, so it covers
every record type this crate defines without having been told about any
of them individually, current or future. `namespace::put_text`/
`submit_text`/`put_bytes` (SPEC.md §5's "confirmed in code" note) are
the concrete answer to "a namespace doesn't have to hold typed records
at all" — shared mutable text or bytes at any key, no lexicon required;
`submit_text` specializes that for uncoordinated public submission (a
public inbox), safe against forgery by construction (`RecordIdentifier`'s
`(namespace, author, key)` shape, §3.4) even when the write ticket
granting access is deliberately posted somewhere public.

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

## Design context for the Tauri reference client's UX

Resolved 2026-09-23, ahead of a real design pass (`/shape`) on
`crates/atproto-iroh-tauri`'s frontend — the reference UI has been
deliberately plain up to now ("functional, not precious," `dist/`'s own
tagline), and this is the direction for moving past that. Full detail
in `.impeccable.md` (design skills read that file directly); summarized
here per this document's own practice of keeping every resolved
decision visible in one place, not scattered.

**Users: broad, not activist- or cooperative-specific**, even though the
protocol's own origin (root README) was a cooperative/solidarity-economy
case. A friend group, a family, a book club, a mutual-aid crew — anyone
coordinating privately without handing it to a platform. Non-technical,
sometimes tech-hostile (CLAUDE.md's batteries-included section already
established this) — the UI needs zero required understanding of
capabilities, `did:iroh`, or tickets to use day to day; that machinery
stays available for the curious, never a prerequisite for anyone else.

**Brand personality: warm and communal — "a shared kitchen table, not a
corporate dashboard."** Handmade, personal, willing to be a little
imperfect on purpose rather than sanded to generic-SaaS smoothness.
Explicitly turned down two other live options during the design
interview ("calm/trustworthy fintech" and "sharp/utilitarian
infrastructure") in favor of this one.

**Theme: dark default, deliberately warm-dark rather than tech-dark.**
The governance/messaging usage pattern (checked at odd hours, often on a
phone) favors dark on its own; the warmth has to come from color
temperature and type choice on top of that, not from defaulting to
light instead. Concretely: warm neutrals (a hint of brown/amber in the
grays, not blue-black) and a genuinely warm accent hue — explicitly
rejecting the reflexive cyan/purple "AI dark mode" palette. No specific
reference sites given; full creative latitude on execution.

**One UX responsibility worth naming explicitly**: the governance
model's "not a majority vote" framing (ratifies by default, absent
enough objection) is unfamiliar to most people. The UI carries real
weight in making silence-means-consent read as calm and legible, not as
a loophole or a trick — this is a design problem, not just a copy one.

**`/shape` ran against this context the same day** and produced a
confirmed design brief — `crates/atproto-iroh-tauri/DESIGN_BRIEF.md`.
Headline decisions from it, worth having here rather than only in that
file: the frontend is being rebuilt in **React + Vite** (a real
departure from the "plain HTML/JS/CSS, no bundler, no framework" rule
that held until now — deliberate, chosen during the interview, not a
regression from it), and a namespace is called a **"Table"** in every
user-facing surface (`.impeccable.md`'s Vocabulary section has the
reasoning).

**Same-day follow-up guidance added three structural decisions**,
folded into the brief rather than treated as separate: a **unified
cross-Table feed is the home screen** (Bluesky-shaped, explicitly
reacting to Discord's per-server-switcher becoming overwhelming past a
couple of servers — Tables and tags become feed filters, not separate
destinations); **pinned messages per Table** (reuses existing
messaging+tagging, resolved as fully open — see below, not admin-only
as first floated); and **built-in profile stickers** as part of
first-run onboarding ("even bots need cute stickers"), custom upload
deliberately deferred.

**"Who can pin?" — resolved the same day: anyone, one pin per author,
newest first.** Jason's exact spec, matching `tagging.rs`'s existing
open/append-only posture rather than inventing a new permission
concept. **Built and live-tested, not just designed**: `tagging::
PIN_LABEL` (`"system:pin"`) and `tagging::pins()` (latest-per-author
resolution, same dedup shape as `governance::latest_signal_per_author`)
— `tests/tagging.rs`'s `tags_can_tag_tags_and_pins_resolve_one_per_
author_by_recency` proves the resolution rule against real synced
history. This also confirmed the deeper "**tags are monads**" claim
behind it: `Tag.subject` can point at *another* `Tag` — proven live,
not just architecturally plausible — so this crate now reserves a
`system:` label-namespace prefix for its own built-in meanings while
leaving every other label open for user-invented ontology, no schema
change either way (`tagging.rs`'s own top doc comment has the full
reasoning). Not yet wired to a Tauri command or CLI subcommand — core
only so far.

**Correction to "nothing about `atproto-iroh-core` changes"**: mostly
still true, but not entirely — profile stickers need one small, real
addition (`NodeProfile` gaining an avatar/sticker field, confirmed
absent from the current schema); pins needed none (`tagging.rs`'s
`label` was already a free string). Said plainly rather than left
standing wrong; `DESIGN_BRIEF.md`'s intro has the same correction with
full detail, and its Open Questions section has what's still actually
open (feed-query architecture at scale, the sticker field itself).

## Observability: release builds only, telemetry rides the sync primitive

Jason's standing constraint (2026-09-23): **he usually won't have a
debugger attached** when running this on his own devices. Two
consequences, both policy from here forward:

**Build release, not debug, as the default — always.** Confirmed the
practical case for this live, not just in principle: building all four
Android ABIs with `--debug` in this sandbox exhausted its entire disk
allowance (debug's `debuginfo=2` made each target's `.a`/`.so` several
times larger than the same target in release — four ABIs' worth of
debug artifacts ran the sandbox out of space mid-build). Release is now
the default for every `cargo tauri android build` / `cargo build`
invocation in this repo's own workflow; `--debug` is for a real
debugger session on a real device, not the default path, here or on
Jason's own machine.

**Telemetry is needed, and it should be built as just another synced
record type, not a new channel.** Raised in conversation: can a Claude
Code sandbox receive telemetry directly over iroh? Answered live, not
assumed: **no, not this kind of sandbox, for three separate reasons.**
(1) Outbound iroh discovery itself already fails here — confirmed twice
this session (`tests/control.rs`, and two real CLI processes) dialing a
bare `did:iroh` under `NetworkPreset::N0` gets "No addressing
information available / all address lookup services failed"; this
environment's egress is an HTTPS-only proxy (its own README says so)
and iroh's relay/DNS discovery doesn't ride on that path — direct QUIC
between two peers *already inside* the sandbox works fine (that's
exactly what the control-endpoint tests prove), it's specifically
*reaching out* to iroh's real relay infrastructure that's blocked. (2)
Nothing indicates this container has any public inbound listener at
all — it's an ephemeral box behind an egress-only proxy, not something
an external phone could dial into even if the discovery problem in (1)
were solved (not independently verified from the outside — there's no
way to test that from inside the sandbox alone — but nothing suggests
otherwise either). (3) The container is reclaimed when the session
ends regardless, so it's not durable even if reachable.

**The actual design, which reuses everything already built rather than
inventing a new transport**: telemetry is an append-only record type,
same shape as `messaging`/`tagging` — one entry per event, written to a
namespace the device already owns, synced by the exact same mechanism
as everything else here. No phone-home, no new channel, same
capability-scoped trust model as every other record type: nothing
leaves the device except into a namespace whose capability Jason
explicitly controls. Two real sink options, not mutually exclusive:
- **The org's durable relay box** (the Raspberry-Pi-as-sold-hardware
  design, above) as the always-on receiver — a device syncs its own
  telemetry namespace to it the same way it syncs anything else.
- **A future Claude Code session with real network access** (unlike
  this specific locked-down sandbox) doing an ordinary `join <ticket>`
  + `dump`/query against that namespace when Jason wants eyes on it —
  no different from inspecting any other namespace this repo already
  knows how to read.

Not built yet: the telemetry record type/module itself, the panic-hook
or `tracing`-layer wiring to actually populate it, and which of the two
sink options (or both) Jason wants wired up first. This section records
the direction and the reasoning, not a commitment to build it this
session.

**Superseded (2026-09-24): telemetry is a local ring buffer, not a
synced record type.** Jason's call, in the same conversation that
resolved the offline-detection design below: a bounded in-memory diary
of connectivity/sync events, exportable to a file on request, not
another namespace to sync. Simpler, and it turns out to share its event
source with the offline-detection work below rather than needing its
own — see that section for the concrete plumbing. The synced-record-
type design above stays on record rather than deleted (this doc's own
practice), but it's not the plan going forward.

**Offline/no-peers-reachable — resolved (2026-09-24), the frontend gap
this session's design-interview check flagged as "invisible."** First
pass at this claimed no primitive existed (`Endpoint::remote_info`
needs a peer id you already have) — wrong, or at least incomplete:
checking `iroh-docs`' own event stream turned up the right layer, which
is per-Table, not per-arbitrary-peer anyway. Two real signals, both
confirmed directly in the vendored crate source, not assumed from
memory:
- **Per-Table peer presence**: `docs.subscribe(namespace)` yields
  `LiveEvent::NeighborUp(PublicKey)`/`NeighborDown(PublicKey)` as peers
  join/leave that Table's sync swarm, plus `SyncFinished(SyncEvent)` on
  a completed reconciliation. A `HashSet<PublicKey>` per namespace,
  maintained by draining this stream, is an honest "how many other
  devices can I sync this Table with right now" count.
- **Global reachability**: `Endpoint::home_relay_status()` (a
  `Watcher<Value = Vec<RelayStatus>>`, each entry's `is_connected()`)
  and the `online()` convenience future built on it — "can I reach the
  outside network at all," independent of any one Table.

**The design, not yet built**: a `connectivity` module in
`atproto-iroh-core`, one background task per spawned `Node` draining
both event sources — per-namespace neighbor counts from the first,
relay-connectedness from the second — feeding two things at once: (1) a
queryable `connectivity_state(namespace_id) -> { peer_count,
relay_connected }` for the frontend's offline banner, and (2) the
telemetry ring buffer above (the same transitions are exactly what's
worth logging for diagnostics — this is the shared plumbing referenced
there). Tauri command + frontend hook follow once the core piece
exists. Not built this session — recorded here so the next session
doesn't re-derive it or repeat the "no primitive exists" mistake.

**Interim (2026-09-25): the frontend polls.** Device testing showed
open screens never refetched, so `usePoll` refreshes a Table every 5s
(Feed 10s) and on returning to the foreground. The `docs.subscribe`
stream above is also the natural push-on-arrival replacement for that
polling — one subscription serving the offline banner, the ring
buffer, and refresh. Same session found a real sync bug under all of
this: `open_namespace`/`create_namespace` never called `start_sync`,
and iroh-docs refuses incoming sync for a namespace that isn't live, so
a restarted node (phone or relay `serve`) silently stopped receiving.
Fixed in core, proven by `tests/reopen_sync.rs`.

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

**The org's lightweight node as sold hardware, not just self-hosted
software** (raised in conversation, 2026-09-22, floated as a real
product direction — a Raspberry Pi or similar, preloaded, a small
screen showing a join QR code, a single physical reset button, battery
option for field use). This is the concrete answer to "who runs the
always-on peer" that doesn't reintroduce a company to trust: the org
buys a box once instead of renting a service forever, so there's still
nothing centralized to acquire, subpoena, or enshittify — the box is
just this repo's own `serve`/CLI (Linux CLI section below) running on
hardware instead of a laptop somebody has to remember to leave on.
Genuinely solves discovery too, not just uptime: `ticket_to_qr` already
renders a join ticket as a QR code, so the box's screen showing it
physically *is* the onboarding flow — no directory, no invite link
infrastructure, someone points a phone at the box. Not built as a
product (no custom OS image, no screen/GPIO integration, no case) — the
software side (a CLI that can `serve` indefinitely, real persistence, a
real join flow) already exists and is what such a box would run
unmodified.

**Sold hardware deferred to V2/V3 (2026-09-24); V1 is a containerized
relay, deploy-it-yourself.** Jason's call: skip the physical-device
product for now and ship a Nix flake producing an OCI image for
`atproto-iroh-cli`'s `serve --share` mode — `nix build .#relay-container`
(exact attribute name TBD), runnable on Docker/Podman/a NixOS host via
the same derivation, no screen/GPIO/case work implied. A NixOS module
(a systemd unit wrapping `serve`) is close to free once the package
derivation exists and is worth adding alongside the container image,
but the container is the actual V1 deliverable. Everything above this
paragraph (relay mode needing no authoring identity, the control
endpoint's join/reset, publication-as-provisioning) is unchanged by
this — it's still the same `serve` binary, just packaged for
self-deploy instead of sold pre-loaded. Not built yet this session;
recorded here so the direction doesn't get re-litigated as "Pi vs.
container" next time — it's "container now, Pi (or similar) later."

**Relay mode — built (2026-09-22): the box doesn't need an authoring
identity, only a network one.** Raised in conversation as a feasibility
question about the Pi box above: does a purely-passive relay need to
"be" anyone? No, and the reason is structural, not a policy choice —
every entry in this design is self-authenticating (signed by its
original author, content-addressed), so a node that only stores and
forwards other people's already-signed entries never has to sign
anything or vouch for content itself, the same way a CDN caches content
it didn't create. Capability here is also a bearer secret, not bound to
who's holding it (SPEC.md §3.4) — a Read-capability relay is sufficient
to receive full history and serve it back to any other peer holding at
least Read into the same namespace; it never needs write access of its
own. **Confirmed live, not just reasoned through** — and a real
correction found in the process (see `crates/atproto-iroh-cli/src/
main.rs`'s top comment for the full account): the first version of this
claimed a relay-mode CLI process creates *zero* `AuthorId` at all;
testing found that's wrong — `iroh-docs`' `DefaultAuthor::load` mints
and persists one unconditionally inside `Docs::persistent(...).spawn(
...)`, regardless of whether application code ever asks for one, and
there's no way to opt out through the public `Docs::persistent()`
builder this crate uses. What *is* true, confirmed by a live two-node
test (`join` a Read-mode ticket, `dump` the relay's copy): the relay's
local unused author never appears in any synced entry, because nothing
in relay mode (`join`/`dump`/`messages`/`tags`/`images`/`serve` — every
CLI command that doesn't need to write) ever calls it to sign anything.
CLI refactored so `author()` is resolved lazily per-command rather than
unconditionally in `main()`, which is what makes that true — a relay
process really can run `join <ticket>` (once per namespace) then
`serve` indefinitely without its authoring key ever leaving local
storage or reaching a peer, even though the key itself still exists on
disk as an iroh-docs implementation detail.

**Publication story — there isn't one, on purpose.** The relay doesn't
discover namespaces; it's provisioned, exactly like any other peer —
whoever sets up the box runs `join <ticket>` once per namespace it
should hold. Raised as a SPOF concern (2026-09-22): is "peers get keys
from the relay" a single point of failure? Narrower than it sounds,
because of the bearer-secret property above — a ticket *is* the
capability, not a claim check for one, so once someone scans the box's
QR code they hold the actual secret on their own device permanently,
independent of the relay's continued existence. The relay is a SPOF for
exactly one moment (the initial handout — if it's offline right when
someone wants to join, they can't onboard that second) and not for
anything after that: existing members keep syncing with each other, or
re-share tickets peer-to-peer, with no further dependency on the box.
The sharper risk isn't availability, it's replication — if the relay is
the only node that ever holds full history, losing it loses whatever
never made it anywhere else, which is a reason to run more than one
relay for anything that matters, not a flaw in the single-relay model
itself.

**Headless provisioning — a real gap, not solved by the CLI existing.**
Raised in conversation (2026-09-22): the box has no keyboard or display,
so how does anyone actually run `join`/`serve --share` on it, and how
does a ticket get *out* of it to onboard people? Two separate problems,
neither built:
- **Initial setup** isn't new work — this is the standard headless-Pi
  pattern (Raspberry Pi Imager pre-flashes SSH-enabled, WiFi-configured
  SD cards already), so the org's admin SSHes in once (or a first-boot
  script does it for them), runs `join <ticket>` per namespace and
  `serve --share`, then walks away. Nothing about this repo needs to
  solve that problem, only use it.
- **Handing out a ticket afterward is genuinely two different
  products, not one box with an optional accessory.** *Admin-
  distributed*: the admin captures the ticket text over that same SSH
  session once, at setup, and distributes it however they already
  communicate (their own laptop can run `ticket_to_qr` and print it) —
  the box stays headless forever, onboarding never touches it again.
  *Self-service walk-up*: the box has a real screen rendering a live
  QR, so anyone can onboard without an admin in the loop — this is the
  version originally floated, and it's real, unbuilt GPIO/display work,
  not something the headless SKU gets by default. Which one to actually
  build is a product decision, not resolved here — they pull toward
  different hardware.
- **Adding a second namespace to an already-running box** currently
  means stopping `serve`, running `join` again, restarting — an SSH
  operation, fine for one-org-per-box, awkward for a box meant to relay
  several groups at once. Not addressed.

**The control endpoint — built (2026-09-22): a raw message endpoint, not
a synced doc, replacing an earlier "docs-inbox" idea, and it doubles as
the poison pill.** The full arc, kept visible rather than collapsed to
just the final design, per this document's own stated practice:

1. **First proposal: the QR code is a docs-namespace inbox.** Raised in
   conversation: "the qr code is an inbox. If anyone sends a repo read
   ticket to that inbox the relay subscribes to the repo. Then we don't
   need a privileged admin role and that secret can be distributed in
   whatever way is appropriate to the org." Feasible — reuses
   `submit_text` (an uncoordinated public-write primitive that already
   exists) plus `Node::join` — but evaluated with two real tradeoffs
   before building it: an open-relay/flooding risk (anyone who can write
   to the inbox can make the box try to join arbitrary namespaces), and
   a genuine cross-org privacy leak specific to a box serving more than
   one group — Read capability into a shared inbox namespace means
   seeing *every* ticket ever submitted to it, not just the one you sent.
2. **Revised: a raw message endpoint instead.** Follow-up: "What if it's
   a message endpoint not a doc?" Strictly better on the leak: a direct,
   ephemeral, point-to-point QUIC connection (this module alone ever
   sees it) has no shared or durable state to leak from at all — the
   cross-org problem disappears structurally, not by policy. Also
   simplifies onboarding: the QR only ever needs to encode the box's
   bare `did:iroh`, the same thing it already needs to be dialable for
   ordinary docs sync, not a capability into anything. Still didn't
   solve authorization-of-who-can-ask (anyone reaching the address can
   ask it to `JOIN`) — deliberately left that way, reasoned through
   below.
3. **Built, with the poison pill added in the same request**: "Build it.
   But remember the reset button I was telling you about? This address
   should also accept a poison pill that wipes the box and generates a
   new qr code. That's why I wanted a screen on it!" —
   `crates/atproto-iroh-core/src/control.rs`, one `ProtocolHandler` on
   its own ALPN (`CONTROL_ALPN`), registered on the same `Router` as
   blobs/gossip/docs. Two plain-text operations, one request/response
   each:
   - **`JOIN <ticket>`** — needs no authorization beyond "you can reach
     this address." Deliberate, not an oversight: joining a namespace
     already requires holding that namespace's own real capability (the
     ticket, a bearer secret per SPEC.md §3.4) — the control endpoint
     doesn't add a new trust requirement, it just gives a private
     channel to deliver one instead of routing it through some other
     medium.
   - **`RESET <token>`** — the poison pill Jason described, and the
     actual reason the box needs a screen (so it can show the *new*
     QR after a reset, not just accept the operation blind). Requires a
     `ResetToken` generated once alongside `Identity`
     (`ResetToken::load_or_generate`, same load-or-create shape as
     `Identity::load_or_generate`/`MuteList::load`) and checked by plain
     string comparison — fine here since QUIC already encrypts the
     channel and a 32-byte random token, not the comparison method, is
     the actual defense. A valid token deletes the box's entire data
     directory (identity included — a real factory reset, not just
     clearing content) and fires a `tokio::sync::Notify` the caller
     awaits alongside Ctrl+C, so `serve` exits cleanly and a process
     supervisor (systemd unit, restart loop, Docker restart policy — none
     built) restarts it fresh against a brand-new `did:iroh`, i.e. a new
     QR code, closing the loop back to the physical reset button.
   `Node::spawn_relay` wires this into node construction (returns the
   node plus its `ResetToken` plus the `Arc<Notify>`); `Node::send_control`
   is the client-side dial helper. `crates/atproto-iroh-cli`'s `serve` now
   uses `spawn_relay` (printing the token once at startup, `select!`-ing
   Ctrl+C against the reset signal), and `control-join`/`control-reset`
   are the client-side commands — see that crate's own README for exact
   shapes.

**Confirmed live, with one real gap found along the way.**
`crates/atproto-iroh-core/tests/control.rs` proves the actual protocol
logic end to end: a real second node dials the box's control endpoint
and joins a namespace with no capability of its own beyond reachability;
a wrong `RESET` token is rejected and touches nothing; a correct one
wipes the data directory, fires the signal, and the box's next
`ResetToken::load_or_generate` produces a different token, confirming a
real restart rather than reused state. Two things found live while
getting this to actually pass, not assumed clean from the code:
- **A real bug, not a sandbox limitation**: the first version of
  `ControlHandler::accept` returned as soon as it finished writing the
  response, which raced the `Router` tearing the connection down against
  the client still reading it (`read error: connection lost / closed by
  peer: 0`) — iroh's own `echo` example does the same
  `connection.closed().await` this was missing, added to both the server
  handler and (as an explicit `connection.close(...)` call) the client
  helper, which fixed it.
- **A sandbox limitation, not a code bug, matching the Android APK build
  gap's honesty pattern**: dialing a target by bare `PublicKey` alone
  (`target.into()` → `EndpointAddr` with no address info, resolved via
  discovery) fails in this sandbox even under `NetworkPreset::N0` — "No
  addressing information available / All address lookup services failed"
  — confirmed again live via the CLI itself (`serve` in the background,
  `control-reset` against its printed `did:iroh` from a second process:
  it hangs rather than connecting). This points to no real outbound
  reachability to n0.computer's relay/DNS infrastructure from this
  specific sandbox, not a flaw in the bare-DID design. The tests instead
  dial the box's actual `EndpointAddr` (`Node::endpoint_addr`, a new
  accessor wrapping `Endpoint::addr()`) via a lower-level
  `Node::send_control_to`/`control::send_control_message_to`, which
  exists specifically to make this verifiable here — the production API
  (`Node::send_control`, the CLI's `control-join`/`control-reset`) stays
  bare-`did:iroh`-only, matching the design intent, and remains something
  to re-verify on a machine with genuine internet access.

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

**Notifications, when this gets built: three tiers, not one mechanism
straining to cover all of it** (resolved in conversation, 2026-09-22).
Latency is fine for a *notification* — it's never fine for what the app
shows once actually opened, and conflating those two is what makes
"just poll in the background" feel like it needs to be fast. Split:
foreground gets a live connection (or tight polling) while the app's
open, full fidelity, no latency budget; app launch/wake does an
immediate pull, so opening the app after being closed never waits on a
background job's schedule to show current state; only the background
tier (option 3 above, WorkManager's ~15-minute-to-hours reality) is
allowed to be slow, because its only job is deciding whether to fire a
local notification ("something happened, go look"), not keeping full
state warm. Nobody's ever waiting on the slow tier to see their own
messages — only to be told to check. Deliberately no FCM/APNs in this
design even for that local-notification trigger: a push-registration
service would put Google or Apple back in the loop knowing who's
pinging whom, which is exactly the kind of intermediary this protocol's
whole premise (no server anyone has to trust) argues against — the
Raspberry-Pi-as-sold-hardware model (below) already gives orgs a durable
peer without needing one.

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

**A real, built, running-capable APK — done (2026-09-23), closing the
gap this section used to just flag.** Earlier sessions deliberately
skipped installing the Android SDK/NDK here over disk-space risk; this
one did it for real once the risk was re-checked and found manageable
(`cargo clean` alone reclaimed ~24GB of stale `target/` build
artifacts, leaving 29GB free — plenty of headroom for the SDK+NDK's few
GB). `sdkmanager --install "platform-tools" "platforms;android-34"
"build-tools;34.0.0" "ndk;27.0.12077973"` (Google's command-line tools
package, downloaded straight from `dl.google.com` — the sandbox's
proxy setup didn't block it) installed cleanly, `cargo tauri android
init` generated a real Gradle project, and `cargo tauri android build
--target aarch64` produced an actual signed-format APK: `gen/android/
app/build/outputs/apk/universal/release/app-universal-release-
unsigned.apk`, 42MB, containing a real `classes.dex`, `AndroidManifest.
xml`, and `lib/arm64-v8a/libatproto_iroh_tauri_lib.so` (the compiled
Rust core, cross-compiled for real, not stubbed) — confirmed by
actually unzipping it and inspecting those entries, not just trusting
a zero exit code. Unsigned, so it needs `apksigner` (or Android
Studio's own signing flow) before it'll install on a real device — that
step wasn't run here.

**One claim from the earlier version of this section turned out to be
wrong, found live**: "nothing about the Rust-side code is expected to
need changes for this" — false. `cargo tauri android init` requires the
app's logic reachable from a *library* crate (Tauri's Android/iOS build
links this crate as a `cdylib`/`staticlib` and calls a `run()` function
directly — there's no `main()` on mobile at all), and this crate only
ever had a `main.rs` with `fn main()`. Fixed by splitting: everything
moved into `lib.rs`'s `pub fn run()` (tagged `#[cfg_attr(mobile, tauri::
mobile_entry_point)]`), `main.rs` reduced to a thin shim calling
`atproto_iroh_tauri_lib::run()` for desktop, and Cargo.toml gained a
`[lib]` `crate-type = ["staticlib", "cdylib", "rlib"]` entry. Desktop
build re-verified clean after the split. Two smaller real fixes along
the way: `tauri.conf.json`'s placeholder `"0.0.0"` version isn't valid
for an Android package (needs ≥0.0.1) — bumped to `0.0.1`; and Gradle's
default per-CPU-core-parallel dependency resolution tripped real `429
Too Many Requests` responses from Maven Central against this sandbox's
shared egress IP, requiring several retries (each one caching more
before failing on a fresh artifact) before it got lucky enough to
finish — a global `~/.gradle/gradle.properties`
(`org.gradle.workers.max=1`, `systemProp.http[s].maxConnections=1`) now
serializes those downloads so a fresh `gen/android` regeneration
doesn't need the same luck. `gen/android/` itself stays git-ignored
(this section already said so before this session; still true — it's a
regenerated Gradle project, not source).

**Follow-up the same day: all four ABIs built, size fixed, signed and
sent.** `--debug` for all four ABIs at once exhausted the sandbox's
whole disk allowance (debug's `debuginfo=2` makes each target several
times larger than release — this is exactly why the Observability
section above now says release, not debug, is the default going
forward, not just in principle). Recovered by building release, one ABI
at a time, then a single combined `cargo tauri android build --target
aarch64 --target armv7 --target i686 --target x86_64` — note that
*each separate* `--target` invocation only bundles that one ABI into
the "universal" APK; getting one real four-ABI universal APK needs all
targets named in the same invocation. Also found: an unstripped release
`.so` ran 28-42MB per ABI, a 155MB universal APK and 30-45MB
split-per-ABI APKs — too big to be a usable download. Fixed in
`Cargo.toml`'s `[profile.release]`: `strip = true` alone only got
arm64's `.so` to 28MB; adding `lto = true`, `opt-level = "z"`,
`codegen-units = 1` got it to 18.8MB (APK 21.4MB) — verified by
rebuilding and inspecting the actual APK contents each time, not
assumed from the flags alone. `panic = "abort"` deliberately left out —
real size lever left on the table, but it changes unwinding semantics
and nothing in this stack (iroh/tauri/wry) was checked for reliance on
`catch_unwind`; not worth the risk once LTO alone hit the target.

**Signed and delivered.** A throwaway local debug keystore
(`~/.android/debug.keystore`, generated with `keytool`, standard debug
alias/password) signed the arm64 split APK; `apksigner verify`
confirmed real v2/v3 scheme signatures, not just a zero exit code. Sent
directly to Jason for device testing — this keystore is local-only,
never meant for real distribution, and would need a real release
keystore before this app goes anywhere beyond Jason's own devices.

To reproduce, from this repo or a machine/CI runner with normal disk
headroom:
```
sdkmanager --install "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;27.0.12077973"
export ANDROID_HOME=~/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973
cd crates/atproto-iroh-tauri
cargo tauri android init
cargo tauri android build --target aarch64 --target armv7 --target i686 --target x86_64 --split-per-abi
# sign each ABI's APK before installing on a real device:
apksigner sign --ks ~/.android/debug.keystore --ks-pass pass:android \
  --key-pass pass:android --ks-key-alias androiddebugkey <apk-path>
```
**Still not done**: hasn't actually been installed on a real device or
emulator (no emulator available in this sandbox to launch and click
through — Jason will do that with his mobile lab); no real release
keystore; `armv7`/`i686`/`x86_64` are built but only arm64 was
signed/sent (same `apksigner` command above covers the others). Worth
pressure-testing for real once there's a device to verify against, the
same caveat this section always carries for anything that can't be
interactively clicked through here.

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
- **Documents UX — built (2026-09-22).** The primitive
  (`save_document_revision`/`list_document_revisions`/`load_document`)
  already existed; what was missing was surfacing the conflict rather
  than silently picking a side. `dist`'s Shared doc section now: "Load"
  always refreshes full history alongside it (one click, not two); a
  client-side check (`main.js`, unit-tested standalone — five assertions
  on the exact boundary condition, since this is pure-logic code with no
  Rust-side test coverage) flags a real concurrent-edit signal — the two
  most recent revisions came from *different authors* within 5 minutes
  of each other, not just "there's more than one revision" (a single
  person saving twice in a row is normal, not a conflict) — with a
  visible `⚠ possible conflict` banner; every revision in history gets a
  "Use this" button that loads its text into the edit box for review.
  **Still no automatic merge** — resolution is a human reading both
  revisions and re-saving, same as it always would be; this closes the
  visibility gap (silently picking one), not the "diff and merge text"
  problem, which was never in scope.
- **Images — built (2026-09-22).** `crates/atproto-iroh-core/src/
  images.rs`: `ImageMeta` (`network.essmesh.chat.image`,
  `lexicons/network/essmesh/chat/image.json`), `upload_image`/
  `list_images`/`load_image_bytes`. Confirmed the prediction ("no new
  sync primitive needed") more precisely than expected: rather than
  `iroh-blobs`' own out-of-band blob-add API (which would mint a hash a
  peer then has to separately dial for), images go through the new
  `namespace::put_bytes`/`get_bytes` — `put_text` generalized past the
  UTF-8 requirement — so an image's bytes are an ordinary doc entry's
  content, synced by the exact same mechanism every other record here
  already relies on. Proven live in `crates/atproto-iroh-core/tests/
  images.rs`: upload on one node, join from a second node *after* the
  upload, both the metadata and the raw bytes sync with no separate
  fetch step, byte-for-byte identical on the other side. Metadata and
  bytes are two entries sharing one `rkey` (not one record with embedded
  bytes) specifically so listing a gallery never means downloading every
  image in it first. Wired into both clients: Tauri's `upload_image`/
  `list_images`/`load_image_bytes` commands (a file `<input>` + gallery
  in `dist`'s new "Images" section, bytes crossing the Tauri IPC
  boundary as a plain JSON byte array — fine for a reference client, not
  tuned for large files) and the CLI's `upload-image`/`images`/
  `download-image` subcommands, verified with a real byte-for-byte round
  trip through a local file. Not built: thumbnailing, image format
  validation, or any size limit.
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
  - **Polls — built (2026-09-22).** A dedicated `dist` section, but
    deliberately not a new mechanism: `create-poll`/the poll list call
    the exact same `create_proposal`/`create_signal`/`list_proposals`
    commands the full Governance section does, hardcoded to
    `GovernanceClass::General` and filtered/relabeled client-side
    (`main.js`) — Support/Object instead of Consent/Block, "passed"/
    "did not pass" instead of Ratified/Blocked. No Rust changes at all.
    Worded carefully, not just relabeled: the hint text says outright
    **"not a majority vote"** — it's the same objection-window mechanic
    as everything else here (passes by default unless enough people
    explicitly object; silence counts as support), and calling it
    "Polls" without saying so would have implied ordinary vote-counting
    behavior the mechanism doesn't have. CLI parity not built — the CLI
    has no governance commands at all yet (its own README's "Not built"
    list), so "polls" there would mean building `propose`/`signal` from
    scratch, not just relabeling; out of scope for what was actually a
    UI-labeling exercise.

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
