//! A Table's display name resolves only from genesis members' entries —
//! `fold::read_table_name`. Every author has their own slot at every key,
//! so without that rule any member could "rename" the Table for
//! everyone just by writing the same key.

use atproto_iroh_core::{
    fold::{found_namespace, read_table_name, write_table_name},
    governance::{FoundingPolicy, PolicyValue},
    namespace::Node,
};

fn policy() -> FoundingPolicy {
    let v = PolicyValue { window_seconds: 86_400, block_threshold: 1 };
    FoundingPolicy { admit_co_signer: v, remove_co_signer: v, change_policy: v }
}

#[tokio::test]
async fn only_a_genesis_member_can_name_a_table() -> anyhow::Result<()> {
    let node = Node::spawn().await?;
    let founder = node.author_create().await?;
    let member = node.author_create().await?;
    let doc = node.create_namespace().await?;

    // Unfounded: no name, even if someone wrote one.
    write_table_name(&doc, member, "Too early").await?;
    assert_eq!(read_table_name(&node, &doc).await?, None);

    found_namespace(&doc, founder, vec![founder], policy()).await?;
    write_table_name(&doc, founder, "The Garden Table").await?;
    assert_eq!(read_table_name(&node, &doc).await?.as_deref(), Some("The Garden Table"));

    // A non-genesis member writing the same key changes nothing.
    write_table_name(&doc, member, "Hijacked").await?;
    assert_eq!(read_table_name(&node, &doc).await?.as_deref(), Some("The Garden Table"));

    // The founder renaming it does.
    write_table_name(&doc, founder, "Garden & Tools").await?;
    assert_eq!(read_table_name(&node, &doc).await?.as_deref(), Some("Garden & Tools"));

    node.shutdown().await;
    Ok(())
}
