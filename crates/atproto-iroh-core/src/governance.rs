//! `network.essmesh.governance.*` — SPEC.md §3.7.2/§3.7.3/§3.9's resolved
//! objection-window model: a `Proposal` ratifies by default once its
//! `deadline` passes, unless enough `block` `Signal`s from
//! governance-eligible members land first. `consent` carries no
//! mechanical power. See `lexicons/network/essmesh/governance/` for the
//! canonical schemas this mirrors.
//!
//! Deliberately generic over the author-id type (`A`) rather than tied to
//! `iroh_docs::AuthorId` — this keeps the ratification math pure and unit
//! -testable with no async runtime or transport involved, per SPEC.md's
//! own "compute what's decidable, delegate what isn't" framing (applied
//! here to "decidable without I/O" vs. "needs the actual synced store").
//! `namespace.rs` is where a real `iroh_docs::AuthorId` gets plugged in.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GovernanceClass {
    AdmitCoSigner,
    RemoveCoSigner,
    ChangePolicy,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Consent,
    StandAside,
    Block,
    Abstain,
    Exit,
}

/// One class's governance policy — SPEC.md §3.7.2/§3.7.3: namespace-owned
/// state, not a protocol-fixed constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyValue {
    pub window_seconds: u64,
    pub block_threshold: u32,
}

/// Only meaningful on a `ChangePolicy`-class `Proposal` — the new values
/// it proposes adopting, one entry per class being changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admit_co_signer: Option<PolicyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_co_signer: Option<PolicyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_policy: Option<PolicyValue>,
}

impl PolicyChange {
    pub fn for_class(&self, class: GovernanceClass) -> Option<PolicyValue> {
        match class {
            GovernanceClass::AdmitCoSigner => self.admit_co_signer,
            GovernanceClass::RemoveCoSigner => self.remove_co_signer,
            GovernanceClass::ChangePolicy => self.change_policy,
            GovernanceClass::General => None,
        }
    }
}

/// `network.essmesh.governance.proposal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub class: GovernanceClass,
    /// Advisory only — see the lexicon's own note. The authoritative
    /// threshold for this Proposal's lifecycle is whatever
    /// `current_policy` resolved to at creation time (locked in per
    /// SPEC.md §3.7.2's mid-window note), not this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_threshold: Option<u32>,
    /// When the objection window closes. Required — SPEC.md §3.9's whole
    /// point for this layer is a reader checking this mechanically.
    pub deadline: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_change: Option<PolicyChange>,
    pub created_at: DateTime<Utc>,
}

/// `network.essmesh.governance.signal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Stand-in for `com.atproto.repo.strongRef` (AT-URI + CID) until
    /// real record referencing exists — see `namespace.rs`'s TODOs.
    pub subject: String,
    pub signal_type: SignalType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ratification<A> {
    /// Window still open. `blockers` is the current state, not final —
    /// re-evaluate after the deadline for the authoritative answer.
    Open { blockers: Vec<A> },
    Ratified,
    Blocked { blockers: Vec<A> },
}

/// Only the most recent `Signal` per author counts, so a later
/// `consent`/`stand_aside` supersedes an earlier `block` from the same
/// person. The lexicons don't define an explicit "withdraw" signal type;
/// this is the natural reading given `Signal.subject` already pins a CID
/// specifically so ordering is unambiguous (`signal.json`'s own
/// description) — decided here, not left ambiguous.
fn latest_signal_per_author<A: Eq + std::hash::Hash + Clone>(
    signals: &[(A, Signal)],
) -> HashMap<A, &Signal> {
    let mut latest: HashMap<A, &Signal> = HashMap::new();
    for (author, signal) in signals {
        match latest.get(author) {
            Some(existing) if existing.created_at >= signal.created_at => {}
            _ => {
                latest.insert(author.clone(), signal);
            }
        }
    }
    latest
}

/// The core rule from SPEC.md §3.9: ratified by default absent enough
/// objection. `block_threshold` must be the caller-resolved authoritative
/// value (`current_policy`, evaluated at the Proposal's `created_at`) —
/// this function trusts whatever threshold it's given, it doesn't derive
/// one, so a caller passing `proposal.block_threshold` (the advisory
/// field) instead of the real governance-history value gets an answer
/// that looks right and isn't authoritative.
pub fn ratification_state<A: Eq + std::hash::Hash + Clone>(
    proposal: &Proposal,
    signals: &[(A, Signal)],
    eligible: &HashSet<A>,
    block_threshold: u32,
    now: DateTime<Utc>,
) -> Ratification<A> {
    let blockers: Vec<A> = latest_signal_per_author(signals)
        .into_iter()
        .filter(|(author, signal)| {
            matches!(signal.signal_type, SignalType::Block) && eligible.contains(author)
        })
        .map(|(author, _)| author)
        .collect();

    if now < proposal.deadline {
        Ratification::Open { blockers }
    } else if blockers.len() as u32 >= block_threshold {
        Ratification::Blocked { blockers }
    } else {
        Ratification::Ratified
    }
}

/// Folds a namespace's ratified `changePolicy` history down to the
/// current `PolicyValue` for one class — SPEC.md §3.7.2: self-amending,
/// the latest ratified change for a class wins, falling back to the
/// namespace's founding policy if none has ratified yet.
///
/// Caller's job, not this function's: filtering `changes` down to
/// `changePolicy`-class Proposals that actually reached
/// `Ratification::Ratified` (via `ratification_state` above) — this
/// function trusts every entry it's handed is already-ratified.
pub fn current_policy(
    class: GovernanceClass,
    founding: PolicyValue,
    ratified_changes: impl IntoIterator<Item = (DateTime<Utc>, PolicyChange)>,
) -> PolicyValue {
    ratified_changes
        .into_iter()
        .filter_map(|(created_at, change)| change.for_class(class).map(|v| (created_at, v)))
        .max_by_key(|(created_at, _)| *created_at)
        .map(|(_, value)| value)
        .unwrap_or(founding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    fn proposal(class: GovernanceClass, deadline: i64) -> Proposal {
        Proposal {
            title: "test".into(),
            description: None,
            class,
            block_threshold: None,
            deadline: at(deadline),
            policy_change: None,
            created_at: at(0),
        }
    }

    fn signal(signal_type: SignalType, created_at: i64) -> Signal {
        Signal {
            subject: "at://ns/network.essmesh.governance.proposal/x".into(),
            signal_type,
            text: None,
            created_at: at(created_at),
        }
    }

    #[test]
    fn silence_ratifies_by_default() {
        let p = proposal(GovernanceClass::AdmitCoSigner, 100);
        let signals: Vec<(&str, Signal)> = vec![];
        let eligible: HashSet<&str> = HashSet::new();
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(200)),
            Ratification::Ratified
        );
    }

    #[test]
    fn one_eligible_block_stops_a_threshold_of_one() {
        let p = proposal(GovernanceClass::AdmitCoSigner, 100);
        let signals = vec![("alice", signal(SignalType::Block, 10))];
        let eligible: HashSet<&str> = ["alice"].into_iter().collect();
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(200)),
            Ratification::Blocked {
                blockers: vec!["alice"]
            }
        );
    }

    #[test]
    fn a_block_from_someone_not_eligible_does_not_count() {
        let p = proposal(GovernanceClass::AdmitCoSigner, 100);
        let signals = vec![("mallory", signal(SignalType::Block, 10))];
        let eligible: HashSet<&str> = ["alice"].into_iter().collect();
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(200)),
            Ratification::Ratified
        );
    }

    #[test]
    fn consent_alone_never_ratifies_early_and_never_blocks() {
        let p = proposal(GovernanceClass::AdmitCoSigner, 100);
        let signals = vec![("alice", signal(SignalType::Consent, 10))];
        let eligible: HashSet<&str> = ["alice"].into_iter().collect();
        // still open before the deadline, consent or not
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(50)),
            Ratification::Open { blockers: vec![] }
        );
        // and ratifies once it closes, same as if alice had said nothing
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(200)),
            Ratification::Ratified
        );
    }

    #[test]
    fn a_later_signal_from_the_same_author_supersedes_an_earlier_block() {
        let p = proposal(GovernanceClass::AdmitCoSigner, 100);
        let signals = vec![
            ("alice", signal(SignalType::Block, 10)),
            ("alice", signal(SignalType::StandAside, 20)),
        ];
        let eligible: HashSet<&str> = ["alice"].into_iter().collect();
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 1, at(200)),
            Ratification::Ratified
        );
    }

    #[test]
    fn threshold_of_two_survives_a_single_block() {
        let p = proposal(GovernanceClass::RemoveCoSigner, 100);
        let signals = vec![("alice", signal(SignalType::Block, 10))];
        let eligible: HashSet<&str> = ["alice", "bob"].into_iter().collect();
        assert_eq!(
            ratification_state(&p, &signals, &eligible, 2, at(200)),
            Ratification::Ratified
        );
    }

    #[test]
    fn current_policy_falls_back_to_founding_with_no_history() {
        let founding = PolicyValue {
            window_seconds: 604_800,
            block_threshold: 1,
        };
        assert_eq!(
            current_policy(GovernanceClass::AdmitCoSigner, founding, []),
            founding
        );
    }

    #[test]
    fn current_policy_takes_the_latest_ratified_change_for_that_class() {
        let founding = PolicyValue {
            window_seconds: 604_800,
            block_threshold: 1,
        };
        let older = PolicyValue {
            window_seconds: 86_400,
            block_threshold: 2,
        };
        let newer = PolicyValue {
            window_seconds: 172_800,
            block_threshold: 3,
        };
        let changes = [
            (
                at(10),
                PolicyChange {
                    admit_co_signer: Some(older),
                    ..Default::default()
                },
            ),
            (
                at(20),
                PolicyChange {
                    admit_co_signer: Some(newer),
                    ..Default::default()
                },
            ),
        ];
        assert_eq!(
            current_policy(GovernanceClass::AdmitCoSigner, founding, changes),
            newer
        );
    }
}
