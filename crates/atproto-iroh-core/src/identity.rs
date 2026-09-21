//! `did:iroh` identity — SPEC.md §3.2. One Ed25519 keypair per member,
//! nothing group-owned. Resolution (dialing the node and asking for its
//! own signed DID document) is not implemented here; this module only
//! covers the keypair and the DID string form.
//!
//! Deliberately a distinct type from `iroh_docs::Author` (see
//! `namespace.rs`) — SPEC.md §3.4's validation note found the two are
//! unrelated in the underlying crate (nothing binds an `Author` signing
//! key to a node's identity key), and this module keeps that distinction
//! in the type system rather than letting the two quietly collapse into
//! one key used for both jobs.

use iroh::{PublicKey, SecretKey};

pub struct Identity {
    secret: SecretKey,
}

impl Identity {
    pub fn generate() -> Self {
        Self {
            secret: SecretKey::generate(),
        }
    }

    pub fn from_secret(secret: SecretKey) -> Self {
        Self { secret }
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret
    }

    pub fn public_key(&self) -> PublicKey {
        self.secret.public()
    }

    /// `did:iroh:<node id>` — SPEC.md §3.2. `PublicKey`'s `Display` is a
    /// hex encoding (iroh-base, not z-base-32 — checked against source,
    /// not assumed).
    pub fn did(&self) -> String {
        format!("did:iroh:{}", self.public_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn did_round_trips_through_the_public_key() {
        let id = Identity::generate();
        assert!(id.did().starts_with("did:iroh:"));
        assert_eq!(id.did(), format!("did:iroh:{}", id.public_key()));
    }
}
