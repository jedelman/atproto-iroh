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

use anyhow::{Context, Result};
use atproto_iroh_core::{
    fold,
    governance::{FoundingPolicy, PolicyValue},
    messaging,
    namespace::{
        dump_all, get_text, list_document_revisions, load_document, put_text,
        save_document_revision, Node,
    },
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
    Serve {
        #[arg(long)]
        share: Option<String>,
        #[arg(long, value_enum, default_value = "write")]
        mode: Mode,
    },
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
    let node = Node::spawn_persistent(&identity, cli_data_dir())
        .await
        .context("spawning persistent node")?;
    let author = node.docs().author_default().await?;

    let output = run(&cli.command, &node, author, &identity).await;

    node.shutdown().await;
    output?;
    Ok(())
}

async fn run(
    command: &Command,
    node: &Node,
    author: iroh_docs::AuthorId,
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
            let doc = open(node, namespace_id).await?;
            put_text(&doc, author, key, text).await?;
        }
        Command::ReadText { namespace_id, key } => {
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
        Command::Serve { share, mode } => {
            // Reopen every namespace this node already holds a capability
            // into — same reason `spawn_node` does this in the Tauri
            // shell (`main.rs`'s own doc comment there): a fresh process
            // remembers nothing in memory, only the persistent store
            // does.
            let namespaces = node.list_local_namespaces().await?;
            for (id, capability) in &namespaces {
                node.open_namespace(*id).await?;
                println!("serving {id} ({capability:?})");
            }
            if let Some(namespace_id) = share {
                let doc = open(node, namespace_id).await?;
                let share_mode = match mode {
                    Mode::Read => ShareMode::Read,
                    Mode::Write => ShareMode::Write,
                };
                let ticket = node.share(&doc, share_mode).await?;
                println!("ticket: {ticket}");
            }
            println!("did: {}", identity.did());
            println!("listening — Ctrl+C to stop");
            tokio::signal::ctrl_c().await?;
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
