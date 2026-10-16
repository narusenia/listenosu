use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Collection, Song};
use crate::store::{read_json, write_json};

/// A song in a playlist: the audio hash plus a copy of what to show when the library lost it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    pub audio_hash: String,
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Playlist {
    pub name: String,
    pub entries: Vec<Entry>,
}

impl Playlist {
    /// Adds the song unless it is already in the playlist. Returns whether it was added.
    pub fn add(&mut self, song: &Song) -> bool {
        if self.entries.iter().any(|e| e.audio_hash == song.audio_hash) {
            return false;
        }
        self.entries.push(Entry {
            audio_hash: song.audio_hash.clone(),
            title: song.title(false).to_owned(),
            artist: song.artist(false).to_owned(),
        });
        true
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
        }
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.entries.len() {
            self.entries.swap(index, index - 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.entries.len() {
            self.entries.swap(index, index + 1);
        }
    }

    /// For each entry, the index of its song in `songs`; None is a song the library no longer has
    /// (shown from the stored copy, skipped on playback).
    pub fn resolve(&self, songs: &[Song]) -> Vec<Option<usize>> {
        let by_hash: HashMap<&str, usize> = songs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.audio_hash.as_str(), i))
            .collect();
        self.entries
            .iter()
            .map(|e| by_hash.get(e.audio_hash.as_str()).copied())
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Playlists {
    pub playlists: Vec<Playlist>,
}

impl Playlists {
    /// Creates an empty playlist; a name already in use gets " (2)", " (3)", … appended.
    /// Returns its index.
    pub fn create(&mut self, name: &str) -> usize {
        let name = self.unique_name(name, None);
        self.playlists.push(Playlist {
            name,
            entries: Vec::new(),
        });
        self.playlists.len() - 1
    }

    pub fn rename(&mut self, index: usize, name: &str) {
        if index < self.playlists.len() {
            self.playlists[index].name = self.unique_name(name, Some(index));
        }
    }

    pub fn delete(&mut self, index: usize) {
        if index < self.playlists.len() {
            self.playlists.remove(index);
        }
    }

    fn unique_name(&self, wanted: &str, ignore: Option<usize>) -> String {
        let base = wanted.trim();
        let base = if base.is_empty() { "Playlist" } else { base };
        let taken = |name: &str| {
            self.playlists
                .iter()
                .enumerate()
                .any(|(i, p)| Some(i) != ignore && p.name == name)
        };
        let mut name = base.to_owned();
        let mut n = 2;
        while taken(&name) {
            name = format!("{base} ({n})");
            n += 1;
        }
        name
    }

    pub fn load(path: &Path) -> Self {
        read_json(path).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_json(path, self)
    }
}

/// Indices into `songs` of a lazer collection's songs (lazer collections hold beatmap MD5s).
pub fn collection_songs(collection: &Collection, songs: &[Song]) -> Vec<usize> {
    let wanted: std::collections::HashSet<&str> =
        collection.beatmap_md5s.iter().map(String::as_str).collect();
    (0..songs.len())
        .filter(|&i| {
            songs[i]
                .beatmap_md5s
                .iter()
                .any(|md5| wanted.contains(md5.as_str()))
        })
        .collect()
}

/// True when the song's audio file is not on disk, so the player must skip it.
pub fn is_missing(song: &Song, data_dir: &Path) -> bool {
    song.audio_path(data_dir).is_none_or(|path| !path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(hash: &str, title: &str) -> Song {
        Song {
            audio_hash: hash.into(),
            title: title.into(),
            ..Song::default()
        }
    }

    #[test]
    fn adding_the_same_song_twice_keeps_one_entry_with_a_copy_of_its_info() {
        let mut list = Playlist::default();
        assert!(list.add(&song("aa", "One")));
        assert!(!list.add(&song("aa", "One")));
        assert_eq!(list.entries.len(), 1);
        assert_eq!(list.entries[0].title, "One");
    }

    #[test]
    fn moving_and_removing_stay_inside_the_list() {
        let mut list = Playlist::default();
        for hash in ["a1", "b2", "c3"] {
            list.add(&song(hash, hash));
        }
        let hashes = |l: &Playlist| {
            l.entries
                .iter()
                .map(|e| e.audio_hash.clone())
                .collect::<Vec<_>>()
        };
        list.move_up(0);
        list.move_down(2);
        assert_eq!(hashes(&list), ["a1", "b2", "c3"]);
        list.move_down(0);
        assert_eq!(hashes(&list), ["b2", "a1", "c3"]);
        list.remove(9);
        list.remove(1);
        assert_eq!(hashes(&list), ["b2", "c3"]);
    }

    #[test]
    fn entries_missing_from_the_library_resolve_to_none_and_come_back_on_reimport() {
        let mut list = Playlist::default();
        list.add(&song("a1", "Kept"));
        list.add(&song("b2", "Gone"));
        let library = [song("a1", "Kept")];
        assert_eq!(list.resolve(&library), [Some(0), None]);
        let reimported = [song("b2", "Gone"), song("a1", "Kept")];
        assert_eq!(list.resolve(&reimported), [Some(1), Some(0)]);
    }

    #[test]
    fn playlist_names_stay_unique_and_non_empty() {
        let mut lists = Playlists::default();
        lists.create("Work");
        let second = lists.create("Work");
        lists.create("  ");
        assert_eq!(lists.playlists[second].name, "Work (2)");
        assert_eq!(lists.playlists[2].name, "Playlist");
        lists.rename(second, "Work"); // taken by the first
        assert_eq!(lists.playlists[second].name, "Work (2)");
        lists.rename(0, "Work"); // its own name is not a clash
        assert_eq!(lists.playlists[0].name, "Work");
    }

    #[test]
    fn playlists_survive_a_save_and_load_and_a_missing_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("listenosu-playlists-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("playlists.json");
        assert_eq!(Playlists::load(&path), Playlists::default());
        let mut lists = Playlists::default();
        let i = lists.create("夜の運転");
        lists.playlists[i].add(&song("a1", "夜に駆ける"));
        lists.save(&path).unwrap();
        assert_eq!(Playlists::load(&path), lists);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn collections_match_songs_by_any_beatmap_md5() {
        let mut a = song("a1", "A");
        a.beatmap_md5s = vec!["m1".into(), "m2".into()];
        let mut b = song("b2", "B");
        b.beatmap_md5s = vec!["m3".into()];
        let collection = Collection {
            name: "Fav".into(),
            beatmap_md5s: vec!["m2".into(), "zz".into()],
        };
        assert_eq!(collection_songs(&collection, &[a, b]), [0]);
    }

    #[test]
    fn a_song_without_an_audio_file_on_disk_is_missing() {
        let dir = std::env::temp_dir().join(format!("listenosu-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let present = song("abcdef", "Here");
        let file = present.audio_path(&dir).unwrap();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"x").unwrap();
        assert!(!is_missing(&present, &dir));
        assert!(is_missing(&song("123456", "Gone"), &dir));
        assert!(is_missing(&song("", "No hash"), &dir));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
