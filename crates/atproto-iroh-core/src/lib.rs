//! Core primitives for the atproto-iroh design — see `../../SPEC.md` for
//! the design record and `../../CLAUDE.md` for how to extend this crate.
//!
//! Module map, and which SPEC.md section each one implements:
//! - [`identity`] — `did:iroh` (§3.2)
//! - [`namespace`] — namespaces as `iroh-docs` documents, typed record
//!   read/write (§3.3/§3.4, validated in `examples/iroh_docs_probe.rs`)
//! - [`records`] — `network.essmesh.node.*` typed records (§3.3)
//! - [`governance`] — objection-window `Proposal`/`Signal` ratification
//!   (§3.7.2/§3.7.3/§3.9), pure/no I/O by design
//! - [`fold`] — the I/O layer wiring `governance` to `namespace`: reads a
//!   namespace's governance collection and folds it into current state
//! - [`images`] — `network.essmesh.chat.image` (batteries-included app
//!   list), reusing `namespace::put_bytes`/`get_bytes` rather than
//!   `iroh-blobs`' own out-of-band blob API — see the module's own note
//! - [`messaging`] — `network.essmesh.chat.message` (batteries-included
//!   app list, CLAUDE.md), append-only, no new sync primitive needed
//! - [`mute`] — local, unsynced per-reader mute (§6 item 12)
//! - [`paths`] — the shared, overridable local-data-directory convention
//!   `identity`/`namespace`/`mute` all persist under
//! - [`tagging`] — `network.essmesh.tag` (batteries-included app list),
//!   cross-lexicon by construction via `records::record_ref`

pub mod fold;
pub mod governance;
pub mod identity;
pub mod images;
pub mod messaging;
pub mod mute;
pub mod namespace;
pub mod paths;
pub mod records;
pub mod tagging;
