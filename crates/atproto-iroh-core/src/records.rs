//! Typed records over a namespace's `iroh-docs` document — SPEC.md §3.3's
//! `network.essmesh.node.*` lexicons, read/written as ordinary JSON at an
//! atproto-shaped path.
//!
//! Key scheme: `{collection}/{rkey}` as the raw `iroh-docs` key bytes.
//! `RecordIdentifier` is already `(namespace, author, key)` — SPEC.md
//! §3.4's validation note confirmed convergence resolves on that whole
//! tuple, not per bare key — so there's no need to also prefix the key
//! with the author's DID the way §3.4's original text sketched
//! (`<member-did>/profile`); the author is already structurally part of
//! the identifier. `record.rs`'s job stops at (de)serializing typed
//! Rust values to/from those key/value pairs; `namespace.rs` does the
//! actual `iroh-docs` I/O.

use serde::{de::DeserializeOwned, Serialize};

use chrono::{DateTime, Utc};

pub trait Record: Serialize + DeserializeOwned {
    /// The lexicon NSID this record type is defined under.
    const COLLECTION: &'static str;
}

/// `{collection}/{rkey}` as raw bytes for `Doc::set_bytes`/`get_many`.
pub fn key(collection: &str, rkey: &str) -> Vec<u8> {
    format!("{collection}/{rkey}").into_bytes()
}

pub fn key_for<R: Record>(rkey: &str) -> Vec<u8> {
    key(R::COLLECTION, rkey)
}

/// `network.essmesh.node.profile` — `lexicons/network/essmesh/node/profile.json`.
/// Single record per repo, key `self` (the lexicon's `"key": "literal:self"`).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct NodeProfile {
    pub name: String,
    pub category: NodeCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neighborhood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Not authoritative — see the lexicon's own description and
    /// `governance.rs`'s fold over the real governance history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_eligible: Option<bool>,
    pub created_at: DateTime<Utc>,
}

impl Record for NodeProfile {
    const COLLECTION: &'static str = "network.essmesh.node.profile";
}

impl NodeProfile {
    pub const SELF_KEY: &'static str = "self";
}

/// Mirrors `profile.json`'s `category.knownValues` verbatim — SPEC.md §3.3
/// and `lexicons/README.md`: must match `street-smarts`'s
/// `tools/sbci/sbci/ess_source.py`'s `CATEGORY_WEIGHTS` keys exactly, kept
/// in sync by hand (no automated check across repos yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    CoopHousingClt,
    WorkerCoop,
    WorkerCoopEthicalFinance,
    PublicCooperativeFacility,
    StandardCommercial,
}

/// `network.essmesh.node.event` — `lexicons/network/essmesh/node/event.json`.
/// Many per repo, keyed by `tid` (caller-supplied — this module doesn't
/// mint tids itself).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct NodeEvent {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<DateTime<Utc>>,
    /// Free text on purpose — see the lexicon's own description on why
    /// this isn't structured geo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Record for NodeEvent {
    const COLLECTION: &'static str = "network.essmesh.node.event";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_scheme_matches_the_lexicon_collection_ids() {
        assert_eq!(
            key_for::<NodeProfile>(NodeProfile::SELF_KEY),
            b"network.essmesh.node.profile/self".to_vec()
        );
    }

    #[test]
    fn category_serializes_to_the_lexicon_s_known_values() {
        let json = serde_json::to_string(&NodeCategory::WorkerCoopEthicalFinance).unwrap();
        assert_eq!(json, "\"worker_coop_ethical_finance\"");
    }
}
