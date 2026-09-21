# atproto-iroh

A capability-scoped, audit-by-construction protocol: atproto's data model
(lexicon-typed, signed, per-author repos) over iroh's transport (direct
peer-to-peer, no relay, no server to trust) instead of atproto's usual
PDS → relay → firehose pipeline.

**Status: design sketch, no code yet.** Full design in [`SPEC.md`](SPEC.md).
Four draft lexicon schemas in [`lexicons/`](lexicons/README.md).

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

`SPEC.md` §6 ranks open questions by "blocks anything getting built."
Several of them are "not validated against real API surface" — that's
true throughout this document, and the honest first implementation task
is probably a small, throwaway experiment against `iroh-docs`'s actual
crate (does it expose an enumerable capability-holder list? does document
sync deliver full history or only future writes on first grant?) before
committing to any of the architecture as load-bearing. See `CLAUDE.md`
for a fuller onboarding note if you're picking this up in a fresh
session.
