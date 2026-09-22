# atproto-iroh — a capability-scoped, audit-by-construction protocol

Status: **design sketch, no code yet**. This document is carried over
verbatim (with cross-repo references fixed up) from where it was
designed: `jedelman/street-smarts`, `ESS_MESH_SPEC.md`, written per that
repo's own norm of writing the spec before the code. See that repo's own
`SPEC.md` / `PRIMITIVES_SPEC.md` for the convention this follows.

The design's motivating case and worked example throughout this document
is ESS (cooperative/solidarity-economy) nodes — that history is real and
worth keeping, but as of the extraction into this repo, the protocol
itself is understood to be general-purpose: nothing past §3.1 is actually
ESS-specific. See §5's note on this and `README.md` for the current
framing.

## 1. The problem this solves

`jedelman/street-smarts`'s `tools/sbci/sbci/data_fabric.py` documents a
finding from building that repo's Spatial Bio-Capital Index pipeline:
every capital-adjacent data source tested (OSM, Census, Open Data BCN) is
a real bulk-query API; every commons/ESS source tested (XES, Pam a Pam)
is a browse-only directory with no API. The naive fix — crawl the
directories, publish a public queryable index — trades one problem for a
worse one. A cooperative like Eleanor's (an anarchofeminist book/info
shop in Norfolk, referenced here as the motivating case, not as a subject
with confirmed technical requirements — nobody has asked them anything
yet) has real reasons to be careful about who can query its location and
activity pattern. A public, crawlable, geolocated database of "here are
the radical spaces" is a liability to the people it lists, not a gift to
them. See `street-smarts`'s `tools/sbci/README.md`'s "Finding" section
and that repo's own anti-measurement stance in its top-level README for
the fuller argument — both worth reading before assuming this design's
"searchable but traced" framing is obviously the right call for every use
case this protocol might serve.

So the actual design target isn't "make ESS data as legible as parcel
data." It's: **searchable, but every search is traced by the node being
searched, and disclosure is the node's own choice, tier by tier.**

## 2. Design goals, in priority order

1. **Zero ambient legibility.** No public index, no relay, no firehose. A
   node that hasn't granted a capability to a peer is invisible to that
   peer, full stop.
2. **Audit-by-construction, not audit-as-a-log.** Because there's no
   intermediary, a query has to physically reach the node being searched.
   The node inherently sees who asked, when, for what. This is a property
   of the topology, not a feature someone could forget to enable or a log
   file someone could fail to check.
3. **Tiered disclosure, node-controlled.** A cooperative should be able to
   publish *something* discoverable (a public storefront: name,
   neighborhood, category) without that implying the full record (address,
   hours, contact, activity data) is public too. Real-world businesses
   already do this — a listed phone number doesn't mean the owner's home
   address is public. The protocol should make that distinction native,
   not bolted on.
4. **Reuse a mature data model.** Don't reinvent "what fields does a
   cooperative-economy record have" or "how do you sign and version a
   repo of records" — atproto's lexicon schema system and Merkle-search-tree
   repo format already solve both, and solve them well. Reuse the parts
   that are genuinely transport-agnostic. As of §3.2–§3.3's current
   revision, "reuse" means reusing atproto *as atproto actually is* —
   individually-keyed repos, no group-owned signing entity — rather than
   an earlier draft's group-DID extension to it.
5. **Zero-labor publishing**, from the earlier conversation about node
   needs — whatever a collective has to do to join, it has to be closer to
   "fill out one form" than "run a server."

Goals 1–3 are in real tension with 4–5: atproto's tooling (indigo, the
official SDKs, AppView infra) is built assuming the PDS→relay→firehose
pipeline, which is exactly the public-by-default topology goal 1 rejects.
That tension is the central design problem this document has to resolve,
not paper over.

## 3. Architecture

### 3.1 Split atproto's layers, keep two, replace one

atproto is usually described as one thing, but it's three separable
layers:

| Layer | What it is | Transport-dependent? |
|---|---|---|
| Identity | DIDs — a public key with a resolvable document | Partially — `did:plc` resolves via a public directory; other methods don't |
| Data | Signed, content-addressed Merkle search tree (MST) per repo, versioned by commit | No — it's just a data structure |
| Schema | Lexicons — typed record definitions (JSON Schema-like) | No — pure typing convention |
| Sync/distribution | PDS hosts a repo publicly; relay crawls it; AppViews index the firehose | **Yes — this is the public-by-default part** |

The proposal: **keep the data layer (MST repos) and schema layer
(lexicons), replace the sync layer with iroh, replace the identity
layer's resolution mechanism.**

### 3.2 Identity: individual, not collective — a correction to earlier
drafts

`did:plc` is still out — it resolves through a public, semi-centralized
directory, which leaks exactly the existence-and-activity signal this
protocol is trying to avoid. `did:web` is still out for the same
collectives that can't run a public server. Both conclusions from earlier
drafts survive. What doesn't survive: earlier drafts derived one
`did:iroh` *per cooperative*, then spent most of §3.7–§3.8 solving how a
group could safely hold that single identity's key together (threshold
signatures, distributed key generation, rekey-as-succession). That whole
problem dissolves once identity is individual: **every member has their
own ordinary `did:iroh:<nodeid>`, one Ed25519 keypair, exactly the key
they'd need for any other iroh use — no group key, no DKG, no threshold
scheme, because there is no group secret for any of that machinery to
protect.** "Eleanor's" is not a DID at all; see §3.3.

Resolution still works the way earlier drafts intended: `did:iroh:<nodeid>`
resolves by dialing the node and asking for its own signed DID document,
which only works if you already have a route to it — identity and
reachability stay collapsed into the same trust boundary, unchanged by
this revision.

### 3.3 Data: a repo is optional per member; a node is a namespace, not
a repo

**Revision — the repo is no longer assumed mandatory, and namespace
entries do not mirror it.** The original framing below ("each member has
an ordinary repo") silently assumed two things that turned out to need
separating: that everyone maintains a persistent MST repo, and that a
namespace's iroh-docs entries are somehow derived from or kept in sync
with that repo. Neither survives scrutiny:

- **Namespace entries are independently, deliberately published — never
  an automatic mirror of repo content.** The alternative (an entry in a
  namespace being a copy of, or kept in sync with, something in your MST
  repo) was checked against this document's own priority-1 goals and
  loses on all three: it requires new propagation machinery (either you
  run it, or a peer runs it "on your behalf," reopening exactly the trust
  question §3.7.5 exists to avoid); it breaks tiered disclosure (§2 goal
  3) unless you build a selective-mirroring filter on top, which is
  disclosure control *bolted on*, the thing goal 3 explicitly rejects;
  and it multiplies the scrubbing surface of anything you want to retract
  — instead of "the namespaces I chose to publish this into" (small,
  explicit, enumerable), it becomes "the repo, plus every namespace that
  happened to mirror it, including ones I may have forgotten I'm in."
  Deliberate per-namespace publishing is also the reading already
  consistent with the rest of this section — "no single canonical
  'Eleanor's says X' statement" (below) presupposes there's no canonical
  upstream copy other copies are tracking, which an automatic mirror
  would quietly contradict.
- **A persistent MST repo is therefore optional, not required, for
  namespace-only participation.** The only reason to keep it at all is
  cross-transport compatibility with the wider atproto ecosystem (see the
  bridge discussion below) — and that turns out to be available more
  cheaply than "maintain a repo" implies. What actually carries
  compatibility is lexicon- and MST-shaped *records*, not the standing
  existence of a repo object. Someone who only ever publishes into
  namespaces, with no ambition to ever be reachable from Bluesky-adjacent
  tooling, doesn't need a repo at all — consistent with goal 5's
  zero-labor publishing, which a mandatory second persistent data
  structure per member actively worked against.

**Cross-transport compatibility with the wider atproto ecosystem is
real but bounded, and only reachable through an explicit, opt-in
bridge — never a core protocol feature.** Split into what's actually two
different claims:

- *Data-shape compatibility is already free.* Goal 4's reuse of MST +
  lexicons means anything published here, if extracted, is already a
  valid atproto record — same signing, same tree structure. No further
  design work buys this; it's a consequence of decisions already made.
- *Live-network compatibility — resolvable by existing atproto tooling,
  crawlable by a relay, indexed by an AppView, found by a Bluesky client
  — is not reachable from inside this design at all, and isn't a gap to
  close so much as the direct expression of the tension §2 already
  names.* Two independent walls: (1) `did:iroh` isn't a DID method any
  existing resolver knows how to fetch — not a missing feature, a
  missing entry in every other implementation's method table, which
  isn't this document's to add unilaterally; (2) real interop requires
  answering `com.atproto.sync.getRepo`/`subscribeRepos` on a public
  surface a relay can crawl — which *is* the firehose goal 1 defines
  itself against. You cannot expose that surface without becoming
  exactly the public-by-default thing this design opted out of.
- *The only feasible shape is a deliberate bridge*: a **separate**
  `did:plc` or `did:web` identity (never the `did:iroh` one wearing a
  costume — the methods can't be dual-homed) running an ordinary,
  minimal PDS-shaped service, which republishes specifically whatever
  records someone chooses to push through it — the same
  deliberate-per-record posture as namespace publishing, applied at the
  ecosystem boundary instead of the repo/namespace one, for the identical
  reason. Anything sent through the bridge becomes exactly as public,
  crawlable, and permanent as any ordinary atproto/Bluesky post the
  moment it crosses — a real, one-way privacy trade accepted per-record,
  not a property of the protocol. This was always the intended shape of
  a future private↔public sync path, not a discovery made under
  pressure; see §6 for what's still unbuilt.

**Implication worth stating plainly: this architecture, run entirely
within its own boundary with no bridge at all, is a local, private,
offline-capable Bluesky-shaped substrate** — same record model
(lexicon-typed, MST-signed), same repo format if someone chooses to keep
one, but direct capability-scoped peer sync instead of
PDS→relay→firehose, and tiered disclosure native rather than "public by
default, DM as the exception." What it does *not* yet have, and what
distinguishes "the data model Bluesky uses" from "a thing that behaves
like Bluesky," is a social-interaction lexicon set (post/like/repost/
follow/thread — this document has only ever drafted the ESS-specific
`node.profile`/`node.event`/governance schemas) and, more structurally, a
**feed/timeline construction layer**. Bluesky's AppView computes that
server-side, over the whole public firehose; nothing here has a
server-side anything, so timeline construction would have to happen
locally, on each reader's own node, over whatever repos and namespaces
they currently hold capabilities into. That's a real, distinct piece of
design work, not a footnote — tracked as a new item in §6.

Each member runs (or has hosted on their behalf — see §3.7) one ordinary,
single-key-signed MST repo — the standard atproto shape, nothing new,
nothing collectively owned; a member who never intends to bridge to the
wider ecosystem may not need one at all, per the revision above. Records
use lexicon collections,
`network.essmesh.node.profile` and `network.essmesh.node.event`, drafted
in full at `lexicons/network/essmesh/node/` — real schema, not a
placeholder, though not yet validated against live atproto tooling (see
`lexicons/README.md`) and still needing input from an actual cooperative
before any field list should be treated as final. This is where the SCED
weighting scheme from `sbci`'s brief (coop_housing_clt=1.0,
worker_coop=0.8, ...) lives as a first-class, self-asserted field —
`profile.category`'s allowed values are copied verbatim from
`street-smarts`'s `tools/sbci/sbci/ess_source.py`'s `CATEGORY_WEIGHTS`
keys, the one place protocol and pipeline shared vocabulary directly
when both lived in the same repo — now cross-repo, and worth keeping in
sync deliberately rather than assuming it stays that way for free (§6).

**"Eleanor's" is a namespace, not an entity with its own signing key.** A
topic — identified by its own ID, not by anyone's DID — aggregates
whichever members' relevant records have been included (§3.4 covers the
propagation mechanism). A reader who's synced that topic receives the
union of included members' individually-signed records. There is no
single canonical
"Eleanor's says X" statement unless the group chooses to converge on one
by their own convention (e.g., "the current profile record is whichever
one carries co-signatures from N current members" — itself an ordinary
governance action per §3.7, not something the protocol requires). This is
closer to how real federated systems already work — individually
attributed statements, trust aggregation left to the reader or to
explicit convention — than earlier drafts' single-voice model was.

### 3.4 Transport: two edge types are the entire membership structure;
direct peering does both aggregation and updates

No separate "membership" object exists anywhere in this design. Two
directed edge types over the same set of individual `did:iroh` identity
nodes:

- **`read(namespace → peer)`**: peer is authorized to receive the
  namespace's records.
- **`write(peer → namespace)`**: peer's own records get included when
  others receive the namespace.

"Is X a member of Eleanor's" has no answer beyond "does X currently hold
a `write` edge into Eleanor's namespace" — directly, mechanically
checkable, not a separate roster that could drift out of sync with the
actual capability grants (§6 covers whether the underlying mechanism
exposes this enumerably).

**Correction to the previous revision, which read "peering for
aggregation and updates" as gossip and overcorrected.** It isn't gossip —
gossip specifically means epidemic, multi-hop relay (a peer forwards what
it received to *its own* peers), and that really would have needed the
strict relay-scoping the previous revision spent a full paragraph
defending. What was actually meant is simpler and doesn't have that
problem: **direct peer-to-peer connections between exactly the edge
holders themselves**, no relay, no intermediary. Two peers who each hold
an edge into the same namespace connect directly and sync — the same
operation whether it's catching up on history or picking up something
published five minutes ago. One mechanism, not the two-mechanism
gossip/backfill split the previous revision invented to solve a
multi-hop leak risk that direct peering never had in the first place.

That also walks back last revision's objection to `iroh-docs`' own
namespace-document model, which was aimed at the wrong target. The
concern was "a converged document implies one canonical merged answer
across every author," which would contradict §3.3's no-single-voice
stance. But `iroh-docs`' actual replication model resolves convergence
*per key*, not across authors — if each member writes under their own
keyspace (e.g. `<member-did>/profile`, `<member-did>/event/<tid>`),
convergence only ever answers "what's the current version of *this
member's own* record," which is exactly wanted (no ambiguity about which
version of someone's own claim is current) and never forces any
resolution *between* different members' claims, which stays exactly as
side-by-side and individually attributed as §3.3 already described. **One
`iroh-docs` document per namespace, synced directly between edge holders,
is the mechanism** — simpler than the previous revision's gossip/backfill
split, and the objection to it doesn't survive close reading of what
`iroh-docs` actually converges on.

**Validated against the real crate (`iroh-docs` 0.101.0), not assumed.**
`crates/atproto-iroh-core/examples/iroh_docs_probe.rs` is the throwaway
experiment CLAUDE.md called for: two real in-process nodes, real QUIC
sync, no mocking. Three findings, one of them a correction to this
section rather than a confirmation of it.

*Confirmed, and sharper than stated above.* Convergence doesn't merely
happen to resolve "per key" as a policy — it's structural.
`RecordIdentifier` (`iroh_docs::sync::RecordIdentifier`) is the compound
key `(namespace, author, key)`, not `(namespace, key)`. Two different
authors writing the literal same key string never contend for one slot at
all; they're different identifiers from the store's point of view, and a
query returns both, independently, forever. There is no merge decision to
have an opinion about. Live: author A and author B both wrote
`"shared-key"` on one node; after sync, the other node read back both
entries under that key, distinctly attributed, byte-identical to what
each author wrote. The "resolves per key" framing above is correct in
effect but undersells it — it reads like a convergence *rule* the design
is relying on, when it's actually a consequence of what the identifier
*is*. Only true same-author-same-key writes ever compete, resolved by
`Record`'s own `Ord` (timestamp, then hash) — which is the only case
where "current version of this member's own record" as stated above
actually needs a tiebreak.

*Confirmed.* §6.10's question, whether granting access syncs full history
or only future writes: full history. Three entries were written and
committed on node0 *before* node1 ever received a ticket for the
namespace. After node1 imported the ticket and synced, it had all three,
unprompted — set reconciliation between two peers converges on the whole
replica state each holds, not a subscription to a tail. New-member access
never needed a separate backfill mechanism because sync was never an
event-log tail to begin with.

**Not confirmed, and this is the one worth stopping on: "write(peer →
namespace)" is not the edge this paragraph's "directly, mechanically
checkable" claim needs it to be.** §3.4 frames membership as "does X
currently hold a write edge," implying a per-peer, individually
grantable and revocable credential — precisely what §3.2's identity
correction and §3.7.2's single-point-of-capture worry both assume is
possible to build. Reading `iroh_docs::sync::Capability` (`keys.rs`,
`sync.rs`) says otherwise: `Capability::Write` wraps exactly one
`NamespaceSecret` — **the same 32 bytes for the entire namespace, held
identically by every writer.** There is no per-peer write credential
anywhere in the crate. What *is* per-peer is the `Author` keypair used to
*sign entries* — which is who gets credited for a given record — but
authoring is gated by holding the shared namespace secret, not by
anything tied to a specific `Author`. Concretely: anyone who has ever
been handed the `Write` ticket can create entries under *any* `Author`
key they generate, including a fresh, previously-unseen one — the crate
has no notion of "this Author is bound to this credential-holder" to
enforce. Two consequences that reach back into earlier sections:

- **§3.7.1's "capabilities expire by default, short-lived, self-expiring"
  has no home in the crate.** A `NamespaceSecret` doesn't expire, doesn't
  carry a validity window, and isn't scoped to a peer. Self-expiry would
  have to be an application-level convention layered on top (e.g.,
  rotating to a fresh namespace on a schedule and re-inviting current
  holders) — real, buildable, but not a crate primitive, and rotation is
  an all-holders event, not a single-peer one.
- **Per-peer write revocation isn't a crate operation.** Because every
  writer holds literally the same secret, there is no "revoke Bob's write
  edge" — only "rotate the namespace secret and redistribute it to
  everyone *except* Bob," which is indistinguishable, from the crate's
  point of view, from kicking out the whole membership and re-admitting
  most of it. §3.7's governance layer (counted `consent` signals,
  §3.7.2–3.7.4) can *decide* to revoke someone; it just can't express
  that decision as a capability-layer operation the way §3.4's "does X
  hold a write edge" phrasing implies. Enforcement has to happen where
  the governance record lives and gets read — readers who honor a
  `governance` collection's revocation record can choose to stop
  accepting that Author's entries as legitimate, but nothing stops the
  revoked peer from continuing to write with the (unrotated) secret; it
  only stops good-faith readers from counting those writes. That's a
  materially weaker guarantee than "revoked" suggests, and worth being
  explicit about rather than letting the edge-graph language imply
  cryptographic enforcement iroh-docs doesn't provide.

Read capability (`Capability::Read(NamespaceId)`) is even thinner: the
`NamespaceId` is the document's own public identifier, not a secret at
all. "Granting read access" is really "telling someone the ID and how to
reach a peer holding it" (a `DocTicket`) — there's nothing to revoke,
because there was never a credential, only knowledge plus reachability.
Once synced, a peer keeps whatever it already received regardless of
anything happening on the write side afterward.

None of this breaks the design — §3.7.4 already put the actual
accountability mechanism in the right place (signed `consent`/`block`
records in an application-level `governance` collection, read and
enforced by peers themselves), which doesn't depend on the transport
layer providing per-peer revocation. But §3.4 and §3.7.1 currently read
as if the crate hands over peer-scoped, expiring, revocable edges for
free, and it doesn't — the accountability §3.7 describes has to be the
*whole* mechanism, not a backstop on top of a capability layer that was
never doing that job. Worth restating §3.4's membership test as "does X
currently write under an Author key the governance record still honors"
rather than "does X hold a write edge," since the latter names a
credential the crate doesn't actually scope to X at all.

### 3.5 Tiered disclosure: two namespaces, not one

To satisfy goal 3 without contradicting goal 1, a node that wants any
public presence runs (or delegates — see below) **two namespaces**:

- A `public` namespace, openly discoverable, containing only what the
  node explicitly wants a stranger to find (name, category, neighborhood
  — no address, no hours, no activity data). This is the "storefront."
- A `private` namespace, capability-gated, containing the full record.
  Access is granted per-peer, per-ticket, revocable.

A search against the mesh is then two different operations with two
different trust models: browsing the public namespaces (which, note,
*could* be aggregated by a willing index like XES without contradicting
anyone's opsec, since it's opt-in-public by design) vs. querying a
specific node's private namespace, which only works if you already hold
a ticket — and which the node sees happen.

### 3.6 Discovery rides on relationships that already exist

Correction to an earlier draft of this document, which framed §3.6 as a
"cold-start problem." It isn't one, for this network specifically:
mutual-aid and cooperative networks aren't cold graphs a stranger
searches into — they're warm, already-existing networks of trust (a
worker at Eleanor's knows someone at Coop57 knows someone doing
tenant organizing knows someone at the next info shop). Ticket exchange
over Signal, in person, at an assembly, is not a workaround for a missing
discovery layer — it *is* the discovery layer, the same one these
networks already run on before any protocol existed. The public
namespace (§3.5) still matters for the genuinely cold case (someone with
zero existing relationship to the movement), but it's a courtesy on-ramp,
not the thing the design has to optimize for. Don't build a search engine
for this network. Build the ticket-passing as easy as the relationships
that already carry it.

### 3.7 Governance: who can grant an edge — the only place a quorum still
earns its keep

A correction to earlier drafts, which spent most of this section on FROST
threshold signatures over a group root key (§3.2's earlier version).
There is no root key anymore, so most of that machinery — DKG, threshold
signing ceremonies, rekey-as-succession — is gone, not patched. What
survives, in simplified form, and what's genuinely new:

**3.7.1 The core inversion survives unchanged: capabilities expire by
default; they are not revoked by exception.** Every granted edge — read
or write — is short-lived and self-expiring (hours to a few days, not
months); staying included requires active re-affirmation, not the absence
of a revocation. Silence is loss of access, not retention of it. This
principle didn't depend on FROST to begin with and fits even better
without it: renewal is now just "sign an ordinary message with your own
key," not a multi-round ceremony.

**3.7.2 A real fork in the design, named rather than silently resolved.**
Ordinary data sharing — granting someone a read or write edge into a
specific namespace — needs no group decision at all under a pure
edge-tracking model: whoever already holds a capability can extend one,
the same way inviting someone into a Signal group usually just takes one
existing member choosing to. That's the fully flat reading of "just track
edges." But one action is different in kind from ordinary sharing:
**admitting a new co-signer with standing to grant further edges on the
namespace's behalf** — because that's the action that determines who
gets to keep making this decision, recursively. Two consistent minimal
designs, not resolved here in favor of one without saying so:

- **(a) Fully flat.** Any current write-edge holder can grant any
  capability, including grant-capable status, to anyone. Maximally
  simple, truest to "all we needed to do was track edges." Real cost: a
  single member — compromised, coerced, or just careless — can quietly
  reshape who has standing, by inviting allies who then hold equal
  authority. That's the single-point-of-capture problem this whole
  document exists to avoid, relocated from "who holds the root key" to
  "who can invite new co-signers," not eliminated.
- **(b) Recommended: flat for data, N-of-M for standing.** Ordinary read
  and write access to data stays unilateral, grantable by any current
  holder. Admitting or removing a *governance-eligible* co-signer (someone
  whose grant counts toward future N-of-M decisions) requires N
  individually-signed `consent` `Signal`s (§3.9) from current
  governance-eligible members, referencing the same `Proposal`. Not
  FROST — literally counting valid, distinct individual signatures, which
  any reader can verify without threshold cryptography of any kind.

(b) is the recommendation, on the same reasoning §3.7's earlier drafts
used for the root key: a keyholder class distinct from the membership,
even an accidentally-emergent one under (a), quietly recreates the
hierarchy this document exists to avoid. But it's a real design choice
with a real cost (an additional conceptual layer — "governance-eligible"
— that pure edge-tracking didn't originally need), not a fact the graph
model hands over for free, and it should be stated as a choice if this
gets built, not assumed.

**Resolved, and the resolution changes what "N-of-M" meant, not just
which of (a)/(b) got picked.** (b) is confirmed — flat for ordinary data,
gated for governance-eligible standing — but (b) as originally specified
requires collecting N affirmative `consent` Signals to ratify anything,
which turns out to have the same disease at any N above the smallest
handful: it needs people to actively show up and sign, and real
cooperative governance's actual failure mode is exactly that they don't
— not out of malice, just because consensus-shaped processes stall on
apathy as reliably as they stall on an actual holdout, and a determined
single objector produces the identical stall on purpose. "Unanimous" for
changing the policy (below) is the sharpest case of this and the least
defensible: it hands one member, in practice, a permanent veto over ever
correcting course.

**The fix inverts what's being counted: ratification requires an
*absence* of sufficient objection within a window, not a *presence* of
sufficient consent.** A `Proposal` opens with a deadline; if it closes
without accumulating enough `block` Signals (§3.9) to meet its class's
current threshold, it ratifies automatically — silence defaults to
*yes*, the same directional move §3.7.1 already made for capability
expiry (silence there defaults to *no* — loss of access — because access
and decision-making want opposite defaults: the cost of a stale grant
lingering is higher than the cost of a stale grant lapsing, while the
cost of a decision never happening is higher than the cost of one nobody
actively fought). `consent` Signals keep their social meaning — visible
affirmation, part of the record — but carry no mechanical counting power
either way; only `block` does. This needed no new primitive: §3.9 already
drafted `consent | stand_aside | block | abstain | exit` and already
said, in its "what a block Signal actually does and doesn't do"
paragraph, that block enforcement is client convention, never
cryptography. What changes is only the default the mechanism resolves to
when nobody acts.

**And the thresholds themselves are not this document's to set.** Not a
fixed table of suggested N-of-M values — a namespace's own current
(window length, block threshold) pair *per class* is itself governance
state, set at namespace creation by whoever founds it and amended
afterward only by a ratified `changePolicy`-class `Proposal`, through the
identical objection-window mechanism, using whatever the *current*
changePolicy threshold happens to be (self-amending, recursive — same
"constitution" property §3.7.3's old "unanimous" row was reaching for,
except the bar itself is now something a group that finds it too high or
too low can actually move, rather than being permanently stuck with
whatever this document guessed). This follows §3.9's own stated
principle more faithfully than the original table did: *"a cooperative
that already knows how to run a hard meeting doesn't need software
telling it how to deliberate."* A protocol-suggested N was already a
mild violation of that; a protocol-*fixed* unanimity requirement was a
bigger one.

One wrinkle worth resolving explicitly rather than leaving implicit: if
the policy changes mid-window (a `changePolicy` Proposal ratifies while
an unrelated Proposal is still open), the open Proposal keeps the
threshold and window that were in effect when *it* was created, for its
entire lifecycle. Nobody's rules change underneath a decision already in
motion.

**3.7.3 Tiered friction, now group-set rather than protocol-suggested.**
The mechanism (objection-window ratification) is fixed by the protocol;
the numbers are not. What the protocol still fixes is which four action
classes exist and that ordinary sharing stays unilateral — the same
shape §3.7.3 always had, just with "suggested N-of-M" replaced by
"group's own current (window, block threshold)":

| Action | Threshold | Rationale |
|---|---|---|
| Grant ordinary read/write access to a specific peer | none — unilateral by any current holder, no Proposal involved at all | Zero-friction sharing was the entire point; gating this defeats §3.6's "ticket-passing as easy as the relationships that carry it." |
| Admit a new governance-eligible co-signer (`admitCoSigner`) | group's current policy for this class; a sensible starting default is a short window and a small block threshold (e.g. 1) | The recursive, standing-conferring action from §3.7.2 — deserves a real chance for someone to object, not maximal friction to make happen. |
| Remove a governance-eligible co-signer's standing (`removeCoSigner`) | group's current policy for this class; a sensible starting default is a longer window and a threshold scaled to group size rather than a fixed small number | Consequential and adversarial, but gating it behind "most of the group must actively agree" was exactly the unanimity-adjacent failure mode this revision exists to remove — a real, considered objection should be able to stop it; disengagement shouldn't be able to. |
| Change the policy itself (`changePolicy`) | group's current policy for *this* class — bootstrapped at namespace creation, thereafter self-amending | The constitution-amendment case — see §3.8. Deliberately still the highest bar of the three by convention (longest window, largest relative threshold), but no longer a protocol-mandated unanimity that a single member can hold hostage forever. |

**3.7.4 Transparency: governance events are records, not administrative
side effects.** Unchanged from earlier drafts in substance, mechanism
updated per §3.4: every grant, renewal, and `consent`/`block`/`exit`
`Signal` is a signed record in a `governance` collection within the
namespace's document, synced directly to every current holder the same
way any other record is — transparency as a property of the topology
(reaching everyone with a live connection as it happens, no separate
broadcast step), not a log someone has to remember to check. A member who
was offline catches up on reconnect via the same direct sync against
another edge holder, same as any other missed history — one mechanism,
not a live/backfill split.

**3.7.5 Portability was never about a group key, and is cleaner now.**
Each member's own repo (§3.3) is theirs regardless of what happens to any
shared namespace — there was never a group key for a hostile operator or
majority to hold hostage in the first place. Losing access to a shared
namespace is a data-*inclusion* problem (your records stop being synced
into that aggregation), never a data-loss event: nothing about your own
repo depends on anyone else's cooperation.

**3.7.6 Honest costs, rewritten, and sharpened again by §3.7.2's later
revision.** Objection-by-counting reveals exactly who signed, always —
unlike FROST's aggregate output, which didn't distinguish which *t* of
*n* participated. Under the original consent-counted design this meant
outsiders or other members could see precisely who did and didn't
consent; under the revision, it's sharper still, because the Signal that
actually does something is `block` — the person who stopped an admission
or a removal from going through is now individually, permanently
identified as having done so, not just absent from a list of consenters.
That's a real, narrow opsec cost, arguably a larger one than the original
framing had: a visible blocker is a more specific target than a visible
non-consenter, if patterns get correlated over time or if the proposal
was itself adversarial. Worth the group deciding that's acceptable, not
assuming it away — and worth weighing directly against the alternative
this revision replaced, which had the opposite failure mode (nothing
ever ratifying) rather than this one (whoever objects is exposed for
having done so). Second cost: "governance-eligible" as a status distinct
from "has a write edge"
is a real conceptual addition this document is choosing to make (§3.7.2),
not something free.

**Validated, per §3.4's probe.** `iroh-docs` exposes no enumerable list
of current capability/topic holders at all — `DocsApi::list()` returns
only the calling node's *own* capabilities, and `Doc::get_sync_peers()`
turned out to be a persistent-store reconnect hint (empty on the
in-memory nodes tested, even mid-sync), not a live-membership view. The
governance-eligible-roster idea was never going to get this from the
transport layer, and §3.7.4 already didn't ask it to: the roster is
whatever the `governance` collection's signed `consent`/`block` records
fold up to, computed and verified by each reader locally, same as any
other application state synced as ordinary entries. That's slightly
different from how §3.7.2–3.7.4 read on a first pass — less "the
namespace tracks who's eligible" and more "eligibility is a value every
reader independently derives by replaying records it already received" —
but it's the same mechanism, just named more precisely now that it's
confirmed nothing lower in the stack does this job instead. See §3.4's
validation note for the sharper and more consequential finding from the
same probe: write capability itself is a single shared secret, not a
per-peer edge, which matters more to §3.7's revocation story than the
roster question did.

**3.7.7 Revocation, resolved: no secret rotation for v1, two lighter
layers instead.** §3.4's shared-secret finding meant §3.7.1's
"capabilities expire by default, individually revocable" has nothing to
attach to at the transport layer — the only mechanical fix (rotating the
namespace secret on every removal) was costed against what it actually
defends against and found not worth building yet; full reasoning and the
two-layer replacement (governance-ratified removal, binding by
convention; a purely local per-reader mute, needing no protocol surface
at all) is in §6 item 12, not repeated here.

### 3.8 Fission and constituent power, without a group key left to fight
over

Earlier drafts located this document's Hardt-and-Negri moment in
unanimous *rekey* — the act of re-founding a threshold-held group
identity. That artifact (a single group `did:iroh`) no longer exists, and
the insight is sharper without it, not weaker: **there was never a
constituted-power object — a group DID — to seize, defect from, or need
re-founding.** Continuing together is an ongoing, individually-signed
cooperative act that never crystallizes into a single sovereign identity
in the first place. Nothing to capture because nothing was ever
centralized to begin with; the multitude staying plural isn't an escape
hatch built into the protocol anymore, it's just what the protocol *is*.

The closest remaining analog to a constituent act is §3.7.3's "change the
N-of-M policy itself" and "admit/remove a governance-eligible co-signer"
rows — still the place the group is deciding who it is, still
meaningfully higher-friction than ordinary sharing, but carrying no
cryptographic ceremony. And fission is, if anything, easier than earlier
drafts described: a subset that can't reach the N-of-M for a governance
change doesn't need §3.7.5's portability guarantee to escape a shared
identity, because per §3.3 there was never a shared identity to escape —
they just spin up their own namespace, populate it from whichever
individually-signed records (their own, already portable by construction)
they want to carry forward, and grant edges to whoever they choose. No
succession record, no redirect, no "official name" to contest, because
there is no name attached to the group at all — only to individuals and
to namespaces, and a new namespace needs nobody's permission to exist.

The `exit` `Signal` (§3.9) still earns its keep here, in simplified form:
a governance-eligible member's own self-signed withdrawal, shrinking the
pool of members eligible to submit a counted `block` `Signal` on future
Proposals (§3.7.2's revision means what shrinks isn't a consent quorum
anymore, but the same idea — someone who's left no longer has standing
to object) — the same voluntary-departure-vs-hostile-holdout distinction
as before, just operating on a counted roster instead of a
threshold-share set.

### 3.9 Vote primitives: minimal cryptography, everything else is convention

The mistake to actively avoid: encoding a specific decision-making
procedure — majority vote, consensus-minus-one-block, Robert's Rules —
into the protocol. A cooperative that already knows how to run a hard
meeting doesn't need software telling it how to deliberate; it needs
software that can't be argued with about whether quorum was actually met.
`Proposal` and `Signal` are drafted in full as lexicons at
`lexicons/network/essmesh/governance/` — there is deliberately no
`Ratification` lexicon; see `lexicons/README.md` for why giving it a
record type of its own would quietly reintroduce the aggregation step
this design spent several revisions removing. Split accordingly, with a
hard boundary between the two layers:

**Social layer — expressive, human, cryptographically inert.** A `Signal`
record: any member can publish one, at any time, attached to a
`Proposal`. Type is one of `consent | stand_aside | block | abstain |
exit`, plus free text — that vocabulary because it's what a
consensus-trained group already uses, and collapsing it to `yes/no` would
be a regression, not a simplification. `exit` is the odd one out and the
only type with a defined mechanical effect rather than being purely
informational: a governance-eligible member's own signed withdrawal of
their claim to future participation, which — per §3.8's fission
discussion — shrinks the pool of members whose `block` `Signal`s count
toward a Proposal's threshold (§3.7.2's revision, §3.7.3) rather than
freezing anything. Every `Signal` type is an
ordinary signed record from the member's own individual key — never a
threshold operation, no special status, no group key involved anywhere in
this design. This is where discussion, "I'll go along but want my concern
noted," and everything else genuinely human-shaped lives, exactly as
messy as a real meeting, because the protocol doesn't touch it.

**Ratification layer — mechanical, minimal, the only thing with actual
teeth. Revised per §3.7.2/§3.7.3: ratification is the absence of
sufficient objection, not the presence of sufficient consent.** A
`Proposal` carries a deadline (derived from its class's current window,
locked in at the moment the Proposal was created — see §3.7.2's note on
mid-window policy changes). It becomes a `Ratification` the moment that
deadline passes with fewer than the class's current block-threshold worth
of outstanding, un-withdrawn `block` `Signal`s from governance-eligible
members — not aggregated, not threshold-signed, just counted by any
reader capable of checking signatures and a timestamp. `consent`
`Signal`s remain real — visible, signed, part of the record, the thing a
member does to say "I actively support this" — but carry no counting
power of their own; a Proposal with zero consent Signals and zero block
Signals still ratifies on schedule. This is the inverse of the
original design (below), kept for the record: that version made
`consent` the only Signal type with power and treated declining to sign
as "every other outcome at once," which is exactly the shape that stalls
on apathy as readily as on genuine opposition. Flipping which Signal type
carries the power, and which outcome silence defaults to, was the whole
fix — nothing else about the layer changed.

**What a `block` Signal actually does, and doesn't do.** Under the
original consent-counted design, a block at a low threshold couldn't
stop willing consenters by itself, and only the unanimous
policy-change tier gave a block real structural force. Under the
objection-window model that asymmetry is gone by construction: a `block`
is the *only* Signal type that does anything mechanically, at every
tier, because ratification is defined as its absence. A group that wants
routine admissions hard to block sets a higher threshold for that class
(§3.7.3); a group that wants any single member able to raise a real
objection sets it to one. Either way, the honest limit from the original
design still holds and is worth restating exactly as before: **a block
is a social fact enforced by client convention, not by cryptography.** An
honest reference client refuses to build, relay, or act on a
`Ratification` whose `Proposal` closed with block Signals at or above
threshold still outstanding — the same way a block works in a real
meeting: nothing physically stops a dishonest client from acting anyway,
the group's shared practice and its choice of software is what makes it
matter. That distinction — cryptography for privacy and authentication;
everything about how a decision is actually made is convention, enforced
by the humans and the software they choose to run — is the design
principle this whole section follows, not just this one paragraph.

**Superseded — kept for the record, not deleted.** The original
Ratification rule: *"N distinct, valid individual `consent` `Signal`s
over the same `Proposal`, counted against whatever N its class requires
... It either meets that count or it doesn't exist. No cryptographic
representation of 'no' is needed: a `consent` `Signal` already is the
only 'yes' that has power, and declining to sign is every other outcome
at once."* This is precisely backwards for a group where declining to
sign is the *normal* outcome regardless of opinion — which, per §3.7.2,
is what real cooperative governance actually looks like.

## 4. Relationship to `street-smarts`'s `tools/sbci/`

Now a cross-repo relationship, not an in-repo one. If this existed,
`street-smarts`'s `sbci/ess_source.py` would gain a mesh-backed loader
next to the GeoJSON-fixture loader it has now — pulling `sced` input from
whichever namespace(s) granted a read edge to the pipeline's own
`did:iroh`, instead of a hand-seeded four-entry fixture with unverified
coordinates. That also closes the loop on `street-smarts`'s
`data_fabric.py` finding: a real, willing, consent-based source on the
commons side, rather than either "no data" or "scrape it without asking."
This is the one place a change here could break something in that other
repo silently (§6) — there's no CI linking them.

## 5. What this document is not

Not a commitment that Eleanor's or any real cooperative wants this, needs
this, or has been asked. Not a claim that iroh's current APIs
(`iroh-docs`, formerly `iroh-sync`) already support everything described
here — they weren't checked against this design in detail and some of
§3.3–3.5 may need real API research before it's buildable. Not a
replacement for actually talking to XES, Pam a Pam, or a
Norfolk cooperative about what they'd want, if anything — this is an
engineer's-eye-view sketch of what's *technically* possible, which is a
different question from what's wanted. And, worth being honest about
given how much of this document has changed shape in conversation: not a
document that arrived at its current architecture on the first pass —
§3.2–§3.9 moved from a group-owned `did:iroh` secured by FROST threshold
signatures to individually-keyed members and a pure edge-graph, because
the group secret the first version was built to protect turned out not
to need existing at all. That revision is recorded in this document's own
git history, not hidden — a design sketch this order of unfinished should
show its work, not just its current conclusion.

**Update — extracted.** §3.2 onward describes a general-purpose
decentralized identity + capability-graph + lightweight-governance
protocol — almost nothing past §3.1 is actually ESS-specific; only the
`node.profile`/`node.event` lexicons and the `CATEGORY_WEIGHTS` tie-in to
`sbci` are. The same substrate (offline-tolerant, no server to trust,
searchable only by people actually granted access) would carry private
messaging, tenant organizing, or any small-group coordination tool just
as well. This document originally recorded a decision to stay in
`street-smarts` "until there's an actual implementation worth giving its
own home" — that threshold was judged met, and this repo
(`jedelman/atproto-iroh`) is that home. `street-smarts` was a narrower-
purpose project (an Alexander-pattern-language provocation engine) this
didn't belong in permanently, same as it didn't belong permanently in
`tools/sbci/` before that. What extraction did *not* do: rename the ESS
lexicons' NSIDs (still `network.essmesh.*`) or otherwise generalize the
concrete schemas — that's a real open decision (§6), not something to
silently decide while moving files.

**Confirmed in code, not just in this paragraph.** The general-purpose
claim above was a prediction about the architecture; `atproto-iroh-core`
now has a second, non-lexicon-typed write path (`namespace::put_text`)
sitting right next to `put_record`, using the identical sync and
capability machinery. Nothing about a namespace requires the four
ESS lexicons, or any lexicon at all — a namespace is a capability-scoped,
multi-writer, synced key/value space; typed records are one way to use
it, freeform shared text is another, and both are live. Realized in
conversation as "a Google doc without Google," which is a more exact
description of what this already was than "a cooperative's shared data"
ever was — the ESS case just happened to be the first one built against
it. `submit_text` (a variant that mints its own unique key per call) is
the same primitive specialized for the uncoordinated-submitters case —
a public inbox, functionally — which composes with a `Write` ticket
turned into a QR and posted publicly: safe by construction, not by
policy, since `RecordIdentifier`'s `(namespace, author, key)` shape
(§3.4) means a stranger can only ever write under an author they
generated, never forge or overwrite anyone else's entry. The genuine
residual risk is volume, not forgery — the same resource-attack case §6
item 12 already named, just with the probability turned up by making the
ticket public on purpose instead of handing it to people individually.

## 6. Open questions, ranked by "blocks anything getting built"

1. **Partially answered, first-person, and it's what drove item 4's
   resolution.** Jason has built cooperatives himself, mostly
   unsuccessfully, and says this design would have solved real blocking
   problems he hit — specifically the consensus/unanimity failure mode
   §3.7.2's revision now targets directly. That's a real answer, not a
   hypothetical one, but it's one founder's retrospective account, not
   the "walk this design past an active cooperative and watch what breaks
   against real, current group dynamics" check this item originally
   asked for — still worth doing before treating the rest as validated,
   still ranked first on purpose, just no longer answered with silence.
2. **Resolved, by a real probe against the crate
   (`crates/atproto-iroh-core/examples/iroh_docs_probe.rs`), not by
   memory.** No — `iroh-docs` exposes no enumerable list of who currently
   holds a capability grant into a namespace. `DocsApi::list()` is local-
   only (this node's own capabilities); `Doc::get_sync_peers()` is a
   persistent-store reconnect hint, not a membership view, and was empty
   even mid-sync on the in-memory nodes tested. Not a blocker: §3.7.4
   already puts the actual roster computation at the application layer
   (folding the `governance` collection's signed records), which needed
   no crate support to begin with — see §3.4's and §3.7.6's validation
   notes for the detail, and for the sharper finding the same probe
   turned up about write capability not being per-peer at all.
3. **Resolved by correcting a misreading, not by new design work**: an
   earlier revision worried about scoping multi-hop gossip relay to
   exactly the edge set, which would have been a real, load-bearing
   constraint. It doesn't apply — §3.4 now describes direct peer-to-peer
   sync between edge holders, not epidemic/relayed gossip, so there's no
   relay boundary to leak beyond in the first place. Kept as a record that
   this was worried about and the worry doesn't survive the correction.
4. **Resolved, with input from exactly the kind of cooperative-builder
   experience item 1 asks whether anyone's talked to.** (b) confirmed —
   flat for ordinary data, gated for governance-eligible standing — but
   the gating mechanism itself changed: not N collected `consent`
   Signals, but ratification-by-default unless enough `block` Signals
   land within a window, with the window and block threshold themselves
   group-set governance state rather than protocol-fixed numbers (full
   reasoning in §3.7.2/§3.7.3's revision, mechanics in §3.9). The reason,
   plainly: real consensus-shaped requirements don't fail by people
   actively voting no, they fail by nobody showing up to reach the
   count, and a single determined holdout produces the same stall as a
   protocol feature, not a bug in any particular group. §3.9's Signal
   vocabulary and §3.7.4's transparency mechanism needed no changes —
   only which Signal type carries mechanical power, and which outcome
   silence defaults to, flipped.
5. Objection-by-counting (§3.7.6) reveals exactly who blocked, every
   time — a real, narrow opsec cost relative to what FROST's aggregate
   signature would have hidden, sharper now than in the original
   consent-counted design because the exposed party is specifically
   whoever stopped something, not whoever declined to affirm it. Is that
   acceptable, given who this is for? Not evaluated here.
6. What does an ordinary member's experience of signing a `Signal` in
   practice actually feel like — is "sign an ordinary message with your
   existing key" as frictionless as this document assumes, or does it
   still need real UX work to not become its own version of the
   threshold-ceremony problem it was designed to avoid? Sharper now than
   when this was first asked: since §3.7.2's revision, `consent` carries
   no mechanical power, only `block` does — does a group still bother
   signing `consent` when it's purely social, or does the record quietly
   go quiet (nobody signs anything, a Proposal just... times out and
   ratifies) in a way that's fine mechanically but loses the "everyone
   was actually paying attention" signal transparency was partly for? And
   does making `block` the one Signal that matters make members more
   reluctant to use it, precisely because §3.7.6 just established it's
   also the one that exposes them?
7. **Deprecated, by decision, not resolution.** Hosting-on-behalf-of
   threat model — what someone running shared infrastructure for a
   namespace could still see or do. Dropped rather than answered: nothing
   in the Phase 1 build (§6 item 15) has anyone hosting on anyone else's
   behalf, so there's no live case to reason about yet. Worth reopening
   the moment that changes, not before.
8. **Superseded, not open**: FROST tooling maturity, UCAN-over-a-threshold-DID,
   rekey-as-succession, and DID-identifier-continuity-across-a-rekey were
   all real open questions against the group-DID/FROST architecture in
   earlier drafts. None of them apply to the current edge-graph
   architecture (§3.2–§3.8) — there is no group DID to rekey or need
   continuity for. Kept here as a record that the architecture changed
   underneath them, not because they're still live.
9. **Deprecated, by decision, not resolution.** New-namespace migration —
   whether there's a clean "this topic supersedes that one" signal for a
   group deliberately relocating. Its main live motivation was rotation
   (§6 item 12): a rotated namespace needing a way to point existing
   holders at its replacement. Item 12 resolved against rotation for v1,
   so the sharpest reason this mattered went with it. Still a real gap if
   it comes up on its own — a group relocating for reasons that have
   nothing to do with revocation is a separate, plausible case — but not
   one anything currently being built needs answered.
10. **Resolved.** New-member historical access: confirmed live against
    the real crate, not just inferred from what document sync should mean
    by definition. Three entries were committed before a second node ever
    held a ticket for the namespace; after that node imported the ticket
    and synced, it had all three, unprompted — full backfill on grant,
    not a tail subscription (`crates/atproto-iroh-core/examples/iroh_docs_probe.rs`,
    also written up under §3.4). Still genuinely open: whether a new
    member's first sync needs one specific reachable peer or can pull
    from any current holder — the probe only ever tested two nodes
    syncing directly, never a three-plus-peer topology, so "any current
    holder" is untested, not confirmed.
11. The four lexicons at `lexicons/` (§3.3, §3.9) are a careful draft
    following documented atproto lexicon conventions, not run through an
    actual lexicon validator or checked against current atproto tooling —
    same caveat as everything else in this document that hasn't touched a
    real API this session. Before anyone builds against them: validate
    the schemas themselves, and get an actual cooperative's eyes on
    `profile`'s and `event`'s field lists specifically, since those are
    the two records asking someone to describe themselves, not just the
    two managing protocol mechanics.
12. **Resolved: no rotation for v1, decided on threat model, not
    convenience.** `iroh-docs` write capability is one shared
    `NamespaceSecret` per namespace, identical across every writer, not a
    per-peer credential (§3.4's validation note). Closing that gap with
    rotation was costed out directly: cheap to mint, expensive to land —
    it touches every remaining holder (not just the removed one), has no
    continuity mechanism (a new `NamespaceId` with nothing carrying over
    automatically, no "this supersedes that" signal since §6 item 9 is
    still open), and doesn't even retroactively unwrite anything the
    removed person already synced elsewhere. Weighed against what it
    actually buys: rotation only defends against a *resource* attack —
    someone who keeps writing to flood or spite the shared document after
    being voted out, imposing real sync/storage cost on everyone who
    remains. Against the ordinary case — a member the group no longer
    trusts or wants to platform, who isn't trying to break the
    infrastructure — it buys nothing that's not already covered by
    convention-based enforcement, and judged (Jason, from direct
    cooperative-building experience) rare enough not to justify that
    coordination cost up front.

    **What v1 ships with instead, and these are two different layers, not
    one:** (1) the governance-level removal already built (§3.7.2's
    ratified `removeCoSigner` Proposal) — a group decision, binding on
    honest clients by the same convention-not-cryptography principle
    §3.9 already established for `block`; and (2) a purely local,
    unsigned, unsynced per-reader **mute** — one person deciding they
    personally don't want to see someone's entries, needing nobody's
    agreement, carrying no protocol surface at all (no lexicon, no
    record, no `governance` collection entry) because a personal
    preference doesn't need cryptographic backing any more than deciding
    not to read a particular news outlet does. Mute is strictly lighter
    than removal — it changes what one reader's own client shows them,
    nothing about anyone else's view or the removed party's standing —
    and either can exist without the other: a member can be muted by one
    person without the group ever voting on anything, or removed by the
    group while individuals who'd already muted them notice nothing new.

    Not deleting the option: if a real resource-attack incident ever
    happens, rotation is still exactly the mechanism described above,
    unbuilt but fully specified — this is a decision against building it
    now, made on a stated threat-model judgment, not a claim that the
    gap doesn't exist.
13. **Resolved, by working through the repo/namespace relationship
    directly (§3.3's revision).** Namespace entries are independent,
    deliberately-published records, never an automatic mirror of a
    member's MST repo — checked against goals 1–3 and against the
    scrubbing-surface reasoning in §3.4's validation note, and it loses on
    every axis. Consequence: a persistent MST repo is optional for
    namespace-only participation, not required. What replaces "keep a
    repo for interop" is narrower and cheaper: keep records
    lexicon/MST-*shaped* when you write them, which costs nothing extra
    given goal 4 already committed to that format.
14. **New.** The opt-in bridge to the wider atproto ecosystem (§3.3) is
    named and reasoned about but not designed: what exactly the minimal
    PDS-shaped service needs to implement (`com.atproto.sync.getRepo` at
    minimum; whether `subscribeRepos` is avoidable or whether any relay
    integration requires it); how a bridge identity's `did:plc`/`did:web`
    keypair relates to someone's `did:iroh` one procedurally (generated
    once at bridge-setup time and treated as fully separate key material,
    presumably — not decided); and whether "republish this one record
    through the bridge" is a per-record manual action or something a
    person can pre-authorize for a whole namespace going forward (the
    latter reopens the automatic-mirroring problem item 13 just resolved
    against, one boundary further out — worth being as careful here as
    §3.3 was about the repo/namespace boundary, not less).
15. **New — explicitly filed as Phase 2, not part of the initial build.**
    Run with no bridge at all, this architecture is a local, private,
    offline-capable Bluesky-shaped substrate (§3.3) — but two pieces are
    missing before "shaped like" becomes "behaves like": (a) a
    social-interaction lexicon set (post/like/repost/follow/thread-shaped
    records; nothing here has drafted these, only the ESS-specific
    `node.profile`/`node.event`/governance schemas at
    `lexicons/network/essmesh/`), and (b) a feed/timeline construction
    layer computed locally by each reader over whatever repos and
    namespaces they currently hold capabilities into, since there is no
    server-side AppView to do that computation the way Bluesky's does.
    (b) is the larger piece — ranking, deduplication, and thread
    assembly done once centrally over a public firehose is a different
    problem from the same computation done independently by every reader
    over a different, smaller, capability-scoped view of the world, and
    nothing in this document has touched that problem yet. Deliberately
    out of scope until the ESS case (Phase 1: identity, namespaces,
    capability grants, governance, the four drafted lexicons) has a real
    implementation — building a feed algorithm before the underlying
    sync/capability substrate has been used for anything real would be
    designing the harder problem first, on no evidence from the easier
    one.
