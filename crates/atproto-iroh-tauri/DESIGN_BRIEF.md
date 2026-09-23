# Design brief — Tauri reference client rebuild

Produced via `/shape` (2026-09-23), confirmed with Jason. Design context
this brief draws on lives in `../../.impeccable.md` and is summarized in
root `CLAUDE.md`'s "Design context for the Tauri reference client's UX"
section — read those first if anything here seems to assume context.

Handoff target: `/impeccable craft` (or equivalent implementation work)
against this brief. The current plain HTML/JS/CSS frontend
(`dist/index.html`/`main.js`/`styles.css`) is being replaced with a real
React + Vite frontend over the same, unchanged Rust/Tauri backend — no
core-crate or Tauri-command changes implied by this brief.

## 1. Feature Summary

A full rebuild of the Tauri reference client from a plain, undesigned
debug UI into a real product: a private, serverless coordination app for
small groups — messaging, shared decisions, documents, and photos.

## 2. Primary User Action

In the first minute: see who's actually here, and register that this is
private — not composing a message, not reading a decision. Trust and
belonging come before any task. Opening a Table should answer "whose
table is this?" before it asks the user to do anything.

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
- **Top level**: a list of Tables the person belongs to — home base, not
  a namespace-id picker.
- **Inside a Table**: Members-first landing (the primary action), then
  Messages, Decisions (governance/polls, relabeled away from raw
  "Proposal"/"Signal"), Photos, Shared docs — tabs or a persistent side
  rail, not a dense multi-section scroll like today's `index.html`.
- **Protocol/advanced stuff** (raw inspector, relay/control, mute
  internals, `did:iroh` display) behind a clearly-secondary
  "Advanced"/settings surface — present for the curious, never in the
  primary path.
- **Joining**: QR scan is the front door for most people — the obvious
  first action, not one option among many on a dense form.

## 5. Key States

- First launch, no identity yet (today: a bare "Spawn node" button —
  needs a real welcome moment).
- No Tables yet — empty state should teach ("scan a code to join, or
  start your own table"), not just say "nothing here."
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
Members. Sending a message, reacting, tagging, and responding to a
Decision should all feel immediate — optimistic UI, since the backend
already syncs asynchronously and the UI shouldn't visibly wait on
network round-trips for local actions. Decisions need an interaction
model that makes "I don't need to act, my silence is fine" a legible,
comfortable choice — not every screen pushing toward an explicit
response.

## 7. Content Requirements

Real UX-writing pass needed throughout — current copy is
developer-facing (`"call spawn_node first"`, raw error strings,
"Proposal"/"Signal"/"Ratified"/"Blocked"). Warm, plain-language
equivalents everywhere a non-technical person will read them;
protocol-accurate terms survive only in the Advanced surface.

## 8. Recommended References

`spatial-design.md` (multi-section app, needs real IA),
`interaction-design.md` (join/propose/signal are all forms that matter),
`motion-design.md` (the join flow and Decision-status changes are the
two moments worth real motion investment), `ux-writing.md` (heaviest
lift — almost everything user-facing needs rewriting).

## 9. Open Questions

- Scope of what ships in v1 of the *implementation* (design covers the
  whole app; build could still stage — e.g. Messages/Members/Decisions/
  Photos first, Documents/Tagging/Advanced following).
- Real device testing is still zero for this whole app — worth
  sequencing a design review against an actual phone before going deep
  on polish, same caveat as the rest of this session's Android work.
