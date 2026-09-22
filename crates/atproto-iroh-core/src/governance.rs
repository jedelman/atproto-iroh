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

use crate::records::Record;

#[derive(Debug, Clone, Copy, PartialEq, Eq, std::hash::Hash, Serialize, Deserialize)]
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
    /// Required on `AdmitCoSigner`/`RemoveCoSigner`, ignored otherwise.
    /// Hex-encoded author id of who the decision is about. Found missing
    /// while wiring this to `fold.rs`, not anticipated: without a
    /// machine-readable subject, "who got admitted" lives only in free
    /// text `title`/`description` — unparseable, which defeats §3.9's own
    /// point for this whole layer ("software that can't be argued about
    /// whether quorum was met" has to extend to *what* was decided, not
    /// just whether enough people agreed). Named `subjectMember` in the
    /// lexicon to avoid colliding with atproto's `subject` convention,
    /// which `Signal.subject` below already uses for a different thing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_member: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// `network.essmesh.governance.signal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Stand-in for `com.atproto.repo.strongRef` (AT-URI + CID) until
    /// real record referencing exists. Convention adopted here rather
    /// than left unspecified: `"{proposal_author_hex}/{proposal_rkey}"` —
    /// enough to identify one `Proposal` unambiguously (namespace is
    /// already implicit in which document this is synced through;
    /// `RecordIdentifier` needs author+key, not just key, to name a
    /// record at all — see `namespace.rs`/`records.rs`). See
    /// `subject_ref`/`parse_subject_ref` below.
    pub subject: String,
    pub signal_type: SignalType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Record for Proposal {
    const COLLECTION: &'static str = "network.essmesh.governance.proposal";
}

impl Record for Signal {
    const COLLECTION: &'static str = "network.essmesh.governance.signal";
}

/// A founder's starting policy for all three governed classes —
/// `PolicyChange`'s shape, but non-optional: a `Founding` claim has to
/// fully specify where the namespace starts, not partially amend it the
/// way a `changePolicy` Proposal can.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundingPolicy {
    pub admit_co_signer: PolicyValue,
    pub remove_co_signer: PolicyValue,
    pub change_policy: PolicyValue,
}

impl FoundingPolicy {
    pub fn as_map(&self) -> HashMap<GovernanceClass, PolicyValue> {
        [
            (GovernanceClass::AdmitCoSigner, self.admit_co_signer),
            (GovernanceClass::RemoveCoSigner, self.remove_co_signer),
            (GovernanceClass::ChangePolicy, self.change_policy),
        ]
        .into_iter()
        .collect()
    }
}

/// `network.essmesh.governance.founding` — SPEC.md §6's founding-record
/// item: one founder's claim about how a namespace starts (who's
/// eligible, what the starting policy is), written at the same
/// per-author fixed key every founder uses (`FOUNDING_KEY`) so multiple
/// co-founders can each post their own claim without colliding —
/// `RecordIdentifier`'s `(namespace, author, key)` shape already makes
/// this safe against forgery the same way every other author-scoped
/// record in this crate is. See `resolve_founding` for how multiple (or
/// late/attempted-backdated) claims resolve into one genesis state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Founding {
    /// Hex-encoded author ids this founder is declaring eligible,
    /// themselves included. Decoding to a real author type is the
    /// caller's job (`fold.rs`), same split `Proposal.subject_member`
    /// already uses — this module stays generic over the author type
    /// and can't do that decoding itself.
    pub eligible: Vec<String>,
    pub policy: FoundingPolicy,
    pub created_at: DateTime<Utc>,
}

impl Record for Founding {
    const COLLECTION: &'static str = "network.essmesh.governance.founding";
}

/// Every founder writes their claim at this same key under their own
/// author — mirrors `records::NodeProfile::SELF_KEY`'s "one canonical
/// slot per author" convention.
pub const FOUNDING_KEY: &str = "self";

/// Default acceptance window for a founding claim to count — SPEC.md §6:
/// generous enough for a real multi-founder bootstrap conversation
/// (co-founders agreeing before anyone shares the namespace), short
/// enough that a member admitted later can't backdate their way into the
/// genesis eligible set by posting their own `Founding` claim months
/// after the fact. No protocol significance beyond that judgment call —
/// namespace-owned policy already covers everything *after* genesis;
/// this only governs what counts as genesis in the first place.
pub const DEFAULT_FOUNDING_WINDOW_SECONDS: i64 = 3600;

/// One decoded founding claim — `Founding`'s wire form with `eligible`
/// resolved from hex strings to real author ids and `policy` flattened
/// to the same map shape `fold`/`current_policy` already use. Building
/// this is `fold.rs`'s job (it has `namespace::decode_author_hex`); this
/// struct exists so `resolve_founding` can stay pure and generic over
/// the author type, same as every other function in this module.
#[derive(Debug, Clone)]
pub struct FoundingClaim<A> {
    pub claimant: A,
    pub eligible: HashSet<A>,
    pub policy: HashMap<GovernanceClass, PolicyValue>,
    pub created_at: DateTime<Utc>,
}

/// Resolves every `Founding` claim found in a namespace down to one
/// genesis state. `None` if `claims` is empty — a namespace with no
/// founding claim at all, either predating this record type or one that
/// skipped it; the caller decides what that means (SPEC.md §6), this
/// function won't invent a default.
///
/// The real question this answers isn't "what does the founder say" —
/// it's "which claims get to count as founding at all." Nothing stops a
/// member admitted long after genesis (anyone ever granted Write
/// capability can write under their own author key) from posting their
/// *own* `Founding` claim, trying to retroactively grant themselves
/// genesis-eligible status. `RecordIdentifier`'s shape (SPEC.md §3.4)
/// already stops them from forging a claim as someone else, but says
/// nothing about *when* a claim was made — so the fix is temporal: take
/// the earliest claim's `created_at` as t0, and only union eligibility
/// from claims within `window_seconds` of t0. A claim outside the window
/// is silently ignored for genesis purposes — its author needs a real
/// `AdmitCoSigner` Proposal instead, same as anyone else joining later.
/// Policy is taken only from the earliest claim — unlike eligibility,
/// starting policy isn't unioned; the first founder sets it, and later
/// co-founders' declared policy (if any) is ignored rather than
/// arbitrated, avoiding a conflicting-policy ambiguity this module isn't
/// going to resolve on anyone's behalf.
pub fn resolve_founding<A: Eq + std::hash::Hash + Clone>(
    claims: &[FoundingClaim<A>],
    window_seconds: i64,
) -> Option<(HashSet<A>, HashMap<GovernanceClass, PolicyValue>)> {
    let earliest = claims.iter().min_by_key(|c| c.created_at)?;
    let t0 = earliest.created_at;
    let policy = earliest.policy.clone();

    let eligible = claims
        .iter()
        .filter(|c| (c.created_at - t0).num_seconds() <= window_seconds)
        .flat_map(|c| c.eligible.iter().cloned().chain(std::iter::once(c.claimant.clone())))
        .collect();

    Some((eligible, policy))
}

/// Builds a `Signal.subject` value referencing one `Proposal` — see
/// `Signal::subject`'s doc comment for the convention.
pub fn subject_ref(proposal_author_hex: &str, proposal_rkey: &str) -> String {
    format!("{proposal_author_hex}/{proposal_rkey}")
}

/// Inverse of `subject_ref`. `None` if `subject` isn't in the expected
/// shape — callers should treat that as "doesn't reference anything this
/// fold understands," not as an error; a signal-shaped record from a
/// future, differently-shaped protocol revision shouldn't crash today's
/// reader.
pub fn parse_subject_ref(subject: &str) -> Option<(&str, &str)> {
    subject.split_once('/')
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
            subject_member: None,
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

    fn founding_policy() -> FoundingPolicy {
        FoundingPolicy {
            admit_co_signer: PolicyValue { window_seconds: 100, block_threshold: 1 },
            remove_co_signer: PolicyValue { window_seconds: 100, block_threshold: 1 },
            change_policy: PolicyValue { window_seconds: 100, block_threshold: 1 },
        }
    }

    fn claim(claimant: &'static str, eligible: &[&'static str], created_at: i64) -> FoundingClaim<&'static str> {
        FoundingClaim {
            claimant,
            eligible: eligible.iter().copied().collect(),
            policy: founding_policy().as_map(),
            created_at: at(created_at),
        }
    }

    #[test]
    fn no_claims_resolves_to_nothing() {
        let claims: Vec<FoundingClaim<&str>> = vec![];
        assert_eq!(resolve_founding(&claims, 3600), None);
    }

    #[test]
    fn a_single_founder_is_eligible_even_if_they_forgot_to_list_themselves() {
        let claims = vec![claim("alice", &[], 0)];
        let (eligible, _) = resolve_founding(&claims, 3600).unwrap();
        assert_eq!(eligible, ["alice"].into_iter().collect());
    }

    #[test]
    fn co_founders_within_the_window_are_unioned() {
        let claims = vec![
            claim("alice", &["bob"], 0),
            claim("bob", &["alice"], 100),
        ];
        let (eligible, _) = resolve_founding(&claims, 3600).unwrap();
        assert_eq!(eligible, ["alice", "bob"].into_iter().collect());
    }

    #[test]
    fn a_claim_outside_the_window_is_ignored_not_unioned() {
        let claims = vec![
            claim("alice", &[], 0),
            // mallory was granted write access much later and posts her
            // own founding claim, trying to backdate her way into the
            // genesis eligible set.
            claim("mallory", &[], 10_000),
        ];
        let (eligible, _) = resolve_founding(&claims, 3600).unwrap();
        assert_eq!(eligible, ["alice"].into_iter().collect());
    }

    #[test]
    fn policy_comes_from_the_earliest_claim_only() {
        let mut later = claim("bob", &[], 100);
        later.policy = HashMap::from([(
            GovernanceClass::AdmitCoSigner,
            PolicyValue { window_seconds: 999, block_threshold: 99 },
        )]);
        let claims = vec![claim("alice", &["bob"], 0), later];
        let (_, policy) = resolve_founding(&claims, 3600).unwrap();
        assert_eq!(
            policy.get(&GovernanceClass::AdmitCoSigner),
            Some(&PolicyValue { window_seconds: 100, block_threshold: 1 })
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
