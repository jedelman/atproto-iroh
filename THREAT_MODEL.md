# Threat model

What atproto-iroh protects against, what it doesn't, and what it is not
for. Written 2026-10-01 in response to a fair public critique ("you have
no moderation tooling — do not release it"). Every claim below is checked
against the code on this branch or against `SPEC.md`, with the section or
file cited. If a claim stops being true, fix this file in the same commit
that changes the behavior.

**Status: unaudited lab build.** No one outside this project has reviewed
the code, the protocol, or this document. Do not rely on it where being
wrong could get someone hurt.

## What this is for

Small groups whose members already know each other: a household, a
co-op, a crew, a reading group. A group is a *Table* (one iroh-docs
namespace plus a governance collection). People join by invitation. There
is no public index, no firehose, and no way to find a Table you weren't
given.

## What this is not for

**A public social network.** Nothing here moderates strangers. There are
no reports, no labelers, no moderation team, and no way to add one that
would hold up against a network full of people who don't know each
other. The critique that started this document is correct on that point.
The release posture below exists to keep the software from drifting into
that shape.

## Who can see what

| Party | What they see |
|---|---|
| Members of a Table | Everything ever written to it, including history from before they joined (`SPEC.md` §6 item 10: joining backfills full history). |
| Anyone holding a ticket | Same as a member. A ticket is the whole credential — see "Invitations" below. |
| Other members' devices | Your IP address(es), whenever your device connects to theirs directly. That is what peer-to-peer means. |
| n0.computer (on phones) | Connection metadata, not content. The mobile app binds `NetworkPreset::N0` (`crates/atproto-iroh-tauri/src-tauri/src/lib.rs`, `network_preset()`), which uses n0's public relays and address lookup when a direct connection fails. As we understand iroh, relayed traffic stays end-to-end encrypted; n0 can still see which nodes talk to which, when, and from where. Desktop builds bind `Minimal` and use no n0 infrastructure. |
| A relay container | Everything in every Table it joined, in plaintext, on its disk. A relay is a full member that never sleeps (`Node::spawn_relay`, `control.rs`). |
| Someone with your unlocked device | Everything. There is no app-level encryption at rest. Protection depends on the OS: Android encrypts app storage by default; on desktop, point `ATPROTO_IROH_DATA_DIR` at an encrypted volume (`CLAUDE.md`, "At-rest encryption"). |

## Invitations

An invite is an iroh-docs `DocTicket`. Two facts follow from how the code
creates one:

1. **A ticket can be forwarded.** Whoever holds it can join. There's no
   check that the person using the ticket is the person it was given to
   (`SPEC.md` §3.4: read access is "knowledge plus reachability", not a
   credential). An invite-only Table is only as closed as its least
   careful member.
2. **A ticket carries the issuer's network addresses.** `Node::share`
   calls `doc.share(mode, AddrInfoOptions::Addresses)`
   (`crates/atproto-iroh-core/src/namespace.rs`), so the ticket, and the
   QR code drawn from it, includes the inviting device's current IP
   addresses. Anyone who sees the QR code learns them.

Write tickets are worse than read tickets: every writer holds the same
namespace secret (`SPEC.md` §3.4).

## Removal and what it does not do

This is the most important section. In a private group, the likely
attacker is not a stranger. It is a member who turns.

- **Mute** (`mute.rs`) is local. It hides someone from you. Nobody else
  is affected, and the muted person can still read and write.
- **Removal** (`GovernanceClass::RemoveCoSigner`, ratified through the
  objection window in `SPEC.md` §3.7) is a group decision that **honest
  clients honor by convention**. It is not cryptographic. A removed
  member keeps the namespace secret, can keep writing, and keeps
  everything they already synced. Good-faith clients stop counting their
  writes (`SPEC.md` §3.4, §3.7.7).
- **There is no revocation.** Nothing can take back data someone has
  already received, and nothing can stop a removed member from reading
  new writes while they still hold the ticket.

The real remedy is **fission**: start a new Table, carry over the
records you want, and invite everyone except the person you're leaving
behind (`SPEC.md` §3.8). Today that is a manual process. Until it's a
one-step action that tells you plainly what the excluded person keeps,
treat removal as advisory.

## Abuse inside a Table

What exists: local mute; signed, attributable writes (every entry is
signed by its author key, so nobody can forge another member's
signature; display names are self-declared, though, so someone can still
pick a name that looks like someone else's);
ratified removal by convention; fission.

What doesn't exist yet: any way to report, a way to hide content for the
whole group, size limits, or rate limits.

The model for moderation here is not a trust-and-safety team. It is
Elinor Ostrom's design principles for commons that last: members monitor
their own group, sanctions are graduated (mute, then objection, then
removal), conflict resolution is cheap, and exit is always possible. The
governance layer has most of the primitives. The product work to make
them usable is not done.

## Hosting and illegal content

Every member's device stores and re-serves what the Table contains,
including images. A relay container does too, in plaintext. If you run a
relay for a Table, you are hosting that Table's content. The project
maintainer does not run relays for other people and won't until the
legal side has had real advice. If you run your own, run it only for
groups you are in.

The relay control endpoint will join any Table whose ticket it's handed,
with no check beyond reachability (`control.rs`: "JOIN needs no
authorization beyond 'you can reach this address'"). Don't expose a
relay's address to people you wouldn't hand the ticket to.

## Release posture

Until each item is done, the release is a lab build:

- [ ] Small, invite-only Tables only. **No public discovery or federation
      in the release build:** not opt-in, absent.
- [ ] A size cap per Table (the number is a maintainer decision; tens,
      not hundreds).
- [ ] "Re-found this Table without X" as one action, with an honest
      statement of what X keeps.
- [ ] Invite QR codes either drop direct addresses or warn that they
      include them.
- [ ] This document linked from the app's onboarding, not just the repo.
- [ ] No hosted relays for strangers.
- [ ] An outside review of the code and this document.

## Known unknowns

- Not verified on real devices yet: APK install, real-camera QR scanning,
  and behavior on networks behind CGNAT (`CLAUDE.md`).
- What n0's relays and address lookup log, and for how long. We haven't
  checked n0's own policies.
- Whether iroh-docs sync leaks metadata (namespace IDs, timing) to
  non-members on the same network.
