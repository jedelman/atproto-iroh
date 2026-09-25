//! `network.essmesh.chat.message` — the first of the batteries-included
//! apps CLAUDE.md's app-list section recommended, and the natural one to
//! build first per its own reasoning: append-only by nature (each
//! message is its own key, minted the same way `submit_text`/
//! `fold::propose`/`fold::signal` already avoid collisions), so it needs
//! no new sync primitive at all — unlike the "Shared doc" primitive,
//! there's no last-write-wins risk here to design around.
//!
//! A namespace *is* the channel — no separate "room" concept layered on
//! top. Consistent with this crate's running theme (`namespace.rs`'s own
//! doc comment on `put_text`/`submit_text`): a namespace is a
//! capability-scoped, synced, multi-writer space, and a chat is just one
//! more way of using it, not a structure bolted on beside it.
//!
//! New Rust module, not code folded into `records.rs` or `namespace.rs`
//! — CLAUDE.md's extension-model decision: a new content type gets its
//! own module and (eventually) its own UI section, same shape
//! governance got, not a dynamically-loaded schema or a grab-bag file.

use anyhow::Result;
use chrono::{DateTime, Utc};
use iroh_docs::{api::Doc, AuthorId};
use serde::{Deserialize, Serialize};

use crate::{
    namespace::{list_records, new_entry_key, put_record, Node},
    records::Record,
};

/// `network.essmesh.chat.message` — `lexicons/network/essmesh/chat/message.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
    /// `"{author_hex}/{rkey}"` naming another `Message` this one replies
    /// to — the same convention `governance::subject_ref` uses for
    /// `Signal.subject`, restated locally rather than made a shared
    /// function across two otherwise-unrelated modules for one string
    /// format. `RecordIdentifier`'s `(namespace, author, key)` shape
    /// (SPEC.md §3.4) is what makes `rkey` alone ambiguous without the
    /// author half, same reasoning as everywhere else in this crate that
    /// needs to reference one specific record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Record for Message {
    const COLLECTION: &'static str = "network.essmesh.chat.message";
}

/// Builds a `Message.reply_to` value. See `Message::reply_to`'s doc
/// comment for the convention.
pub fn reply_ref(author_hex: &str, rkey: &str) -> String {
    format!("{author_hex}/{rkey}")
}

/// Posts a message under `author`, minting its own key — no
/// coordination needed between senders, same as `submit_text`'s inbox
/// case. Returns the `rkey` so a caller can build a `reply_to` for a
/// later message.
pub async fn send_message(
    doc: &Doc,
    author: AuthorId,
    text: String,
    reply_to: Option<String>,
) -> Result<String> {
    let message = Message { text, reply_to, created_at: Utc::now() };
    let rkey = new_entry_key();
    put_record(doc, author, &rkey, &message).await?;
    Ok(rkey)
}

/// Every message currently synced into `doc`, oldest first —
/// `list_records` doesn't sort on its own (SPEC.md §3.3: it's the union
/// of independently-published records, no ordering promised), so this
/// wrapper sorts by `rkey`, which is `new_entry_key`'s sortable form —
/// the same "sort by key, not by a separately-tracked index" approach
/// `namespace::list_document_revisions` already uses.
pub async fn list_messages(node: &Node, doc: &Doc) -> Result<Vec<(AuthorId, String, Message)>> {
    let mut messages = list_records::<Message>(node, doc).await?;
    messages.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_ref_matches_the_authorhex_slash_rkey_convention() {
        assert_eq!(reply_ref("abcd", "0001"), "abcd/0001");
    }
}
