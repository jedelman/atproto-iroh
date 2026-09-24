# atproto-iroh-tauri

Reference client for `atproto-iroh-core` — see the root `CLAUDE.md`'s
"Client" section for why Tauri, and `../../SPEC.md` for the protocol
this is a UI on top of.

## Frontend rebuild (2026-09-23) — React + Vite, in progress

The plain HTML/JS/CSS frontend (`dist/index.html`/`main.js`/`styles.css`,
the vendored `jsQR`) is **gone**, replaced by a real React + Vite app
under `src/` per the confirmed `/shape` design brief
(`DESIGN_BRIEF.md`) — a deliberate departure from the "no bundler, no
framework" rule this crate held to until now, not a regression from it.
`dist/` is now a *build output* directory (`.gitignore`'d), regenerated
by `npm run build` / Tauri's own `beforeBuildCommand`, not source.

**Built and working**: the design-token system (warm-dark palette, Young
Serif/Hanken Grotesk, from the confirmed mockup artifact), the `Sticker`
component (built-in blob avatars), the **Feed** screen (the unified
cross-Table timeline — messages/photos/decisions merged, Table filter
chips, the pinned-message excerpt), the **Onboarding** screen (sticker +
name picker), the **Table detail** screen (Members-first landing, the
Table's pinned message(s) in full, tabbed Messages/Decisions/Photos/
Shared doc — DESIGN_BRIEF.md §4), and the **Join** screen (camera QR
scan, re-ported from the old plain-JS app onto the real `jsqr` npm
package, with a paste-a-ticket fallback). All wired to a real API
abstraction (`src/api/`) that talks to the actual Tauri backend when
running inside Tauri and falls back to an in-memory mock backend
(`src/api/mockClient.ts`) otherwise — see "Mock dataset and tests," below.
Tapping a Table's sticker in the Feed navigates to its detail screen;
the filter chips below it filter the Feed in place instead; the dashed
"+ Join" circle at the end of that strip (and the Feed's own empty
state) opens Join. Two more screens: **Profile edit**
(`/table/:id/profile`, reached from "Edit your profile" in Table
detail's Members section) — a NodeProfile is per-Table (SPEC.md §3.4's
`RecordIdentifier` is `(namespace, author, key)`, and `NodeProfile::
SELF_KEY` is a fixed per-author slot *within one namespace's doc*), so
there's no single "your profile" to edit, only "your profile in this
Table"; prefills from the real synced profile via `listProfiles` +
`nodeDid()`, same sticker picker as Onboarding, and calls the
already-existing `updateProfile` (wired into `Client` early this
session but never reached from any screen until now). **Mute**
(`/mute`, reached from the Feed header's own avatar) — global, not
per-Table (`mute::MuteList`'s own doc comment: local-only, no sync, no
lexicon), a plain hex-id mute/unmute list against the three new
`Client` methods (`muteAuthor`/`unmuteAuthor`/`listMuted`). A real
`useMutedAuthors` hook (polls `listMuted()` on mount) filters muted
authors' content out of the Feed (messages/photos/decisions together,
via a small `itemAuthorHex` switch over `FeedItem`'s three variants)
and Table detail's Messages and Photos tabs. **The pinned-messages
section (both Feed's featured excerpt and Table detail's own pinned
card) also filters now** — a code-review pass caught that it originally
read straight from the unfiltered `pins`/`pinsByTable`, so a muted
author's message still showed there in full even though the identical
content was correctly hidden everywhere else; fixed by hiding a pin
when *either* the person who pinned it *or* the pinned message's own
author is muted, with a regression test for each case
(`TableDetail.test.tsx`). Governance/Decisions still isn't filtered —
a real, named gap, not silently dropped. Not just plumbed and left
untested: `Feed.test.tsx` and `TableDetail.test.tsx` each got a real
mute-then-assert-gone test, not just a "the command exists" check.

Five of the Table detail tabs are new since Composing shipped:
- **Composer** (Messages tab) — optimistic: the sent message appears
  immediately, resolved through the real self author hex via
  `nodeDid()` rather than a placeholder, so it renders through
  `profileFor()` exactly the way the synced-back copy eventually will.
- **Shared doc** — one fixed doc id (`"notes"`) per Table for now (no
  doc picker/creation UI yet, a real named gap); re-ports the old
  plain-JS app's conflict-visibility logic (`src/lib/docConflict.ts`,
  unit-tested — `hasPossibleConflict` diffs the two most recent
  revisions' rev timestamps as `BigInt`s, flags it only when they came
  from *different authors* within 5 minutes of each other, not just
  "there's more than one revision") rather than reinventing it. Every
  revision in history gets a "Use this" button that loads its text into
  the edit box for review — **still no automatic merge**, resolution is
  a human reading both and re-saving, same as the old app; this closes
  the visibility gap, not the "diff and merge text" problem, which was
  never in scope. `docSave`/`docLoad`/`docHistory` added to `Client`,
  implemented against the real `doc_save`/`doc_load`/`doc_history`
  commands and against `mockClient`'s own per-Table doc-revision map
  (seeded from `DOC_REVISIONS` in fixtures.ts, which now includes a
  genuine concurrent-edit case on the Garden Table so the conflict
  banner has something real to trigger on, not just a contrived test).
- **Decisions** — a propose form (title + a 1 day/3 day/1 week deadline
  picker, no subject-member/policy-change UI — that's real cosigner-
  governance complexity out of scope for this pass) plus Support/Object
  buttons on each open decision, gated on the real self author hex
  being in the Table's `eligible_hex`. Deliberately hardcoded to
  `GovernanceClass::General` — the exact same relabeling the old
  plain-JS app's Polls section used (CLAUDE.md's batteries-included
  list): calls the same `create_proposal`/`create_signal` commands the
  full governance model uses, no new mechanism. Worded carefully, not
  just built: the form's hint text says outright **"not a majority
  vote"** — this ratifies by default absent enough objection, and
  calling it a plain "propose" form without that line would have
  implied ordinary vote-counting behavior it doesn't have.
  `useTable`'s `refreshProposals` re-fetches proposals + governance
  state after a propose/signal (re-running the real fold, not
  guessing the new ratification status client-side) — `mockClient`'s
  own signal handling is honest about the same limit: it only
  simulates the one thing this UI acts on immediately (a Block moves a
  decision to a visibly-objected state), since real ratification also
  depends on the deadline passing, which the mock doesn't simulate.
- **Photos** — a real upload (file input + optional caption) and a real
  gallery, replacing what used to be a decorative gradient placeholder
  for every image regardless of content. `Client` gains `uploadImage`/
  `loadImageBytes`; `tauriClient` passes bytes across the Tauri IPC
  boundary as a plain number array, same as the old plain-JS app did
  (no separate binary-transfer path in this reference app);
  `mockClient` keeps a real byte store (`Map`, keyed by
  `${namespaceId}/${authorHex}/${rkey}`) separate from image metadata —
  mirroring the real backend's own split between `list_images` and
  `load_image_bytes` — so a freshly-uploaded photo's bytes actually
  round-trip through `ImageThumb`'s `loadImageBytes` call into a real
  `Blob` object URL, not a `content_type`-tinted rectangle. The
  pre-existing fixture photo (Sequoia's overlook, from before this
  session) deliberately has no bytes in that store, so it renders the
  same "not synced yet" state a real node that hasn't synced those
  bytes would show — an honest gap, not silently faked. Same optimistic
  self-author-hex pattern as Composer's `send()`: the local entry
  resolves through `nodeDid()` before `ImageThumb` ever tries to load
  its bytes, so the lookup key actually matches what `uploadImage` just
  wrote. `src/test/setup.ts` gained two small jsdom polyfills
  (`File.prototype.arrayBuffer`, `URL.createObjectURL`/
  `revokeObjectURL`) neither of which jsdom implements — real Tauri-
  webview/browser APIs missing only from the test environment, not
  worked around in the app code itself.
- **Tagging** — a "+ Tag" affordance on every message (Messages tab)
  showing its existing tags as chips and letting you add a new one, plus
  a filter-chip row above the list that narrows the tab to messages
  carrying a clicked label. New Tauri command `list_all_tags`
  (`tagging::list_all_tags` unwrapped — `tags_for` alone only answers
  "what tags does *this* subject have," not "what labels exist to
  browse by"), plus `Client.listAllTags` and a `useTags` hook. Reserved
  `system:`-prefixed labels (`PIN_LABEL` and anything else this crate
  later claims — `tagging.rs`'s own top doc comment) are filtered out of
  both the chip row and the per-message tag list, so pins never show up
  disguised as a browsable tag. Images and Shared docs aren't wired into
  this pass — messages only, a real scoped gap, not silently dropped.

  **Found and fixed a real cross-feature bug while building this, not
  cosmetic**: `mockClient`'s `listMessages`/`listImages`/
  `listProposals`/`listAllTags`/`docHistory` all originally returned the
  live, mutable array a later `push()` would go on to mutate *in place*
  — not a copy. Tagging a message called `refresh()`, which fetched
  that same array reference (now containing the new tag) and handed it
  straight to `setState`; React's `Object.is` bailout compared it
  against the *exact same reference* already sitting in state and
  silently skipped the re-render, even though the underlying data had
  genuinely changed. Confirmed live via an isolated repro
  (`useTags` driving a two-line probe component) before touching the
  fix, not assumed from reading the code — the repro's own
  `before`/`after` array aliasing is what gave it away: capturing
  `before = await api.listAllTags(...)` and asserting against
  `before.length + 1` failed because the *same array* `before` pointed
  at had already grown once `addTag` ran. Fixed in `mockClient.ts`:
  every list-returning method now spreads into a fresh array
  (`[...(x[id] ?? [])]`) before returning it — `useTags.ts`'s own top
  comment carries the postmortem for whoever touches this file next.

**Code-review passes (2026-09-23/24) — four more real bugs found and
fixed, plus two follow-up corrections found reviewing those same
fixes**, on top of the ones each feature's own section above already
documents:
- **Onboarding's name/sticker never reached a Table.** `Join.tsx`'s
  `doJoin()` called `joinNamespace` but never `updateProfile` — a new
  member showed up as "Someone" with a default sticker in every Table
  they joined until they separately visited Profile edit and re-entered
  everything. Fixed: `doJoin` now also calls `updateProfile` with the
  `useProfileDraft` pick right after a successful join (best-effort — a
  failure there doesn't strand the person on an error screen for a join
  that actually succeeded), with a regression test asserting the joined
  Table's Members list shows the real name, not a placeholder. **A
  follow-up review pass caught the first version of this fix writing
  those onboarding defaults unconditionally** — Join is a persistent,
  always-reachable entry point (Feed's "Join or start a table" link),
  and re-scanning/re-pasting a ticket for a Table already belonged to
  is a real path, not hypothetical, since `joinNamespace` is safely
  re-callable; the unconditional write would silently clobber an
  existing member's real name/avatar/neighborhood/description/
  `governance_eligible` back to onboarding defaults on every re-join —
  the exact class of data loss the `governance_eligible` round-trip fix
  below exists to prevent. Fixed by checking `listProfiles` for an
  existing self entry first and only writing the draft if none exists;
  a second regression test re-joins an already-profiled Table with a
  different draft name and asserts the original name survives.
- **The camera never stopped when Join was navigated away from mid-scan.**
  `useQrScanner` only released `getUserMedia`'s `MediaStream` on an
  explicit `stop()` call, never on unmount — clicking a nav link while
  `status === "scanning"` left the browser's camera indicator on
  indefinitely. Fixed with a `useEffect` cleanup; `useQrScanner.test.tsx`
  (new) mocks `getUserMedia` and asserts the track's `stop()` actually
  fires on `unmount()`. **A follow-up review pass caught a narrower race
  in that same fix**: unmounting while `status === "requesting"` (the
  permission prompt still pending) meant `streamRef` was still empty
  when the cleanup ran, so a stream that showed up *after* unmount —
  because the person granted camera access only after navigating
  away — had nothing left to stop it. Fixed with a `mountedRef` that
  `start()`'s own `await getUserMedia(...)` continuation checks before
  ever assigning the stream, shutting it down immediately instead of
  turning the camera on for an unmounted component; a second
  regression test resolves a pending `getUserMedia` promise only after
  `unmount()` and asserts the late stream still gets stopped.
- **The Feed header showed a random member's avatar, not the viewer's
  own.** `profilesByAuthor[Object.keys(profilesByAuthor)[0]]` picked
  whichever author happened to be first in a cross-Table map built by
  `Promise.all` (async completion order, not self-identity) — someone
  in several Tables would typically see someone else's sticker as their
  own account icon. Fixed by resolving self via `nodeDid()`, the same
  pattern Composer/DecisionsPanel/ProfileEdit already use; regression
  test asserts the header's sticker is specifically the fixture self
  author's real avatar.
- **A checkbox in Profile edit that did nothing.** "I can weigh in on
  this Table's decisions" wrote `NodeProfile.governance_eligible`, a
  field no code path anywhere in core, the CLI, or the Tauri commands
  reads to determine real eligibility — that's decided entirely by
  synced `Founding`/`AdmitCoSigner` records via `fold::fold_namespace`
  (`create_namespace_with_profile`'s own comment already says the
  self-assertion heuristic this field once meant was replaced by that
  real mechanism). Checking it silently had zero effect on whether
  Support/Object actually appeared. Removed the checkbox rather than
  wire it to something it was never connected to; the field itself is
  still round-tripped on save (read, then written back unchanged) so a
  profile edit can't accidentally erase a founder's existing
  `Some(true)`.

**A third review pass, against those same fixes, found three more —
none of them "new" bugs so much as robustness/hygiene gaps in how the
fixes above were written:**
- **The re-join-clobber fix could silently skip writing a first-time
  member's profile too.** The existence check (`nodeDid`/`listProfiles`)
  and the write shared one `try`/`catch`, so a transient failure in
  either read — not the write itself — would skip the write on a
  brand-new join, reintroducing "Onboarding's name/sticker never
  reached a Table" on the common path instead of the rare re-join one.
  Fixed by giving the existence check its own `try`/`catch` that
  defaults to "no profile yet" on failure (erring toward writing, the
  safe direction — worst case is an extra overwrite on a genuine
  re-join, never a missing profile on a first join); only the write
  itself stays best-effort against the outer catch.
- **The `did:iroh:` prefix-stripping snippet was duplicated six times**
  across Composer, PhotosPanel, DecisionsPanel, ProfileEdit, Feed's
  header, and now Join — a future change to the prefix format would
  need finding and fixing in six places, and any one missed would
  silently reintroduce the exact "resolve self, not a placeholder"
  class of bug several of those six were themselves fixes for.
  Extracted to `src/lib/identity.ts` (`resolveSelfAuthorHex`); every
  call site now goes through it.
- **`Join.test.tsx`'s re-join test only passed because an earlier test
  in the file happened to run first** and seed the profile it then
  checked wasn't clobbered — a real ordering dependency the suite
  didn't structurally enforce (confirmed by running with
  `--sequence.shuffle`, which broke it). Fixed by giving it its own
  dedicated fixture Table (`fixtures.ts`'s `PHOTOCLUB_TABLE_ID`/
  `"mock-ticket-photoclub"`, alongside the existing book-club one) and
  making the test self-contained — join once to seed a profile, join
  again with a different draft, assert the original survives, all
  within the one test.

**A `/simplify` pass (2026-09-24) — not a bug hunt, a code-quality pass**
(reuse, simplification, efficiency, altitude, via four parallel review
agents against the diff), run after the three code-review rounds above
were already clean. Real duplication and inefficiency found and fixed,
none of it a correctness bug:
- **`PinIcon` and the pin-visibility filter were each defined twice**
  (Feed.tsx and TableDetail.tsx independently) — extracted to
  `components/PinIcon.tsx` and `lib/mutedPins.ts` (`isPinVisible`).
- **`findMessageBySubject`** extracted to `lib/feedMessages.ts`,
  replacing Feed's inline `pinExcerpt()` scan.
- **The "resolve self author hex, in state, cancel-safe on unmount"
  effect was duplicated between Feed's header and DecisionsPanel** —
  extracted to a shared `hooks/useSelfAuthorHex.ts`.
- **`useTags.ts`/`useTableDoc.ts` each duplicated their own mount-effect
  body and (`useTags.ts` only) an inline `isSystemLabel` check** written
  twice in the same file — deduped, and both hooks' `cancelled`-flag
  pattern replaced with a monotonic `requestId` ref so a manual
  `refresh()` call is race-safe too, not just the mount effect.
- **`resolveSelfAuthorHex` (`lib/identity.ts`) re-resolved `nodeDid()`
  on every call**, including from hooks now called in more places after
  the dedup above — added `WeakMap`-keyed memoization (keyed by the
  `api` object's identity, not a module-level singleton, so tests don't
  leak state across files).
- **`Join.tsx`/`ProfileEdit.tsx` awaited `resolveSelfAuthorHex` and
  `listProfiles` sequentially** despite being independent calls —
  switched to `Promise.all`.
- **`ProfileEdit.tsx`'s six separate `useState` fields** (name,
  category, avatar, neighborhood, description, governanceEligible) —
  always read together on save and always set together on load —
  consolidated into one `ProfileForm` object.
- **The sticker-picker grid (4-column grid of `STICKER_IDS` as
  selectable buttons) was duplicated between Onboarding and
  ProfileEdit**, differing only in size/gap — extracted to
  `components/StickerPicker.tsx`.

A shared `lib/cardStyle.ts` (`cardStyle`, radius 16; `rowCardStyle`,
radius 12) followed the same day, once actually built and
screenshot-verified rather than skipped as originally noted below: a
dozen-plus call sites across Feed/TableDetail/Mute had each written the
same `background: var(--surface); border: 1px solid var(--border);
border-radius: …` inline. Call sites spread the shared object in and
override padding/border-color locally (the rose-bordered conflict alert
in TableDetail's doc panel is `{...rowCardStyle, border: "1px solid
var(--rose)"}`). **Caught a real bug introduced by this refactor before
it shipped, not a pre-existing one**: Feed.tsx's `FeedItemCard` had its
own locally-scoped `cardStyle` const (with `overflow: "hidden"`, needed
to clip the photo item's image to the rounded corners) that shadowed
the newly-imported shared one — the first pass left two of its three
`return` branches (photo and message) pointing at the *shared* import
instead of the local `overflow: hidden` variant, which would have
un-clipped those cards' corners. Renamed the local variant to
`itemCardStyle` and fixed both call sites; caught by re-screenshotting
every card variant (pinned message, feed item cards, decisions panel,
muted-author row, doc conflict alert) after the change, not just by the
type-checker, since a shadowed-import bug like this type-checks fine.

Skipped, with reasoning rather than silently dropped: a generic
`useAsyncEffect` to unify the several remaining cancelled-flag fetch
effects elsewhere (judged too invasive to apply blindly); a shared cache between `useFeed` and
`useTable` to avoid re-fetching on Feed→Table navigation (real
architectural change, not a local fix); generalizing `mockClient.ts`'s
`signalDecision` beyond its current block-only special case (no UI
exercises the general case yet — would be speculative); deduping the
rkey zero-padding logic reimplemented across `mockClient.ts`/
`fixtures.ts`/`docConflict.test.ts` (two of the three sites are test
fixtures, not production code — the agent that raised it called its own
finding weak).

**Screenshot-verified, not just test-verified** — a real Playwright +
headless-Chromium pipeline against `npm run dev`'s mock-backed browser
session caught three real bugs the type-checker and the test suite both
missed: table names truncating illegibly, a pinned excerpt silently
falling back to placeholder text (a dangling fixture reference — a real
message never existed at that pinned subject), and Table detail showing
*every* mock member on *every* table instead of each table's real
members (`mockClient.ts` was seeding all tables from the same full
profile set). All three fixed, and turned into real test assertions
afterward so they'd be caught by the suite next time, not just eyeballs.

**Reply-to threading — built (2026-09-24).** The backend was already
fully ready (`messaging::reply_ref`, the Tauri `send_message` command's
`reply_to_author_hex`/`reply_to_rkey` params, `MessageView.reply_to` —
all wired since before this frontend rebuild started); the gap was
entirely frontend. `Client.sendMessage` gained an optional `replyTo`
param (threaded through `tauriClient`/`mockClient`); `TableDetail.tsx`
gained a "Reply" action per message (alongside Pin/+Tag), a lifted
`replyTarget` state showing a "Replying to X: '…'" chip above the
Composer with a cancel button, and a "↳ Replying to X: '…'" indicator
on any message that has a `reply_to`, resolved via a new
`messageByReplyRef` map — deliberately separate from the existing
`messageBySubject` map, since `reply_to` uses `messaging::reply_ref`'s
`"{author_hex}/{rkey}"` convention, not `record_ref`'s tagging-oriented
`"{author_hex}/{collection}/{rkey}"` one; reusing the wrong map would
have silently never matched. `TableDetail.test.tsx`'s new test exercises
the whole path against the mock backend (click Reply, compose, send,
confirm both the composer's chip clears and the new message shows its
own reply-to indicator), scoped to Weekend Hikers rather than Garden to
avoid leaking a second fixture-text match into a later test via
mockClient's shared module-level state (the same hazard a code-review
pass already flagged once for `Join.test.tsx`). Still not built: a
dedicated thread view reachable *outside* a Table's own Messages tab —
this closes reply-to itself, not a separate threaded-conversation
screen, which was never clearly asked for and is a real IA decision on
its own.

**Full governance UI — built (2026-09-24).** Same pattern as reply-to
above: the Tauri `create_proposal` command already accepted `class`,
`subject_member_hex`, and `policy_change` (nothing about that command
changed), and the frontend types (`GovernanceClass`, `PolicyChange`,
`Proposal.subject_member`) were already fully modeled — the gap was
entirely the "New decision" form only ever sending
`GovernanceClass::General`. `Client.createDecision` gained an optional
4th `extra` param (`class`/`subjectMemberHex`/`policyChange`); the New
decision card in `TableDetail.tsx`'s `DecisionsPanel` gained a type
selector — Poll (unchanged default), Admit a co-signer, Remove a
co-signer, Change policy — each revealing its own fields: a member
picker (filtered to non-eligible members for admit, eligible members
for remove — `PolicyChange.for_class`'s own three real targets, `admit
CoSigner`/`removeCoSigner`/`changePolicy`, for the policy-change
target dropdown, matching `governance.rs`'s own `for_class` exactly,
since `General` was never policy-governed), and a window/threshold pair
for policy changes. Every proposal in the list now shows a small
class label (and the resolved subject member's name) when it isn't a
plain Poll, reusing the same Support/Object voting UI regardless of
class — no new interaction model, since ratification works identically
across classes. Not built: the mock backend doesn't actually apply a
ratified admit/remove/policy-change proposal's effect back onto
`eligible_hex` (the real backend's `fold::fold_namespace` does this by
construction; the mock's `signalDecision` already had a narrower
"block-only" special case flagged as intentionally not generalized in
an earlier `/simplify` pass, and this is the same call extended to a
second field) — so admitting/removing someone in the mock demo creates
a real, votable proposal but won't visibly change the Members list's
"can weigh in on decisions" line once it ratifies. Three new
`TableDetail.test.tsx` tests exercise remove-co-signer, the
no-admittable-candidates empty state, and change-policy, all scoped to
Weekend Hikers rather than Garden — Garden is the one Table other tests
assert exact proposal counts/text against, and `--sequence.shuffle`
confirmed proposing on it would leak an extra "Open" pill into those
assertions depending on run order (the same shared-module-state hazard
this file already works around elsewhere, now hit by a new class of
test). A pre-existing, unrelated flake was also found running the full
suite under `--sequence.shuffle` (not introduced by this work, and not
present under a normal `npx vitest run`): `Join.test.tsx`'s "carries
the Onboarding name/sticker" test can fail if another test joins the
same Book Club table first in a shuffled order, since Join.tsx's own
"don't clobber an existing profile" check then correctly skips writing
the draft profile — flagged here rather than silently left for the
next person to rediscover, not fixed in this pass.

**Not yet ported — real, current gaps, not oversights**: Tagging on
Images/Shared docs (Messages only, above), QR *generation*
(`ticket_to_qr` — scanning is built, Join screen above), the raw
inspector, relay/control.

**Design-interview edge cases fleshed out (2026-09-24)**, following a
gap check against `.impeccable.md`/`DESIGN_BRIEF.md` after the
`/simplify` pass above — three real, previously-flagged gaps closed:

- **No pinned-welcome empty state.** §5's "a brand-new Table without a
  pinned welcome shouldn't feel broken or half-finished" had no real
  copy — the pinned-messages block just rendered nothing when
  `visiblePins.length === 0`. Fixed with a real line ("Nothing pinned
  yet — pin a message below to say what this Table's about"). **Found
  a deeper gap fixing this**: there was no way to pin a message from
  the UI at all — `tagging::pins()`/`PIN_LABEL` were read-only wired
  (the top-of-screen strip), with no write path, even though pinning
  has been "anyone can pin, one per author" since the design brief.
  Added a real Pin action to `MessageTags` (`api.addTag(tableId,
  subject, PIN_LABEL)`) and a `useTable.ts` `refreshPins()` (same shape
  as the existing `refreshProposals()`) so pinning refreshes the strip
  without a full Table reload. `TableDetail.test.tsx`'s new test
  exercises the whole path against the mock backend: empty-state copy
  → click Pin → the encouragement text is replaced by a real pinned
  card, not just a state assertion.
- **No motion anywhere** — confirmed by grep, not assumed: zero CSS
  transitions, keyframes, or inline `transition` properties existed in
  the whole frontend before this pass, despite DESIGN_BRIEF.md §6/§8
  naming three specific moments worth real investment. Added, scoped to
  exactly those three, plus a `prefers-reduced-motion` guard in
  `global.css` covering all of it:
  - **Sticker-picking**: `StickerPicker`'s buttons get a hover/press
    scale (`.sticker-btn`), and `Sticker`'s selection ring transitions
    in rather than popping instantly.
  - **The join flow**: the camera view fades/scales in on reveal
    (`.fade-scale-in`); landing on a Table right after joining (not
    just opening one you already belonged to) gives the "who's here"
    strip a real arrival moment (`.joined-pop`), threaded through via
    `navigate(..., { state: { justJoined: true } })` from `Join.tsx`
    and read with `useLocation()` in `TableDetail.tsx`.
  - **Decision-status changes**: the status pill's color transitions
    (`.status-pill`) rather than snapping when a signal flips Open to
    Passed/Blocked.
- **Offline/no-peers-reachable state — deliberately NOT built.** §5
  flags this as "a real state given the P2P model, currently
  invisible," and it still is: checked whether it could be closed
  alongside the other two, and it can't be without new backend surface
  first. iroh's `Endpoint::remote_info` (checked directly in the
  vendored crate source) needs a specific peer id to query, not a
  general "how many peers can I currently reach" — there's no existing
  `Client` command, mock or real, that could back a UI state honestly.
  Building one is a real, scoped addition (a core method over
  `Node`'s `router.endpoint()`, a Tauri command, a frontend hook) but a
  materially bigger one than the two above, and out of proportion to
  this pass — flagged here rather than faked with a placeholder banner
  that isn't wired to anything real.

## What's real here

- `src-tauri/` is a genuine Tauri 2 app: `atproto-iroh-core` is a normal
  path dependency, no IPC or FFI boundary between UI-adjacent code and
  protocol logic.
- Thirty commands (28 plus `pins` and `list_all_tags`, both added
  2026-09-23 alongside their `tagging::` core-crate counterparts), each
  a direct call into the core crate:
  `spawn_node` (loads or generates a persistent `did:iroh` identity and
  starts a real `iroh` node against real on-disk storage — deliberately
  not done at app launch; see `main.rs`'s comment on why), `node_did`,
  `create_namespace_with_profile` (creates a namespace, publishes a
  `NodeProfile` into it, **and posts this author's real `Founding` claim**
  — `fold::found_namespace`, replacing what used to be a placeholder
  heuristic; see the Governance bullet below), `list_namespaces`,
  `share_namespace` (`Node::share`, unwrapped to a plain paste-able
  ticket string), `join_namespace` (`Node::join`, which SPEC.md §6 item
  10 confirmed backfills full history, not just future writes — a real
  join), `dump_namespace` (the inspector, below), `ticket_to_qr` (an SVG
  QR code of a ticket string, nothing more), `doc_save`/`doc_load`/
  `doc_history`/`submit_to_inbox` (the freeform layer, below),
  `send_message`/`list_messages` (Messaging, below),
  `upload_image`/`list_images`/`load_image_bytes` (Images, below),
  `update_profile`/`list_profiles` (Members/Profile, below),
  `mute_author`/`unmute_author`/`list_muted` (Mute, below),
  `add_tag`/`tags_for` (Tags, below), and
  `list_proposals`/`governance_state`/`create_proposal`/`create_signal`
  (governance, below). `AppState.docs` holds every namespace this node
  currently has open — repopulated from disk on every `spawn_node` call
  now, not just built up in memory during one session — and
  `AppState.author` this node's persistent default record-signing author
  (`DocsApi::author_default`, not minted fresh), so edits from one
  person land under one consistent author across restarts, not a new
  stranger each launch.
- **The inspector** (`namespace::dump_all`, `dist`'s "Inspector"
  section): every raw entry currently synced into a namespace — author,
  key, timestamp, content — with no per-record-type code anywhere in the
  chain. Rust has no runtime reflection, so this isn't literally
  automatic over arbitrary types, but every record this crate writes is
  JSON or plain text, so "try JSON, then UTF-8 text, then fall back to
  hex" gets the same practical result: one generic view that already
  covers `NodeProfile`, governance `Proposal`/`Signal`/`Founding`
  records, and freeform text writes without having been told about any
  of them, and covers whatever gets added next the same way. A debugging
  view for building against, not a feature end users need.
- **Shared doc** (`namespace::save_document_revision`/
  `list_document_revisions`/`load_document`, `dist`'s "Shared doc"
  section): a namespace doesn't have to hold typed records at all —
  nothing about `iroh-docs` or this crate requires it. **No longer backed
  by `put_text` at a fixed key** — that was confirmed live (SPEC.md's
  2026-09-22 CRDT note) to be last-write-wins on concurrent offline
  edits, silently dropping one side's edit on reconnect. `doc_save` now
  writes an immutable, uniquely-keyed revision instead; `doc_load` shows
  the latest by save order, `doc_history` shows every revision so the UI
  can surface "someone else edited this while you were offline" instead
  of quietly discarding it. `namespace::put_text`/`get_text` still exist
  in the core crate (right primitive for genuinely single-writer text)
  but nothing in this UI calls them anymore. **Conflict visibility, not
  just history, built 2026-09-22**: "Load" now always refreshes history
  alongside it, and `main.js` flags a real concurrent-edit signal — the
  two most recent revisions from *different authors* within 5 minutes of
  each other (not just "more than one revision exists," which a single
  person saving twice in a row also produces) — with a visible
  `⚠ possible conflict` banner and a "Use this" button per revision that
  loads it into the edit box for review. Still no automatic merge —
  resolving a conflict is a human reading both revisions and re-saving,
  same as it always would be; this closes the *visibility* gap (silently
  picking a side), which is what was actually missing, not a text-diff
  problem this session never claimed to solve. The banner logic is
  pure/no-DOM and unit-tested standalone (five assertions including the
  exact window-boundary case) since nothing else in this reference app's
  JS has test coverage otherwise.
- **Inbox** (`namespace::submit_text`, `dist`'s "Inbox" section): the
  freeform layer's other half — `submit_text` mints a fresh key per call
  (`new_entry_key`, shared with `fold.rs`'s `propose`/`signal`), so any
  number of strangers can write without colliding or needing to agree on
  anything first. Unlike `doc_save`, this one's already collision-free by
  construction (nobody's overwriting anybody else's submission), which is
  exactly why it didn't need the same CRDT fix the Shared doc primitive
  did. This is the actual mechanism behind "a public inbox" — see the QR
  note below for why that's safe.
- **Messaging** (`messaging::send_message`/`list_messages`, `dist`'s
  "Messages" section): CLAUDE.md's batteries-included app list's first
  build, and the one it predicted would need no new primitive — a
  namespace is the channel, each message mints its own key
  (`new_entry_key`, same generator `submit_text`/`fold::propose`/
  `fold::signal` already share), so there's no collision or
  last-write-wins risk the way the old `put_text`-backed Shared doc had.
  `list_messages` sorts by key (the same "sort by the sortable key, not
  a separately tracked index" approach `list_document_revisions` uses)
  since `list_records` itself makes no ordering promise. `reply_to` is a
  plain `"{author_hex}/{rkey}"` string, same convention
  `governance::subject_ref` uses for `Signal.subject` — restated locally
  in `messaging.rs` rather than sharing one function across two
  unrelated modules for a one-line format.
- **Tags** (`tagging::add_tag`/`tags_for`, `dist`'s "Tags" section):
  cross-lexicon by construction (Jason's explicit ask) — one `Tag` type
  can tag a message, a Shared doc revision, a `NodeProfile`, anything,
  via `records::record_ref`'s `"{author_hex}/{collection}/{rkey}"`
  (the one reference format in this crate that actually carries the
  collection name, since `governance::subject_ref`/`messaging::reply_ref`
  only ever point within their own collection). Each message in the
  Messages section gets a "Tag" button that fills in this panel's
  subject field with that exact message's `record_ref` — no manual
  copy-pasting required for the common case.
- **Images** (`images::upload_image`/`list_images`/`load_image_bytes`,
  `dist`'s "Images" section): a file `<input>`, `file.arrayBuffer()` →
  `Array.from(new Uint8Array(...))` → `upload_image`, bytes crossing the
  Tauri IPC boundary as a plain JSON array (fine at reference-app scale,
  not a streaming upload path). No separate blob-fetch step on the read
  side either — `images::load_image_bytes` returns the same bytes
  `iroh-docs` already synced as the entry's content, rendered via
  `URL.createObjectURL(new Blob([...]))` rather than a base64 data URL,
  so no manual encoding on either side of the wire.
- **Members / Profile** (`update_profile`/`list_profiles`, `dist`'s
  "Members" section): `NodeProfile` editing separated from namespace
  creation — `create_namespace_with_profile` still sets an initial
  profile, but this lets an author update their own profile (name,
  category, neighborhood) any time afterward, an overwrite of their own
  fixed `NodeProfile::SELF_KEY` slot, safe by the same per-author-key
  construction as everything else (SPEC.md §3.4). `list_profiles` is the
  "who's here" view — every synced `NodeProfile`, unfiltered.
- **Mute, wired now** (`mute_author`/`unmute_author`/`list_muted`, `dist`'s
  "Mute" section): purely local, no sync, no lexicon (`mute::MuteList`'s
  own doc comment). The one place it actually changes anything yet —
  `list_messages` filters out muted authors before returning; every
  other read path (the inspector, `list_profiles`) is untouched, since
  filtering everything preemptively wasn't what was asked for and mute
  is deliberately scoped per-feature, not global. A muted author's
  messages still sync and still count for everyone else — this only
  changes what this one reader's client shows them.
- **QR codes** (`ticket_to_qr`, generation; camera scanning — built
  2026-09-23, see below): renders a ticket as an SVG QR. No native OS
  share sheet integration (unverified whether Tauri 2 has one; not
  checked). No restriction on which access mode gets turned into a code: a `Write`
  ticket posted publicly is a real, intended pattern here, not a mistake
  to guard against — a public inbox, a dead drop, a graffiti wall.
  Mechanically safe by construction, not by policy: `RecordIdentifier`
  is `(namespace, author, key)` (SPEC.md §3.4's validated finding), so a
  stranger holding a Write ticket can only ever write under an author
  *they* generated — they can't forge entries as you or overwrite anyone
  else's. What a public Write ticket actually needs is volume/moderation
  handling on the read side (see `mute.rs`, already built, not yet wired
  to inbox reading) — the real risk is flooding, which §6 item 12 already
  named as the live case rotation was weighed against, not forgery.
  Genuinely relevant to SPEC.md §3.6's discovery story either way: a QR
  code physically posted somewhere is the same trust shape as a ticket
  shared in a DM — nothing discoverable until someone already has the
  capability — just analog instead of digital.
- **QR scanning** (`dist/main.js`'s "Scan QR" button, next to Join):
  reads a ticket back out of a QR code via `getUserMedia` + a decode
  loop, no Tauri command involved — pure client-side JS, the same shape
  on desktop and mobile. The obvious approach — a Tauri plugin — turned
  out not to be an option: the only maintained one
  (`tauri-plugin-barcode-scanner`) depends on `tauri = "3.0.0-alpha.2"`,
  and this app is on stable Tauri 2 (checked live against the crate's
  own `Cargo.toml` before writing any code, not assumed). Went with a
  vendored decoder instead (`dist/vendor/jsQR.min.js`, jsQR 1.4.0,
  Apache-2.0) — fits this reference client's own "no bundler, no
  framework" rule better than a plugin would have anyway. Verified live,
  not just wired: a Node script round-trips a real QR-encoded string
  through the exact vendored file and confirms an exact match (the risk
  worth checking was the vendored/minified build itself, not the
  well-established jsQR algorithm). **Camera permission on Android**
  goes through wry's own `WebChromeClient.onPermissionRequest`
  (confirmed against the vendored wry 0.55.1 source: it already handles
  `getUserMedia`'s video-capture request, prompting for
  `android.permission.CAMERA` at runtime) — this app only needed to
  declare that permission in the manifest, done via
  `scripts/patch-android-manifest.sh` (idempotent, since `gen/android`
  regenerates from scratch and isn't committed — run it again after any
  fresh `cargo tauri android init`). **Not verified live**: an actual
  camera scan on a real device or emulator (none available in this
  sandbox) — the decode logic and the Android permission path are each
  independently confirmed, but the full path (open camera → point at a
  real QR → fill the field) hasn't been clicked through by a human yet.
  Desktop camera support depends on the OS webview actually exposing
  `getUserMedia` in a secure context; the code degrades to a visible
  error message (not a crash) if it's unavailable, but that path is
  also unverified without a desktop machine with a webcam to try it on.
- **Governance** (`fold::fold_namespace`/`propose`/`signal`, `dist`'s
  "Governance" section): `list_proposals` re-runs the whole fold on
  every call and returns each `Proposal` with its live `Ratification`
  status; `create_proposal`/`create_signal` post new records. **The
  founding-record gap this bullet used to describe is closed** (SPEC.md
  §6 item 16): `create_namespace_with_profile` now calls
  `fold::found_namespace` to post a real `Founding` claim at namespace
  creation (this app's own default starting policy —
  `default_founding_policy`, still explicitly *this app's* choice, not a
  protocol constant SPEC.md §3.7.2 says stays namespace-owned), and
  `list_proposals`/`governance_state` now call `fold::fold_namespace`,
  which resolves real genesis state from whatever `Founding` claims are
  synced instead of the old heuristic (self-asserted
  `governance_eligible: true` in a `NodeProfile`, which that lexicon's
  own field description already called non-authoritative). What's still
  a reference-app shortcut, not a protocol gap: the founder is always
  the namespace's sole genesis member here — this UI never prompts for
  co-founders at creation time, even though `found_namespace`/
  `resolve_founding` support a real multi-founder bootstrap. Anyone else
  has to be admitted afterward through a real `AdmitCoSigner` Proposal.
  **Deliberate, not a gap**: discussed with Jason (2026-09-22) and
  decided not to build a multi-founder prompt — silence-ratifies-by-
  default means inviting co-founders after solo creation costs about as
  much friction as founding together would, and toxic-founder removal
  already exists via governance; the mechanism (`resolve_founding`'s
  acceptance-window design) still supports real multi-founder bootstrap
  for anyone who builds a client that prompts for it, this one just
  won't. `list_proposals`/`governance_state` are also O(every record in
  the namespace) per call — a caching or incremental-fold question once
  a namespace has more than a handful of proposals, not addressed here.
- **Polls** (`dist`'s "Polls" section, above Governance): a
  `GovernanceClass::General` Proposal, relabeled — `create-poll` and the
  poll list call the *exact same* `create_proposal`/`create_signal`/
  `list_proposals` commands as the full Governance section, hardcoded to
  `class: "general"` and filtered/relabeled client-side only (Support/
  Object instead of Consent/Block). No new Rust. Said outright in the
  hint text, not left implied: **this is not a majority vote** — same
  objection-window mechanic as everything else (passes by default unless
  enough people explicitly object; silence counts as support) — calling
  it "Polls" without that caveat would have implied ordinary
  vote-counting behavior the mechanism doesn't have.
- **Persistence** (`identity::Identity::load_or_generate`,
  `namespace::Node::spawn_persistent`, both new): real, not cosmetic.
  Found and fixed a genuine bug while wiring this, not something
  cosmetic: before this, `Node::spawn()` always generated a random
  `SecretKey` internally, completely disconnected from whatever
  `Identity` a caller displayed as a `did:iroh` — the DID shown to a
  person and the node's actual network identity were two unrelated
  keys. `spawn_persistent` threads `Identity`'s own key through to the
  `Endpoint`, so persisting the identity file actually persists the
  network identity, not just a label next to a different one each
  restart. Data lives under `atproto_iroh_core::paths::data_dir()`
  (`$ATPROTO_IROH_DATA_DIR` if set, else `$XDG_DATA_HOME/atproto-iroh` or
  `~/.local/share/atproto-iroh`) — an `identity` file (raw 32-byte
  secret, same protection any other private key file needs, not
  enforced by this code) and a `node/` directory holding the persistent
  docs and blobs stores. **At-rest encryption is the env var, not app
  code**: point `ATPROTO_IROH_DATA_DIR` at a volume the OS already
  encrypts (FileVault/BitLocker/LUKS/a mounted encrypted container) —
  deliberately not this crate's own crypto layer, since it doesn't need
  to own that security model, and "if the volume's unlocked, everything
  on it is" was the explicit call made choosing this over app-level
  encryption. `mute::MuteList::default_path` and
  `identity::Identity::default_path` honor the same override
  (`paths.rs`), so one setting covers all of it, not three separately.
  `spawn_node` also repopulates `AppState.docs`
  from `Node::list_local_namespaces` on every launch, so a restarted app
  can immediately act on namespaces from a previous session. Proven
  live, not just by compiling: `tests/persistence.rs` spawns a node,
  creates a namespace, writes a record, does a *real* shutdown, spawns a
  brand-new `Node` against the same identity file and data directory,
  and checks that the DID, the namespace, and the record all survived
  with nothing handed to the second node directly.
- `dist/` is plain HTML/CSS/JS — no bundler, no `node_modules`, no
  framework. `tauri.conf.json` sets `withGlobalTauri: true` so
  `window.__TAURI__` is injected directly; `main.js` calls `invoke()`
  against it with zero build step. Deliberate, not a placeholder waiting
  for a real frontend stack — see the styling philosophy note below.

## What's not built yet

This proves the wiring, not a usable app. Missing, in roughly the order
a real client would need them:

- ~~QR scanning.~~ **Re-ported (2026-09-23)** — `src/hooks/
  useQrScanner.ts`, the same `getUserMedia` + decode-loop approach as
  the old plain-JS app, now against the real `jsqr` npm package instead
  of a vendored file. Wired into the new **Join** screen
  (`src/screens/Join.tsx`, at `/join`) — camera-first with a real
  paste-a-ticket fallback, reachable from the Feed's empty state and
  a persistent "+ Join" entry in the "your people" strip. Camera
  access itself isn't testable in this sandbox (no real camera, and
  jsdom can't fake `getUserMedia`) or in headless Chromium screenshots
  — the paste-ticket path is fully tested instead
  (`Join.test.tsx`: valid ticket → lands on the new table, invalid
  ticket → real error, not a silent failure).
- **Every screen except Feed, Onboarding, Table detail, Join, Profile
  edit, and Mute** — see "Frontend rebuild," above, for the full list
  and why this is a deliberate, temporary step backward in feature
  coverage, not an oversight.
- **Any error/loading state beyond a basic "Settling in…"/empty-state
  message.** Better than the old app's raw `textContent = "error:
  ..."` but still not a real design pass on error states.
- **No automatic merge for concurrent doc revisions.** The Shared doc
  section flags the conflict and lets a person load either revision for
  review (see its own bullet above), but reconciling two edits into one
  is still on the person reading them and re-saving by hand — a real
  text-diff/merge tool was never in this session's scope.

## Building

**Verified, not just written.** `cargo build -p atproto-iroh-tauri`
compiles clean (zero warnings) as of this scaffold — Tauri's Linux
backend needs `webkit2gtk-4.1` and `gtk3` development headers available
to `pkg-config` at build time (`apt-get install libwebkit2gtk-4.1-dev
libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` on Debian/Ubuntu;
see [Tauri's prerequisites docs](https://tauri.app/start/prerequisites/)
for other platforms), which the container this was scaffolded in didn't
have until they were installed specifically to run this check — noted
because "it builds" should mean it was actually built, not assumed from
the source looking right. macOS/Windows ship their own webviews and
don't need this step. `icons/icon.png` is a flat placeholder color, not
real artwork — `tauri::generate_context!()` requires *some* icon to
exist at that path even with `bundle.active: false`, found by hitting
the error, not anticipated.

Actually running it (`cargo tauri dev`) needs `tauri-cli`
(`cargo install tauri-cli --version "^2"`) and, in a container/CI
context, a display — not attempted here, since a headless environment
has nothing to show a window on regardless of whether the binary itself
is correct.

**Frontend build, verified 2026-09-23**: `npm install && npm run build`
(from this directory) runs `tsc -b && vite build` and produces a real
`dist/` — confirmed by then running `cargo build -p atproto-iroh-tauri`
against it, clean. `npm test` runs the Vitest suite (10 tests as of this
writing) against the mock API and the canonical fixtures — see "Mock
dataset and tests," below. `npm run dev` starts a plain Vite dev server
on `localhost:1420` (Tauri's `beforeDevCommand`) — since it has no
`window.__TAURI__`, it automatically uses the mock backend, so the UI is
directly viewable in an ordinary browser without a real iroh node or a
GTK/WebKit build at all.

## Mock dataset and tests

`src/mocks/fixtures.ts` is the canonical mock dataset — Jason's explicit
ask alongside "go ahead and implement": one realistic, small multi-Table
world (three Tables, five members, messages, an image, a Decision, and
pins — including one author pinning twice, so the "latest wins" rule has
something real to prove itself against) used three ways: `src/api/
mockClient.ts` reads and mutates a copy of it so `npm run dev` in a
plain browser has real content to render with zero backend; every test
imports it directly for assertions; and it's the *same* content the
`/shape` mockup artifact used (Garden Table, Weekend Hikers, New Parents
Crew), so what a plain-browser `npm run dev` shows matches what was
designed, not a different placeholder set that happens to compile.

`src/lib/pins.ts` mirrors `atproto-iroh-core`'s `tagging::
latest_per_author_with_label`/`tagging::pins()` resolution in
TypeScript (the mock backend has no Rust process to call into) and is
unit-tested directly against the fixtures in `pins.test.ts` — the same
"one per author, most recent wins, newest first" property
`tests/tagging.rs`'s `tags_can_tag_tags_and_pins_resolve_one_per_
author_by_recency` proves on the Rust side, now proven independently on
the TS side too. `src/mocks/fixtures.test.ts` checks the dataset's own
internal consistency (every reference resolves to something real in it)
rather than trusting it stays coherent as it's edited. `src/screens/
Feed.test.tsx` is a real React Testing Library render test — proves the
Feed screen shows real dataset content and that clicking a Table filter
chip actually filters, not just that the component compiles.

**Known gap, not silently accepted**: `npm audit` reports 7 advisories
(5 moderate, 1 high, 1 critical) as of this writing, all in dev-only
tooling (`esbuild`'s dev-server request handling, `vitest`'s mocker,
`react-router`'s SSR/open-redirect surface) — none of which apply to
this app's actual runtime (a local Tauri desktop/mobile shell, no SSR,
no public dev server). `npm audit fix --force` would jump to Vite 8 /
Vitest 5 / React Router 7, major versions not checked for compatibility
here — deferred deliberately rather than force-upgraded blind mid an
already-large change.

## Styling philosophy

**Superseded 2026-09-23** — kept below for the historical record this
document's own practice calls for, not as current guidance. The old
`dist/styles.css` (deleted along with the rest of the plain-JS
frontend) really was intentionally undesigned; the confirmed `/shape`
design brief (`DESIGN_BRIEF.md`) is now this project's real design
direction — warm-dark, "a shared kitchen table, not a corporate
dashboard," specific typefaces and an OKLCH palette in `src/styles/
tokens.css`, not an absence of one.

Original framing, for the record: repo is MIT-licensed specifically so
a rougher-but-functional reference UI was a fine place to *start*
(Jason's framing, kept verbatim in spirit): *"I spent my time in the
trenches pushing pixels, I've earned some inconsistencies. If people
want pretty, they can make it pretty."* — which is exactly what this
rebuild is: someone (Jason, this session) deciding it was time to make
it pretty.
