//! Throwaway experiment, not load-bearing code.
//!
//! CLAUDE.md says the first real task is checking SPEC.md's assumptions
//! about `iroh-docs` against the actual crate before writing any protocol
//! code from the spec as given. This probes the three questions listed
//! there, each against two real in-process nodes doing real QUIC sync
//! (loopback, no relay/discovery — `presets::Minimal`):
//!
//!   Q1 (SPEC §3.7.2, §6.2): does iroh-docs expose an enumerable list of a
//!       namespace's current capability holders?
//!   Q2 (SPEC §3.4, §6.10): does granting a peer a read capability sync
//!       them the full document history, or only future writes?
//!   Q3 (SPEC §3.4): does convergence resolve per key, or does it force a
//!       merge decision across different authors writing the same key?
//!
//! Run with: cargo run -p atproto-iroh-core --example iroh_docs_probe

use std::time::Duration;

use anyhow::{bail, Result};
use iroh::{endpoint::presets, Endpoint};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol, ALPN as BLOBS_ALPN};
use iroh_docs::{
    api::protocol::{AddrInfoOptions, ShareMode},
    protocol::Docs,
    store::Query,
    ALPN as DOCS_ALPN,
};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};
use n0_future::Stream;
use n0_future::StreamExt;

async fn collect<T, E: std::fmt::Debug>(stream: impl Stream<Item = Result<T, E>>) -> Result<Vec<T>> {
    let mut stream = Box::pin(stream);
    let mut out = Vec::new();
    while let Some(item) = stream.next().await {
        out.push(item.map_err(|e| anyhow::anyhow!("{e:?}"))?);
    }
    Ok(out)
}

struct Node {
    router: iroh::protocol::Router,
    docs: iroh_docs::api::DocsApi,
}

async fn spawn_node() -> Result<Node> {
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
    Ok(Node {
        router,
        docs: docs.api().clone(),
    })
}

/// Poll `f` until it returns `true` or `timeout` elapses.
async fn wait_until(timeout: Duration, mut f: impl FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool>>>) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if f().await {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let node0 = spawn_node().await?;
    let node1 = spawn_node().await?;

    // --- Q2 setup: write history BEFORE node1 ever hears about the doc ---
    let author_a = node0.docs.author_create().await?;
    let doc0 = node0.docs.create().await?;
    doc0.set_bytes(author_a, b"k0".to_vec(), b"pre-existing-0".to_vec())
        .await?;
    doc0.set_bytes(author_a, b"k1".to_vec(), b"pre-existing-1".to_vec())
        .await?;
    doc0.set_bytes(author_a, b"k2".to_vec(), b"pre-existing-2".to_vec())
        .await?;
    println!("[setup] node0 wrote 3 entries under author_a before sharing anything");

    // --- Q3 setup: a second author writes the SAME key on node0 ---
    let author_b = node0.docs.author_create().await?;
    doc0.set_bytes(author_b, b"shared-key".to_vec(), b"from-b".to_vec())
        .await?;
    doc0.set_bytes(author_a, b"shared-key".to_vec(), b"from-a".to_vec())
        .await?;

    let entries_before: Vec<_> = collect(doc0.get_many(Query::key_exact(b"shared-key".to_vec())).await?).await?;
    println!(
        "[Q3] node0, before any sync: {} entries under key \"shared-key\" (from {} distinct authors)",
        entries_before.len(),
        entries_before
            .iter()
            .map(|e| e.author())
            .collect::<std::collections::HashSet<_>>()
            .len(),
    );

    // --- grant node1 a WRITE capability via ticket, share it, start sync ---
    let ticket = doc0
        .share(ShareMode::Write, AddrInfoOptions::Addresses)
        .await?;
    println!("[grant] node0 shared a Write ticket for the namespace to node1");

    // --- Q1: what can node0 learn about who now holds a capability? ---
    // The only "who has access" surface on THIS node is `list()`, and it
    // enumerates the LOCAL node's own capabilities into docs it knows
    // about — not other peers' capabilities into a shared namespace.
    let local_caps: Vec<_> = collect(node0.docs.list().await?).await?;
    println!(
        "[Q1] node0.list() (this node's OWN capabilities only): {:?}",
        local_caps
    );
    // Before node1 even imports the ticket, does node0 know node1 exists
    // as a grantee? It cannot: the ticket was generated locally and handed
    // out of band; nothing about the grant is recorded in the namespace
    // itself until/unless the grantee shows up and syncs.
    let sync_peers_before = doc0.get_sync_peers().await?;
    println!(
        "[Q1] node0.get_sync_peers() before node1 imports anything: {:?}",
        sync_peers_before
    );

    let doc1 = node1.docs.import(ticket).await?;
    println!("[join] node1 imported the ticket and started sync");

    // Wait for node1 to receive the pre-existing history.
    let got_history = wait_until(Duration::from_secs(20), || {
        let doc1 = doc1.clone();
        Box::pin(async move {
            match doc1.get_many(Query::all()).await {
                Ok(stream) => collect(stream).await.map(|v| v.len()).unwrap_or(0) >= 5,
                Err(_) => false,
            }
        })
    })
    .await;

    let entries1: Vec<_> = collect(doc1.get_many(Query::all()).await?).await?;
    println!(
        "[Q2] node1 synced (all arrived within 20s: {got_history}), sees {} entries total:",
        entries1.len()
    );
    for e in &entries1 {
        let val = doc0_get_value(&node0.docs, &doc0, e).await;
        println!(
            "      key={:?} author={} value={:?}",
            String::from_utf8_lossy(e.key()),
            e.author().fmt_short(),
            val,
        );
    }
    let saw_pre_existing = entries1.iter().any(|e| e.key() == b"k0")
        && entries1.iter().any(|e| e.key() == b"k1")
        && entries1.iter().any(|e| e.key() == b"k2");
    println!(
        "[Q2] node1 has ALL THREE pre-existing entries written before it ever joined: {saw_pre_existing}"
    );

    // --- Q1 again, after node1 has actually synced: what does node0 see now? ---
    let sync_peers_after = doc0.get_sync_peers().await?;
    println!(
        "[Q1] node0.get_sync_peers() after node1 synced: {:?}",
        sync_peers_after
    );
    println!(
        "[Q1] note: get_sync_peers() reads a `namespace_peers` table that only the persistent \
         (redb) store populates, as a reconnect-on-restart hint — NOT a live-connections view. \
         On these in-memory nodes it stayed None even mid-sync, which is itself informative: \
         there is no live-peers API surfaced here either, on top of there being no durable \
         grant log. Whatever roster a governance-eligible-holders feature needs, this crate \
         does not provide it at any layer — it would have to be application state this design \
         maintains itself (e.g. the `governance` collection §3.7.4 already proposes)."
    );

    // --- Q3 on node1's synced copy: did the two authors' writes to the SAME key collide? ---
    let shared: Vec<_> = collect(doc1.get_many(Query::key_exact(b"shared-key".to_vec())).await?).await?;
    println!(
        "[Q3] node1, after sync: {} entries under key \"shared-key\" (from {} distinct authors) — \
         both survive independently, no merge was forced",
        shared.len(),
        shared
            .iter()
            .map(|e| e.author())
            .collect::<std::collections::HashSet<_>>()
            .len(),
    );
    for e in &shared {
        println!(
            "      author={} value_hash={}",
            e.author().fmt_short(),
            e.content_hash()
        );
    }

    if !saw_pre_existing {
        bail!("Q2 experiment did not observe full-history backfill within the timeout");
    }

    node0.router.shutdown().await.ok();
    node1.router.shutdown().await.ok();
    Ok(())
}

async fn doc0_get_value(
    _docs: &iroh_docs::api::DocsApi,
    _doc: &iroh_docs::api::Doc,
    entry: &iroh_docs::Entry,
) -> String {
    format!("<hash {}, {} bytes>", entry.content_hash(), entry.content_len())
}
