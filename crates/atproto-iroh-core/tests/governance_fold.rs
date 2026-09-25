//! Live, real-network integration test for `fold.rs`: two real nodes,
//! a real `admitCoSigner` Proposal, a real `block` Signal from a
//! governance-eligible founder, synced and folded on both sides.
//! Exercises the whole chain governance.rs and namespace.rs were each
//! unit-tested for separately, now wired together for real.

use std::collections::{HashMap, HashSet};

use atproto_iroh_core::{
    fold::{fold, propose, signal},
    governance::{GovernanceClass, PolicyValue, Proposal, Ratification, Signal, SignalType},
    namespace::Node,
};
use chrono::{Duration as ChronoDuration, Utc};
use iroh_docs::api::protocol::ShareMode;
use tokio::time::{sleep, Duration, Instant};

fn founding_policy() -> HashMap<GovernanceClass, PolicyValue> {
    [
        (
            GovernanceClass::AdmitCoSigner,
            PolicyValue {
                window_seconds: 0,
                block_threshold: 1,
            },
        ),
        (
            GovernanceClass::RemoveCoSigner,
            PolicyValue {
                window_seconds: 0,
                block_threshold: 2,
            },
        ),
        (
            GovernanceClass::ChangePolicy,
            PolicyValue {
                window_seconds: 0,
                block_threshold: 1,
            },
        ),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn a_single_eligible_block_stops_admission_end_to_end() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let joiner_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let candidate = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;

    // Founder proposes admitting `candidate`, deadline already in the
    // past so the test doesn't need to sleep through a real window.
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
    let proposal_rkey = propose(&doc, founder, &proposal).await?;

    // Founder blocks their own proposal.
    signal(
        &doc,
        founder,
        founder,
        &proposal_rkey,
        Signal {
            subject: String::new(), // overwritten by `signal()`
            signal_type: SignalType::Block,
            text: Some("actually, let's not".into()),
            created_at: Utc::now(),
        },
    )
    .await?;

    let ticket = founder_node.share(&doc, ShareMode::Write).await?;
    let joiner_doc = joiner_node.join(ticket).await?;

    let founding_eligible: HashSet<_> = [founder].into_iter().collect();
    let policy = founding_policy();

    // Poll fold() on the joiner's synced copy until it sees the proposal
    // *and* the signal — both are separate entries with their own
    // content-blob lag (namespace.rs's note), so retry the whole fold.
    let deadline = Instant::now() + Duration::from_secs(20);
    let (state, outcomes) = loop {
        match fold(
            &joiner_node,
            &joiner_doc,
            founding_eligible.clone(),
            policy.clone(),
            Utc::now(),
        )
        .await
        {
            Ok((state, outcomes)) if !outcomes.is_empty() => {
                if let Ratification::Blocked { .. } = outcomes[0].ratification {
                    break (state, outcomes);
                }
            }
            _ => {}
        }
        if Instant::now() >= deadline {
            panic!("governance state never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        &outcomes[0].ratification,
        Ratification::Blocked { blockers } if blockers == &vec![founder]
    ));
    // The block held: candidate never becomes eligible.
    assert!(!state.eligible.contains(&candidate));
    assert!(state.eligible.contains(&founder));

    founder_node.shutdown().await;
    joiner_node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn silence_admits_by_default_end_to_end() -> anyhow::Result<()> {
    let founder_node = Node::spawn().await?;
    let joiner_node = Node::spawn().await?;

    let founder = founder_node.author_create().await?;
    let candidate = founder_node.author_create().await?;
    let doc = founder_node.create_namespace().await?;

    let proposal = Proposal {
        title: "Admit candidate, nobody objects".into(),
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

    let founding_eligible: HashSet<_> = [founder].into_iter().collect();
    let policy = founding_policy();

    let deadline = Instant::now() + Duration::from_secs(20);
    let state = loop {
        if let Ok((state, outcomes)) = fold(
            &joiner_node,
            &joiner_doc,
            founding_eligible.clone(),
            policy.clone(),
            Utc::now(),
        )
        .await
        {
            if outcomes.len() == 1 && outcomes[0].ratification == Ratification::Ratified {
                break state;
            }
        }
        if Instant::now() >= deadline {
            panic!("governance state never synced within 20s");
        }
        sleep(Duration::from_millis(50)).await;
    };

    assert!(state.eligible.contains(&candidate));

    founder_node.shutdown().await;
    joiner_node.shutdown().await;
    Ok(())
}
