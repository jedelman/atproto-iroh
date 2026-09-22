//! A Linux-first CLI so a local script or agent can drive
//! `atproto-iroh-core` without a GUI — see the root `CLAUDE.md`'s
//! federation-model section: an org's always-on lightweight node is
//! meant to be something other than a person clicking a Tauri window,
//! and this is that other thing. Each invocation is a fresh process:
//! spawn a persistent node against on-disk state, do one thing, shut
//! down. No daemon, no long-running process — "lightweight is best"
//! (Jason's framing) applies here too, not just to the mobile
//! background-execution question this was written alongside.
//!
//! **Deliberately a separate identity from the Tauri app.** Both bind to
//! the same `atproto_iroh_core::paths::data_dir()` convention (so the
//! same `$ATPROTO_IROH_DATA_DIR` encrypted-volume override covers both),
//! but under a distinct `cli-agent/` subdirectory — running this
//! alongside a Tauri app on the same machine with the same data dir would
//! otherwise mean two processes opening the same `redb` files at once,
//! which is a file-lock collision, not graceful concurrent access.
//! Override with `$ATPROTO_IROH_CLI_DATA_DIR` if a script wants a
//! specific location instead.
//!
//! **Relay mode is `join` + `serve`, and this code never asks for an
//! authoring identity to do it.** A node whose only job is passively
//! syncing/relaying already-signed entries never has to sign anything
//! itself — `join`/`dump`/`messages`/`tags`/`images`/`serve` and every
//! other read-only command never call `author()` below.
//!
//! **Correction, found live while verifying this, not assumed clean:**
//! that does *not* mean zero `AuthorId` ever touches disk. iroh-docs'
//! own `DefaultAuthor::load` (`engine.rs`) runs unconditionally inside
//! `Docs::persistent(...).spawn(...)` — "if the storage is empty
//! creates a new author and persists it" — independent of whether
//! application code ever asks for one, so `Node::spawn_persistent`
//! alone is enough to write a `default-author` file, confirmed by
//! running `join` on a fresh data directory and checking. There's no
//! way to opt out of this through the public `Docs::persistent()`
//! builder this crate uses (the lower-level `Engine::spawn` takes an
//! explicit author-storage choice, but nothing here calls it directly).
//! What *is* still true, and is the property that actually matters: an
//! `AuthorId` that's created but never used to call `put_record`/
//! `put_text`/`put_bytes` never appears in any synced entry, so no peer
//! ever sees it or has to trust it — "never signs anything, never
//! vouches for content it didn't write" holds regardless of whether an
//! unused key sits in local storage nobody else can see. The claim was
//! overstated before this comment; this is the accurate version.

use anyhow::{Context, Result};
use atproto_iroh_core::{
    fold,
    governance::{FoundingPolicy, PolicyValue},
    images, messaging,
    mute::MuteList,
    namespace::{
        decode_author_hex, dump_all, get_text, list_document_revisions, load_document, put_text,
        save_document_revision, NetworkPreset, Node,
    },
    records::{record_ref, Record},
    tagging,
};
use clap::{Parser, Subcommand};
use iroh_docs::{api::protocol::ShareMode, DocTicket, NamespaceId};

/// This CLI's own default starting policy for a namespace it founds —
/// same 24-hour/threshold-1 default the Tauri shell uses
/// (`default_founding_policy` there), kept in sync deliberately: both
/// are reference-client defaults, not a protocol constant (SPEC.md
/// §3.7.2 — namespace-owned, group-set state). A script that wants
/// different starting policy has to build its own `Founding` record for
/// now; no flag exposes this yet (README's "Not built" list).
fn default_founding_policy() -> FoundingPolicy {
    let day = PolicyValue { window_seconds: 86_400, block_threshold: 1 };
    FoundingPolicy {
        admit_co_signer: day,
        remove_co_signer: day,
        change_policy: day,
    }
}

#[derive(Parser)]
#[command(name = "atproto-iroh", about = "Drive an atproto-iroh node from the command line")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print this node's did:iroh, spawning/persisting identity if needed.
    Did,
    /// Every namespace this node currently holds a capability into.
    List,
    /// Create a new namespace; prints its id.
    CreateNamespace,
    /// Print a ticket for an existing namespace this node holds.
    Share {
        namespace_id: String,
        #[arg(long, value_enum, default_value = "write")]
        mode: Mode,
    },
    /// Join a namespace from a ticket string; prints its id.
    Join { ticket: String },
    /// Dump every entry in a namespace as JSON (the reflective inspector).
    Dump { namespace_id: String },
    /// Write freeform text at a fixed key (last-write-wins — see
    /// SPEC.md's CRDT note; prefer `doc-save` for anything more than one
    /// writer).
    WriteText {
        namespace_id: String,
        key: String,
        text: String,
    },
    /// Read freeform text back from a fixed key.
    ReadText { namespace_id: String, key: String },
    /// Save a new revision of a document — safe under concurrent offline
    /// edits, unlike `write-text` (SPEC.md's CRDT finding).
    DocSave {
        namespace_id: String,
        doc_id: String,
        text: String,
    },
    /// The latest revision of a document by key order (a default, not
    /// necessarily the "right" one if two edits landed close together —
    /// see `doc-history`).
    DocLoad { namespace_id: String, doc_id: String },
    /// Every revision of a document, oldest first — use this to notice
    /// when two offline edits both landed instead of trusting `doc-load`
    /// picked the one you wanted.
    DocHistory { namespace_id: String, doc_id: String },
    /// Stay running and reachable until interrupted — every other
    /// command is a one-shot process that spawns, acts, and shuts the
    /// node down, which is fine for local reads/writes but means nothing
    /// else can ever dial *in* to it. Real P2P sync needs at least one
    /// side of a `share`/`join` pair to actually be listening when the
    /// other connects — an ephemeral `share` process that's already
    /// exited by the time a peer tries to `join` its ticket has nothing
    /// left to dial (found live: a two-one-shot-process test produced an
    /// empty `dump` on the joining side, because the sharer had already
    /// shut down). This is the concrete shape of "an org stands up its
    /// own lightweight always-on node" from CLAUDE.md's federation-model
    /// section — run `serve` on that node, and every other peer's
    /// one-shot `share`/`join`/`dump` calls have something real to sync
    /// against.
    ///
    /// `--share`/`--mode` print a fresh ticket for a namespace from
    /// *this* running process before it starts listening — required, not
    /// optional, for a namespace nobody's joined yet: `share` on its own
    /// is a one-shot process that binds a random port, prints a ticket
    /// naming that port, and immediately exits, so the ticket points at
    /// a port nothing is listening on by the time anyone tries to join
    /// it (found live — a `share` ticket handed to a `serve` process
    /// started afterward produced an empty `dump` on the joining side,
    /// because the address in the ticket belonged to the already-dead
    /// `share` process, not to `serve`'s own, differently-bound port).
    /// `serve --share` sidesteps that by only ever printing a ticket for
    /// the address it's actually listening on.
    ///
    /// **This is also relay mode, with no flag needed.** `serve` never
    /// calls `author()` — it only reopens namespaces this node already
    /// holds a capability into (via prior `join` calls) and answers
    /// sync requests for them. A box meant to be a pure relay
    /// (CLAUDE.md's federation-model section: the
    /// Raspberry-Pi-as-sold-hardware idea) is provisioned by running
    /// `join <ticket>` once per namespace it should hold, then `serve`
    /// indefinitely. It still has an unused `default-author` sitting in
    /// its local docs store — iroh-docs creates one unconditionally on
    /// spawn regardless of application code (`main.rs`'s top comment has
    /// the live finding) — but nothing ever signs with it, so no peer
    /// ever sees or has to trust it; only its network identity
    /// (`did:iroh`) is ever exposed, needed to be dialable at all, never
    /// used to vouch for content it didn't write.
    /// **Serve now also spawns the control endpoint (`control.rs`) and
    /// binds with `NetworkPreset::N0`, not `Minimal`** — a relay box is
    /// exactly the case CLAUDE.md's federation-model section means by
    /// "the org's always-on node needs to be reachable from anywhere,"
    /// which needs the relay/DNS discovery `N0` adds; every other
    /// one-shot command still defaults to `Minimal` via
    /// `spawn_persistent`, unchanged. Prints the box's `ResetToken` once
    /// at startup — save it, it's the only proof accepted by
    /// `control-reset` (the "poison pill"). Exits either on Ctrl+C or on
    /// a valid remote `RESET`, in which case a process supervisor
    /// (systemd unit, restart loop, Docker restart policy — none set up
    /// by this repo) is expected to start this process again fresh
    /// against a brand-new identity, since `RESET` wipes the whole data
    /// directory including it.
    Serve {
        #[arg(long)]
        share: Option<String>,
        #[arg(long, value_enum, default_value = "write")]
        mode: Mode,
    },
    /// Client side of the control endpoint: privately hand a namespace
    /// ticket to a remote box by its bare `did:iroh` (or `did:iroh:...`),
    /// no capability of the sender's own required to reach it — see
    /// `control.rs`'s module doc for why `JOIN` needs no authorization
    /// beyond "you can reach this address."
    ///
    /// **Bare-`did:iroh`-only dialing (resolved via `NetworkPreset::N0`'s
    /// relay/DNS discovery) is unverified in this dev sandbox** — tried
    /// live here (two local `atproto-iroh` processes, one `serve`, one
    /// `control-reset` against its printed did), and it hangs rather than
    /// connecting, consistent with `tests/control.rs`'s finding that this
    /// sandbox has no real outbound reachability to n0.computer's
    /// infrastructure. The underlying `JOIN`/`RESET` protocol logic
    /// itself *is* proven live (`tests/control.rs`, dialing an explicit
    /// `EndpointAddr`) — what's unverified is specifically the
    /// "just the DID, nothing else" production path this command takes,
    /// same honesty pattern as the Android APK build gap.
    ControlJoin { did: String, ticket: String },
    /// Client side of the poison pill: remotely wipe a box's entire data
    /// directory (identity included), proven with its `ResetToken` — only
    /// whoever set the box up or was handed the token can do this. Same
    /// bare-DID-dialing caveat as `control-join`, above.
    ControlReset { did: String, token: String },
    /// Send a message into a namespace — the namespace is the channel,
    /// no separate room concept. `--reply-to` takes
    /// "{author_hex}/{rkey}" (as printed by `messages`) to reply to an
    /// existing message.
    Send {
        namespace_id: String,
        text: String,
        #[arg(long)]
        reply_to: Option<String>,
    },
    /// Every message in a namespace, oldest first.
    Messages { namespace_id: String },
    /// Tags a record anywhere in the namespace — `subject` is
    /// "{author_hex}/{collection}/{rkey}" (`messages`'/`doc-history`'s
    /// output gives you the author_hex/rkey half; the collection for a
    /// message is `network.essmesh.chat.message`). One `Tag` type works
    /// across every record type, in any lexicon.
    Tag {
        namespace_id: String,
        subject: String,
        label: String,
    },
    /// Every tag on one subject.
    Tags { namespace_id: String, subject: String },
    /// Mutes an author's content in this identity's own client — local
    /// only, no sync, no lexicon (`mute::MuteList`'s doc comment).
    Mute { author_hex: String },
    Unmute { author_hex: String },
    /// Every currently-muted author.
    Muted,
    /// Uploads a local file as an image — bytes sync as an ordinary
    /// namespace entry (`namespace::put_bytes`), not through a separate
    /// blob-fetch step.
    UploadImage {
        namespace_id: String,
        path: std::path::PathBuf,
        #[arg(long, default_value = "application/octet-stream")]
        content_type: String,
        #[arg(long)]
        caption: Option<String>,
    },
    /// Every image's metadata in a namespace, oldest first.
    Images { namespace_id: String },
    /// Downloads one image's bytes to a local file.
    DownloadImage {
        namespace_id: String,
        author_hex: String,
        rkey: String,
        out_path: std::path::PathBuf,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum Mode {
    Read,
    Write,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let identity = atproto_iroh_core::identity::Identity::load_or_generate(identity_path())
        .context("loading/generating CLI identity")?;

    // `Serve` alone spawns the control endpoint and binds N0 (this
    // file's own doc comment on `Command::Serve` has why); `control-join`/
    // `control-reset` also need N0 since they're specifically about
    // dialing a box that may not be on the same LAN. Every other
    // command stays on plain `spawn_persistent` (Minimal), unchanged.
    let needs_relay_network = matches!(
        cli.command,
        Command::Serve { .. } | Command::ControlJoin { .. } | Command::ControlReset { .. }
    );

    if let Command::Serve { share, mode } = &cli.command {
        return serve(&identity, share.as_deref(), mode).await;
    }

    let node = if needs_relay_network {
        Node::spawn_persistent_with_preset(&identity, cli_data_dir(), NetworkPreset::N0)
            .await
            .context("spawning node")?
    } else {
        Node::spawn_persistent(&identity, cli_data_dir())
            .await
            .context("spawning persistent node")?
    };

    let output = run(&cli.command, &node, &identity).await;

    node.shutdown().await;
    output?;
    Ok(())
}

/// `Serve`'s own entry point, separate from `run()`: it needs
/// `Node::spawn_relay` (a different return shape — node plus reset token
/// plus reset signal) and a `select!` on exit that no other command
/// needs, so it doesn't fit `run()`'s one-node-in, one-output-out shape.
async fn serve(
    identity: &atproto_iroh_core::identity::Identity,
    share: Option<&str>,
    mode: &Mode,
) -> Result<()> {
    let (node, reset_token, reset_signal) =
        Node::spawn_relay(identity, cli_data_dir(), NetworkPreset::N0)
            .await
            .context("spawning relay node")?;

    let namespaces = node.list_local_namespaces().await?;
    for (id, capability) in &namespaces {
        node.open_namespace(*id).await?;
        println!("serving {id} ({capability:?})");
    }
    if let Some(namespace_id) = share {
        let doc = open(&node, namespace_id).await?;
        let share_mode = match mode {
            Mode::Read => ShareMode::Read,
            Mode::Write => ShareMode::Write,
        };
        let ticket = node.share(&doc, share_mode).await?;
        println!("ticket: {ticket}");
    }
    println!("did: {}", identity.did());
    println!("reset token (save this — required to remotely wipe this box): {}", reset_token.as_str());
    println!("listening — Ctrl+C to stop, or a remote control-reset with the token above");

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            result?;
            node.shutdown().await;
        }
        () = reset_signal.notified() => {
            println!("received a valid RESET — data directory wiped, exiting for a supervisor to restart fresh");
            node.shutdown().await;
        }
    }
    Ok(())
}

/// Accepts a bare `PublicKey`'s display form or a `did:iroh:<...>` string
/// — `control-join`/`control-reset` take whichever a user has on hand
/// (the box's `did` command prints the `did:iroh:` form).
fn parse_did(did: &str) -> Result<iroh::PublicKey> {
    let key_str = did.strip_prefix("did:iroh:").unwrap_or(did);
    key_str.parse().with_context(|| format!("invalid did:iroh / public key: {did}"))
}

/// Resolves this node's default authoring identity — lazily, only when a
/// command actually needs one, instead of the old unconditional call in
/// `main()`. **What this does and doesn't buy** (see `main.rs`'s own top
/// doc comment for the live finding this correction is based on):
/// `DocsApi::author_default` isn't the only thing that can create an
/// `AuthorId` — `Docs::persistent(...).spawn(...)` does it regardless,
/// so a relay-mode node (`join` + `serve`, never calling this function)
/// still has one sitting on disk. What laziness here *does* guarantee:
/// no command that only reads/relays ever calls `put_record`/`put_text`/
/// `put_bytes` under that key, so it never signs anything and never
/// appears in any entry a peer receives — the property that actually
/// matters (nothing to trust the relay's authorship of, because it
/// never claims any) holds either way.
async fn author(node: &Node) -> Result<iroh_docs::AuthorId> {
    Ok(node.docs().author_default().await?)
}

async fn run(
    command: &Command,
    node: &Node,
    identity: &atproto_iroh_core::identity::Identity,
) -> Result<()> {
    match command {
        Command::Did => {
            println!("{}", identity.did());
        }
        Command::List => {
            let namespaces = node.list_local_namespaces().await?;
            for (id, capability) in namespaces {
                println!("{id} {capability:?}");
            }
        }
        Command::CreateNamespace => {
            let author = author(node).await?;
            let doc = node.create_namespace().await?;
            // Posts this identity's Founding claim (SPEC.md §6) right
            // after creation, before anything is ever shared — the real
            // bootstrap path `fold::fold_namespace` needs to resolve
            // genesis state instead of returning an empty eligible set.
            fold::found_namespace(&doc, author, vec![author], default_founding_policy()).await?;
            println!("{}", doc.id());
        }
        Command::Share { namespace_id, mode } => {
            let doc = open(node, namespace_id).await?;
            let share_mode = match mode {
                Mode::Read => ShareMode::Read,
                Mode::Write => ShareMode::Write,
            };
            let ticket = node.share(&doc, share_mode).await?;
            println!("{ticket}");
        }
        Command::Join { ticket } => {
            let ticket: DocTicket = ticket.parse().context("invalid ticket")?;
            let doc = node.join(ticket).await?;
            // Found running this live, not anticipated: this process is
            // about to exit and shut the node down right after this
            // function returns, but sync (QUIC handshake, gossip,
            // pulling existing history) takes real wall-clock time — a
            // `join` that shuts down immediately can exit before a
            // single entry has actually synced, leaving nothing on disk
            // for the *next* invocation to `dump`. A fixed grace period
            // is a heuristic, not a guarantee — a script issuing `join`
            // immediately followed by `dump` against a large namespace
            // should still expect to retry, the same way
            // `tests/namespace_records.rs`'s live tests do.
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            println!("{}", doc.id());
        }
        Command::Dump { namespace_id } => {
            let doc = open(node, namespace_id).await?;
            let entries = dump_all(node, &doc).await?;
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        Command::WriteText {
            namespace_id,
            key,
            text,
        } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            put_text(&doc, author, key, text).await?;
        }
        Command::ReadText { namespace_id, key } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            match get_text(node, &doc, author, key).await? {
                Some(text) => println!("{text}"),
                None => eprintln!("(no entry at {key:?} for this node's author)"),
            }
        }
        Command::DocSave {
            namespace_id,
            doc_id,
            text,
        } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            let rev = save_document_revision(&doc, author, doc_id, text).await?;
            println!("{rev}");
        }
        Command::DocLoad {
            namespace_id,
            doc_id,
        } => {
            let doc = open(node, namespace_id).await?;
            match load_document(node, &doc, doc_id).await? {
                Some((rev, author, text)) => {
                    println!("# revision {rev} by {}", hex::encode(author.as_bytes()));
                    println!("{text}");
                }
                None => eprintln!("(no revisions of {doc_id:?} yet)"),
            }
        }
        Command::DocHistory {
            namespace_id,
            doc_id,
        } => {
            let doc = open(node, namespace_id).await?;
            let revisions = list_document_revisions(node, &doc, doc_id).await?;
            for (author, rev, text) in revisions {
                println!("{rev} {} {text}", hex::encode(author.as_bytes()));
            }
        }
        Command::Send { namespace_id, text, reply_to } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            let rkey = messaging::send_message(&doc, author, text.clone(), reply_to.clone()).await?;
            println!("{}/{rkey}", hex::encode(author.as_bytes()));
        }
        Command::Messages { namespace_id } => {
            let doc = open(node, namespace_id).await?;
            let messages = messaging::list_messages(node, &doc).await?;
            for (msg_author, rkey, message) in messages {
                let author_hex = hex::encode(msg_author.as_bytes());
                let reply_note = message
                    .reply_to
                    .map(|r| format!(" (reply to {r})"))
                    .unwrap_or_default();
                // First field is this message's own ref
                // ("{author_hex}/{rkey}"), copy-pasteable straight into
                // `send --reply-to`.
                println!("{author_hex}/{rkey}  {author_hex}: {}{reply_note}", message.text);
            }
        }
        Command::Tag { namespace_id, subject, label } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            let rkey = tagging::add_tag(&doc, author, subject.clone(), label.clone()).await?;
            println!(
                "{}",
                record_ref(&hex::encode(author.as_bytes()), tagging::Tag::COLLECTION, &rkey)
            );
        }
        Command::Tags { namespace_id, subject } => {
            let doc = open(node, namespace_id).await?;
            let tags = tagging::tags_for(node, &doc, subject).await?;
            for (tag_author, rkey, tag) in tags {
                println!("{}/{rkey}  {}", hex::encode(tag_author.as_bytes()), tag.label);
            }
        }
        Command::Mute { author_hex } => {
            let target = decode_author_hex(author_hex).context("invalid author id")?;
            let mut list: MuteList<iroh_docs::AuthorId> = MuteList::load(mute_path())?;
            list.mute(target);
            list.save(mute_path())?;
        }
        Command::Unmute { author_hex } => {
            let target = decode_author_hex(author_hex).context("invalid author id")?;
            let mut list: MuteList<iroh_docs::AuthorId> = MuteList::load(mute_path())?;
            list.unmute(&target);
            list.save(mute_path())?;
        }
        Command::Muted => {
            let list: MuteList<iroh_docs::AuthorId> = MuteList::load(mute_path())?;
            for author in list.iter() {
                println!("{}", hex::encode(author.as_bytes()));
            }
        }
        Command::UploadImage { namespace_id, path, content_type, caption } => {
            let author = author(node).await?;
            let doc = open(node, namespace_id).await?;
            let bytes = std::fs::read(path)
                .with_context(|| format!("reading {}", path.display()))?;
            let rkey = images::upload_image(&doc, author, bytes, content_type.clone(), caption.clone())
                .await?;
            println!("{rkey}");
        }
        Command::Images { namespace_id } => {
            let doc = open(node, namespace_id).await?;
            let list = images::list_images(node, &doc).await?;
            for (img_author, rkey, meta) in list {
                let caption = meta.caption.as_deref().unwrap_or("(no caption)");
                println!(
                    "{}/{rkey}  {} bytes, {}  {caption}",
                    hex::encode(img_author.as_bytes()),
                    meta.len,
                    meta.content_type
                );
            }
        }
        Command::DownloadImage { namespace_id, author_hex, rkey, out_path } => {
            let doc = open(node, namespace_id).await?;
            let img_author = decode_author_hex(author_hex).context("invalid author id")?;
            match images::load_image_bytes(node, &doc, img_author, rkey).await? {
                Some(bytes) => {
                    std::fs::write(out_path, &bytes)
                        .with_context(|| format!("writing {}", out_path.display()))?;
                    println!("wrote {} bytes to {}", bytes.len(), out_path.display());
                }
                None => eprintln!("(image bytes not found — not synced yet, or wrong ref)"),
            }
        }
        Command::Serve { .. } => {
            // Handled entirely by `serve()` in `main()` before `run()` is
            // ever called — it needs `Node::spawn_relay`'s different
            // return shape (node + reset token + reset signal) and a
            // `select!` on exit, neither of which fits this function's
            // one-node-in shape. Unreachable in practice.
            unreachable!("Serve is dispatched to serve() before run() is called")
        }
        Command::ControlJoin { did, ticket } => {
            let target = parse_did(did)?;
            let response = node.send_control(target, &format!("JOIN {ticket}")).await?;
            println!("{response}");
        }
        Command::ControlReset { did, token } => {
            let target = parse_did(did)?;
            let response = node.send_control(target, &format!("RESET {token}")).await?;
            println!("{response}");
        }
    }
    Ok(())
}

/// Opens a namespace by its printed id — the CLI's own process holds
/// nothing in memory between invocations, so every command that acts on
/// an existing namespace starts from this instead of an `AppState`-style
/// cache the way the Tauri client can.
async fn open(node: &Node, namespace_id: &str) -> Result<iroh_docs::api::Doc> {
    let id: NamespaceId = namespace_id.parse().context("invalid namespace id")?;
    node.open_namespace(id)
        .await?
        .with_context(|| format!("no local capability into namespace {namespace_id}"))
}

fn cli_data_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("ATPROTO_IROH_CLI_DATA_DIR") {
        return std::path::PathBuf::from(dir);
    }
    atproto_iroh_core::paths::data_dir().join("cli-agent")
}

fn identity_path() -> std::path::PathBuf {
    cli_data_dir().join("identity")
}

/// Deliberately not `mute::MuteList::default_path()` — that resolves
/// under the shared `paths::data_dir()` convention, the same place the
/// Tauri app's own mute list lives, but this CLI already keeps its own
/// identity and node storage under a separate `cli-agent/` subdirectory
/// (this file's own top note on why) to avoid exactly this kind of
/// cross-client collision. A local preference file isn't a file-lock
/// hazard the way the docs/blobs stores are, but the CLI's own identity
/// is a different `did:iroh` from the Tauri app's, so its mute
/// preferences shouldn't silently share a file with a different
/// identity's either.
fn mute_path() -> std::path::PathBuf {
    cli_data_dir().join("mute.json")
}
