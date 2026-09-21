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

Design sketch, no code. `SPEC.md` §6 is a ranked open-questions list —
several items are explicitly "not validated against real API surface,"
which is true of the whole document, not just those items. **The first
real task is probably a small, throwaway experiment against the actual
`iroh-docs` crate**, not writing the protocol's architecture in Rust from
the spec as given. Concretely, before committing to anything as
load-bearing:

- Does `iroh-docs` expose an enumerable list of a namespace's current
  capability holders? (§3.7.2's governance-eligible-roster idea depends
  on this.)
- Does granting a peer a read capability into an existing namespace sync
  them the full document history, or only writes going forward? (§3.4,
  §6.10 — the backfill/new-member-access story depends on this being
  "full history.")
- Does `iroh-docs`' replication model actually resolve convergence
  per-key rather than forcing a merge decision across different authors
  writing to the same document? (§3.4's justification for using one
  document per namespace depends on this.)

If any of these turn out false, don't patch around it quietly — go back
to `SPEC.md` and revise the relevant section the same way previous
revisions are recorded, with the old reasoning kept visible, not deleted.
That's the established practice in this document; keep it.

## Language and structure

Rust (`.gitignore` already assumes it; iroh's canonical SDK is Rust).
`street-smarts` (this design's origin repo, also Jason's) uses a Cargo
workspace under `crates/` with a `[workspace.package]` block for shared
metadata — this repo now has a minimal one-crate version of that same
shape (`crates/atproto-iroh-core`, currently a stub). Don't invent the
full crate breakdown before the `iroh-docs` experiments above answer
enough of §6 to know where the real module boundaries are — a premature
multi-crate split encodes uncertainty as if it were architecture.

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
