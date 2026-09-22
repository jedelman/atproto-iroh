// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Reference Tauri client for `atproto-iroh-core` — CLAUDE.md's "Client"
//! section, scaffolded. Thin on purpose: every command here is a direct
//! call into the core crate, no protocol logic duplicated or
//! reimplemented at this layer. See `../README.md` for what's real here
//! and what's still a placeholder.

use std::collections::HashMap;

use atproto_iroh_core::{
    fold::{self, GovernanceState},
    governance::{
        FoundingPolicy, GovernanceClass, PolicyChange, PolicyValue, Proposal, Ratification,
        Signal, SignalType,
    },
    identity::Identity,
    messaging,
    namespace::{
        decode_author_hex, dump_all, list_document_revisions, load_document, put_record,
        save_document_revision, submit_text, NetworkPreset, Node, RawEntry,
    },
    records::{NodeCategory, NodeProfile},
};
use chrono::Duration as ChronoDuration;
use iroh_docs::{api::protocol::ShareMode, api::Doc, AuthorId, DocTicket};
#[cfg(any(target_os = "android", target_os = "ios"))]
use tauri::Manager;
use tauri::State;
use tokio::sync::Mutex;

/// Android and iOS have no shell environment for `paths::data_dir()`'s
/// env-var convention to read — Android gives each app a fixed
/// per-app-private directory instead, reachable only through Tauri's own
/// path resolver. Called once from `main()`'s `.setup()` hook, before any
/// command can run, so every later `paths::data_dir()` call (identity,
/// namespace storage, mute list) resolves to a real, writable, already
/// sandboxed-and-private location instead of whatever `$HOME` happens to
/// mean inside an Android process (usually nothing usable). Desktop
/// builds don't need this — the env var convention already covers them —
/// which is why this is behind the same `cfg` `spawn_node`'s preset
/// choice below uses.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn install_mobile_data_dir(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    atproto_iroh_core::paths::set_data_dir_override(dir);
    Ok(())
}

/// Which `NetworkPreset` `spawn_node` binds with — `N0` (relay fallback,
/// DNS address lookup) on mobile, where CGNAT and network-switching mean
/// direct QUIC often isn't reachable even in the foreground;
/// `Minimal` everywhere else, matching every test and desktop default in
/// the core crate. See `namespace::NetworkPreset`'s doc comment for the
/// real tradeoff (relay reachability vs. depending on n0.computer's
/// infrastructure and the connection-metadata visibility that implies).
fn network_preset() -> NetworkPreset {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        NetworkPreset::N0
    } else {
        NetworkPreset::Minimal
    }
}

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

/// Spawns the node against real persistent storage and loads (or
/// generates, on first run) this machine's `did:iroh` identity —
/// `Identity::load_or_generate`/`Node::spawn_persistent`, both new. Data
/// lives under `atproto_iroh_core::paths::data_dir()` — `$ATPROTO_IROH_
/// DATA_DIR` if set, so pointing this at an already-encrypted volume
/// (FileVault/BitLocker/LUKS/a mounted container) is one environment
/// variable, not app-level crypto this project doesn't need to own.
/// Also repopulates `AppState.docs` from whatever
/// `Node::list_local_namespaces` finds already on disk, so a restarted
/// app can immediately share, inspect, or govern namespaces from a
/// previous session without anyone re-pasting a ticket.
#[tauri::command]
async fn spawn_node(state: State<'_, AppState>) -> Result<String, String> {
    let did = {
        let mut identity = state.identity.lock().await;
        if identity.is_none() {
            *identity = Some(
                Identity::load_or_generate(Identity::default_path()).map_err(|e| e.to_string())?,
            );
        }
        identity.as_ref().expect("just set").did()
    };

    let mut node_guard = state.node.lock().await;
    if node_guard.is_none() {
        let identity = state.identity.lock().await;
        let identity = identity.as_ref().expect("set above");
        let node = Node::spawn_persistent_with_preset(
            identity,
            atproto_iroh_core::paths::data_dir().join("node"),
            network_preset(),
        )
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

/// This UI's own default starting policy for a namespace it founds — a
/// real answer now, not `placeholder_founding_policy`'s old
/// not-a-protocol-default heuristic, but still just *this app's* choice
/// of default, not a protocol constant: SPEC.md §3.7.2 is explicit these
/// numbers are namespace-owned, and a real client should let the founder
/// pick them (a form, not a hardcoded call) before this is more than a
/// reference default. 24-hour objection windows, block threshold 1 —
/// long enough to be a real window for a small group, not the reference
/// app's old 1-hour placeholder chosen only for someone actively poking
/// at the UI.
fn default_founding_policy() -> FoundingPolicy {
    let day = PolicyValue { window_seconds: 86_400, block_threshold: 1 };
    FoundingPolicy {
        admit_co_signer: day,
        remove_co_signer: day,
        change_policy: day,
    }
}

/// Creates a brand-new namespace, publishes a `NodeProfile` into it, and
/// posts this author's `Founding` claim (`fold::found_namespace`) — the
/// real founding-record mechanism (SPEC.md §6), replacing what used to
/// be a placeholder heuristic bootstrapped from
/// `NodeProfile.governance_eligible` self-assertions. The founder is the
/// namespace's sole genesis member; admitting anyone else afterward goes
/// through a real `AdmitCoSigner` Proposal, same governance path as
/// every later change.
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

    fold::found_namespace(&doc, author, vec![author], default_founding_policy())
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

/// Saves a new revision of the "Shared doc" — `namespace::
/// save_document_revision` unwrapped for the UI. **Replaces this
/// command's old `write_text`/`put_text` implementation**, which wrote
/// every edit to the same fixed key and was confirmed live
/// (SPEC.md's 2026-09-22 CRDT note) to silently drop one side's edit
/// when two people edited offline and reconnected later. This writes an
/// immutable, uniquely-keyed revision instead — nothing is ever
/// silently lost, at the cost of the UI having to show more than one
/// revision when concurrent edits land close together (see
/// `doc_history`).
#[tauri::command]
async fn doc_save(
    state: State<'_, AppState>,
    namespace_id: String,
    doc_id: String,
    text: String,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    save_document_revision(doc, author, &doc_id, &text)
        .await
        .map_err(|e| e.to_string())
}

/// The latest revision of a "Shared doc" by key order — `namespace::
/// load_document`. A default for "what to show right now," not a claim
/// it's the semantically correct pick if two edits landed close
/// together; a caller that cares should also call `doc_history`.
#[tauri::command]
async fn doc_load(
    state: State<'_, AppState>,
    namespace_id: String,
    doc_id: String,
) -> Result<Option<(String, String, String)>, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    let result = load_document(node, doc, &doc_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.map(|(rev, author, text)| (rev, hex::encode(author.as_bytes()), text)))
}

/// Every revision of a "Shared doc," oldest first — `namespace::
/// list_document_revisions`. The UI's hook for showing "someone else
/// edited this while you were offline" instead of silently picking one
/// side, which is exactly the gap `doc_save` replacing `write_text`
/// exists to close.
#[tauri::command]
async fn doc_history(
    state: State<'_, AppState>,
    namespace_id: String,
    doc_id: String,
) -> Result<Vec<(String, String, String)>, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    let revisions = list_document_revisions(node, doc, &doc_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(revisions
        .into_iter()
        .map(|(author, rev, text)| (rev, hex::encode(author.as_bytes()), text))
        .collect())
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

/// Posts a message into a namespace — `messaging::send_message`
/// unwrapped for the UI. The namespace itself is the channel; there's no
/// separate room concept. `reply_to_hex`/`reply_to_rkey` are optional and
/// together build the `reply_to` reference (`messaging::reply_ref`) —
/// exposed as two plain strings rather than asking the frontend to know
/// the "{authorHex}/{rkey}" convention itself.
#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    namespace_id: String,
    text: String,
    reply_to_author_hex: Option<String>,
    reply_to_rkey: Option<String>,
) -> Result<String, String> {
    let author = ensure_author(&state).await?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    let reply_to = match (reply_to_author_hex, reply_to_rkey) {
        (Some(author_hex), Some(rkey)) => Some(messaging::reply_ref(&author_hex, &rkey)),
        _ => None,
    };
    messaging::send_message(doc, author, text, reply_to)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct MessageView {
    author_hex: String,
    rkey: String,
    text: String,
    reply_to: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Every message in a namespace, oldest first — `messaging::list_messages`
/// unwrapped for the UI.
#[tauri::command]
async fn list_messages(
    state: State<'_, AppState>,
    namespace_id: String,
) -> Result<Vec<MessageView>, String> {
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let docs = state.docs.lock().await;
    let doc = docs
        .get(&namespace_id)
        .ok_or("unknown namespace — has this node opened it?")?;
    let messages = messaging::list_messages(node, doc)
        .await
        .map_err(|e| e.to_string())?;
    Ok(messages
        .into_iter()
        .map(|(author, rkey, message)| MessageView {
            author_hex: hex::encode(author.as_bytes()),
            rkey,
            text: message.text,
            reply_to: message.reply_to,
            created_at: message.created_at,
        })
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
/// status — `fold::fold_namespace` unwrapped for the UI: real genesis
/// state from a synced `Founding` claim, not a heuristic. Re-runs the
/// whole fold on every call (O(every record in the namespace)); fine at
/// reference-app scale, a real cache/incremental-fold question once a
/// namespace has more than a handful of proposals.
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

    let (_, outcomes) = fold::fold_namespace(node, doc, chrono::Utc::now())
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

/// Current governance state — who's eligible, per the namespace's synced
/// `Founding` claim(s) plus every ratified admit/remove since. Separate
/// command from `list_proposals` because a caller often wants one
/// without the other (e.g. populating a "who can I address a
/// removeCoSigner proposal at" list without re-rendering every
/// proposal).
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

    let (governance_state, _) = fold::fold_namespace(node, doc, chrono::Utc::now())
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
        .setup(|_app| {
            #[cfg(any(target_os = "android", target_os = "ios"))]
            install_mobile_data_dir(_app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            spawn_node,
            node_did,
            create_namespace_with_profile,
            list_namespaces,
            share_namespace,
            join_namespace,
            dump_namespace,
            ticket_to_qr,
            doc_save,
            doc_load,
            doc_history,
            submit_to_inbox,
            send_message,
            list_messages,
            list_proposals,
            governance_state,
            create_proposal,
            create_signal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
