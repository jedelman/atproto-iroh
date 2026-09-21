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
/// individually-signed records," not a single converged answer.
pub async fn list_records<R: Record>(
    node: &Node,
    doc: &Doc,
) -> Result<Vec<(AuthorId, R)>> {
    let prefix = format!("{}/", R::COLLECTION).into_bytes();
    let stream = doc.get_many(Query::key_prefix(prefix)).await?;
    tokio::pin!(stream);
    let mut out = Vec::new();
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let author = entry.author();
        let record: R = read_entry(node, &entry).await?;
        out.push((author, record));
    }
    Ok(out)
}

async fn read_entry<R: Record>(node: &Node, entry: &Entry) -> Result<R> {
    let bytes = node.blob_store().get_bytes(entry.content_hash()).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
