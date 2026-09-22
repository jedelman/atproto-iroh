//! The I/O layer over `governance.rs`'s pure ratification math: read a
//! namespace's `network.essmesh.governance.*` collection and fold it into
//! current state — "the roster is whatever the governance collection's
//! signed consent/block records fold up to" (SPEC.md §3.7.6's validation
//! note), made real.
//!
//! Order matters and is the whole trick: SPEC.md §3.7.2 locks a
//! `Proposal`'s threshold to whatever policy was current *at its
//! creation*, which is exactly what makes a single forward pass over
//! proposals sorted by `created_at` sufficient — each proposal's
//! ratification only ever depends on state built by strictly earlier
//! ratified proposals, never on anything later. No fixed point to solve
//! for, just a fold.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::{DateTime, Utc};
use iroh_docs::{api::Doc, AuthorId};

use crate::governance::{
    self, subject_ref, Founding, FoundingClaim, FoundingPolicy, GovernanceClass, PolicyValue,
    Proposal, Ratification, Signal,
};
use crate::namespace::{decode_author_hex, list_records, new_entry_key, put_record, Node};

#[derive(Debug, Clone)]
pub struct GovernanceState {
    pub eligible: HashSet<AuthorId>,
    /// Only classes with a ratified change are present; look up a
    /// missing class's value in the founding policy passed to `fold`.
    pub policy: HashMap<GovernanceClass, PolicyValue>,
}

/// One proposal's outcome as of `now`, carried alongside enough to find
/// it again (`author`/`rkey`) and to inspect the decision itself.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub author: AuthorId,
    pub rkey: String,
    pub proposal: Proposal,
    pub ratification: Ratification<AuthorId>,
}

/// Reads every `Proposal` and `Signal` in `doc`, sorts proposals by
/// `created_at`, and applies each one's effect in order — membership and
/// policy changes only take effect once `Ratification::Ratified`, per
/// `governance.rs`.
///
/// `founding_eligible`/`founding_policy` are the namespace's bootstrap
/// state (SPEC.md §3.7.2: "bootstrapped at namespace creation by
/// whoever founds it"). Not itself a record type yet — SPEC.md doesn't
/// specify how founding state is recorded or discovered, only that it
/// exists; a caller has to supply it out of band for now. Worth a real
/// answer (a `founding` record type, most likely) before this is
/// anything more than a reference implementation detail.
pub async fn fold(
    node: &Node,
    doc: &Doc,
    founding_eligible: HashSet<AuthorId>,
    founding_policy: HashMap<GovernanceClass, PolicyValue>,
    now: DateTime<Utc>,
) -> Result<(GovernanceState, Vec<Outcome>)> {
    let mut proposals = list_records::<Proposal>(node, doc).await?;
    proposals.sort_by_key(|(_, _, p)| p.created_at);

    let signals = list_records::<Signal>(node, doc).await?;
    let mut signals_by_subject: HashMap<String, Vec<(AuthorId, Signal)>> = HashMap::new();
    for (author, _rkey, signal) in signals {
        signals_by_subject
            .entry(signal.subject.clone())
            .or_default()
            .push((author, signal));
    }

    let mut state = GovernanceState {
        eligible: founding_eligible,
        policy: founding_policy.clone(),
    };
    let mut outcomes = Vec::with_capacity(proposals.len());

    for (author, rkey, proposal) in proposals {
        let threshold = state
            .policy
            .get(&proposal.class)
            .or_else(|| founding_policy.get(&proposal.class))
            .map(|p| p.block_threshold)
            .unwrap_or(1);

        let author_hex = hex::encode(author.as_bytes());
        let subject = subject_ref(&author_hex, &rkey);
        let relevant_signals = signals_by_subject.get(&subject).cloned().unwrap_or_default();

        let ratification =
            governance::ratification_state(&proposal, &relevant_signals, &state.eligible, threshold, now);

        if ratification == Ratification::Ratified {
            apply(&mut state, &proposal);
        }

        outcomes.push(Outcome {
            author,
            rkey,
            proposal,
            ratification,
        });
    }

    Ok((state, outcomes))
}

fn apply(state: &mut GovernanceState, proposal: &Proposal) {
    match proposal.class {
        GovernanceClass::AdmitCoSigner => {
            if let Some(member) = decode_subject_member(proposal) {
                state.eligible.insert(member);
            }
        }
        GovernanceClass::RemoveCoSigner => {
            if let Some(member) = decode_subject_member(proposal) {
                state.eligible.remove(&member);
            }
        }
        GovernanceClass::ChangePolicy => {
            if let Some(change) = &proposal.policy_change {
                for class in [
                    GovernanceClass::AdmitCoSigner,
                    GovernanceClass::RemoveCoSigner,
                    GovernanceClass::ChangePolicy,
                ] {
                    if let Some(value) = change.for_class(class) {
                        state.policy.insert(class, value);
                    }
                }
            }
        }
        GovernanceClass::General => {}
    }
}

fn decode_subject_member(proposal: &Proposal) -> Option<AuthorId> {
    decode_author_hex(proposal.subject_member.as_deref()?)
}

/// Writes this author's founding claim — SPEC.md §6's founding-record
/// item. The caller's job, not enforced here: call this once, right
/// after `Node::create_namespace`, before the namespace is ever shared —
/// a claim posted well after sharing has begun is exactly the
/// backdating case `governance::resolve_founding`'s acceptance window
/// exists to reject; a naturally-timed multi-founder bootstrap (everyone
/// posting within the window) resolves correctly regardless of exactly
/// when `share()` happens to be called relative to it.
pub async fn found_namespace(
    doc: &Doc,
    author: AuthorId,
    eligible: Vec<AuthorId>,
    policy: FoundingPolicy,
) -> Result<()> {
    let founding = Founding {
        eligible: eligible.iter().map(|a| hex::encode(a.as_bytes())).collect(),
        policy,
        created_at: Utc::now(),
    };
    put_record(doc, author, governance::FOUNDING_KEY, &founding).await?;
    Ok(())
}

/// Reads every `Founding` claim currently synced into `doc` and resolves
/// them into genesis state — `governance::resolve_founding`, plus the
/// hex-decoding that module can't do itself (it stays generic over the
/// author type; this crate's concrete one is `iroh_docs::AuthorId`).
/// `None` means no claim has ever been posted (SPEC.md §6: a namespace
/// created before this record type existed, or one that skipped it) —
/// the caller decides what that means, same as `governance::
/// resolve_founding` itself.
pub async fn read_founding(
    node: &Node,
    doc: &Doc,
    window_seconds: i64,
) -> Result<Option<(HashSet<AuthorId>, HashMap<GovernanceClass, PolicyValue>)>> {
    let records = list_records::<Founding>(node, doc).await?;
    let claims: Vec<FoundingClaim<AuthorId>> = records
        .into_iter()
        .map(|(author, _rkey, founding)| FoundingClaim {
            claimant: author,
            eligible: founding
                .eligible
                .iter()
                .filter_map(|hex_str| decode_author_hex(hex_str))
                .collect(),
            policy: founding.policy.as_map(),
            created_at: founding.created_at,
        })
        .collect();
    Ok(governance::resolve_founding(&claims, window_seconds))
}

/// The real end-to-end path an app should use: resolves genesis state
/// from whatever `Founding` claims are synced (falling back to an empty
/// eligible set and no policy overrides if none exist yet — silence
/// still ratifies everything by default per SPEC.md §3.9, so an
/// unfounded namespace isn't unsafe, just unable to ever block anything
/// until a claim exists), then folds governance history the same as
/// `fold` above. Supersedes passing `founding_eligible`/`founding_policy`
/// in by hand — those parameters on `fold` stay for direct/testing use,
/// this is what a real client calls.
pub async fn fold_namespace(
    node: &Node,
    doc: &Doc,
    now: DateTime<Utc>,
) -> Result<(GovernanceState, Vec<Outcome>)> {
    let (founding_eligible, founding_policy) =
        read_founding(node, doc, governance::DEFAULT_FOUNDING_WINDOW_SECONDS)
            .await?
            .unwrap_or_default();
    fold(node, doc, founding_eligible, founding_policy, now).await
}

/// Posts a new `Proposal` under `author`, generating its `rkey`. Returns
/// the `rkey` so the caller can build `subject_ref`s for `Signal`s that
/// respond to it.
pub async fn propose(
    doc: &Doc,
    author: AuthorId,
    proposal: &Proposal,
) -> Result<String> {
    let rkey = new_entry_key();
    put_record(doc, author, &rkey, proposal).await?;
    Ok(rkey)
}

/// Posts a new `Signal` under `author`, targeting the given `Proposal`'s
/// `(author, rkey)` via `subject_ref`.
pub async fn signal(
    doc: &Doc,
    author: AuthorId,
    proposal_author: AuthorId,
    proposal_rkey: &str,
    mut signal: Signal,
) -> Result<String> {
    signal.subject = subject_ref(&hex::encode(proposal_author.as_bytes()), proposal_rkey);
    let rkey = new_entry_key();
    put_record(doc, author, &rkey, &signal).await?;
    Ok(rkey)
}
