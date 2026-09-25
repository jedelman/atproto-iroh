//! `network.essmesh.chat.image` — the next batteries-included app
//! (CLAUDE.md's app-list section). Append-only, same shape as
//! `messaging.rs`: each upload mints its own key, so no collision, no
//! last-write-wins risk.
//!
//! **No separate blob-fetch step.** `iroh_blobs::api::Store::add_bytes`
//! exists and would mint a hash a peer would then have to separately
//! dial for — this module deliberately doesn't use it. Instead, an
//! image's raw bytes are written as a doc entry's own content via
//! `namespace::put_bytes`, the same mechanism `NodeProfile`, `Message`,
//! and every other record in this crate already relies on for content
//! to sync — `iroh-docs` transfers a doc entry's content blob alongside
//! its metadata as part of ordinary sync (the same "content lags
//! metadata" caveat `namespace::get_record`'s doc comment already
//! documents applies here too, nothing new). Confirms the app-list
//! section's own prediction: images needed no new sync primitive.
//!
//! Two entries per image, sharing one `rkey`: a typed `ImageMeta` record
//! (content type, length, caption — the searchable/listable part) at
//! the normal `{collection}/{rkey}` key, and the raw bytes at a
//! sibling freeform key. Splitting them keeps `list_images` cheap (it
//! never has to download image bytes just to show a caption list) while
//! still letting a caller fetch one image's bytes directly once they
//! decide they want it.

use anyhow::Result;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use iroh_docs::{api::Doc, AuthorId};
use serde::{Deserialize, Serialize};

use crate::{
    namespace::{get_bytes, list_records, new_entry_key, put_bytes, put_record, Node},
    records::Record,
};

/// `network.essmesh.chat.image` — `lexicons/network/essmesh/chat/image.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMeta {
    /// e.g. `"image/png"` — not validated against a fixed list; a
    /// reader that doesn't understand a given type can still show the
    /// caption and skip rendering, same "don't crash on the unknown"
    /// posture `namespace::EntryContent::Raw` already takes.
    pub content_type: String,
    pub len: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Record for ImageMeta {
    const COLLECTION: &'static str = "network.essmesh.chat.image";
}

/// The raw-bytes key sibling to an `ImageMeta` sharing the same `rkey` —
/// not itself a collection, since `put_bytes` doesn't use one.
fn blob_key(rkey: &str) -> String {
    format!("network.essmesh.chat.image.blob/{rkey}")
}

/// Uploads an image: writes its raw bytes, then a typed `ImageMeta`
/// pointing at the same `rkey`. Not atomic across the two writes (no
/// transaction spans two `iroh-docs` entries), but the failure mode is
/// contained — the same underlying safety `submit_text`'s own posture
/// relies on: nothing else's data can be corrupted by a partial upload,
/// only this record's own two entries can end up inconsistent (bytes
/// with no metadata, or vice versa if the first `.await?` fails).
pub async fn upload_image(
    doc: &Doc,
    author: AuthorId,
    bytes: Vec<u8>,
    content_type: String,
    caption: Option<String>,
) -> Result<String> {
    let rkey = new_entry_key();
    let len = bytes.len() as u64;
    put_bytes(doc, author, &blob_key(&rkey), bytes).await?;
    let meta = ImageMeta { content_type, len, caption, created_at: Utc::now() };
    put_record(doc, author, &rkey, &meta).await?;
    Ok(rkey)
}

/// Every image's metadata, oldest first — never touches image bytes, so
/// listing a gallery doesn't mean downloading every image in it first.
pub async fn list_images(node: &Node, doc: &Doc) -> Result<Vec<(AuthorId, String, ImageMeta)>> {
    let mut images = list_records::<ImageMeta>(node, doc).await?;
    images.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(images)
}

/// One image's raw bytes, once a caller actually wants to render it.
pub async fn load_image_bytes(
    node: &Node,
    doc: &Doc,
    author: AuthorId,
    rkey: &str,
) -> Result<Option<Bytes>> {
    get_bytes(node, doc, author, &blob_key(rkey)).await
}
