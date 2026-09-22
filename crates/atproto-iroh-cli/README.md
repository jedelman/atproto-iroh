# atproto-iroh-cli

A Linux-first CLI (binary name `atproto-iroh`) for driving
`atproto-iroh-core` from a script or a local agent, without the Tauri
shell — Jason's ask (2026-09-22): "someone on Linux to be able to run a
local agent against this... probably just a cli or skill." See the root
`CLAUDE.md`'s federation-model section for why this matters beyond
convenience: an org's own always-on node is meant to be driven by
something other than a person clicking a GUI, and this is that thing.

## Model: one-shot processes, plus `serve`

Every command except `serve` spawns a node against on-disk state, does
one thing, and shuts down — no daemon, consistent with "lightweight is
best." This is fine for anything that only touches local state (`did`,
`create-namespace`, `write-text`, `dump`, the `doc-*` commands) since
that's all reading/writing a persistent store that survives between
invocations, proven the same way `tests/persistence.rs` proves it for the
core crate.

**It does not work for accepting an incoming connection**, and this was
found live, not anticipated: a `share` ticket bakes in the address of the
process that generated it (`iroh::Endpoint` binds a fresh port every
process start), so a one-shot `share` that prints a ticket and
immediately exits hands out a ticket to a port nothing is listening on
anymore. The first version of this CLI hit exactly that — two one-shot
processes (`share` then, later, `join`) produced a real two-peer `dump`
that came back empty, even with several seconds' grace period, because
the sharer had already exited before the joiner tried to connect.

`serve` is the fix and the actual point of this crate: it stays up
(reopens every local namespace, optionally prints a fresh `share` ticket
of its own before it starts listening, then blocks on Ctrl+C) so a
one-shot `join`/`dump` from anywhere else has something real to sync
against. **Verified live**, not just written: two real processes with
separate identities and data directories, one running `serve --share
<ns> --mode write` in the background, the other running `join <ticket>`
then `dump` — the joining side's dump showed the exact entry the serving
side had written, over a real QUIC connection between two OS processes on
this machine.

## Relay mode

`join` (once per namespace) then `serve`, no flag needed — this is
already the full "passive relay, no authoring identity" story the
Raspberry-Pi-as-sold-hardware idea (root `CLAUDE.md`'s federation-model
section) needs. Why it works: every entry here is self-authenticating
(signed by its original author, content-addressed), so a node that only
stores and forwards other people's entries never has to sign anything
itself, and capability is a bearer secret (SPEC.md §3.4) rather than
tied to who's holding it — a Read-mode ticket is enough for the relay to
receive full history and serve it to any other peer, no write access of
its own required. Confirmed live: `join`-ing a Read-mode ticket and
dumping the relay's own copy shows every entry from the ticket's issuer,
none from the relay's own (unused) author.

**One correction, found live while verifying this, not assumed clean**:
"no authoring identity" doesn't mean zero `AuthorId` ever touches disk.
`iroh-docs` itself creates and persists a default author unconditionally
inside `Docs::persistent(...).spawn(...)` — regardless of whether
application code ever asks for one — so even a pure `join`+`serve`
process ends up with an unused `default-author` file in its data
directory; there's no way to opt out of this through the public
`Docs::persistent()` builder this crate uses. What's still true, and is
the property that actually matters: `author()` (`main.rs`) is resolved
lazily, only by commands that write something, so a relay-mode process
never calls it — its unused key never signs an entry, never appears in
anything a peer receives, and never has to be trusted by anyone. Only
its network identity (`did:iroh`) is ever exposed, and only because it
has to be dialable at all.

## The control endpoint (relay mode's onboarding + poison pill)

`serve` now also spawns a raw QUIC control endpoint (`atproto-iroh-core`'s
`control.rs`) and binds with `NetworkPreset::N0` instead of `Minimal` —
the design resolved in conversation (2026-09-22), replacing an earlier
docs-namespace "inbox" idea; root `CLAUDE.md`'s federation-model section
has the full trail. Two operations, both plain text over a direct,
ephemeral connection to the box's bare `did:iroh` — nothing synced,
nothing durable, nothing a shared namespace could ever leak:

- **`JOIN <ticket>`** — hands the box a namespace ticket to import
  privately. No authorization beyond reachability: the ticket itself is
  already the real capability (SPEC.md §3.4's bearer-secret model), so
  this doesn't add a new trust requirement, just a private channel to
  deliver one.
- **`RESET <token>`** — the poison pill. Wipes the box's entire data
  directory (identity included) and signals `serve`'s own loop to exit,
  so a process supervisor restarts it fresh against a brand-new
  `did:iroh`. Requires the box's own `ResetToken` (printed once at
  `serve` startup) — reaching the box is not enough to destroy it.

Client side: `control-join <did> <ticket>` / `control-reset <did>
<token>` dial a remote box directly, no local capability into anything
the box holds required.

**Verified live in this sandbox, with a real caveat.** The protocol
logic itself (`JOIN`/`RESET` end to end, including a real data-directory
wipe and the reset signal firing) is proven live —
`atproto-iroh-core/tests/control.rs`'s two tests dial an explicit
`EndpointAddr` rather than a bare public key. **What's unverified here
specifically is bare-`did:iroh`-only dialing via `NetworkPreset::N0`'s
relay/DNS discovery** — tried directly with two real `atproto-iroh`
processes (`serve` in the background, then `control-reset <its did>
wrong-token` from a second process), and it hangs rather than
connecting or erroring, consistent with this sandbox having no real
outbound reachability to n0.computer's infrastructure. This is an
environment limitation, not a design or code defect — same honesty
pattern as the Android APK build gap in root `CLAUDE.md`. Worth
re-testing `control-join`/`control-reset` for real on a machine or two
network-separated boxes with genuine internet access.

## Commands

- `did` — print this CLI identity's `did:iroh`.
- `list` — every namespace this identity holds a capability into.
- `create-namespace` — creates one, prints its id.
- `share <namespace-id> [--mode read|write]` — prints a ticket. **Only
  useful if something is still listening on the address it names** — see
  above; prefer `serve --share` unless you know the process issuing this
  will stay alive.
- `join <ticket>` — imports a namespace from a ticket, prints its id.
  Waits a few seconds after import for initial sync before exiting — a
  heuristic grace period, not a guarantee; a script chaining `join` into
  an immediate `dump` against a large namespace should still expect to
  retry.
- `dump <namespace-id>` — every synced entry as JSON (the same reflective
  inspector `namespace::dump_all` gives the Tauri shell).
- `write-text <namespace-id> <key> <text>` / `read-text <namespace-id>
  <key>` — the freeform primitive. **Last-write-wins on that exact key**
  (SPEC.md's CRDT finding) — fine for single-writer content, wrong for
  anything more than one person might edit.
- `doc-save <namespace-id> <doc-id> <text>` — writes a new, immutable
  revision instead of overwriting a shared key; the safe primitive for
  anything more than one writer might touch.
- `doc-load <namespace-id> <doc-id>` — the latest revision by key order
  (a default, not a claim it's the "right" one if two edits landed
  close together).
- `doc-history <namespace-id> <doc-id>` — every revision, oldest first;
  use this to notice when two offline edits both survived instead of
  trusting `doc-load` picked the one you wanted.
- `serve [--share <namespace-id> [--mode read|write]]` — stays running
  and reachable until Ctrl+C or a valid remote `control-reset`. Reopens
  every namespace this identity already holds; `--share` additionally
  prints a fresh ticket for one namespace from this same live process
  before it starts listening. Also spawns the control endpoint and
  prints the box's `ResetToken` once at startup — see "The control
  endpoint," above.
- `control-join <did> <ticket>` — privately hands a namespace ticket to a
  remote box by its `did:iroh`, no local capability required.
- `control-reset <did> <token>` — the poison pill: remotely wipes a
  box's entire data directory given its `ResetToken`.
- `send <namespace-id> <text> [--reply-to <ref>]` — posts a message
  (`network.essmesh.chat.message`); the namespace is the channel, no
  separate room concept. Prints the new message's own
  `{author_hex}/{rkey}` ref, copy-pasteable into a later `send
  --reply-to`. Append-only by construction — unlike `write-text`, no
  last-write-wins risk, since every message gets its own key.
- `messages <namespace-id>` — every message, oldest first, each line
  prefixed with its own ref.
- `tag <namespace-id> <subject> <label>` — tags any record, in any
  lexicon (`network.essmesh.tag`). `subject` is
  `"{author_hex}/{collection}/{rkey}"` — `messages`'/`doc-history`'s
  output gives the `author_hex`/`rkey` half; for a message the
  collection is `network.essmesh.chat.message`.
- `tags <namespace-id> <subject>` — every tag on one subject.
- `mute <author-hex>` / `unmute <author-hex>` / `muted` — purely local,
  no sync, no lexicon. Stored under this CLI's own `cli-agent/` data
  directory, deliberately not `mute::MuteList::default_path()`'s shared
  convention — see `mute_path`'s doc comment in `main.rs` for why: this
  identity is a different `did:iroh` from the Tauri app's, so its mute
  preferences shouldn't silently share a file with a different
  identity's.
- `upload-image <namespace-id> <path> [--content-type <type>] [--caption <text>]`
  — reads a local file and uploads its bytes as `network.essmesh.chat.image`.
  No separate blob-fetch step, same as the Tauri shell's Images section —
  see `images.rs`'s own doc comment for why.
- `images <namespace-id>` — every image's metadata, oldest first.
- `download-image <namespace-id> <author-hex> <rkey> <out-path>` —
  writes one image's bytes to a local file. Verified as a real
  byte-for-byte round trip (upload a file, download it back, `diff`
  clean), not just "the types line up."

## Identity and storage

Deliberately **separate from the Tauri app's identity**, not shared:
`identity`/`docs`/`blobs` live under
`atproto_iroh_core::paths::data_dir().join("cli-agent")` (override with
`$ATPROTO_IROH_CLI_DATA_DIR` directly), so running this alongside a Tauri
instance on the same machine with the same `$ATPROTO_IROH_DATA_DIR` can't
collide on the same on-disk `redb` files — that would be a file-lock
error, not graceful shared access. Both still honor the same
`$ATPROTO_IROH_DATA_DIR` encrypted-volume override underneath, so pointing
that at an OS-encrypted mount covers this CLI's data the same way it
covers everything else (root `CLAUDE.md`'s at-rest-encryption note).

## Building and running

`cargo build -p atproto-iroh-cli` — no system dependencies beyond what
`atproto-iroh-core` itself needs (unlike the Tauri crate, nothing here
touches GTK/WebKit). Verified end to end in this sandbox: `did`,
`create-namespace`, `write-text`/`read-text`, `doc-save`/`doc-history`/
`doc-load`, `list`, and `dump` all run against real on-disk state and
produce correct output; `serve --share` + `join` + `dump` across two
separate identities/data directories was verified as a real two-process
QUIC sync, not assumed from the single-process command tests alone.

## Not built

- **No output format beyond human-readable text and `dump`'s JSON.** No
  `--json` flag on the other commands, no scripting-friendly exit-code
  conventions beyond "non-zero on error via `anyhow`."
- **No governance commands yet** (`propose`/`signal`/`governance-state`)
  — `fold.rs`'s API exists and is tested, just not wired into this crate.
  Would follow the same shape as the `doc-*` commands.
- **No profile commands.** `mute`/`tag`/messaging all have CLI parity
  with the Tauri shell now; `update_profile`/`list_profiles` don't —
  this CLI's `create-namespace` still doesn't publish a `NodeProfile` at
  all, unlike Tauri's `create_namespace_with_profile`.
- **Not packaged as a Claude Code skill.** Jason's ask named "a cli or
  skill" as alternatives; this is the CLI half. A skill wrapping these
  commands (so an agent invokes `/atproto-iroh ...` instead of shelling
  out directly) is straightforward once this CLI itself proves out, but
  wasn't built this session.
