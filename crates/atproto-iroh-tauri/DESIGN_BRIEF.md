# Design brief — Tauri reference client rebuild

Produced via `/shape` (2026-09-23), confirmed with Jason. Design context
this brief draws on lives in `../../.impeccable.md` and is summarized in
root `CLAUDE.md`'s "Design context for the Tauri reference client's UX"
section — read those first if anything here seems to assume context.

Handoff target: `/impeccable craft` (or equivalent implementation work)
against this brief. The current plain HTML/JS/CSS frontend
(`dist/index.html`/`main.js`/`styles.css`) is being replaced with a real
React + Vite frontend over the same Rust/Tauri backend. **Correction
(2026-09-23, same day, after Jason's follow-up guidance below)**: this
is *mostly* a frontend-only rebuild, but not entirely — profile
stickers need one small, real addition to `atproto-iroh-core`
(`NodeProfile` gaining an avatar/sticker field). Said plainly here
rather than letting the earlier "no core-crate changes" claim stand
uncorrected — see §4's Profile Customization note and §9.

## 1. Feature Summary

A full rebuild of the Tauri reference client from a plain, undesigned
debug UI into a real product: a private, serverless coordination app for
small groups — messaging, shared decisions, documents, and photos —
fronted by a unified activity feed across every Table you're in (not a
Discord-style per-server switcher, which is a named anti-pattern here),
with real profile personality (stickers, not just a name) and an
admin-authored pinned welcome per Table.

## 2. Primary User Action

In the first minute: see who's actually here, and register that this is
private — not composing a message, not reading a decision. Trust and
belonging come before any task. **Revised with the Feed-as-home decision
(§4)**: this doesn't mean landing on a bare member list anymore — it
means the Feed's own top-of-screen has to carry "these are your people,
this is just us" (a Tables/people strip, pinned-welcome excerpts) before
the activity stream itself, so the feed still answers "whose table is
this?" even though it's the very first screen, not one tap in.

## 3. Design Direction

Warm and communal — a shared kitchen table, not a corporate dashboard.
Dark default, but *warm*-dark: warm-tinted neutrals (a hint of
brown/amber, never blue-black), a genuinely warm accent hue, explicitly
rejecting the reflexive cyan/purple "AI dark mode" palette. Handmade and
willing to be a little imperfect rather than sanded to generic-SaaS
smoothness.

**Anti-goals** (all four confirmed): must not read as crypto/web3 wallet
software; must not go cold/corporate despite being dark; protocol
mechanics (`did:iroh`, tickets, capabilities, governance thresholds)
must never leak into everyday flows; must clear the AI-slop test
(`impeccable`'s own mandate).

## 4. Layout Strategy

- **"namespace" → "Table" everywhere in user-facing copy/IA** — confirmed
  naming decision, not open anymore. `namespace_id` etc. stay as the
  literal Rust/Tauri-command parameter names (backend is unchanged);
  this is a frontend vocabulary decision only.
- **Feed is the home screen — a deliberate reaction to Discord's failure
  mode.** Jason's framing, direct from the guidance session: Discord's
  per-server switcher becomes overwhelming the moment someone's in more
  than a couple of servers; the fix (per Bluesky's model) is one unified
  timeline across everything you're in, with Table and tag as *filters*
  on that timeline, not separate destinations you have to remember to
  check. Concretely: opening the app lands on a merged feed of Messages,
  Photos, Decisions, and Shared-doc updates across every Table, newest
  first, filterable by Table and by tag. A visually distinct feed-item
  treatment per content type matters here (a message doesn't look like a
  photo doesn't look like a Decision) so the merge reads as "one
  coherent stream," not "four features awkwardly interleaved."
- **A Tables list still exists**, one level in from the feed (e.g. a
  rail or a picker), for when someone wants a specific Table's full
  context rather than the cross-cutting view — not removed, just no
  longer the front door.
- **Inside a Table**: Members-first landing (the original primary
  action, still true when someone deliberately opens a specific Table),
  led by that Table's **pinned welcome message** if one exists (see
  below), then Messages, Decisions, Photos, Shared docs — tabs or a
  persistent side rail, not a dense multi-section scroll like today's
  `index.html`.
- **Pinned messages, per Table — resolved and built (2026-09-23):
  anyone can pin, one pin per author, shown newest-pin-first.** Not
  admin-only after all — Jason's exact spec, matching `tagging.rs`'s
  existing open/append-only posture rather than inventing a new
  permission concept. Backend is real and live-tested, not just
  designed: `tagging::PIN_LABEL` (`"system:pin"`, reserved under the
  namespaced-label convention below so a user's own unrelated "pin"
  tag can never collide with this behavior) and `tagging::pins()`
  (list every current pin, one per author, most recent wins, newest
  first — mirrors `governance::latest_signal_per_author`'s exact
  dedup shape). `tests/tagging.rs`'s
  `tags_can_tag_tags_and_pins_resolve_one_per_author_by_recency` proves
  the resolution rule against real synced history, not just unit logic.
  Shown at the top of a Table whenever anyone opens it, and the most
  recent one excerpted in the Feed the first time a person sees a new
  Table's activity — the concrete answer to "whose table is this, and
  what's it for."
- **Namespaced tag labels — resolved and built (2026-09-23): "tags are
  monads."** `tagging::Tag.label` stays a plain free-text `String` (no
  schema change), but this crate now reserves the `system:` prefix for
  labels it gives built-in meaning to (`system:pin` is the first),
  leaving every other label — bare words, or a user's own `topic:`/
  `mood:`/whatever convention — fully open for people to extend the
  ontology themselves, exactly as asked. The deeper point behind "tags
  are monads" is also now confirmed true, not just architecturally
  plausible: a `Tag`'s `subject` can point at *another* `Tag`
  (`record_ref` never restricted the collection a subject names), and
  `tests/tagging.rs` proves this round-trips through real two-node sync
  — tagging a tag, reacting to a tag, or building further layers on top
  of this one primitive all fall out for free.
- **Profile customization: built-in stickers now, custom upload later.**
  Every member gets a sticker/avatar, not just a name — confirmed
  explicitly: "even bots need cute stickers." A curated, on-brand
  sticker set ships with the app (art/asset work, not a backend
  concern) and picking one is part of first-run onboarding, not buried
  in settings. Custom image upload (reusing the already-proven
  `images.rs` infrastructure) is flagged for a later pass, not this one
  — see §9 for the moderation/federation question that makes it a
  separate decision, not just more work.
- **Protocol/advanced stuff** (raw inspector, relay/control, mute
  internals, `did:iroh` display) behind a clearly-secondary
  "Advanced"/settings surface — present for the curious, never in the
  primary path.
- **Joining**: QR scan is the front door for most people — the obvious
  first action, not one option among many on a dense form.

## 5. Key States

- First launch, no identity yet (today: a bare "Spawn node" button —
  needs a real welcome moment) — **now includes picking a sticker** as
  part of that first-run flow, not an optional settings detour.
- No Tables yet — empty state should teach ("scan a code to join, or
  start your own table"), not just say "nothing here." The Feed itself,
  with zero Tables, is a distinct empty state from "a Table with no
  recent activity."
- A brand-new Table with a pinned welcome vs. one without — the
  no-welcome case shouldn't feel broken or half-finished; a real
  encouragement-to-write-one prompt for whoever founded it.
- A Table with 2 people vs. a few dozen (realistic range: small-group,
  not enterprise scale).
- Mid-sync ("full history sync can take a moment" is currently a raw
  string) — needs a real loading/settling state; sync latency is
  structural here, not a bug to hide.
- A concurrent-edit conflict on a shared doc (backend logic already
  real: conflict detection, "Use this" per-revision) — needs to feel
  like a normal, safe thing to resolve, not an error.
- An open Decision nearing its deadline with a blocker — must read as
  calm and legible, not alarming.
- Camera permission denied / no camera — graceful fallback to manual
  paste (already works functionally; needs real design).
- Offline / no peers reachable — a real state given the P2P model,
  currently invisible.

## 6. Interaction Model

Join is the hero flow: scan → see the Table's people appear → land on
the Feed with that Table's pinned welcome surfaced. Sending a message,
reacting, tagging, and responding to a Decision should all feel
immediate — optimistic UI, since the backend already syncs
asynchronously and the UI shouldn't visibly wait on network round-trips
for local actions. Decisions need an interaction model that makes "I
don't need to act, my silence is fine" a legible, comfortable choice —
not every screen pushing toward an explicit response. **Feed filtering**
(by Table, by tag) needs to feel as fast as a client-side toggle even
though it's filtering already-synced local data, not a new fetch —
there's no reason this should ever feel like a loading state.
**Choosing a sticker** during onboarding should feel like the fun part,
not a form field — a real moment to invest motion/delight in, not a
dropdown.

## 7. Content Requirements

Real UX-writing pass needed throughout — current copy is
developer-facing (`"call spawn_node first"`, raw error strings,
"Proposal"/"Signal"/"Ratified"/"Blocked"). Warm, plain-language
equivalents everywhere a non-technical person will read them;
protocol-accurate terms survive only in the Advanced surface. **New**:
the built-in sticker set itself is a content deliverable (needs curating
— a cohesive, warm, on-brand set, not clip art), and pinned-welcome
copy needs a real prompt/placeholder for whoever's writing one ("what
should people know about this table?") so it doesn't read as a blank
intimidating text box.

## 8. Recommended References

`spatial-design.md` (multi-section app, needs real IA — now also a
merged-feed layout problem, distinguishing four content types in one
stream), `interaction-design.md` (join/propose/signal are all forms
that matter), `motion-design.md` (the join flow, sticker-picking, and
Decision-status changes are the moments worth real motion investment),
`ux-writing.md` (heaviest lift — almost everything user-facing needs
rewriting, now including the sticker set's own personality).

## 9. Open Questions

- **`tagging::pins()` is core-only so far** — proven live against
  `atproto-iroh-core` directly, not yet exposed as a Tauri command
  (`add_tag`/`tags_for` already are; `pins` needs the same one-line
  wrapper treatment during implementation) or a CLI subcommand.
- **The sticker/avatar field is a real, small `atproto-iroh-core`
  change** (`NodeProfile` gaining something like `avatar: Option<
  String>`, an id into the client's built-in set, not a blob — current
  schema confirmed to have no such field). Small and backward-compatible
  (`skip_serializing_if`, same pattern every other optional `NodeProfile`
  field already uses), but real — flagging so it isn't discovered
  mid-implementation as a surprise.
- **Feed performance/architecture**: today's Tauri commands
  (`list_messages`, `list_images`, `list_proposals`,
  `list_document_revisions`) are all per-Table. A merged feed needs
  either N client-side calls merged in JS (simplest, fine at small
  scale) or a new aggregating Tauri command (better at real scale) —
  worth deciding once real usage patterns exist rather than
  over-building now; simplest option first is consistent with this
  repo's whole build history.
- Custom-avatar-upload's federation/moderation question, deferred
  deliberately (§4): once images can represent *people* rather than
  just shared content, "who can see/serve someone's avatar across
  Tables" becomes a real design question this brief isn't resolving.
- Scope of what ships in v1 of the *implementation* (design covers the
  whole app; build could still stage — e.g. Feed/Messages/Decisions
  first, Documents/Tagging/Advanced following).
- Real device testing is still zero for this whole app — worth
  sequencing a design review against an actual phone before going deep
  on polish, same caveat as the rest of this session's Android work.
