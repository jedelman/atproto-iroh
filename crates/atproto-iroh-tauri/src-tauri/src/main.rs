// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Reference Tauri client for `atproto-iroh-core` — CLAUDE.md's "Client"
//! section, scaffolded. Thin on purpose: every command here is a direct
//! call into the core crate, no protocol logic duplicated or
//! reimplemented at this layer. See `../README.md` for what's real here
//! and what's still a placeholder.

use std::collections::HashMap;

use std::collections::HashSet;

use atproto_iroh_core::{
    fold::{self, GovernanceState},
    governance::{GovernanceClass, PolicyChange, PolicyValue, Proposal, Ratification, Signal, SignalType},
    identity::Identity,
    namespace::{decode_author_hex, dump_all, get_text, list_records, put_record, put_text, submit_text, Node, RawEntry},
    records::{NodeCategory, NodeProfile},
};
use chrono::Duration as ChronoDuration;
use iroh_docs::{api::protocol::ShareMode, api::Doc, AuthorId, DocTicket};
use tauri::State;
use tokio::sync::Mutex;

/// Tauri-managed app state. Deliberately empty until a command asks for
/// it — spawning a real iroh endpoint and gossip swarm at app launch,
/// before the person using it has done anything, would be exactly the
/// kind of ambient background activity SPEC.md's goal 1 (zero ambient
/// legibility) argues against, extended to the client's own behavior on
/// its own machine, not just what it exposes to the network.
///
/// `docs` is keyed by `NamespaceId`'s string form (`Doc::id().to_string()`)
/// — every namespace this node currently holds a capability into, so a
/// later command (share, publish, governance) can name one without
/// re-deriving it from a ticket each time. Backed by a real persistent
/// node now (`Node::spawn_persistent`) — `spawn_node` repopulates this
/// map from disk on every launch (`Node::list_local_namespaces`), so
/// what's here after startup isn't "whatever's been created this
/// session," it's everything this node has ever been granted into.
#[derive(Default)]
struct AppState {
    identity: Mutex<Option<Identity>>,
    node: Mutex<Option<Node>>,
    docs: Mutex<HashMap<String, Doc>>,
    /// This node's default record-signing author — `DocsApi::author_default`,
    /// not a fresh one minted per call. On a persistent node this is the
    /// same author every restart (iroh-docs' own `DefaultAuthorStorage::
    /// Persistent`, found while wiring this rather than built by hand —
    /// no reason to duplicate author persistence this crate already
    /// gets for free). Distinct from `identity` (the node's own
    /// `did:iroh`) for the reason `namespace.rs`'s own doc comment
    /// gives: an `Author` signs entries, it isn't the node's identity.
    author: Mutex<Option<AuthorId>>,
}

async fn ensure_author(state: &AppState) -> Result<AuthorId, String> {
    let mut author = state.author.lock().await;
    if let Some(author) = *author {
        return Ok(author);
    }
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let created = node.docs().author_default().await.map_err(|e| e.to_string())?;
    *author = Some(created);
    Ok(created)
}

/// `~/.local/share/atproto-iroh` (or `$XDG_DATA_HOME/atproto-iroh`) —
/// same convention `mute::MuteList::default_path` already established,
/// duplicated rather than imported: it's three lines, and pulling in a
/// real `dirs` crate for one path is what that function's own comment
/// already argued against.
fn data_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("atproto-iroh")
}

/// Spawns the node against real persistent storage and loads (or
/// generates, on first run) this machine's `did:iroh` identity —
/// `Identity::load_or_generate`/`Node::spawn_persistent`, both new. Also
/// repopulates `AppState.docs` from whatever `Node::list_local_namespaces`
/// finds already on disk, so a restarted app can immediately share,
/// inspect, or govern namespaces from a previous session without anyone
/// re-pasting a ticket.
#[tauri::command]
async fn spawn_node(state: State<'_, AppState>) -> Result<String, String> {
    let did = {
        let mut identity = state.identity.lock().await;
        if identity.is_none() {
            let path = data_dir().join("identity");
            *identity = Some(Identity::load_or_generate(path).map_err(|e| e.to_string())?);
        }
        identity.as_ref().expect("just set").did()
    };

    let mut node_guard = state.node.lock().await;
    if node_guard.is_none() {
        let identity = state.identity.lock().await;
        let identity = identity.as_ref().expect("set above");
        let node = Node::spawn_persistent(identity, data_dir().join("node"))
            .await
            .map_err(|e| e.to_string())?;

        let mut docs = state.docs.lock().await;
        for (namespace_id, _capability) in node
            .list_local_namespaces()
            .await
            .map_err(|e| e.to_string())?
        {
            if let Some(doc) = node.open_namespace(namespace_id).await.map_err(|e| e.to_string())? {
                docs.insert(namespace_id.to_string(), doc);
            }
        }

        *node_guard = Some(node);
    }

    Ok(did)
}

#[tauri::command]
async fn node_did(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.identity.lock().await.as_ref().map(Identity::did))
}

/// Creates a brand-new namespace and publishes a `NodeProfile` into it —
/// enough to prove the wiring (identity → namespace → typed record) end
/// to end from the UI, not a real "create a cooperative" flow.
#[tauri::command]
async fn create_namespace_with_profile(
    state: State<'_, AppState>,
    name: String,
    category: NodeCategory,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;

    let doc = node.create_namespace().await.map_err(|e| e.to_string())?;

    let profile = NodeProfile {
        name,
        category,
        neighborhood: None,
        description: None,
        governance_eligible: Some(true),
        created_at: chrono::Utc::now(),
    };
    put_record(&doc, author, NodeProfile::SELF_KEY, &profile)
        .await
        .map_err(|e| e.to_string())?;

    let namespace_id = doc.id().to_string();
    state.docs.lock().await.insert(namespace_id.clone(), doc);
    Ok(namespace_id)
}

/// Every namespace this node currently holds open — same list `dist/`
/// would show as "your namespaces," backed by nothing more than
/// `AppState.docs`'s keys.
#[tauri::command]
async fn list_namespaces(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.docs.lock().await.keys().cloned().collect())
}

/// Shares a namespace this node already has open — `Node::share`,
/// unwrapped as a plain ticket string (`DocTicket`'s `Display`) so it's
/// paste-able anywhere: chat, email, a QR code later. This is the whole
/// grant mechanism SPEC.md §3.4/§3.6 describes — a ticket handed to
/// someone out of band, not a server either side has to trust.
#[tauri::command]
async fn share_namespace(
    state: State<'_, AppState>,
    namespace_id: String,
    mode: ShareMode,
) -> Result<String, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;

    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;

    let ticket = node.share(doc, mode).await.map_err(|e| e.to_string())?;
    Ok(ticket.to_string())
}

/// Joins a namespace from a ticket someone shared — `Node::join`, which
/// SPEC.md §6 item 10 confirmed live backfills full history, not just
/// future writes, so this is a real join, not a partial one.
#[tauri::command]
async fn join_namespace(state: State<'_, AppState>, ticket: String) -> Result<String, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;

    let ticket: DocTicket = ticket.parse().map_err(|e| format!("invalid ticket: {e}"))?;
    let doc = node.join(ticket).await.map_err(|e| e.to_string())?;

    let namespace_id = doc.id().to_string();
    state.docs.lock().await.insert(namespace_id.clone(), doc);
    Ok(namespace_id)
}

/// Naive reflective inspector, per the request that started this: every
/// entry currently in a namespace, raw — no per-record-type command, no
/// per-record-type frontend code. `namespace::dump_all` already does the
/// actual work (try each entry's bytes as JSON, fall back to hex); this
/// is just the Tauri boundary around it.
#[tauri::command]
async fn dump_namespace(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<Vec<RawEntry>, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;

    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;

    dump_all(node, doc).await.map_err(|e| e.to_string())
}

/// Renders a ticket string as an SVG QR code — nothing more than that.
/// No restriction on which mode gets turned into a code: a `Write` QR
/// posted somewhere public is a real, intended pattern (a public inbox —
/// anyone can submit under their own author, nobody can forge or
/// overwrite someone else's entry, SPEC.md §3.4's per-author identifier
/// finding already makes that safe). What that pattern actually needs is
/// moderation/volume handling on the read side, not a restriction on the
/// write side — see `submit_text` below and README.md.
#[tauri::command]
fn ticket_to_qr(ticket: String) -> Result<String, String> {
    let code = qrcode::QrCode::new(ticket.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render()
        .min_dimensions(256, 256)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build())
}

/// Writes freeform text at a caller-chosen key — the "Google doc without
/// Google" primitive, `namespace::put_text` unwrapped for the UI. No
/// lexicon, no `Record` type; a namespace used this way is just a shared,
/// synced, capability-scoped key/value space, same machinery as every
/// typed record above, used with no schema at all.
#[tauri::command]
async fn write_text(
    state: State<'_, AppState>,
    namespace_id: String,
    key: String,
    text: String,
) -> Result<(), String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    put_text(doc, author, &key, &text)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads freeform text back from an exact key.
#[tauri::command]
async fn read_text(
    state: State<'_, AppState>,
    namespace_id: String,
    key: String,
) -> Result<Option<String>, String> {
    let author = ensure_author(&state).await?;
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    get_text(node, doc, author, key.as_str())
        .await
        .map_err(|e| e.to_string())
}

/// Submits text under a fresh, auto-generated key beneath `prefix` — the
/// public-inbox primitive. Every submitter calls this the same way; no
/// coordination on keys, no collision, no way to overwrite someone
/// else's submission (or the namespace owner's own records) — see
/// `namespace::submit_text`'s doc comment for exactly why that's safe by
/// construction rather than by convention.
#[tauri::command]
async fn submit_to_inbox(
    state: State<'_, AppState>,
    namespace_id: String,
    prefix: String,
    text: String,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    submit_text(doc, author, &prefix, &text)
        .await
        .map_err(|e| e.to_string())
}

/// Placeholder starting policy until SPEC.md §6 item 12's still-open gap
/// (no `founding` record type — `fold::fold`'s own doc comment names
/// this) gets a real answer. **Not a protocol default** — SPEC.md §3.7.2
/// is explicit that these numbers are namespace-owned, group-set state,
/// never something this document (or this app) gets to fix. Short
/// windows on purpose, for a reference client someone's actively poking
/// at, not because a real namespace should use them.
fn placeholder_founding_policy() -> HashMap<GovernanceClass, PolicyValue> {
    [
        (
            GovernanceClass::AdmitCoSigner,
            PolicyValue { window_seconds: 3600, block_threshold: 1 },
        ),
        (
            GovernanceClass::RemoveCoSigner,
            PolicyValue { window_seconds: 3600, block_threshold: 1 },
        ),
        (
            GovernanceClass::ChangePolicy,
            PolicyValue { window_seconds: 3600, block_threshold: 1 },
        ),
    ]
    .into_iter()
    .collect()
}

/// Same gap, same honesty: with no `founding` record naming who a
/// namespace's founder(s) were, this heuristic treats every author who's
/// ever self-asserted `governance_eligible: true` in their own
/// `NodeProfile` as founding-eligible. `profile.json`'s own lexicon
/// already says that field "is not authoritative" for exactly this
/// reason — a compromised or just-optimistic client could set it. Fine
/// for a reference client proving the wiring; not fine as the actual
/// membership check a real deployment should trust.
async fn bootstrap_founding_eligible(node: &Node, doc: &Doc) -> Result<HashSet<AuthorId>, String> {
    let profiles = list_records::<NodeProfile>(node, doc)
        .await
        .map_err(|e| e.to_string())?;
    Ok(profiles
        .into_iter()
        .filter(|(_, _, profile)| profile.governance_eligible == Some(true))
        .map(|(author, _, _)| author)
        .collect())
}

#[derive(serde::Serialize)]
struct ProposalView {
    author_hex: String,
    rkey: String,
    proposal: Proposal,
    status: RatificationView,
}

#[derive(serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum RatificationView {
    Open { blockers: Vec<String> },
    Ratified,
    Blocked { blockers: Vec<String> },
}

impl From<Ratification<AuthorId>> for RatificationView {
    fn from(r: Ratification<AuthorId>) -> Self {
        let hex_all = |blockers: Vec<AuthorId>| {
            blockers.iter().map(|a| hex::encode(a.as_bytes())).collect()
        };
        match r {
            Ratification::Open { blockers } => RatificationView::Open { blockers: hex_all(blockers) },
            Ratification::Ratified => RatificationView::Ratified,
            Ratification::Blocked { blockers } => RatificationView::Blocked { blockers: hex_all(blockers) },
        }
    }
}

#[derive(serde::Serialize)]
struct GovernanceStateView {
    eligible_hex: Vec<String>,
}

impl From<GovernanceState> for GovernanceStateView {
    fn from(s: GovernanceState) -> Self {
        GovernanceStateView {
            eligible_hex: s.eligible.iter().map(|a| hex::encode(a.as_bytes())).collect(),
        }
    }
}

/// Every `Proposal` in a namespace, each with its live ratification
/// status — `fold::fold` unwrapped for the UI. Re-runs the whole fold on
/// every call (O(every record in the namespace)); fine at reference-app
/// scale, a real cache/incremental-fold question once a namespace has
/// more than a handful of proposals.
#[tauri::command]
async fn list_proposals(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<Vec<ProposalView>, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;

    let founding_eligible = bootstrap_founding_eligible(node, doc).await?;
    let (_, outcomes) = fold::fold(
        node,
        doc,
        founding_eligible,
        placeholder_founding_policy(),
        chrono::Utc::now(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(outcomes
        .into_iter()
        .map(|o| ProposalView {
            author_hex: hex::encode(o.author.as_bytes()),
            rkey: o.rkey,
            proposal: o.proposal,
            status: o.ratification.into(),
        })
        .collect())
}

/// Current governance state — who's eligible, per `bootstrap_founding_eligible`
/// plus every ratified admit/remove since. Separate command from
/// `list_proposals` because a caller often wants one without the other
/// (e.g. populating a "who can I address a removeCoSigner proposal at"
/// list without re-rendering every proposal).
#[tauri::command]
async fn governance_state(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<GovernanceStateView, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;

    let founding_eligible = bootstrap_founding_eligible(node, doc).await?;
    let (governance_state, _) = fold::fold(
        node,
        doc,
        founding_eligible,
        placeholder_founding_policy(),
        chrono::Utc::now(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(governance_state.into())
}

/// Posts a new `Proposal` — `fold::propose` unwrapped for the UI.
/// `deadline_hours` is the only timing control exposed here rather than
/// a raw deadline, since "how far in the future" is what a person
/// actually thinks in, not an absolute timestamp.
#[tauri::command]
async fn create_proposal(
    state: State<'_, AppState>,
    namespace_id: String,
    title: String,
    description: Option<String>,
    class: GovernanceClass,
    deadline_hours: i64,
    subject_member_hex: Option<String>,
    policy_change: Option<PolicyChange>,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;

    let now = chrono::Utc::now();
    let proposal = Proposal {
        title,
        description,
        class,
        block_threshold: None,
        deadline: now + ChronoDuration::hours(deadline_hours),
        policy_change,
        subject_member: subject_member_hex,
        created_at: now,
    };
    fold::propose(doc, author, &proposal)
        .await
        .map_err(|e| e.to_string())
}

/// Posts a `Signal` responding to a `Proposal` — `fold::signal` unwrapped
/// for the UI. Needs the target proposal's author (not just its `rkey`)
/// because `RecordIdentifier` is `(namespace, author, key)` — SPEC.md
/// §3.4 — so `rkey` alone doesn't uniquely name a record.
#[tauri::command]
async fn create_signal(
    state: State<'_, AppState>,
    namespace_id: String,
    proposal_author_hex: String,
    proposal_rkey: String,
    signal_type: SignalType,
    text: Option<String>,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    let proposal_author =
        decode_author_hex(&proposal_author_hex).ok_or("invalid proposal author id")?;

    let sig = Signal {
        subject: String::new(), // overwritten by fold::signal
        signal_type,
        text,
        created_at: chrono::Utc::now(),
    };
    fold::signal(doc, author, proposal_author, &proposal_rkey, sig)
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            spawn_node,
            node_did,
            create_namespace_with_profile,
            list_namespaces,
            share_namespace,
            join_namespace,
            dump_namespace,
            ticket_to_qr,
            write_text,
            read_text,
            submit_to_inbox,
            list_proposals,
            governance_state,
            create_proposal,
            create_signal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
