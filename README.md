# atproto-iroh

A capability-scoped, audit-by-construction protocol: atproto's data model
(lexicon-typed, signed, per-author repos) over iroh's transport (direct
peer-to-peer, no relay, no server to trust) instead of atproto's usual
PDS → relay → firehose pipeline.

**Status (2026-09-23): real, working, two clients.** This stopped being a
design sketch a while ago and this README stopped saying so even longer —
fixed now. `crates/atproto-iroh-core` has ten real modules (identity,
namespace, records, governance, fold, mute, paths, messaging, images,
tagging, control) exercised by a dozen-plus live two-node QUIC
integration tests, not just unit tests of pure logic. Governance
(objection-window `Proposal`/`Signal` ratification, a real `Founding`
genesis mechanism) is real end to end. Persistence and at-rest encryption
are real. Four "batteries-included" apps are built (Messaging, Images,
cross-lexicon Tagging, Documents UX with conflict visibility), plus
Polls as a thin relabeling of Governance.

Two clients: a Tauri reference GUI (28 commands, plain HTML/JS/CSS, no
bundler/framework, now including camera QR scanning) with real Android
APKs built for all four ABIs, and `atproto-iroh-cli`, a Linux CLI with a
`serve` mode for relay/always-on use (including a poison-pill remote
reset over its own raw control endpoint) and, as of today, full
governance parity (`propose`/`signal`/`proposals`/`governance-state`).

None of this is hearsay — every claim above has a live test, a built
binary, or a real run backing it, and where something *isn't* verified
(an actual camera scan on a real device, an APK actually installed and
clicked through), the relevant README says so explicitly rather than
implying it works. **`CLAUDE.md` is the living, continuously-updated
account of exactly what's real, what's still a gap, and the reasoning
behind every non-obvious decision** — read it before assuming anything
about current state; this file stays high-level on purpose. Full design
record in [`SPEC.md`](SPEC.md); four draft lexicon schemas in
[`lexicons/`](lexicons/README.md).

## What this actually is

No public index, no relay, no firehose. A peer who hasn't been granted a
read edge into a namespace can't see it exists. Identity is per-person
(`did:iroh`, one ordinary Ed25519 key each — no group keys, no threshold
signing, no DKG). Membership in a shared namespace is nothing but the
read/write edge graph over those individual identities — not a separate
roster that could drift out of sync with reality. Lightweight,
counted-not-aggregated governance (`Proposal` / `Signal` / a derived
`Ratification`) handles the one place a quorum still earns its keep:
admitting a new co-signer with standing to grant further access.

Read `SPEC.md` end to end before writing code against this — it's dense,
it changed shape substantially several times during design (that history
is recorded in this repo's git log, not hidden), and the reasoning behind
each choice matters more than the choice itself for anyone extending it.

## Origin

Designed in [`jedelman/street-smarts`](https://github.com/jedelman/street-smarts)
as `ESS_MESH_SPEC.md`, motivated by a finding from that repo's `tools/sbci`
pipeline: every capital-adjacent geospatial data source it touched (OSM,
Census, Open Data BCN) turned out to be a real bulk-query API, while every
cooperative/solidarity-economy data source it touched (XES, Pam a Pam) was
a browse-only directory with no API at all. The naive fix — crawl the
directories, publish a public queryable index — would have created real
opsec exposure for the cooperatives it listed. This protocol is the answer
to "how would you actually close that gap without recreating the problem
it was trying to avoid."

The design turned out to be general-purpose well before it was extracted
here — see `SPEC.md` §5. Nothing past §3.1 references cooperatives,
mapping, or governance specifically; the same substrate would carry
private messaging, offline-tolerant community tools, or any small-group
coordination that needs to work without a server anyone has to trust.
Only the four lexicons in `lexicons/` (still under the `network.essmesh.*`
namespace) are ESS-specific — whether to rename, keep as a reference
implementation, or add a second generic set is an open decision, not
something this extraction resolved on its own (`SPEC.md` §6).

## Getting started

```
cargo build --workspace          # core + CLI + Tauri (needs GTK/WebKit dev libs on Linux)
cargo test -p atproto-iroh-core  # unit tests + live two-node integration tests
cd crates/atproto-iroh-cli && cargo run -- --help   # the Linux/agent client
cd crates/atproto-iroh-tauri/src-tauri && cargo tauri dev   # the reference GUI
```

Each crate's own README has the real detail: `crates/atproto-iroh-core`'s
module docs (`src/lib.rs`) map every module to the `SPEC.md` section it
implements; `crates/atproto-iroh-cli/README.md` and
`crates/atproto-iroh-tauri/README.md` document exact commands, what's
verified live vs. not, and real bugs found and fixed along the way.
**`CLAUDE.md` is the single most useful file to read first** if you're
picking this up fresh — it's kept current after every session, with
reasoning for every non-obvious decision kept visible rather than
overwritten, and it's honest about what's still a gap (currently: no
real Android release keystore, no camera-scan click-through on an actual
device, a handful of CLI/UI parity gaps — see its own "Status" section
for the exact, current list rather than trusting this summary to stay
fresh).

Historical note, kept for context rather than as current guidance:
`SPEC.md` §6 originally ranked open questions by "blocks anything
getting built," several flagged as "not validated against real API
surface." That validation happened — `examples/iroh_docs_probe.rs` and
every live test in `crates/atproto-iroh-core/tests/` are the record of
it — so this section no longer describes the actual next step; it's left
here as a reminder that unresolved design questions in `SPEC.md` §6
still get resolved the same way, by validating against the real crate,
not by assumption.
