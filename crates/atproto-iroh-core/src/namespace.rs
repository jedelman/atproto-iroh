//! A namespace ("Eleanor's" in SPEC.md's running example) as an
//! `iroh-docs` document, plus typed record read/write over
//! `records::Record`. Mechanics validated live in
//! `examples/iroh_docs_probe.rs` before any of this was written —
//! see SPEC.md §3.4's validation note and §6 items 2/3/10 for what was
//! actually checked against the crate rather than assumed.
//!
//! One structural reminder worth keeping close while reading this file:
//! `iroh_docs::Capability::Write` is a single secret shared by every
//! writer in a namespace, not a per-peer credential (SPEC.md §3.4/§6 item
//! 12). Nothing here pretends otherwise — `Node::author_create` mints a
//! signing identity for *entries*, which is a different thing from *who
//! can write at all*.

use anyhow::Result;
use iroh::{endpoint::presets, Endpoint};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol, ALPN as BLOBS_ALPN};
use iroh_docs::{
    api::{
        protocol::{AddrInfoOptions, ShareMode},
        Doc, DocsApi,
    },
    protocol::Docs,
    store::Query,
    AuthorId, DocTicket, Entry, ALPN as DOCS_ALPN,
};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};
use n0_future::StreamExt;

use crate::records::{key_for, Record};

/// A running local node: identity/transport (`iroh::Endpoint`), content
/// storage (`iroh-blobs`), and the docs engine, wired together the same
/// way `examples/iroh_docs_probe.rs` did it for the throwaway experiment
/// — this is that same wiring, kept for real use instead of discarded
/// after the probe.
pub struct Node {
    router: iroh::protocol::Router,
    blobs: MemStore,
    docs: DocsApi,
}

impl Node {
    /// `presets::Minimal` — no relay/discovery dependency, matching the
    /// probe. A persistent deployment will want `presets::N0` (or
    /// whatever real discovery this design ends up using) and
    /// `Docs::persistent` instead of `Docs::memory` below; both are
    /// swap-ins, not a different architecture.
    pub async fn spawn() -> Result<Self> {
        let endpoint = Endpoint::bind(presets::Minimal).await?;
        let blobs = MemStore::default();
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let docs = Docs::memory()
            .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
            .await?;
        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(BLOBS_ALPN, BlobsProtocol::new(&blobs, None))
            .accept(GOSSIP_ALPN, gossip)
            .accept(DOCS_ALPN, docs.clone())
            .spawn();
        Ok(Self {
            router,
            blobs,
            docs: docs.api().clone(),
        })
    }

    pub fn docs(&self) -> &DocsApi {
        &self.docs
    }

    fn blob_store(&self) -> iroh_blobs::api::Store {
        (*self.blobs).clone()
    }

    /// Mints a new `network.essmesh.node.*` record-signing identity.
    /// Distinct from `identity::Identity` (the node's own `did:iroh`) —
    /// see this module's top-level note and `identity.rs`'s.
    pub async fn author_create(&self) -> Result<AuthorId> {
        Ok(self.docs.author_create().await?)
    }

    /// Creates a brand-new namespace and grants this node its write
    /// capability — SPEC.md §3.4/§3.7.2: nobody else has an edge into it
    /// yet, the founder holds the whole `NamespaceSecret` until they
    /// share it.
    pub async fn create_namespace(&self) -> Result<Doc> {
        Ok(self.docs.create().await?)
    }

    /// A ticket for `mode`, ready to hand to whoever's being granted
    /// access — `Doc::share`/`DocsApi::import` are exactly the probe's
    /// grant/join path, unchanged.
    pub async fn share(&self, doc: &Doc, mode: ShareMode) -> Result<DocTicket> {
        Ok(doc.share(mode, AddrInfoOptions::Addresses).await?)
    }

    /// Imports a namespace from a ticket and starts sync — confirmed live
    /// (§6 item 10) to backfill full history, not just future writes.
    pub async fn join(&self, ticket: DocTicket) -> Result<Doc> {
        Ok(self.docs.import(ticket).await?)
    }

    pub async fn shutdown(self) {
        let _ = self.router.shutdown().await;
    }
}

/// Writes a typed record at `{collection}/{rkey}` — `records::key_for`.
/// Returns the content hash `Doc::set_bytes` already gives back; callers
/// don't need `Node`'s blob store for this direction.
pub async fn put_record<R: Record>(
    doc: &Doc,
    author: AuthorId,
    rkey: &str,
    record: &R,
) -> Result<iroh_blobs::Hash> {
    let bytes = serde_json::to_vec(record)?;
    Ok(doc.set_bytes(author, key_for::<R>(rkey), bytes).await?)
}

/// Reads one typed record back. Needs `node`'s blob store because a
/// synced `Entry` only carries a content hash + length — the bytes
/// themselves live in the blobs store, fetched separately (checked
/// against the real API rather than assumed; `Entry` has no
/// `content_bytes()` of its own).
///
/// Found live in `tests/namespace_records.rs`, not anticipated: entry
/// *metadata* and the content *blob* it references sync on separate
/// schedules, and the blob lags. `doc.get_exact` can return `Some` for an
/// entry whose content hasn't finished downloading yet — reading it too
/// early doesn't hang, it fails (`LeafHashMismatch`, from a partial read).
/// Callers polling for a record to arrive need to treat that error the
/// same as "not synced yet," not as a real failure — see the test's
/// retry loop for the shape of that.
pub async fn get_record<R: Record>(
    node: &Node,
    doc: &Doc,
    author: AuthorId,
    rkey: &str,
) -> Result<Option<R>> {
    let Some(entry) = doc.get_exact(author, key_for::<R>(rkey), false).await? else {
        return Ok(None);
    };
    Ok(Some(read_entry(node, &entry).await?))
}

/// All entries of one record type across every author currently included
/// in this namespace — SPEC.md §3.3: "the union of included members'
/// individually-signed records," not a single converged answer. Returns
/// the `rkey` alongside author and record, not just the record — needed
/// by anything that has to name a specific entry afterward (a strongRef,
/// a `governance::subject_ref`, an update to the same key), which
/// dropping it would make impossible to do correctly.
pub async fn list_records<R: Record>(
    node: &Node,
    doc: &Doc,
) -> Result<Vec<(AuthorId, String, R)>> {
    let prefix = format!("{}/", R::COLLECTION).into_bytes();
    let stream = doc.get_many(Query::key_prefix(prefix)).await?;
    tokio::pin!(stream);
    let mut out = Vec::new();
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let author = entry.author();
        let rkey = rkey_of::<R>(&entry)?;
        let record: R = read_entry(node, &entry).await?;
        out.push((author, rkey, record));
    }
    Ok(out)
}

/// Strips `{collection}/` off an entry's key to recover the `rkey` —
/// inverse of `records::key_for`.
fn rkey_of<R: Record>(entry: &Entry) -> Result<String> {
    let key = std::str::from_utf8(entry.key())?;
    let prefix = format!("{}/", R::COLLECTION);
    key.strip_prefix(&prefix)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("entry key {key:?} missing expected prefix {prefix:?}"))
}

async fn read_entry<R: Record>(node: &Node, entry: &Entry) -> Result<R> {
    let bytes = node.blob_store().get_bytes(entry.content_hash()).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// One synced entry, read generically — no `Record` type parameter, no
/// knowledge of what collection it belongs to. This is the whole trick
/// behind a "naive reflective" inspector: Rust has no runtime reflection,
/// but every record here happens to be JSON, so "try to parse the bytes,
/// fall back to hex" gets the same practical result — one view that
/// covers `NodeProfile`, `Proposal`, anything added later, with no new
/// per-type UI code ever needed. An inspector, not a claim that this
/// *is* reflection.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RawEntry {
    /// Hex-encoded, matching `governance.rs`'s convention elsewhere in
    /// this crate — readable in a generic inspector, where `AuthorId`'s
    /// own derived `Serialize` (a bare `[u8; 32]`) wouldn't be.
    pub author_hex: String,
    /// Full key, `{collection}/{rkey}` — unlike `list_records`, this
    /// function doesn't know the collection ahead of time to strip it.
    pub key: String,
    /// Microseconds since the Unix epoch — `iroh_docs::sync::Record`'s
    /// own unit, passed through rather than converted, since an
    /// inspector should show what's actually stored, not a
    /// reinterpretation of it.
    pub timestamp_micros: u64,
    pub content_len: u64,
    pub content: EntryContent,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum EntryContent {
    /// Parsed successfully — every record type this crate writes lands
    /// here, since they're all JSON.
    Json(serde_json::Value),
    /// Didn't parse as JSON (or the blob hadn't finished downloading —
    /// see `get_record`'s note on content lagging metadata; a `Raw`
    /// entry the caller expected to be JSON is worth a retry, not
    /// necessarily proof of a non-JSON record).
    Raw { hex: String },
}

/// Every entry currently in `doc`, across every collection and author —
/// the raw material for a generic state inspector. Deliberately returns
/// `Vec<RawEntry>` rather than anything collection-specific; building a
/// UI that groups/filters/labels these is real UX work this function
/// doesn't do on the caller's behalf.
pub async fn dump_all(node: &Node, doc: &Doc) -> Result<Vec<RawEntry>> {
    let stream = doc.get_many(Query::all()).await?;
    tokio::pin!(stream);
    let mut out = Vec::new();
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let key = String::from_utf8_lossy(entry.key()).into_owned();
        let content = match node.blob_store().get_bytes(entry.content_hash()).await {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(value) => EntryContent::Json(value),
                Err(_) => EntryContent::Raw {
                    hex: hex::encode(&bytes),
                },
            },
            // Content blob still downloading (see read_entry's note) or
            // genuinely unavailable — either way, nothing to show yet.
            Err(_) => EntryContent::Raw {
                hex: String::new(),
            },
        };
        out.push(RawEntry {
            author_hex: hex::encode(entry.author().as_bytes()),
            key,
            timestamp_micros: entry.timestamp(),
            content_len: entry.content_len(),
            content,
        });
    }
    Ok(out)
}
