//! The song library as the app sees it: the helper's snapshots, the on-disk cache, and the missing-file check.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use ui::i18n::{t, tr};

use library::helper::{Event, Helper};
use library::playlist::{collection_songs, is_missing};
use library::protocol::Message;
use library::store::{load_library, save_library};
use library::{Collection, Song};

pub struct Catalog {
    pub songs: Vec<Song>,
    pub collections: Vec<Collection>,
    pub data_dir: PathBuf,
    /// Audio hashes whose file is not on disk.
    missing: HashSet<String>,
    /// Shown as a banner while the library comes from the cache because the helper cannot read it.
    pub problem: Option<String>,
    cache_file: Option<PathBuf>,
    helper: Option<Helper>,
}

impl Catalog {
    /// Shows the cached library right away and asks the helper for the real one.
    pub fn open(data_dir: PathBuf, cache_file: Option<PathBuf>) -> Self {
        let (songs, collections) = cache_file
            .as_deref()
            .and_then(load_library)
            .unwrap_or_default();
        let helper = helper_exe().map(|exe| Helper::start(exe, data_dir.clone()));
        let mut catalog = Self {
            problem: helper
                .is_none()
                .then(|| t("problem.helper_missing").to_owned()),
            songs,
            collections,
            data_dir,
            missing: HashSet::new(),
            cache_file,
            helper,
        };
        catalog.refresh_missing();
        catalog
    }

    /// Takes in what the helper sent. True when the song list changed.
    pub fn poll(&mut self) -> bool {
        let Some(helper) = &self.helper else {
            return false;
        };
        let events: Vec<Event> = helper.events().try_iter().collect();
        let mut changed = false;
        for event in events {
            match event {
                Event::Message(Message::Snapshot { songs, collections })
                | Event::Message(Message::Changed { songs, collections }) => {
                    self.songs = songs;
                    self.collections = collections;
                    self.problem = None;
                    self.refresh_missing();
                    if let Some(file) = &self.cache_file {
                        // The cache only speeds up the next start, so a failed write is ignored.
                        let _ = save_library(file, &self.songs, &self.collections);
                    }
                    changed = true;
                }
                Event::Message(Message::Error { message, .. }) => {
                    self.problem = Some(tr("problem.read_failed", &[("message", &message)]));
                }
                Event::Message(Message::Version { .. }) => {}
                Event::Failed(reason) => {
                    self.problem = Some(tr(
                        "problem.helper_stopped",
                        &[("reason", &reason.to_string())],
                    ));
                }
            }
        }
        changed
    }

    pub fn is_missing(&self, song: &Song) -> bool {
        self.missing.contains(&song.audio_hash)
    }

    /// Indices of a lazer collection's songs.
    pub fn collection(&self, index: usize) -> Vec<usize> {
        self.collections
            .get(index)
            .map(|c| collection_songs(c, &self.songs))
            .unwrap_or_default()
    }

    fn refresh_missing(&mut self) {
        self.missing = self
            .songs
            .iter()
            .filter(|song| is_missing(song, &self.data_dir))
            .map(|song| song.audio_hash.clone())
            .collect();
    }
}

/// The helper sits next to the app. A development build also finds the one `dotnet build` leaves in the repository.
fn helper_exe() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "listenosu-helper.exe"
    } else {
        "listenosu-helper"
    };
    let beside = std::env::current_exe().ok()?.parent()?.join(name);
    let mut candidates = vec![beside];
    if cfg!(debug_assertions) {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../helper/Helper/bin/Release/net10.0")
                .join(name),
        );
    }
    candidates.into_iter().find(|path| path.is_file())
}
