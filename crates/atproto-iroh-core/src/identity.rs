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

use std::{fs, io, path::Path};

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

    /// Loads a saved identity from `path`, generating and saving a new
    /// one if the file doesn't exist yet — the same load-or-create shape
    /// as `mute::MuteList::load`, for the same reason: "not found" here
    /// means "first run," not an error.
    ///
    /// The file holds the raw 32-byte secret key, nothing else — no
    /// encoding, no metadata. Whoever can read it can act as this node's
    /// `did:iroh` identity end to end (sign the DID document, hold every
    /// namespace this node has ever been granted into), so it wants the
    /// same filesystem protection any other private key file gets — not
    /// this function's job to enforce, but worth being explicit that
    /// it's assuming the caller's data directory is already private.
    pub fn load_or_generate(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        match fs::read(path) {
            Ok(bytes) => {
                let array: [u8; 32] = bytes.try_into().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{} is not a 32-byte secret key", path.display()),
                    )
                })?;
                Ok(Self::from_secret(SecretKey::from_bytes(&array)))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let identity = Self::generate();
                identity.save(path)?;
                Ok(identity)
            }
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.secret.to_bytes())
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

    #[test]
    fn load_or_generate_persists_across_calls() {
        let dir = std::env::temp_dir().join(format!(
            "atproto-iroh-identity-test-{}",
            std::process::id()
        ));
        let path = dir.join("identity");

        let first = Identity::load_or_generate(&path).unwrap();
        let second = Identity::load_or_generate(&path).unwrap();
        assert_eq!(first.did(), second.did());

        let _ = fs::remove_dir_all(&dir);
    }
}
