// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Reference Tauri client for `atproto-iroh-core` — CLAUDE.md's "Client"
//! section, scaffolded. Thin on purpose: every command here is a direct
//! call into the core crate, no protocol logic duplicated or
//! reimplemented at this layer. See `../README.md` for what's real here
//! and what's still a placeholder.

use atproto_iroh_core::{
    identity::Identity,
    namespace::{put_record, Node},
    records::{NodeCategory, NodeProfile},
};
use tauri::State;
use tokio::sync::Mutex;

/// Tauri-managed app state. Deliberately empty until a command asks for
/// it — spawning a real iroh endpoint and gossip swarm at app launch,
/// before the person using it has done anything, would be exactly the
/// kind of ambient background activity SPEC.md's goal 1 (zero ambient
/// legibility) argues against, extended to the client's own behavior on
/// its own machine, not just what it exposes to the network.
#[derive(Default)]
struct AppState {
    identity: Mutex<Option<Identity>>,
    node: Mutex<Option<Node>>,
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
/// to end from the UI, not a real "create a cooperative" flow. No
/// sharing/ticket UI yet; see `../README.md`.
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

    Ok(doc.id().to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            spawn_node,
            node_did,
            create_namespace_with_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
