use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One song: all beatmaps that share an audio file, as sent by the helper
/// (see docs/helper-protocol.md).
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Song {
    pub audio_hash: String,
    pub background_hash: Option<String>,
    /// The beatmap set's id on osu!; None for sets not downloaded from it.
    pub online_id: Option<i64>,
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub source: String,
    pub tags: String,
    pub author: String,
    pub bpm: f64,
    /// 0 when lazer has no length for the beatmap; the player then uses the decoded length.
    pub length_ms: f64,
    pub date_added_ms: i64,
    pub last_played_ms: Option<i64>,
    pub beatmap_md5s: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Collection {
    pub name: String,
    pub beatmap_md5s: Vec<String>,
}

impl Song {
    /// Title in the chosen script, falling back to the other one when it is empty.
    pub fn title(&self, romanized: bool) -> &str {
        pick(romanized, &self.title, &self.title_unicode)
    }

    pub fn artist(&self, romanized: bool) -> &str {
        pick(romanized, &self.artist, &self.artist_unicode)
    }

    /// `<data_dir>/files/<h[0]>/<h[0..2]>/<hash>`; None for a hash too short to be one.
    pub fn audio_path(&self, data_dir: &Path) -> Option<PathBuf> {
        file_path(data_dir, &self.audio_hash)
    }

    pub fn background_path(&self, data_dir: &Path) -> Option<PathBuf> {
        file_path(data_dir, self.background_hash.as_deref()?)
    }

    pub fn length_secs(&self) -> f32 {
        (self.length_ms / 1000.0) as f32
    }
}

fn pick<'a>(romanized: bool, latin: &'a str, original: &'a str) -> &'a str {
    let (first, second) = if romanized {
        (latin, original)
    } else {
        (original, latin)
    };
    if first.is_empty() { second } else { first }
}

fn file_path(data_dir: &Path, hash: &str) -> Option<PathBuf> {
    // `get` rather than slicing: a malformed hash must not panic on a char boundary.
    let (a, ab) = (hash.get(..1)?, hash.get(..2)?);
    Some(data_dir.join("files").join(a).join(ab).join(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(title: &str, unicode: &str) -> Song {
        Song {
            title: title.into(),
            title_unicode: unicode.into(),
            ..Song::default()
        }
    }

    #[test]
    fn title_follows_the_script_choice_and_falls_back() {
        let s = song("Yoru ni Kakeru", "夜に駆ける");
        assert_eq!(s.title(false), "夜に駆ける");
        assert_eq!(s.title(true), "Yoru ni Kakeru");
        assert_eq!(song("only latin", "").title(false), "only latin");
        assert_eq!(song("", "原語だけ").title(true), "原語だけ");
    }

    #[test]
    fn online_id_may_be_missing_from_old_caches_and_playlists() {
        let json = r#"{"audio_hash":"ab","background_hash":null,"title":"t","title_unicode":"",
            "artist":"a","artist_unicode":"","source":"","tags":"","author":"","bpm":120.0,
            "length_ms":1000.0,"date_added_ms":0,"last_played_ms":null,"beatmap_md5s":[]}"#;
        let s: Song = serde_json::from_str(json).unwrap();
        assert_eq!(s.online_id, None);
    }

    #[test]
    fn audio_path_uses_the_two_level_hash_layout() {
        let s = Song {
            audio_hash: "abcdef".into(),
            ..Song::default()
        };
        let path = s.audio_path(Path::new("data")).unwrap();
        assert_eq!(path, Path::new("data/files/a/ab/abcdef"));
        assert!(Song::default().audio_path(Path::new("data")).is_none());
    }
}
