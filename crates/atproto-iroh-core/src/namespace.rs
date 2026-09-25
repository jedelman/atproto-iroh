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

use std::{path::Path, sync::Arc};

use anyhow::Result;
use iroh::{endpoint::presets, Endpoint, PublicKey, SecretKey};
use iroh_blobs::{api::Store as BlobStore, store::mem::MemStore, BlobsProtocol, ALPN as BLOBS_ALPN};
use iroh_docs::{
    api::{
        protocol::{AddrInfoOptions, ShareMode},
        Doc, DocsApi,
    },
    protocol::{Builder as DocsBuilder, Docs},
    store::Query,
    AuthorId, CapabilityKind, DocTicket, Entry, NamespaceId, ALPN as DOCS_ALPN,
};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};
use n0_future::StreamExt;
use tokio::sync::Notify;

use crate::{
    control::{self, ControlConfig, ResetToken, CONTROL_ALPN},
    identity::Identity,
    records::{key_for, Record},
};

/// A running local node: identity/transport (`iroh::Endpoint`), content
/// storage (`iroh-blobs`), and the docs engine, wired together the same
/// way `examples/iroh_docs_probe.rs` did it for the throwaway experiment
/// — this is that same wiring, kept for real use instead of discarded
/// after the probe.
pub struct Node {
    router: iroh::protocol::Router,
    blobs: BlobStore,
    docs: DocsApi,
}

/// Which `iroh::endpoint::presets` bundle a `Node` binds with —
/// CLAUDE.md's platform-priority section: `Minimal` (no relay, no
/// address-lookup service) is what every test and desktop default here
/// use, hermetic and fine on a LAN or between two processes on one
/// machine, but wrong for the platform this project actually prioritizes
/// first. Android phones sit behind CGNAT and switch networks constantly
/// (wifi to cellular and back), so direct QUIC isn't always reachable
/// even in the foreground — `N0` adds the relay fallback and DNS-based
/// address lookup that make a real connection possible when a direct one
/// isn't, at the cost of depending on n0.computer's relay/DNS
/// infrastructure rather than staying purely peer-to-peer. Not
/// unconditionally "the right default everywhere": SPEC.md's goal 1
/// ("zero ambient legibility") is about namespace content and capability
/// grants, not transport-layer relay use, but a relay operator can still
/// see connection metadata (who's talking to whom, roughly when) that a
/// pure direct-QUIC connection wouldn't expose — a real tradeoff, made
/// explicitly per-platform here rather than silently defaulted for
/// everyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPreset {
    /// No relay, no discovery service — direct QUIC only. Every test in
    /// this crate uses this; right for two processes that can already
    /// reach each other directly (localhost, a LAN, an org's own server
    /// on a routable address).
    Minimal,
    /// Relay fallback plus DNS-based address lookup (`presets::N0`) —
    /// the mobile-appropriate choice per CLAUDE.md's platform-priority
    /// section.
    N0,
}

impl Node {
    /// In-memory, throwaway identity and storage — nothing here survives
    /// process exit. What every test and the probe example use; real
    /// persistence is `spawn_persistent` below, which shares this
    /// function's wiring end to end (`spawn_inner`), not a parallel
    /// implementation of it. Binds with `NetworkPreset::Minimal` — a
    /// throwaway node has no real-world reachability problem to solve.
    pub async fn spawn() -> Result<Self> {
        let blobs = MemStore::default();
        Self::spawn_inner(
            SecretKey::generate(),
            (*blobs).clone(),
            Docs::memory(),
            NetworkPreset::Minimal,
            None,
        )
        .await
    }

    /// Real persistence: `identity`'s own secret key becomes the
    /// `iroh::Endpoint`'s actual key (not just a label — see this
    /// function's note below on the bug that was, before this), and both
    /// the docs store and blob store live under `data_dir` via
    /// `Docs::persistent`/`FsStore::load` rather than in memory.
    ///
    /// **Found while wiring this, not anticipated**: before this
    /// function existed, `Node::spawn()` always generated a random
    /// `SecretKey` internally, completely disconnected from whatever
    /// `Identity` a caller had generated or loaded to display a `did:iroh`
    /// — the DID shown to a person and the actual network identity of
    /// their node were two unrelated keys. Persisting `Identity` alone,
    /// without also using it to build the `Endpoint`, would have made
    /// that worse, not better: a *stable-looking* DID hiding a node whose
    /// real identity still changed every restart. Fixed here by taking
    /// the identity as a parameter and threading its key through to
    /// `Endpoint::builder(...).secret_key(...)`, which `spawn()` above
    /// also now does (with a throwaway key) for the same reason — one
    /// code path, so this can't silently regress for either caller.
    pub async fn spawn_persistent(identity: &Identity, data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::spawn_persistent_with_preset(identity, data_dir, NetworkPreset::Minimal).await
    }

    /// Same as `spawn_persistent`, with an explicit `NetworkPreset` —
    /// what a mobile host (CLAUDE.md's platform-priority section) should
    /// call with `NetworkPreset::N0` instead of relying on
    /// `spawn_persistent`'s `Minimal` default, which assumes direct
    /// reachability a phone on CGNAT often doesn't have.
    pub async fn spawn_persistent_with_preset(
        identity: &Identity,
        data_dir: impl AsRef<Path>,
        preset: NetworkPreset,
    ) -> Result<Self> {
        Self::spawn_persistent_inner(identity, data_dir, preset, None).await
    }

    /// Same as `spawn_persistent_with_preset`, with the control endpoint
    /// (`control` module) enabled — relay mode: the box's onboarding
    /// address is just its `did:iroh` (`identity.did()`), and anyone who
    /// can reach it can privately hand it a namespace ticket to join, or
    /// (with proof of the returned `ResetToken`) tell it to wipe itself.
    /// See `control.rs`'s module doc for the full design this implements
    /// and why `JOIN` and `RESET` have different authorization bars.
    /// Returns the reset signal alongside the node so a caller (the CLI's
    /// `serve` loop) can await it next to Ctrl+C and know when a remote
    /// `RESET` has wiped local data and the process should exit for a
    /// supervisor to restart it fresh.
    pub async fn spawn_relay(
        identity: &Identity,
        data_dir: impl AsRef<Path>,
        preset: NetworkPreset,
    ) -> Result<(Self, ResetToken, Arc<Notify>)> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let reset_token = ResetToken::load_or_generate(data_dir.join("reset-token"))?;
        let reset_signal = Arc::new(Notify::new());
        let config = ControlConfig {
            reset_token: reset_token.clone(),
            wipe_dir: data_dir.clone(),
            reset_signal: reset_signal.clone(),
        };
        let node = Self::spawn_persistent_inner(identity, &data_dir, preset, Some(config)).await?;
        Ok((node, reset_token, reset_signal))
    }

    async fn spawn_persistent_inner(
        identity: &Identity,
        data_dir: impl AsRef<Path>,
        preset: NetworkPreset,
        control: Option<ControlConfig>,
    ) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        let blobs_dir = data_dir.join("blobs");
        let docs_dir = data_dir.join("docs");
        // iroh-docs' persistent store opens `docs_dir/docs.redb` directly
        // and doesn't create `docs_dir` itself — found by hitting the
        // error on first run, not anticipated from the API alone.
        std::fs::create_dir_all(&blobs_dir)?;
        std::fs::create_dir_all(&docs_dir)?;
        let blobs = iroh_blobs::store::fs::FsStore::load(blobs_dir).await?;
        let docs_builder = Docs::persistent(docs_dir);
        Self::spawn_inner(identity.secret_key().clone(), (*blobs).clone(), docs_builder, preset, control).await
    }

    async fn spawn_inner(
        secret_key: SecretKey,
        blobs: BlobStore,
        docs_builder: DocsBuilder,
        preset: NetworkPreset,
        control: Option<ControlConfig>,
    ) -> Result<Self> {
        let endpoint = match preset {
            NetworkPreset::Minimal => Endpoint::builder(presets::Minimal).secret_key(secret_key),
            NetworkPreset::N0 => Endpoint::builder(presets::N0).secret_key(secret_key),
        }
        .bind()
        .await?;
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let docs = docs_builder
            .spawn(endpoint.clone(), blobs.clone(), gossip.clone())
            .await?;
        let mut router_builder = iroh::protocol::Router::builder(endpoint.clone())
            .accept(BLOBS_ALPN, BlobsProtocol::new(&blobs, None))
            .accept(GOSSIP_ALPN, gossip)
            .accept(DOCS_ALPN, docs.clone());
        if let Some(config) = &control {
            router_builder = router_builder.accept(
                CONTROL_ALPN,
                control::ControlHandler::new(docs.api().clone(), config),
            );
        }
        let router = router_builder.spawn();
        Ok(Self {
            router,
            blobs,
            docs: docs.api().clone(),
        })
    }

    /// Sends a control message directly to `target` — no capability
    /// needed, just its `did:iroh` public key. See `control.rs` for the
    /// two supported requests (`"JOIN <ticket>"`, `"RESET <token>"`) and
    /// the design this implements.
    pub async fn send_control(&self, target: PublicKey, message: &str) -> Result<String> {
        control::send_control_message(self.router.endpoint(), target, message).await
    }

    /// This node's own full current network address (relay URL and any
    /// direct socket addresses it currently has), not just its bare
    /// `did:iroh` public key. Exists for dialing a target explicitly when
    /// bare-key address-lookup discovery isn't available — see
    /// `tests/control.rs`'s doc comment for why the live tests need this
    /// (this sandbox has no outbound reachability to n0.computer's actual
    /// relay/DNS infrastructure, so `NetworkPreset::N0`'s discovery can't
    /// be exercised here even though it's the intended production path).
    pub fn endpoint_addr(&self) -> iroh::EndpointAddr {
        self.router.endpoint().addr()
    }

    /// Same as [`Node::send_control`] but dials an explicit
    /// [`iroh::EndpointAddr`] instead of relying on discovery to resolve a
    /// bare public key. See `endpoint_addr`'s doc comment for why this
    /// exists.
    pub async fn send_control_to(&self, target: iroh::EndpointAddr, message: &str) -> Result<String> {
        control::send_control_message_to(self.router.endpoint(), target, message).await
    }

    pub fn docs(&self) -> &DocsApi {
        &self.docs
    }

    pub(crate) fn blob_store(&self) -> BlobStore {
        self.blobs.clone()
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
        let doc = self.docs.create().await?;
        // Live from the start, not only once shared — see open_namespace.
        doc.start_sync(Vec::new()).await?;
        Ok(doc)
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

    /// Reopens a namespace this node already holds a capability into —
    /// the missing piece for persistence: on a fresh process, nothing in
    /// memory remembers which namespaces were open before, but a
    /// persistent docs store (`spawn_persistent`) still has every
    /// capability this node was ever granted. `list_local_namespaces`
    /// finds the ids; this reopens one of them.
    ///
    /// Also starts live sync. iroh-docs' `open` only opens the local
    /// replica, and a namespace outside its live set turns incoming
    /// peers away (`AbortReason::NotFound`), so without this a restarted
    /// phone or relay stopped syncing everything it held until something
    /// happened to call `share`. `start_sync` with no explicit peers
    /// still reconnects to the ones iroh-docs remembers as useful for
    /// this namespace. Proven across a real restart in
    /// `tests/reopen_sync.rs`.
    pub async fn open_namespace(&self, id: NamespaceId) -> Result<Option<Doc>> {
        let Some(doc) = self.docs.open(id).await? else {
            return Ok(None);
        };
        doc.start_sync(Vec::new()).await?;
        Ok(Some(doc))
    }

    /// Every namespace this node currently holds *any* capability into —
    /// on a persistent node, this is what survives a restart; on a
    /// memory node, it's whatever's been created or joined so far this
    /// process. Read/write is `DocsApi::list()`'s own `CapabilityKind`,
    /// passed through rather than collapsed, since a caller reopening
    /// namespaces after restart needs to know which capability it's
    /// getting back.
    pub async fn list_local_namespaces(&self) -> Result<Vec<(NamespaceId, CapabilityKind)>> {
        let stream = self.docs.list().await?;
        tokio::pin!(stream);
        let mut out = Vec::new();
        while let Some(item) = stream.next().await {
            out.push(item?);
        }
        Ok(out)
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

/// Writes plain text at a caller-chosen key — no `Record` impl, no
/// lexicon, no collection prefix. The freeform half of the namespace
/// model: everything above this function treats a namespace as a bag of
/// typed collections, but nothing about `iroh-docs` — or this crate —
/// actually requires that. A namespace is a capability-scoped, synced,
/// multi-writer key/value space; typed records are one way to use that,
/// shared mutable text at whatever key a person picks is another. Same
/// sync, same capability model, same `dump_all` inspector (which reads
/// this back as `EntryContent::Text`), no new machinery.
pub async fn put_text(
    doc: &Doc,
    author: AuthorId,
    key: &str,
    text: &str,
) -> Result<iroh_blobs::Hash> {
    Ok(doc
        .set_bytes(author, key.as_bytes().to_vec(), text.as_bytes().to_vec())
        .await?)
}

/// Reads freeform text back. Fails (rather than returning `None`) if the
/// entry exists but isn't valid UTF-8 — unlike `dump_all`, which falls
/// back to hex for exactly that case, this function is for a caller who
/// specifically expects text and wants to know if that expectation was
/// wrong, not to silently paper over it.
pub async fn get_text(
    node: &Node,
    doc: &Doc,
    author: AuthorId,
    key: &str,
) -> Result<Option<String>> {
    let Some(entry) = doc.get_exact(author, key.as_bytes().to_vec(), false).await? else {
        return Ok(None);
    };
    let bytes = node.blob_store().get_bytes(entry.content_hash()).await?;
    Ok(Some(String::from_utf8(bytes.to_vec())?))
}

/// Writes arbitrary bytes at a caller-chosen key — `put_text` without
/// the UTF-8 requirement. Exists for `images.rs`: an image's raw bytes
/// aren't text, but they're still just content at a key, same sync and
/// capability machinery as everything else. Deliberately *not* using
/// `iroh_blobs`' own out-of-band blob-add API (`Store::add_bytes`,
/// which mints a hash a peer would then have to separately fetch by
/// dialing for it) — writing through `Doc::set_bytes` instead means the
/// bytes are a doc entry's content, which `iroh-docs` already syncs
/// alongside the entry metadata by the same mechanism every other
/// record in this crate relies on (the same content-lags-metadata
/// caveat `get_record`'s doc comment already documents, nothing new).
/// One primitive, not two.
pub async fn put_bytes(
    doc: &Doc,
    author: AuthorId,
    key: &str,
    bytes: Vec<u8>,
) -> Result<iroh_blobs::Hash> {
    Ok(doc.set_bytes(author, key.as_bytes().to_vec(), bytes).await?)
}

/// Reads raw bytes back from an exact key — `get_text` without the
/// UTF-8 decode.
pub async fn get_bytes(
    node: &Node,
    doc: &Doc,
    author: AuthorId,
    key: &str,
) -> Result<Option<bytes::Bytes>> {
    let Some(entry) = doc.get_exact(author, key.as_bytes().to_vec(), false).await? else {
        return Ok(None);
    };
    Ok(Some(node.blob_store().get_bytes(entry.content_hash()).await?))
}

/// Submits text under a fresh, auto-generated key beneath `prefix` —
/// the "public inbox" primitive: any number of strangers holding a write
/// ticket can each call this without coordinating on a key, because
/// `new_entry_key` gives every call its own. What it does *not* do,
/// deliberately: nothing about rate-limiting, moderation, or who gets to
/// read the inbox back — a namespace opened this widely is exactly the
/// case where flooding stops being unlikely (SPEC.md §6 item 12's
/// resource-attack case), and this function's job is the write path
/// only, not a policy about it.
pub async fn submit_text(
    doc: &Doc,
    author: AuthorId,
    prefix: &str,
    text: &str,
) -> Result<String> {
    let rkey = new_entry_key();
    let key = format!("{prefix}/{rkey}");
    put_text(doc, author, &key, text).await?;
    Ok(key)
}

/// A revision key under `{doc_id}/rev/` — see `save_document_revision`.
fn revision_key(doc_id: &str, rev: &str) -> String {
    format!("{doc_id}/rev/{rev}")
}

/// Writes a new, immutable revision of a document rather than overwriting
/// a shared key in place. **Why this exists, not `put_text`**: `put_text`
/// writes every edit to the *same* `(namespace, author, key)` entry, and
/// `iroh-docs` resolves two writes to that exact tuple by its own `Record`
/// ordering (timestamp, then hash) — last write wins, and the loser is
/// gone, not merged. That's silent and lossless-looking right up until two
/// people edit the same shared doc while offline from each other: each
/// sees their own edit locally, and when they finally reconnect, one
/// edit vanishes with no conflict, no warning, nothing in the UI to show
/// it ever existed. Fine for genuinely single-writer text (notes only one
/// person ever touches) or content where "latest wins" is actually the
/// desired semantics (a status line); wrong for anything meant as a real
/// shared "Google doc without Google."
///
/// This sidesteps the problem the same way `submit_text` sidesteps
/// inbox-submission collisions: give every write its own key
/// (`new_entry_key`, sortable by creation time) instead of sharing one.
/// Nothing is ever overwritten, so nothing is ever silently lost — two
/// offline edits to the same document become two revisions that both
/// survive sync, at the cost of the caller (or a human) having to decide
/// what "current" means when more than one revision lands close together.
/// `load_document` below picks the latest by key as a default, not a
/// claim that it's automatically the "right" one to keep.
pub async fn save_document_revision(
    doc: &Doc,
    author: AuthorId,
    doc_id: &str,
    text: &str,
) -> Result<String> {
    let rev = new_entry_key();
    put_text(doc, author, &revision_key(doc_id, &rev), text).await?;
    Ok(rev)
}

/// Every revision of `doc_id` currently synced, oldest first (sorted by
/// `new_entry_key`'s own sortable form, which is also `iroh-docs`' key
/// sort order since it's a zero-padded decimal string). A caller that
/// wants to show "someone else edited this while you were offline —
/// here's both versions" reads this, not just `load_document`.
pub async fn list_document_revisions(
    node: &Node,
    doc: &Doc,
    doc_id: &str,
) -> Result<Vec<(AuthorId, String, String)>> {
    let prefix = format!("{doc_id}/rev/").into_bytes();
    let stream = doc.get_many(Query::key_prefix(prefix)).await?;
    tokio::pin!(stream);
    let mut out = Vec::new();
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let author = entry.author();
        let key = std::str::from_utf8(entry.key())?;
        let rev = key
            .rsplit('/')
            .next()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("malformed revision key {key:?}"))?;
        let bytes = node.blob_store().get_bytes(entry.content_hash()).await?;
        let text = String::from_utf8(bytes.to_vec())?;
        out.push((author, rev, text));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

/// The latest revision of `doc_id` by key order — a convenience default
/// for "what should I show right now," not a claim that a later
/// timestamp is semantically correct when two people edited concurrently.
/// A caller that cares about that distinction should call
/// `list_document_revisions` directly and decide.
pub async fn load_document(
    node: &Node,
    doc: &Doc,
    doc_id: &str,
) -> Result<Option<(String, AuthorId, String)>> {
    let mut revisions = list_document_revisions(node, doc, doc_id).await?;
    Ok(revisions.pop().map(|(author, rev, text)| (rev, author, text)))
}

/// Parses a hex-encoded `AuthorId` — the inverse of the
/// `hex::encode(author.as_bytes())` convention used throughout this
/// crate (`governance.rs`'s `subject_ref`, `Proposal.subject_member`) and
/// by any UI that has to accept one as user input. `None` rather than an
/// error on anything malformed — callers decide whether that's worth
/// surfacing.
pub fn decode_author_hex(hex_str: &str) -> Option<AuthorId> {
    let bytes = hex::decode(hex_str).ok()?;
    let array: [u8; 32] = bytes.try_into().ok()?;
    Some(AuthorId::from(&array))
}

/// A sortable, timestamp-derived key. **Not** a real atproto TID (that's
/// a specific base32-sortable, clock-and-counter scheme this doesn't
/// implement) — good enough for uniqueness and creation-order sorting
/// within this scaffold, not yet interoperable with real atproto
/// tooling. Named plainly rather than `tid()` so nobody mistakes it for
/// the real thing. Shared by `fold.rs`'s `propose`/`signal` and
/// `submit_text` above — any caller that needs "a fresh, sortable key,
/// nobody else will pick the same one" wants this same primitive.
pub fn new_entry_key() -> String {
    format!("{:019}", chrono::Utc::now().timestamp_micros())
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
    /// Parsed successfully — every lexicon-typed record this crate
    /// writes lands here, since they're all JSON.
    Json(serde_json::Value),
    /// Valid UTF-8 but not JSON — `put_text`'s freeform writes land here.
    /// A namespace isn't only ever a bag of typed records; SPEC.md §5's
    /// "almost nothing past §3.1 is actually ESS-specific" already said
    /// this substrate generalizes, and this is that made concrete: shared
    /// mutable text at any key, no schema, no `Record` impl required —
    /// the same sync/capability machinery, used as a plain document
    /// instead of a typed collection.
    Text(String),
    /// Neither of the above (or the blob hadn't finished downloading —
    /// see `get_record`'s note on content lagging metadata; a `Raw` entry
    /// the caller expected to be JSON or text is worth a retry, not
    /// necessarily proof of anything about the record itself).
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
                Err(_) => match String::from_utf8(bytes.to_vec()) {
                    Ok(text) => EntryContent::Text(text),
                    Err(_) => EntryContent::Raw {
                        hex: hex::encode(&bytes),
                    },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entry_key_is_monotonic_enough_to_sort_by() {
        let a = new_entry_key();
        let b = new_entry_key();
        assert!(b >= a);
    }
}
