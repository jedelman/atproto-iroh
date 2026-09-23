//! Live proof that tagging actually crosses lexicons (Jason's explicit
//! requirement): tags a real `Message` (network.essmesh.chat.message)
//! and a real document revision (a plain freeform-text key, no lexicon
//! at all) with the *same* `Tag` type, syncs both to a second node, and
//! confirms `tags_for` finds each by its own `record_ref` without any
//! per-type tagging code.

use atproto_iroh_core::{
    messaging::send_message,
    namespace::{save_document_revision, Node},
    records::{record_ref, Record},
    tagging::{add_tag, pins, tags_for, Tag, PIN_LABEL},
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

/// Proves two things Jason's design-brief follow-up ("tags are monads";
/// "one pin per author, shown in order of recency") depends on being
/// really true, not just plausible from reading the code:
///
/// 1. A `Tag`'s own `subject` can point at another `Tag` — nothing about
///    `record_ref`/`Tag` restricts the collection a subject names, but
///    "nothing stops it" isn't the same as "it actually round-trips
///    through real sync," which this confirms live (two nodes, a tag on
///    a message, then a second tag on *that tag*, both synced and both
///    found by `tags_for`).
/// 2. `pins()`'s "one per author, most recent wins, newest first"
///    resolution is correct against a real synced tag history: one
///    author posts two `PIN_LABEL` tags (their first pin should stop
///    counting), a second author posts one — `pins()` should return
///    exactly two entries, each author's latest, pin-recency order.
#[tokio::test]
async fn tags_can_tag_tags_and_pins_resolve_one_per_author_by_recency() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let tagger_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;
    let founder_hex = hex::encode(founder.as_bytes());

    let message_rkey = send_message(&doc, founder, "welcome to the table".into(), None).await?;
    let message_ref = record_ref(&founder_hex, "network.essmesh.chat.message", &message_rkey);

    let ticket = founder_node.share(&doc, ShareMode::Write).await?;
    let tagger_doc = tagger_node.join(ticket).await?;
    let tagger = tagger_node.author_create().await?;
    let tagger_hex = hex::encode(tagger.as_bytes());

    // Part 1: tag the message, then tag *that tag* — a tag whose
    // subject is another tag's own record_ref.
    let first_tag_rkey = add_tag(&tagger_doc, tagger, message_ref.clone(), "nice".into()).await?;
    let first_tag_ref = record_ref(&tagger_hex, Tag::COLLECTION, &first_tag_rkey);
    add_tag(&tagger_doc, tagger, first_tag_ref.clone(), "agreed".into()).await?;

    // Part 2: two pins from the founder (only the second should count),
    // one pin from the tagger. The founder signs through their own
    // node's doc handle (`doc`, from founder_node) — an author id is
    // only usable for signing on the node that created it, not on
    // whichever Doc handle happens to be in scope.
    add_tag(&doc, founder, message_ref.clone(), PIN_LABEL.into()).await?;
    sleep(Duration::from_millis(5)).await; // force a strictly later created_at
    let founders_real_pin =
        add_tag(&doc, founder, message_ref.clone(), PIN_LABEL.into()).await?;
    add_tag(&tagger_doc, tagger, first_tag_ref.clone(), PIN_LABEL.into()).await?;

    // Poll the founder's own node until everything above has synced back.
    let deadline = Instant::now() + Duration::from_secs(20);
    let (tag_on_tag, resolved_pins) = loop {
        let tag_on_tag = tags_for(&founder_node, &doc, &first_tag_ref).await.unwrap_or_default();
        let resolved_pins = pins(&founder_node, &doc).await.unwrap_or_default();
        if tag_on_tag.len() >= 2 && resolved_pins.len() >= 2 {
            break (tag_on_tag, resolved_pins);
        }
        if Instant::now() >= deadline {
            panic!("tag-on-a-tag or pins never fully synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    // A tag really can name another tag as its subject, and that
    // reference is findable the same way any other subject is.
    let labels: Vec<&str> = tag_on_tag.iter().map(|(_, _, t)| t.label.as_str()).collect();
    assert!(labels.contains(&"agreed"), "tag-on-a-tag not found: {labels:?}");
    assert!(labels.contains(&PIN_LABEL), "pin-on-a-tag not found: {labels:?}");

    // Exactly one pin per author, and it's each author's *latest* one.
    assert_eq!(resolved_pins.len(), 2, "expected one pin per author: {resolved_pins:?}");
    let founder_pin = resolved_pins.iter().find(|(a, _, _)| hex::encode(a.as_bytes()) == founder_hex);
    assert_eq!(
        founder_pin.map(|(_, rkey, _)| rkey.as_str()),
        Some(founders_real_pin.as_str()),
        "founder's pin should resolve to their second (later) pin, not their first"
    );

    // Newest-pin-first ordering: the tagger's pin (posted after both of
    // the founder's) should sort ahead of the founder's.
    assert_eq!(
        hex::encode(resolved_pins[0].0.as_bytes()),
        tagger_hex,
        "most recently pinned author should be first: {resolved_pins:?}"
    );

    founder_node.shutdown().await;
    tagger_node.shutdown().await;
    Ok(())
}
