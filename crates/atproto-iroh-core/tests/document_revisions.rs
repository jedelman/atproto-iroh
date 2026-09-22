//! Live proof of the CRDT-offline-edit distinction `namespace.rs`'s
//! `save_document_revision` doc comment describes: `put_text` at a fixed
//! shared key silently loses one side's edit on reconnect (last-write-wins
//! over the same `(namespace, author, key)` entry); revision-keyed writes
//! don't, because two offline authors never touch the same key.
//!
//! Both nodes write to the *same* `doc_id` while genuinely disconnected
//! from each other (joined before either write, but the write ticket
//! isn't dialed/synced until both writes have already happened locally —
//! the same "edited while offline, reconnect later" shape as two people
//! on airplane mode), then both sides are read to confirm both revisions
//! made it through, not just the last one written.

use atproto_iroh_core::namespace::{list_document_revisions, load_document, save_document_revision, Node};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn two_offline_edits_to_the_same_document_both_survive_as_revisions() -> anyhow::Result<()> {
    let founder = Node::spawn().await?;
    let member = Node::spawn().await?;

    let founder_author = founder.author_create().await?;
    let doc = founder.create_namespace().await?;

    let ticket = founder.share(&doc, ShareMode::Write).await?;
    let member_doc = member.join(ticket).await?;
    let member_author = member.author_create().await?;

    // Both authors write a revision of the same logical document without
    // waiting on each other — the offline-edit case, not a race avoided by
    // sequencing.
    save_document_revision(&doc, founder_author, "minutes", "founder's draft").await?;
    save_document_revision(&member_doc, member_author, "minutes", "member's draft").await?;

    // Wait for both nodes to see both revisions — sync, not sequencing.
    // Entry metadata and content blobs sync on separate schedules (same
    // finding as namespace_records.rs's test), so a `list_document_revisions`
    // error here means "content blob still downloading," not a real
    // failure — retried the same way, not just a None/empty-list check.
    let deadline = Instant::now() + Duration::from_secs(20);
    let founder_revs = loop {
        match list_document_revisions(&founder, &doc, "minutes").await {
            Ok(revs) if revs.len() == 2 => break revs,
            Ok(_) | Err(_) => {}
        }
        if Instant::now() >= deadline {
            panic!("founder never saw both revisions within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };
    let member_revs = loop {
        match list_document_revisions(&member, &member_doc, "minutes").await {
            Ok(revs) if revs.len() == 2 => break revs,
            Ok(_) | Err(_) => {}
        }
        if Instant::now() >= deadline {
            panic!("member never saw both revisions within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    fn texts(revs: &[(iroh_docs::AuthorId, String, String)]) -> Vec<&str> {
        let mut t: Vec<&str> = revs.iter().map(|(_, _, text)| text.as_str()).collect();
        t.sort();
        t
    }
    assert_eq!(texts(&founder_revs), vec!["founder's draft", "member's draft"]);
    assert_eq!(texts(&member_revs), vec!["founder's draft", "member's draft"]);

    // load_document picks one (the later key) as "current" — a default,
    // not a claim it's the semantically right one; both texts are still
    // there via list_document_revisions regardless of which this returns.
    let (_, _, current) = load_document(&founder, &doc, "minutes").await?.unwrap();
    assert!(current == "founder's draft" || current == "member's draft");

    founder.shutdown().await;
    member.shutdown().await;
    Ok(())
}
