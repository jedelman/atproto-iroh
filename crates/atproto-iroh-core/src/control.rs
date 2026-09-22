//! A raw message endpoint for relay-mode control operations — the design
//! resolved in conversation (2026-09-22), replacing an earlier
//! docs-namespace-based "inbox" idea. The box's onboarding QR is not a
//! capability into any namespace at all, just its own dialable network
//! identity (`did:iroh`) — the same thing it already needs to be reachable
//! for ordinary docs sync. Anyone who can reach that address can privately
//! hand it a namespace ticket to join, over a direct, ephemeral,
//! point-to-point QUIC connection this module alone ever sees — nothing
//! synced, nothing durable, nothing a third party with capability into a
//! shared "inbox" namespace could ever read. That's the property the
//! rejected docs-based design didn't have: a shared inbox namespace means
//! anyone who can read it sees *every* ticket ever submitted, a real
//! cross-org leak if one box serves more than one group. A raw connection
//! has no such shared state to leak from.
//!
//! **`JOIN` needs no authorization beyond "you can reach this address."**
//! That's deliberate, not an oversight: joining a namespace already
//! requires the caller to hold that namespace's own real capability (the
//! ticket itself) — the same bar `Node::join` always had. The control
//! endpoint doesn't add a new trust requirement, it just gives a way to
//! deliver that capability privately instead of over some other channel.
//!
//! **`RESET` needs its own proof — a `ResetToken` generated once alongside
//! `Identity`.** This is the "poison pill" from Jason's reset-button idea:
//! wiping this node's entire data directory (identity included) and
//! signaling the caller to exit, so a process supervisor restarts it
//! against a fresh `did:iroh` — a real factory reset, remote-triggerable
//! as well as (eventually) physical-button-triggerable, which is why the
//! token exists: unlike JOIN, a caller who can merely reach the box has no
//! business being able to destroy it, so reaching it isn't enough — they
//! also have to prove they hold the token, which only whoever set the box
//! up (or was handed it) has.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, EndpointAddr, PublicKey, SecretKey,
};
use iroh_docs::{api::DocsApi, DocTicket};
use tokio::sync::Notify;

/// The ALPN this module's protocol handler answers — registered on the
/// same `Router` as blobs/gossip/docs (`namespace.rs`'s `spawn_inner`),
/// one more accepted protocol, not a separate transport.
pub const CONTROL_ALPN: &[u8] = b"atproto-iroh/control/1";

/// Generated once, alongside `Identity`, and required to authorize a
/// remote `RESET` — see this module's top doc comment for why `JOIN`
/// doesn't need an equivalent. Plain string comparison is fine here: QUIC
/// already encrypts the connection this travels over, and this isn't a
/// public web-facing endpoint meant to withstand high-volume guessing —
/// a 32-byte random token is the actual defense, not the comparison
/// method.
#[derive(Clone)]
pub struct ResetToken(String);

impl ResetToken {
    /// Loads a saved token from `path`, generating and saving a new one
    /// if the file doesn't exist yet — same load-or-create shape as
    /// `identity::Identity::load_or_generate` and `mute::MuteList::load`.
    /// Randomness reuses `iroh::SecretKey::generate()` (already a
    /// transitive dependency this crate exercises constantly) rather than
    /// adding a new `rand` dependency for one token.
    pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(token) => Ok(Self(token.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let token = hex::encode(SecretKey::generate().to_bytes());
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, &token)?;
                Ok(Self(token))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What `namespace::Node::spawn_relay` needs to wire the control endpoint
/// into a node at router-build time.
pub struct ControlConfig {
    pub reset_token: ResetToken,
    /// Deleted in full on a valid `RESET` — the whole data directory
    /// (identity included), not just the docs/blobs stores, since a real
    /// factory reset means the next start generates a brand-new
    /// `did:iroh`, not a stable identity with empty content.
    pub wipe_dir: PathBuf,
    /// Notified once a `RESET` has actually wiped `wipe_dir` — a caller
    /// (the CLI's `serve` loop) awaits this alongside Ctrl+C so it knows
    /// to exit and let a process supervisor restart fresh.
    pub reset_signal: Arc<Notify>,
}

/// The server-side `ProtocolHandler` — one per node, registered once.
/// Deliberately implements `Debug` by hand rather than deriving it: a
/// derived impl would print `reset_token` in any `{:?}` logging, and
/// nothing about accidentally logging this secret should be easier than
/// deliberately reading it.
pub struct ControlHandler {
    docs: DocsApi,
    reset_token: String,
    wipe_dir: PathBuf,
    reset_signal: Arc<Notify>,
}

impl std::fmt::Debug for ControlHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlHandler").finish_non_exhaustive()
    }
}

impl ControlHandler {
    pub fn new(docs: DocsApi, config: &ControlConfig) -> Self {
        Self {
            docs,
            reset_token: config.reset_token.0.clone(),
            wipe_dir: config.wipe_dir.clone(),
            reset_signal: config.reset_signal.clone(),
        }
    }

    /// One request, one response, both plain text — `"JOIN <ticket>"` or
    /// `"RESET <token>"`, nothing more elaborate; this is a control
    /// channel for two operations, not a general RPC protocol.
    async fn handle(&self, request: &str) -> String {
        if let Some(ticket_str) = request.strip_prefix("JOIN ") {
            match ticket_str.trim().parse::<DocTicket>() {
                Ok(ticket) => match self.docs.import(ticket).await {
                    Ok(doc) => format!("OK {}", doc.id()),
                    Err(e) => format!("ERROR {e}"),
                },
                Err(e) => format!("ERROR invalid ticket: {e}"),
            }
        } else if let Some(token) = request.strip_prefix("RESET ") {
            if token.trim() == self.reset_token {
                match std::fs::remove_dir_all(&self.wipe_dir) {
                    Ok(()) => {
                        self.reset_signal.notify_one();
                        "OK resetting".to_string()
                    }
                    Err(e) => format!("ERROR {e}"),
                }
            } else {
                "ERROR invalid token".to_string()
            }
        } else {
            "ERROR unrecognized request".to_string()
        }
    }
}

impl ProtocolHandler for ControlHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await.map_err(AcceptError::from_err)?;
        let request = recv
            .read_to_end(64 * 1024)
            .await
            .map_err(AcceptError::from_err)?;
        let request = String::from_utf8_lossy(&request);
        let response = self.handle(request.trim()).await;
        send.write_all(response.as_bytes())
            .await
            .map_err(AcceptError::from_err)?;
        let _ = send.finish();
        // Without this, the Router can tear the connection down as soon as
        // this future returns, racing the client's own read of the
        // response it just finished writing — matches iroh's own `echo`
        // example, found by hitting exactly this race live.
        connection.closed().await;
        Ok(())
    }
}

/// Client side: dials `target`'s control endpoint directly (no capability
/// needed, just its `did:iroh` — see this module's top doc comment) and
/// sends one request, returning the one-line response. `target` alone is
/// enough to dial — `iroh::EndpointAddr` derives from a bare
/// `PublicKey`/`EndpointId`, resolved via whatever address-lookup service
/// the caller's `NetworkPreset` configured (relay/DNS for `N0`, direct
/// addresses only for `Minimal`).
pub async fn send_control_message(
    endpoint: &Endpoint,
    target: PublicKey,
    message: &str,
) -> Result<String> {
    send_control_message_to(endpoint, target.into(), message).await
}

/// Same as [`send_control_message`] but takes a full [`EndpointAddr`]
/// rather than relying on discovery to resolve a bare public key — the
/// bare-key path is the intended production shape (a `did:iroh` alone,
/// resolved via the caller's `NetworkPreset`'s discovery service), this
/// one exists so a caller with an explicit address (or a test lacking
/// working discovery) can dial directly.
pub async fn send_control_message_to(
    endpoint: &Endpoint,
    target: EndpointAddr,
    message: &str,
) -> Result<String> {
    let connection = endpoint.connect(target, CONTROL_ALPN).await?;
    let (mut send, mut recv) = connection.open_bi().await?;
    send.write_all(message.as_bytes()).await?;
    send.finish()?;
    let response = recv.read_to_end(64 * 1024).await?;
    // Signals the server side's `connection.closed().await` (in
    // `ControlHandler::accept`) so it doesn't wait out a timeout.
    connection.close(0u32.into(), b"done");
    Ok(String::from_utf8_lossy(&response).into_owned())
}
