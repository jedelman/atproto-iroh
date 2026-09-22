//! Live integration test for `namespace::dump_all` — the generic,
//! per-record-type-agnostic dump behind the Tauri app's inspector.
//! Proves all three of `EntryContent`'s branches: a real `NodeProfile`
//! comes back as `Json`, a freeform `put_text` write comes back as
//! `Text`, and genuinely non-UTF-8 bytes (something dump_all's caller
//! never wrote as a record or text at all) fall back to `Raw` rather
//! than making the whole dump fail — a debugging view that can't read
//! one entry shouldn't refuse to show the rest.

use atproto_iroh_core::{
    namespace::{dump_all, put_record, put_text, EntryContent, Node},
    records::{NodeCategory, NodeProfile},
};
use chrono::Utc;
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn dump_all_shows_json_text_and_raw_entries_alike() -> anyhow::Result<()> {
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

    // Freeform, no Record impl, no lexicon — the "Google doc without
    // Google" primitive: shared mutable text at a caller-chosen key.
    put_text(&doc, author, "notes/agenda", "1. roof\n2. dues\n3. potluck").await?;

    // Genuinely not text at all (invalid UTF-8: a lone continuation
    // byte) — proves dump_all doesn't assume every non-JSON entry is
    // text just because most freeform writes will be.
    doc.set_bytes(author, b"not-a-record/binary".to_vec(), vec![0xff, 0xfe, 0x00])
        .await?;

    let ticket = founder.share(&doc, ShareMode::Write).await?;
    let reader_doc = reader.join(ticket).await?;

    let deadline = Instant::now() + Duration::from_secs(20);
    let entries = loop {
        if let Ok(entries) = dump_all(&reader, &reader_doc).await {
            if entries.len() == 3 && entries.iter().all(|e| e.content_len > 0) {
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
        EntryContent::Json(value) => assert_eq!(value["name"], "Eleanor's"),
        other => panic!("expected the profile to parse as Json, got {other:?}"),
    }

    let notes_entry = entries
        .iter()
        .find(|e| e.key == "notes/agenda")
        .expect("notes entry present");
    match &notes_entry.content {
        EntryContent::Text(text) => assert!(text.contains("potluck")),
        other => panic!("expected freeform text to come back as Text, got {other:?}"),
    }

    let binary_entry = entries
        .iter()
        .find(|e| e.key == "not-a-record/binary")
        .expect("binary entry present");
    match &binary_entry.content {
        EntryContent::Raw { hex } => assert_eq!(hex, "fffe00"),
        other => panic!("expected invalid UTF-8 to fall back to Raw, got {other:?}"),
    }

    founder.shutdown().await;
    reader.shutdown().await;
    Ok(())
}
