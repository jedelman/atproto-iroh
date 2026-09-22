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
  and reachable until Ctrl+C. Reopens every namespace this identity
  already holds; `--share` additionally prints a fresh ticket for one
  namespace from this same live process before it starts listening.
- `send <namespace-id> <text> [--reply-to <ref>]` — posts a message
  (`network.essmesh.chat.message`); the namespace is the channel, no
  separate room concept. Prints the new message's own
  `{author_hex}/{rkey}` ref, copy-pasteable into a later `send
  --reply-to`. Append-only by construction — unlike `write-text`, no
  last-write-wins risk, since every message gets its own key.
- `messages <namespace-id>` — every message, oldest first, each line
  prefixed with its own ref.

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
- **No mute-list commands.** `mute::MuteList` exists and is tested,
  unused here, same gap the Tauri README already names for its own UI.
- **Not packaged as a Claude Code skill.** Jason's ask named "a cli or
  skill" as alternatives; this is the CLI half. A skill wrapping these
  commands (so an agent invokes `/atproto-iroh ...` instead of shelling
  out directly) is straightforward once this CLI itself proves out, but
  wasn't built this session.
