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

What's still genuinely unbuilt: governance isn't wired to `namespace.rs`
yet (the ratification logic and the sync layer are both real and tested,
just not plugged into each other), and there's no client at all — see
"Client" below. If a future `iroh-docs` upgrade or new finding turns any
of the resolved SPEC.md forks out to be wrong, don't patch around it
quietly — revise the relevant section the same way every previous
revision is recorded, old reasoning kept visible, not deleted. That's the
established practice in this document; keep it.

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
ends up as). Three commands proving the wiring end to end
(`spawn_node`/`node_did`/`create_namespace_with_profile`), a plain
HTML/JS/CSS frontend with no bundler and no framework. `cargo build -p
atproto-iroh-tauri` verified clean, not just written — see its own
`README.md` for exactly what that check did and didn't cover, and for
everything real still missing (sharing/joining, governance UI,
persistence, mute UI) before this is a usable app rather than a proof
the layers connect.

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
