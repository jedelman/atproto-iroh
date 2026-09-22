//! Where this crate's local state lives on disk — one convention, one
//! override, shared by every module that persists something
//! (`identity::Identity::load_or_generate`,
//! `namespace::Node::spawn_persistent`, `mute::MuteList::default_path`),
//! so "point everything at a specific volume" is one setting, not three
//! independently-drifting ones.

use std::{path::PathBuf, sync::OnceLock};

/// Set once, at startup, by a host that has no shell environment to read
/// `$ATPROTO_IROH_DATA_DIR` from — see `set_data_dir_override`.
static OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// Hands `data_dir()` an explicit path instead of an environment
/// variable — the mobile case. Android and iOS apps have no shell
/// environment for this crate's native library to read
/// `$ATPROTO_IROH_DATA_DIR`/`$XDG_DATA_HOME`/`$HOME` from at all; Android
/// gives each app a fixed per-app private directory instead
/// (`Context.getFilesDir()`), reachable only through the host app side —
/// Tauri's own `app.path().app_data_dir()` resolver, itself backed by
/// that same Android API. A mobile entry point calls this once, at
/// startup, before any `identity`/`namespace`/`mute` call ever asks
/// `data_dir()` for a path (CLAUDE.md's platform-priority section:
/// Android is the actual priority-one platform, not a desktop
/// afterthought this crate happens to also run on). Desktop and CLI
/// callers never need this — the env var convention below already
/// covers them, and this is checked first specifically so it can
/// override them if a future desktop embedding ever wants to. Calling it
/// more than once is a programming error this function doesn't flag:
/// `OnceLock` just silently keeps whatever the first call set.
pub fn set_data_dir_override(path: PathBuf) {
    let _ = OVERRIDE.set(path);
}

/// `$ATPROTO_IROH_DATA_DIR` if set, else `$XDG_DATA_HOME/atproto-iroh`
/// (or `~/.local/share/atproto-iroh`), else a temp directory as a last
/// resort so callers always get *a* path rather than an `Option` to
/// handle. `mute.rs`'s original comment on this pattern still applies —
/// a minimal stand-in for the `dirs`/`directories` crate, not a claim
/// this covers every platform's actual convention (Windows' real answer
/// is `%APPDATA%`, not checked here).
///
/// The override exists for exactly the "encrypt data at rest" question
/// that asked for it: at-rest encryption here means pointing this at a
/// volume the OS already encrypts (FileVault, BitLocker, LUKS, a mounted
/// encrypted container) — never this crate implementing its own crypto
/// layer for a data directory it doesn't need to own the security model
/// of. "Nothing can protect someone from themselves" was the explicit
/// call made choosing this over app-level encryption: if that volume is
/// unlocked, everything on it is, same as any other file there. This
/// function's only job is to not hardcode a path, so that choice is
/// actually available rather than assumed away.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = OVERRIDE.get() {
        return dir.clone();
    }
    if let Some(dir) = std::env::var_os("ATPROTO_IROH_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("atproto-iroh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_wins_over_every_other_convention() {
        // SAFETY: test-only, single-threaded within this process's test
        // (this crate doesn't run tests with a custom harness that
        // parallelizes within a process across env mutation).
        unsafe {
            std::env::set_var("ATPROTO_IROH_DATA_DIR", "/tmp/wherever-i-said");
        }
        assert_eq!(data_dir(), PathBuf::from("/tmp/wherever-i-said"));
        unsafe {
            std::env::remove_var("ATPROTO_IROH_DATA_DIR");
        }
    }
}
