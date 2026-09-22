//! Live proof that tagging actually crosses lexicons (Jason's explicit
//! requirement): tags a real `Message` (network.essmesh.chat.message)
//! and a real document revision (a plain freeform-text key, no lexicon
//! at all) with the *same* `Tag` type, syncs both to a second node, and
//! confirms `tags_for` finds each by its own `record_ref` without any
//! per-type tagging code.

use atproto_iroh_core::{
    messaging::send_message,
    namespace::{save_document_revision, Node},
    records::record_ref,
    tagging::{add_tag, tags_for},
};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn a_tag_crosses_lexicons_and_syncs() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let tagger_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;

    // Two records from two different collections — messaging.rs's
    // `Message` (a real lexicon) and namespace.rs's document-revision
    // primitive (no lexicon at all, just `{doc_id}/rev/{rkey}`).
    let message_rkey = send_message(&doc, founder, "welcome!".into(), None).await?;
    let founder_hex = hex::encode(founder.as_bytes());
    let message_ref = record_ref(&founder_hex, "network.essmesh.chat.message", &message_rkey);

    let doc_rev = save_document_revision(&doc, founder, "readme", "hello").await?;
    let doc_ref = record_ref(&founder_hex, "network.essmesh.namespace.doc.readme.rev", &doc_rev);

    let ticket = founder_node.share(&doc, ShareMode::Write).await?;
    let tagger_doc = tagger_node.join(ticket).await?;
    let tagger = tagger_node.author_create().await?;

    add_tag(&tagger_doc, tagger, message_ref.clone(), "pinned".into()).await?;
    add_tag(&tagger_doc, tagger, doc_ref.clone(), "reference".into()).await?;

    // Poll the founder's side until both tags have synced back.
    let deadline = Instant::now() + Duration::from_secs(20);
    let (message_tags, doc_tags) = loop {
        let message_tags = tags_for(&founder_node, &doc, &message_ref).await.unwrap_or_default();
        let doc_tags = tags_for(&founder_node, &doc, &doc_ref).await.unwrap_or_default();
        if !message_tags.is_empty() && !doc_tags.is_empty() {
            break (message_tags, doc_tags);
        }
        if Instant::now() >= deadline {
            panic!("tags never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(message_tags.len(), 1);
    assert_eq!(message_tags[0].2.label, "pinned");
    assert_eq!(message_tags[0].2.subject, message_ref);

    assert_eq!(doc_tags.len(), 1);
    assert_eq!(doc_tags[0].2.label, "reference");
    assert_eq!(doc_tags[0].2.subject, doc_ref);

    founder_node.shutdown().await;
    tagger_node.shutdown().await;
    Ok(())
}
