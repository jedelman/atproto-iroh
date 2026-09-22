# Lexicons

Five atproto-format lexicon schemas, following the design in
`../SPEC.md`. Status: drafted following the documented lexicon format
conventions, **not validated against live atproto tooling** (no lexicon
validator was run against them; treat as a careful best-effort draft, not
a certified schema) — same caveat as the rest of `SPEC.md`.

These five are the concrete ESS (cooperative/solidarity-economy) schemas
this protocol was originally designed against — see `SPEC.md`'s note on
extraction. The protocol itself (identity, edge-graph membership,
governance primitives) is general-purpose; these particular NSIDs
(`network.essmesh.*`) are not, and whether to keep them as a reference
implementation, rename them, or add a second, genuinely generic set
alongside them is an open decision (`SPEC.md` §6), not something this
extraction silently resolved.

```
network/essmesh/node/profile.json          one per member (key: self)
network/essmesh/node/event.json            one per event (key: tid)
network/essmesh/governance/proposal.json   one per decision (key: tid)
network/essmesh/governance/signal.json     one per member response (key: tid)
network/essmesh/governance/founding.json   one per founder (key: literal "self")
```

## `governance.founding` — closing the gap this section used to flag

Every version of this file before 2026-09-22 said the founding-record
gap out loud: `SPEC.md` §3.7.2 requires genesis state ("who's eligible,"
"what's the starting policy") to exist, but nothing here defined how it
gets recorded, and the reference client filled it with a heuristic
(anyone who'd self-asserted `governanceEligible: true`) explicitly
flagged as not trustworthy. `founding.json` is the real answer: see its
own `description` for the full mechanism (per-founder claims at a fixed
key, resolved by taking the earliest claim as t0 and only honoring
claims within a bounded window of it — so a member admitted long after
genesis can't backdate their way into the founding eligible set just by
posting their own claim). Implemented in
`crates/atproto-iroh-core/src/governance.rs` (`Founding`,
`resolve_founding`, pure and unit-tested) and `fold.rs`
(`found_namespace`/`read_founding`/`fold_namespace`, the I/O and
hex-decoding layer), proven live in
`crates/atproto-iroh-core/tests/founding.rs`. Both the Tauri shell and
the CLI now call `fold_namespace`/`found_namespace` for real instead of
the old placeholder.

## Why lexicons at all — the case, briefly

The design has accumulated four distinct record shapes across several
revisions of `SPEC.md` (profile, event, `Proposal`, `Signal`)
without ever formally defining one. That stops being optional once you
take §3.3's "no single canonical voice, individually attributed records"
model seriously: if every member's client is independently producing and
consuming these records with no central authority reconciling them,
**the schema is the only thing making independent implementations
interoperable at all.** Concretely, this is doing three jobs at once:

1. **Interop across independently-built clients.** Nothing in this design
   assumes one reference implementation — a member could run any client
   that speaks the same schema. Lexicons are the only contract that makes
   that possible without a central authority.
2. **Validation.** A reader syncing a namespace needs to reject a
   malformed or malicious record (a `signalType` that isn't one of the
   five real values, a `Proposal` missing its `class`) rather than
   silently accept anything shaped vaguely like JSON.
3. **Deliberate, versioned evolution.** This design has changed shape
   substantially several times in one conversation. Lexicons give a
   principled way to version a record shape (`network.essmesh.node.profile`
   v1 vs. a future v2) without breaking every existing reader the moment
   it changes again — which, given this document's own track record, it
   probably will.

## What's deliberately *not* a lexicon: `Ratification`

`SPEC.md` §3.9 defines a `Ratification` as a derived fact, not a signed
artifact of its own. **Revised per §3.7.2/§3.9:** a `Proposal` ratifies by
default once its `deadline` passes, unless enough `block`
`network.essmesh.governance.signal` records reference it to meet that
class's current threshold — the inverse of the original rule (N
`consent` records required), kept in `SPEC.md` for the record because the
original stalls exactly where real cooperative governance stalls: on
people not showing up, not on people actively disagreeing. Either way,
this is a fact any reader computes by counting signed records against a
`deadline`, never a signed artifact of its own. There is no
`network.essmesh.governance.ratification` lexicon and there shouldn't be
one: giving it its own record type would imply someone produces and signs
a "ratification event," which is exactly the aggregation step this design
spent several revisions removing (see `SPEC.md`'s FROST →
N-of-M-by-counting → objection-window history). A reader computes
ratification; nobody issues it.

## Schema choices worth flagging, not just declaring

- **`profile.category`'s `knownValues` are copied verbatim from
  `street-smarts`'s `tools/sbci/sbci/ess_source.py`'s `CATEGORY_WEIGHTS` keys**, on purpose
  — this is the one place the protocol and the `sbci` pipeline share
  vocabulary directly. If one changes, the other drifts silently unless
  someone keeps them in sync by hand; there's no automated check for that
  yet (worth adding once either side is actually built).
- **`profile.governanceEligible` is explicitly marked non-authoritative**
  in its own description. Actual standing lives in the governance
  history (§3.7.2), not in a self-asserted boolean a compromised or
  outdated client could just set. A field that exists for convenient
  display but isn't the source of truth needs to say so in the schema
  itself, not rely on every implementer remembering a design doc.
- **`event.locationNote` is free text, not structured geo**, on purpose —
  a lat/lon field would silently imply "this is safe to put in the public
  disclosure tier" (§3.5) in a way free text doesn't. Precision here is a
  disclosure decision each event's author makes, not something a
  structured field should default them into.
- **`signal.subject` pins a `CID`, not just an AT-URI**, via
  `com.atproto.repo.strongRef` (atproto's own standard cross-record
  reference type, reused rather than reinvented) — so a `Signal` is
  unambiguous about exactly which version of a `Proposal` it responded
  to, even if the author edited it after some members had already
  signaled.
