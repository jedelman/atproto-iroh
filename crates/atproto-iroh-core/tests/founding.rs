//! Live proof of the founding-record mechanism (SPEC.md §6): a real
//! `Founding` claim, synced to a second node, resolved by
//! `fold::fold_namespace` into real genesis state — replacing the
//! Tauri/CLI placeholder heuristic that used to fabricate an eligible
//! set from self-asserted `NodeProfile.governance_eligible` flags.

use atproto_iroh_core::{
    fold::{found_namespace, propose, fold_namespace},
    governance::{FoundingPolicy, GovernanceClass, PolicyValue, Proposal, Ratification},
    namespace::Node,
};
use chrono::{Duration as ChronoDuration, Utc};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

fn policy() -> FoundingPolicy {
    FoundingPolicy {
        admit_co_signer: PolicyValue { window_seconds: 0, block_threshold: 1 },
        remove_co_signer: PolicyValue { window_seconds: 0, block_threshold: 2 },
        change_policy: PolicyValue { window_seconds: 0, block_threshold: 1 },
    }
}

#[tokio::test]
async fn a_founding_claim_syncs_and_resolves_into_real_genesis_state() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let joiner_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let candidate = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;

    // The real bootstrap path: post a Founding claim right after
    // creating the namespace, before anyone else is granted access.
    found_namespace(&doc, founder, vec![founder], policy()).await?;

    // A proposal to admit `candidate`, already past its deadline so the
    // test doesn't sleep through a real window — same shape as
    // governance_fold.rs's tests.
    let proposal = Proposal {
        title: "Admit candidate".into(),
        description: None,
        class: GovernanceClass::AdmitCoSigner,
        block_threshold: None,
        deadline: Utc::now() - ChronoDuration::seconds(1),
        policy_change: None,
        subject_member: Some(hex::encode(candidate.as_bytes())),
        created_at: Utc::now() - ChronoDuration::seconds(2),
    };
    propose(&doc, founder, &proposal).await?;

    let ticket = founder_node.share(&doc, ShareMode::Write).await?;
    let joiner_doc = joiner_node.join(ticket).await?;

    // Poll fold_namespace — no founding_eligible/founding_policy handed
    // in by the caller at all, unlike governance_fold.rs's tests. Real
    // genesis state comes purely from the synced Founding claim.
    let deadline = Instant::now() + Duration::from_secs(20);
    let state = loop {
        if let Ok((state, outcomes)) = fold_namespace(&joiner_node, &joiner_doc, Utc::now()).await
        {
            if outcomes.len() == 1 && outcomes[0].ratification == Ratification::Ratified {
                break state;
            }
        }
        if Instant::now() >= deadline {
            panic!("founding claim + proposal never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    // The founder was genesis-eligible purely from the synced Founding
    // claim, and their silent-by-default admit of `candidate` went
    // through exactly like governance_fold.rs's non-founding-record test.
    assert!(state.eligible.contains(&founder));
    assert!(state.eligible.contains(&candidate));

    founder_node.shutdown().await;
    joiner_node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn no_founding_claim_means_an_empty_eligible_set_not_an_error() -> anyhow::Result<()> {
    let node = Node::spawn().await?;
    let doc = node.create_namespace().await?;

    // Nobody ever called found_namespace — SPEC.md §6's documented
    // fallback: fold_namespace still returns cleanly, just with nobody
    // able to block anything (silence-ratifies-by-default still applies).
    let (state, outcomes) = fold_namespace(&node, &doc, Utc::now()).await?;
    assert!(state.eligible.is_empty());
    assert!(outcomes.is_empty());

    node.shutdown().await;
    Ok(())
}
