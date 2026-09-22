//! Live proof that `Node::spawn_persistent` actually persists — not just
//! that it compiles. Spawns a node, creates a namespace, writes a
//! record, shuts the node down (a real drop, not a mock restart), spawns
//! a *new* `Node` against the same identity file and data directory, and
//! checks that both the node's network identity and the namespace's
//! content survived.

use atproto_iroh_core::{
    identity::Identity,
    namespace::{get_record, put_record, Node},
    records::{NodeCategory, NodeProfile},
};
use chrono::Utc;

#[tokio::test]
async fn identity_and_namespace_content_survive_a_restart() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!(
        "atproto-iroh-persistence-test-{}",
        std::process::id()
    ));
    let identity_path = dir.join("identity");
    let data_dir = dir.join("data");

    // First "run": generate an identity, spawn persistently, create a
    // namespace, write a record.
    let identity = Identity::load_or_generate(&identity_path)?;
    let did_before = identity.did();

    let node = Node::spawn_persistent(&identity, &data_dir).await?;
    let author = node.author_create().await?;
    let doc = node.create_namespace().await?;
    let namespace_id = doc.id();

    let profile = NodeProfile {
        name: "Eleanor's".into(),
        category: NodeCategory::WorkerCoop,
        neighborhood: None,
        description: None,
        governance_eligible: Some(true),
        created_at: Utc::now(),
    };
    put_record(&doc, author, NodeProfile::SELF_KEY, &profile).await?;

    // A real shutdown, not a mock one — the router (and with it, the
    // endpoint) actually stops.
    node.shutdown().await;

    // Second "run": load the same identity file, spawn against the same
    // data directory, and check what came back without anything handed
    // to it directly (no ticket, no in-memory state) — only what's on
    // disk.
    let reloaded_identity = Identity::load_or_generate(&identity_path)?;
    assert_eq!(
        reloaded_identity.did(),
        did_before,
        "identity must be the same across a restart, not just look the same"
    );

    let node2 = Node::spawn_persistent(&reloaded_identity, &data_dir).await?;

    let local_namespaces = node2.list_local_namespaces().await?;
    assert!(
        local_namespaces.iter().any(|(id, _)| *id == namespace_id),
        "the namespace created before the restart should still be listed after it"
    );

    let reopened_doc = node2
        .open_namespace(namespace_id)
        .await?
        .expect("namespace should reopen after restart");

    let recovered: NodeProfile = get_record(&node2, &reopened_doc, author, NodeProfile::SELF_KEY)
        .await?
        .expect("the record written before the restart should still be there");
    assert_eq!(recovered.name, "Eleanor's");
    assert_eq!(recovered.category, NodeCategory::WorkerCoop);

    node2.shutdown().await;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
