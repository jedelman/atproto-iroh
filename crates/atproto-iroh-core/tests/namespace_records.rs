//! Live, real-network integration test: two real nodes, a real namespace
//! grant, a typed `NodeProfile` record written on one and read back on
//! the other after sync. Not a mock — same shape as
//! `examples/iroh_docs_probe.rs`, now exercising the library code that
//! probe's findings turned into instead of the raw `iroh-docs` API.

use atproto_iroh_core::{
    namespace::{get_record, list_records, put_record, Node},
    records::{NodeCategory, NodeProfile},
};
use chrono::Utc;
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

#[tokio::test]
async fn a_profile_written_on_one_node_syncs_readable_to_another() -> anyhow::Result<()> {
    let founder = Node::spawn().await?;
    let member = Node::spawn().await?;

    let author = founder.author_create().await?;
    let doc = founder.create_namespace().await?;

    let profile = NodeProfile {
        name: "Eleanor's".into(),
        category: NodeCategory::WorkerCoop,
        neighborhood: Some("Ghent".into()),
        description: None,
        governance_eligible: Some(true),
        created_at: Utc::now(),
    };
    put_record(&doc, author, NodeProfile::SELF_KEY, &profile).await?;

    let ticket = founder.share(&doc, ShareMode::Write).await?;
    let member_doc = member.join(ticket).await?;

    // Entry *metadata* (the docs-layer record: key, author, content hash)
    // and the *content blob* that hash points at sync separately and the
    // content lags — get_exact can return Some before the blob has
    // finished downloading, so a get_record error here means "not fully
    // synced yet," same as a None does, not a real failure. Found live
    // running this test, not anticipated; see namespace.rs's note.
    let deadline = Instant::now() + Duration::from_secs(20);
    let synced = loop {
        match get_record::<NodeProfile>(&member, &member_doc, author, NodeProfile::SELF_KEY).await
        {
            Ok(Some(got)) => break got,
            Ok(None) | Err(_) => {}
        }
        if Instant::now() >= deadline {
            panic!("profile never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(synced.name, "Eleanor's");
    assert_eq!(synced.category, NodeCategory::WorkerCoop);
    assert_eq!(synced.neighborhood.as_deref(), Some("Ghent"));
    assert_eq!(synced.governance_eligible, Some(true));

    // list_records finds it too, under the right author, via the
    // collection-prefix query rather than an exact key.
    let all: Vec<_> = list_records::<NodeProfile>(&member, &member_doc).await?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, author);
    assert_eq!(all[0].1.name, "Eleanor's");

    founder.shutdown().await;
    member.shutdown().await;
    Ok(())
}
