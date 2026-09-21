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
    namespace::{put_record, Node},
    records::{NodeCategory, NodeProfile},
};
use iroh_docs::{api::protocol::ShareMode, api::Doc, DocTicket};
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
    let node = state.node.lock().await;
    let node = node.as_ref().ok_or("call spawn_node first")?;

    let author = node.author_create().await.map_err(|e| e.to_string())?;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
