use std::path::{Path, PathBuf};

/// The default osu!lazer data folders for this OS, most likely first.
pub fn default_dirs() -> Vec<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let dir = match std::env::consts::OS {
        "windows" => var("APPDATA"),
        "macos" => var("HOME").map(|home| home.join("Library/Application Support")),
        _ => var("XDG_DATA_HOME").or_else(|| var("HOME").map(|home| home.join(".local/share"))),
    };
    dir.map(|dir| dir.join("osu")).into_iter().collect()
}

/// The first candidate (or the folder its `storage.ini` points to) that holds a `client.realm`.
/// `Err` lists every folder that was looked in, for the first-run screen.
pub fn find_in(candidates: &[PathBuf]) -> Result<PathBuf, Vec<PathBuf>> {
    let mut searched = Vec::new();
    for candidate in candidates {
        // lazer keeps `storage.ini` in the default folder even when the data lives elsewhere.
        let dirs = redirected(candidate)
            .into_iter()
            .chain(std::iter::once(candidate.clone()));
        for dir in dirs {
            if dir.join("client.realm").is_file() {
                return Ok(dir);
            }
            searched.push(dir);
        }
    }
    Err(searched)
}

pub fn detect() -> Result<PathBuf, Vec<PathBuf>> {
    find_in(&default_dirs())
}

/// `FullPath = D:\osu-data` in `<dir>/storage.ini`.
fn redirected(dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dir.join("storage.ini")).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        let value = value.trim();
        (key.trim() == "FullPath" && !value.is_empty()).then(|| PathBuf::from(value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("listenosu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_the_default_folder_when_it_has_a_database() {
        let dir = temp("default");
        std::fs::write(dir.join("client.realm"), b"").unwrap();
        assert_eq!(find_in(std::slice::from_ref(&dir)), Ok(dir.clone()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn follows_storage_ini_to_the_custom_folder() {
        let default = temp("ini-default");
        let custom = temp("ini-custom");
        std::fs::write(custom.join("client.realm"), b"").unwrap();
        std::fs::write(
            default.join("storage.ini"),
            format!("; comment\nFullPath = {}\n", custom.display()),
        )
        .unwrap();
        assert_eq!(find_in(std::slice::from_ref(&default)), Ok(custom.clone()));
        std::fs::remove_dir_all(default).unwrap();
        std::fs::remove_dir_all(custom).unwrap();
    }

    #[test]
    fn reports_every_folder_it_looked_in() {
        let default = temp("none");
        let custom = std::env::temp_dir().join("listenosu-missing-custom");
        std::fs::write(
            default.join("storage.ini"),
            format!("FullPath={}", custom.display()),
        )
        .unwrap();
        let Err(searched) = find_in(std::slice::from_ref(&default)) else {
            panic!("nothing should be found");
        };
        assert_eq!(searched, [custom, default.clone()]);
        std::fs::remove_dir_all(default).unwrap();
    }
}
