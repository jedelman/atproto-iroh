//! Live proof of the first batteries-included app: two real nodes, one
//! sends a message, the other replies, both synced and read back in
//! order on a third viewpoint (the original sender re-reading its own
//! doc after both messages have landed).

use atproto_iroh_core::{
    messaging::{list_messages, reply_ref, send_message},
    namespace::Node,
};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn two_nodes_exchange_messages_and_both_see_the_full_thread_in_order() -> anyhow::Result<()> {
    let alice_node = Node::spawn().await?;
    let bob_node = Node::spawn().await?;

    let alice = alice_node.author_create().await?;
    let doc = alice_node.create_namespace().await?;

    let ticket = alice_node.share(&doc, ShareMode::Write).await?;
    let bob_doc = bob_node.join(ticket).await?;
    let bob = bob_node.author_create().await?;

    let greeting_rkey = send_message(&doc, alice, "hey, anyone around?".into(), None).await?;

    // Bob waits to see Alice's message before replying — realistic
    // ordering, not required by the mechanism itself (messaging.rs's
    // append-only keys mean unordered arrival would still be safe, just
    // not exercised by this test).
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match list_messages(&bob_node, &bob_doc).await {
            Ok(messages) if !messages.is_empty() => break,
            _ => {}
        }
        if Instant::now() >= deadline {
            panic!("bob never saw alice's message within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    }

    let reply_to = reply_ref(&hex::encode(alice.as_bytes()), &greeting_rkey);
    send_message(&bob_doc, bob, "yep, right here".into(), Some(reply_to.clone())).await?;

    // Alice sees both messages, in order, with Bob's reply correctly
    // referencing her original message.
    let deadline = Instant::now() + Duration::from_secs(20);
    let messages = loop {
        match list_messages(&alice_node, &doc).await {
            Ok(messages) if messages.len() == 2 => break messages,
            _ => {}
        }
        if Instant::now() >= deadline {
            panic!("alice never saw bob's reply within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(messages[0].0, alice);
    assert_eq!(messages[0].2.text, "hey, anyone around?");
    assert_eq!(messages[0].2.reply_to, None);

    assert_eq!(messages[1].0, bob);
    assert_eq!(messages[1].2.text, "yep, right here");
    assert_eq!(messages[1].2.reply_to, Some(reply_to));

    alice_node.shutdown().await;
    bob_node.shutdown().await;
    Ok(())
}
