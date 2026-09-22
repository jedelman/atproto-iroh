//! `network.essmesh.tag` — cross-lexicon tagging (Jason's ask, 2026-09-22:
//! "tags should cross lexicons"). A `Tag` is its own record type, not a
//! field bolted onto `Message` or any other type it might describe —
//! `Tag.subject` is a `records::record_ref` (`"{author_hex}/{collection}/
//! {rkey}"`), which is the one reference format in this crate that
//! actually names the collection, so one `Tag` type works for a message,
//! a document revision, a `NodeProfile`, a governance `Proposal` — any
//! record in any lexicon — without a per-type tagging mechanism for each.
//!
//! Append-only, same shape as `messaging.rs`: every tag mints its own key
//! (`new_entry_key`), so any number of taggers can tag the same or
//! different subjects without colliding. Deliberately *not* a mutation of
//! the tagged record itself — the record being tagged never needs to be
//! touched, and its own writer's capability isn't required to tag it (any
//! writer in the namespace can tag anything, same open-by-default
//! posture the inbox/messaging primitives already have).

use anyhow::Result;
use chrono::{DateTime, Utc};
use iroh_docs::{api::Doc, AuthorId};
use serde::{Deserialize, Serialize};

use crate::{
    namespace::{list_records, new_entry_key, put_record, Node},
    records::Record,
};

/// `network.essmesh.tag` — `lexicons/network/essmesh/tag.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// `records::record_ref` naming the tagged record, in any collection.
    pub subject: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

impl Record for Tag {
    const COLLECTION: &'static str = "network.essmesh.tag";
}

/// Tags `subject` (a `records::record_ref`) with `label`, minting a
/// fresh key — no coordination needed between taggers, same as
/// `messaging::send_message`.
pub async fn add_tag(doc: &Doc, author: AuthorId, subject: String, label: String) -> Result<String> {
    let tag = Tag { subject, label, created_at: Utc::now() };
    let rkey = new_entry_key();
    put_record(doc, author, &rkey, &tag).await?;
    Ok(rkey)
}

/// Every tag currently synced into `doc`, across every subject —
/// `list_records` unwrapped with no filtering; a caller that wants only
/// one subject's tags should filter this (or call `tags_for`, below),
/// same O(every tag in the namespace) caveat `fold::fold`'s
/// `list_proposals` caller already accepts at reference-app scale.
pub async fn list_all_tags(node: &Node, doc: &Doc) -> Result<Vec<(AuthorId, String, Tag)>> {
    list_records::<Tag>(node, doc).await
}

/// Every tag whose `subject` matches `subject` exactly — a client-side
/// filter over `list_all_tags`, not a server-side index (there's no
/// server); fine at the scale a reference app cares about, a real
/// per-subject index once a namespace has enough tags for the linear
/// scan to matter.
pub async fn tags_for(node: &Node, doc: &Doc, subject: &str) -> Result<Vec<(AuthorId, String, Tag)>> {
    Ok(list_all_tags(node, doc)
        .await?
        .into_iter()
        .filter(|(_, _, tag)| tag.subject == subject)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_serializes_with_a_plain_string_subject() {
        let tag = Tag {
            subject: "abcd/network.essmesh.chat.message/0001".into(),
            label: "important".into(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_value(&tag).unwrap();
        assert_eq!(
            json["subject"],
            "abcd/network.essmesh.chat.message/0001"
        );
        assert_eq!(json["label"], "important");
    }
}
