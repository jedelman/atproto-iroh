//! Live proof of the relay-mode control endpoint (`control.rs`): a real
//! second node dials a "box" node's raw message endpoint directly — no
//! capability, just its `did:iroh` — and successfully joins a namespace
//! by sending it a ticket over a private, ephemeral connection. Then
//! proves the RESET half: a wrong token is rejected and changes nothing,
//! a correct token wipes the box's entire data directory and fires its
//! reset signal.

use atproto_iroh_core::{
    control::ResetToken,
    identity::Identity,
    namespace::{dump_all, list_document_revisions, Node, NetworkPreset},
};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

fn temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("atproto-iroh-control-test-{name}-{}", std::process::id()))
}

#[tokio::test]
async fn a_stranger_can_join_the_box_over_the_control_endpoint_with_no_capability() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let founder = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;
    atproto_iroh_core::namespace::save_document_revision(&doc, founder, "readme", "hello box")
        .await?;
    let ticket = founder_node.share(&doc, ShareMode::Read).await?;

    // The box: a real relay-mode node with the control endpoint enabled.
    let box_dir = temp_dir("box");
    let _ = std::fs::remove_dir_all(&box_dir);
    let box_identity = Identity::generate();
    let (box_node, _reset_token, _reset_signal) =
        Node::spawn_relay(&box_identity, &box_dir, NetworkPreset::N0).await?;

    // The sender only needs the box's public key — no ticket, no
    // capability into anything the box holds, just its did:iroh. In
    // production that's resolved via NetworkPreset::N0's relay/DNS
    // discovery; this sandbox has no real outbound reachability to
    // n0.computer's infrastructure (confirmed: bare-key dialing under N0
    // fails here with "No addressing information available" even though
    // N0 is configured), so the test dials the box's own reported
    // EndpointAddr explicitly to prove the JOIN/RESET protocol logic
    // live. The bare-did:iroh-only path itself remains unverified in
    // this sandbox — same honesty as the Android APK build gap.
    let sender_node = Node::spawn().await?;
    let response = sender_node
        .send_control_to(box_node.endpoint_addr(), &format!("JOIN {ticket}"))
        .await?;
    assert!(response.starts_with("OK"), "unexpected response: {response}");

    // Poll the box's own copy until the ticket's namespace actually
    // synced — same content-blob-lag caveat every other live test here
    // already documents.
    let deadline = Instant::now() + Duration::from_secs(20);
    let doc_id = doc.id();
    let box_doc = loop {
        if let Some(d) = box_node.open_namespace(doc_id).await? {
            let entries = dump_all(&box_node, &d).await.unwrap_or_default();
            if !entries.is_empty() {
                break d;
            }
        }
        if Instant::now() >= deadline {
            panic!("box never synced the namespace it was told to JOIN within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };
    let revisions = list_document_revisions(&box_node, &box_doc, "readme").await?;
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0].2, "hello box");

    founder_node.shutdown().await;
    sender_node.shutdown().await;
    box_node.shutdown().await;
    let _ = std::fs::remove_dir_all(&box_dir);
    Ok(())
}

#[tokio::test]
async fn reset_requires_the_real_token_and_actually_wipes_the_box() -> anyhow::Result<()> {
    let box_dir = temp_dir("reset");
    let _ = std::fs::remove_dir_all(&box_dir);
    let box_identity = Identity::generate();
    let (box_node, reset_token, reset_signal) =
        Node::spawn_relay(&box_identity, &box_dir, NetworkPreset::N0).await?;

    let sender_node = Node::spawn().await?;
    let box_addr = box_node.endpoint_addr();

    // Wrong token: rejected, nothing happens.
    let response = sender_node
        .send_control_to(box_addr.clone(), "RESET not-the-real-token")
        .await?;
    assert!(response.starts_with("ERROR"), "wrong token should be rejected: {response}");
    assert!(box_dir.exists(), "a rejected reset must not touch the data directory");

    // Right token: wipes the box's entire data directory.
    let response = sender_node
        .send_control_to(box_addr, &format!("RESET {}", reset_token.as_str()))
        .await?;
    assert!(response.starts_with("OK"), "correct token should be accepted: {response}");
    assert!(!box_dir.exists(), "a valid reset must remove the whole data directory");

    // The reset signal fired — this is what `serve` awaits to know it
    // should exit for a supervisor to restart it against a fresh
    // identity.
    tokio::time::timeout(Duration::from_secs(2), reset_signal.notified())
        .await
        .expect("reset_signal should have fired after a valid RESET");

    // A fresh identity generated after a real reset gets its own token —
    // confirms load_or_generate actually starts over, not reuses state
    // that should have been wiped.
    let new_token = ResetToken::load_or_generate(box_dir.join("reset-token"))?;
    assert_ne!(new_token.as_str(), reset_token.as_str());

    sender_node.shutdown().await;
    box_node.shutdown().await;
    let _ = std::fs::remove_dir_all(&box_dir);
    Ok(())
}
