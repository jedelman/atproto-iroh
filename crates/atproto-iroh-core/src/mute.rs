//! Per-reader local mute — SPEC.md §6 item 12's resolution, layer (2) of
//! two: a personal preference, not a group decision. Deliberately no
//! lexicon, no signature, no sync of any kind — this is local state only,
//! same as deciding not to read a particular news outlet. Never conflate
//! this with `governance::GovernanceClass::RemoveCoSigner` (a ratified
//! group decision, binding on honest clients by convention): a member can
//! be muted by one person without the group ever voting, or removed by
//! the group while someone who'd already muted them notices nothing new.

use std::{
    collections::HashSet,
    fs, io,
    hash::Hash,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Clone)]
pub struct MuteList<A> {
    muted: HashSet<A>,
}

impl<A: Eq + Hash> Default for MuteList<A> {
    fn default() -> Self {
        Self {
            muted: HashSet::new(),
        }
    }
}

impl<A: Eq + Hash + Clone> MuteList<A> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mute(&mut self, author: A) {
        self.muted.insert(author);
    }

    pub fn unmute(&mut self, author: &A) {
        self.muted.remove(author);
    }

    pub fn is_muted(&self, author: &A) -> bool {
        self.muted.contains(author)
    }

    pub fn iter(&self) -> impl Iterator<Item = &A> {
        self.muted.iter()
    }
}

impl<A: Eq + Hash + Clone + Serialize + DeserializeOwned> MuteList<A> {
    /// Loads from a local file if it exists, otherwise an empty list —
    /// there is nothing to sync, so "not found" is just "never muted
    /// anyone," not an error.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(contents) => {
                let muted: HashSet<A> = serde_json::from_str(&contents)?;
                Ok(Self { muted })
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(&self.muted)?;
        fs::write(path, contents)
    }

    /// Conventional per-user local path — `crate::paths::data_dir`, the
    /// same convention (and the same `$ATPROTO_IROH_DATA_DIR` override)
    /// `identity::Identity`/`namespace::Node::spawn_persistent` use, so
    /// pointing everything at one volume is one setting, not three.
    pub fn default_path() -> PathBuf {
        crate::paths::data_dir().join("mute.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muting_is_purely_local_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("atproto-iroh-mute-test-{}", std::process::id()));
        let path = dir.join("mute.json");

        let mut list: MuteList<String> = MuteList::new();
        assert!(!list.is_muted(&"alice".to_string()));
        list.mute("alice".to_string());
        assert!(list.is_muted(&"alice".to_string()));

        list.save(&path).unwrap();
        let reloaded: MuteList<String> = MuteList::load(&path).unwrap();
        assert!(reloaded.is_muted(&"alice".to_string()));
        assert!(!reloaded.is_muted(&"bob".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loading_a_missing_file_is_an_empty_list_not_an_error() {
        let list: MuteList<String> = MuteList::load("/nonexistent/atproto-iroh-mute-test.json").unwrap();
        assert!(!list.is_muted(&"anyone".to_string()));
    }
}
