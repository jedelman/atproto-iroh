//! Live integration test for `namespace::dump_all` — the generic,
//! per-record-type-agnostic dump behind the Tauri app's inspector.
//! Proves both halves of `EntryContent`: a real `NodeProfile` comes back
//! as `Json`, and an entry that was never JSON in the first place (some
//! other protocol's raw bytes, or corrupted content) comes back as `Raw`
//! rather than making `dump_all` fail outright — a debugging view that
//! can't read one entry shouldn't refuse to show the rest.

use atproto_iroh_core::{
    namespace::{dump_all, put_record, EntryContent, Node},
    records::{NodeCategory, NodeProfile},
};
use chrono::Utc;
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn dump_all_shows_json_and_raw_entries_alike() -> anyhow::Result<()> {
    let founder = Node::spawn().await?;
    let reader = Node::spawn().await?;

    let author = founder.author_create().await?;
    let doc = founder.create_namespace().await?;

    let profile = NodeProfile {
        name: "Eleanor's".into(),
        category: NodeCategory::WorkerCoop,
        neighborhood: None,
        description: None,
        governance_eligible: None,
        created_at: Utc::now(),
    };
    put_record(&doc, author, NodeProfile::SELF_KEY, &profile).await?;

    // Not JSON at all — proves dump_all doesn't assume every entry is a
    // record this crate defined.
    doc.set_bytes(author, b"not-a-record/raw".to_vec(), b"\x00\x01not json".to_vec())
        .await?;

    let ticket = founder.share(&doc, ShareMode::Write).await?;
    let reader_doc = reader.join(ticket).await?;

    let deadline = Instant::now() + Duration::from_secs(20);
    let entries = loop {
        if let Ok(entries) = dump_all(&reader, &reader_doc).await {
            if entries.len() == 2 && entries.iter().all(|e| e.content_len > 0) {
                break entries;
            }
        }
        if Instant::now() >= deadline {
            panic!("entries never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    let profile_entry = entries
        .iter()
        .find(|e| e.key == "network.essmesh.node.profile/self")
        .expect("profile entry present");
    match &profile_entry.content {
        EntryContent::Json(value) => {
            assert_eq!(value["name"], "Eleanor's");
        }
        EntryContent::Raw { .. } => panic!("expected the profile to parse as JSON"),
    }

    let raw_entry = entries
        .iter()
        .find(|e| e.key == "not-a-record/raw")
        .expect("raw entry present");
    match &raw_entry.content {
        EntryContent::Raw { hex } => {
            assert_eq!(hex, "00016e6f74206a736f6e"); // "\x00\x01not json"
        }
        EntryContent::Json(_) => panic!("expected non-JSON bytes to fall back to Raw"),
    }

    founder.shutdown().await;
    reader.shutdown().await;
    Ok(())
}
