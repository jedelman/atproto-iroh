//! A namespace reopened after a restart must actually sync again — found
//! on the first real two-phone test: `Node::open_namespace` used to only
//! open the local replica, and iroh-docs turns away incoming sync for any
//! namespace that isn't in its live set (`AbortReason::NotFound`), so a
//! restarted phone (or a relay box's `serve`) silently stopped syncing
//! everything it held until something happened to call `share()`.

use atproto_iroh_core::{
    identity::Identity,
    namespace::{get_text, put_text, Node},
};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

async fn wait_for_text(node: &Node, doc: &iroh_docs::api::Doc, author: iroh_docs::AuthorId, key: &str) -> anyhow::Result<Option<String>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(text) = get_text(node, doc, author, key).await.ok().flatten() {
            return Ok(Some(text));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn a_reopened_namespace_accepts_sync_after_restart() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("atproto-iroh-reopen-sync-{}", std::process::id()));
    let identity_path = dir.join("identity");
    let data_dir = dir.join("data");

    // Alice creates a Table and invites Bob.
    let identity = Identity::load_or_generate(&identity_path)?;
    let alice = Node::spawn_persistent(&identity, &data_dir).await?;
    let alice_author = alice.author_create().await?;
    let doc = alice.create_namespace().await?;
    let namespace_id = doc.id();
    put_text(&doc, alice_author, "note", "before the restart").await?;
    let ticket = alice.share(&doc, ShareMode::Write).await?;

    let bob = Node::spawn().await?;
    let bob_doc = bob.join(ticket).await?;
    assert_eq!(
        wait_for_text(&bob, &bob_doc, alice_author, "note").await?.as_deref(),
        Some("before the restart"),
        "baseline: the initial join should sync"
    );

    // Alice's phone restarts: same identity, same data, Table reopened
    // from disk the way the app and the relay's `serve` both do it.
    alice.shutdown().await;
    let alice = Node::spawn_persistent(&identity, &data_dir).await?;
    let reopened = alice
        .open_namespace(namespace_id)
        .await?
        .expect("namespace should reopen after restart");

    // Bob posts and dials Alice's restarted node.
    let bob_author = bob.author_create().await?;
    put_text(&bob_doc, bob_author, "reply", "after the restart").await?;
    bob_doc.start_sync(vec![alice.endpoint_addr()]).await?;

    assert_eq!(
        wait_for_text(&alice, &reopened, bob_author, "reply").await?.as_deref(),
        Some("after the restart"),
        "a reopened namespace must accept sync, not turn peers away"
    );

    alice.shutdown().await;
    bob.shutdown().await;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
