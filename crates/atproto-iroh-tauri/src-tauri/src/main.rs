// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Reference Tauri client for `atproto-iroh-core` — CLAUDE.md's "Client"
//! section, scaffolded. Thin on purpose: every command here is a direct
//! call into the core crate, no protocol logic duplicated or
//! reimplemented at this layer. See `../README.md` for what's real here
//! and what's still a placeholder.

use std::collections::HashMap;

use atproto_iroh_core::{
    identity::Identity,
    namespace::{dump_all, get_text, put_record, put_text, submit_text, Node, RawEntry},
    records::{NodeCategory, NodeProfile},
};
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
/// — every open namespace this node currently holds a capability into,
/// so a later command (share, publish, eventually governance) can name
/// one without re-deriving it from a ticket each time. In-memory only,
/// same as `Node::spawn`'s `Docs::memory()` — see README.md's
/// persistence gap; nothing here survives a restart yet.
#[derive(Default)]
struct AppState {
    identity: Mutex<Option<Identity>>,
    node: Mutex<Option<Node>>,
    docs: Mutex<HashMap<String, Doc>>,
    /// One record-signing identity, reused across every write this
    /// session makes rather than minted fresh per call — so edits to the
    /// same freeform doc (or the same inbox) from one person show up
    /// under one consistent author, not a different stranger each time.
    /// Distinct from `identity` (the node's own `did:iroh`) for the same
    /// reason `namespace.rs`'s own doc comment gives: an `Author` signs
    /// entries, it isn't the node's identity.
    author: Mutex<Option<AuthorId>>,
}

async fn ensure_author(state: &AppState) -> Result<AuthorId, String> {
    let mut author = state.author.lock().await;
    if let Some(author) = *author {
        return Ok(author);
    }
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;
    let created = node.author_create().await.map_err(|e| e.to_string())?;
    *author = Some(created);
    Ok(created)
}

#[tauri::command]
async fn spawn_node(state: State<'_, AppState>) -> Result<String, String> {
    {
        let mut node = state.node.lock().await;
        if node.is_none() {
            *node = Some(Node::spawn().await.map_err(|e| e.to_string())?);
        }
    }
    let mut identity = state.identity.lock().await;
    if identity.is_none() {
        *identity = Some(Identity::generate());
    }
    Ok(identity.as_ref().expect("just set").did())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
