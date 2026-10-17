use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::model::{Collection, Song};

/// Where the app keeps its own files. Never inside the lazer data folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    pub config: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    pub fn from_env() -> Option<Self> {
        Self::for_os(std::env::consts::OS, |name| {
            std::env::var_os(name).map(PathBuf::from)
        })
    }

    fn for_os(os: &str, var: impl Fn(&str) -> Option<PathBuf>) -> Option<Self> {
        let (config, cache) = match os {
            "windows" => (var("APPDATA")?, var("LOCALAPPDATA")?),
            "macos" => {
                let library = var("HOME")?.join("Library");
                (library.join("Application Support"), library.join("Caches"))
            }
            _ => {
                let home = var("HOME");
                let under_home = |dir: &str| home.as_ref().map(|home| home.join(dir));
                (
                    var("XDG_CONFIG_HOME").or_else(|| under_home(".config"))?,
                    var("XDG_CACHE_HOME").or_else(|| under_home(".cache"))?,
                )
            }
        };
        Some(Self {
            config: config.join("listenosu"),
            cache: cache.join("listenosu"),
        })
    }

    pub fn playlists_file(&self) -> PathBuf {
        self.config.join("playlists.json")
    }

    /// Where a user puts theme files (`*.toml`).
    pub fn themes_dir(&self) -> PathBuf {
        self.config.join("themes")
    }

    pub fn library_cache_file(&self) -> PathBuf {
        self.cache.join("library.json")
    }
}

/// Writes through a temporary file, so a crash never leaves a half-written file behind.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(value).map_err(io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

/// None when the file is missing or unreadable: callers start from empty instead of failing.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

const CACHE_VERSION: u32 = 1;

#[derive(Serialize)]
struct CacheOut<'a> {
    version: u32,
    songs: &'a [Song],
    collections: &'a [Collection],
}

#[derive(serde::Deserialize)]
struct CacheIn {
    version: u32,
    songs: Vec<Song>,
    collections: Vec<Collection>,
}

/// Saves the last library the helper could read, for when lazer's database cannot be read.
pub fn save_library(path: &Path, songs: &[Song], collections: &[Collection]) -> io::Result<()> {
    let cache = CacheOut {
        version: CACHE_VERSION,
        songs,
        collections,
    };
    write_json(path, &cache)
}

/// None for a missing, corrupt or older-format cache.
pub fn load_library(path: &Path) -> Option<(Vec<Song>, Vec<Collection>)> {
    let cache: CacheIn = read_json(path)?;
    (cache.version == CACHE_VERSION).then_some((cache.songs, cache.collections))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("listenosu-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn app_dirs_follow_each_os_convention() {
        let env = |name: &str| {
            Some(PathBuf::from(match name {
                "APPDATA" => "R",
                "LOCALAPPDATA" => "L",
                "HOME" => "H",
                _ => return None,
            }))
        };
        let windows = AppDirs::for_os("windows", env).unwrap();
        assert_eq!(windows.config, Path::new("R/listenosu"));
        assert_eq!(windows.cache, Path::new("L/listenosu"));
        let mac = AppDirs::for_os("macos", env).unwrap();
        assert_eq!(
            mac.config,
            Path::new("H/Library/Application Support/listenosu")
        );
        assert_eq!(mac.cache, Path::new("H/Library/Caches/listenosu"));
        let linux = AppDirs::for_os("linux", env).unwrap();
        assert_eq!(linux.config, Path::new("H/.config/listenosu"));
        assert_eq!(linux.cache, Path::new("H/.cache/listenosu"));
        assert!(AppDirs::for_os("windows", |_| None).is_none());
    }

    #[test]
    fn library_cache_round_trips_and_leaves_no_temp_file() {
        let dir = temp("cache");
        let path = dir.join("library.json");
        let song = Song {
            title: "夜に駆ける".into(),
            audio_hash: "ab".into(),
            ..Song::default()
        };
        let collection = Collection {
            name: "Fav".into(),
            beatmap_md5s: vec!["x".into()],
        };
        save_library(
            &path,
            std::slice::from_ref(&song),
            std::slice::from_ref(&collection),
        )
        .unwrap();
        assert_eq!(load_library(&path), Some((vec![song], vec![collection])));
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_missing_or_old_caches_load_as_none() {
        let dir = temp("bad-cache");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("library.json");
        assert_eq!(load_library(&path), None);
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(load_library(&path), None);
        std::fs::write(&path, br#"{"version":0,"songs":[],"collections":[]}"#).unwrap();
        assert_eq!(load_library(&path), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
