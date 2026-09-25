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
//!
//! **Composable by construction, not by extension** (Jason's framing,
//! 2026-09-23: "tags are monads") — a `Tag` is itself an ordinary
//! `Record` in the `network.essmesh.tag` collection, and `subject` is a
//! generic `records::record_ref` that names *any* record in *any*
//! collection, including this one. Nothing stops a `Tag`'s `subject`
//! from being another `Tag`'s own `record_ref` — confirmed live in
//! `tests/tagging.rs`, not just true in principle — so tagging a tag,
//! reacting to a tag, or building a whole folksonomy on top of this one
//! primitive all fall out for free, no new record type per layer.
//!
//! **Namespaced labels, so built-in behavior and user-invented ontology
//! never collide.** `label` stays a plain free-text `String` — no schema
//! change — but this module reserves the `system:` prefix for labels
//! this crate itself gives meaning to (currently just `system:pin`,
//! below). Anything else — bare words, a user's own `topic:`/`mood:`/
//! whatever convention — is the open, user-extensible ontology Jason's
//! ask was about: this module never validates or restricts it, only
//! reserves its own corner of the namespace so a feature built here
//! can't be shadowed by an unrelated tag someone else invents.

use std::collections::HashMap;

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

/// Reserved label for the pinned-welcome-message feature (`DESIGN_BRIEF.
/// md`'s Layout Strategy) — "anyone can pin, one pin per author, shown
/// in order of recency" (Jason's exact spec, 2026-09-23). Lives under
/// the `system:` namespace reserved in this module's own top doc
/// comment, so a user inventing an unrelated tag literally named
/// "pin" for their own purposes can never collide with this behavior.
pub const PIN_LABEL: &str = "system:pin";

/// Collapses a tag list down to each author's single most recent tag
/// with the given `label`, newest first — the exact resolution rule a
/// "one per author, shown in order of recency" feature needs (pins
/// now; anything with the same shape later, which is why this takes a
/// `label` rather than being pin-specific). Same dedup shape as
/// `governance::latest_signal_per_author` — "only the most recent
/// counts" is already an established pattern in this crate, not a new
/// one invented for this feature.
pub fn latest_per_author_with_label(
    tags: &[(AuthorId, String, Tag)],
    label: &str,
) -> Vec<(AuthorId, String, Tag)> {
    let mut latest: HashMap<AuthorId, (AuthorId, String, Tag)> = HashMap::new();
    for (author, rkey, tag) in tags {
        if tag.label != label {
            continue;
        }
        match latest.get(author) {
            Some((_, _, existing)) if existing.created_at >= tag.created_at => {}
            _ => {
                latest.insert(*author, (*author, rkey.clone(), tag.clone()));
            }
        }
    }
    let mut result: Vec<_> = latest.into_values().collect();
    result.sort_by(|a, b| b.2.created_at.cmp(&a.2.created_at));
    result
}

/// Every currently-pinned subject in `doc`, one per author, most
/// recently pinned first — `list_all_tags` plus
/// `latest_per_author_with_label(PIN_LABEL)` unwrapped for a caller that
/// just wants "what's pinned right now." Scans every tag in the
/// namespace (same O(every tag) caveat `list_all_tags`'s own doc
/// comment already names), fine at reference-app scale.
pub async fn pins(node: &Node, doc: &Doc) -> Result<Vec<(AuthorId, String, Tag)>> {
    let tags = list_all_tags(node, doc).await?;
    Ok(latest_per_author_with_label(&tags, PIN_LABEL))
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
