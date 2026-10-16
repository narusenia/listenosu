use std::path::Path;

use serde::Deserialize;

use crate::model::{Collection, Song};

/// A message from the helper (docs/helper-protocol.md).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Version {
        helper: String,
        realm: String,
        lazer: Option<String>,
    },
    Snapshot {
        songs: Vec<Song>,
        collections: Vec<Collection>,
    },
    Changed {
        songs: Vec<Song>,
        collections: Vec<Collection>,
    },
    Error {
        code: String,
        message: String,
    },
}

pub fn open_command(data_dir: &Path) -> String {
    serde_json::json!({ "cmd": "open", "data_dir": data_dir }).to_string()
}

pub const REFRESH_COMMAND: &str = r#"{"cmd":"refresh"}"#;
pub const QUIT_COMMAND: &str = r#"{"cmd":"quit"}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_the_helper_sends() {
        let snapshot = r#"{"type":"snapshot","songs":[{"audio_hash":"ab","background_hash":null,
            "title":"t","title_unicode":"t2","artist":"a","artist_unicode":"a2","source":"","tags":"",
            "author":"m","bpm":170.0,"length_ms":0,"date_added_ms":1,"last_played_ms":null,
            "beatmap_md5s":["x"]}],"collections":[{"name":"Fav","beatmap_md5s":["x"]}]}"#;
        let Message::Snapshot { songs, collections } = serde_json::from_str(snapshot).unwrap()
        else {
            panic!("not a snapshot");
        };
        assert_eq!(songs[0].title_unicode, "t2");
        assert_eq!(songs[0].last_played_ms, None);
        assert_eq!(collections[0].name, "Fav");

        let error = r#"{"type":"error","code":"open_failed","message":"nope"}"#;
        assert!(matches!(
            serde_json::from_str::<Message>(error),
            Ok(Message::Error { code, .. }) if code == "open_failed"
        ));
        let version = r#"{"type":"version","helper":"0.1.0","realm":"20.1.0","lazer":null}"#;
        assert!(matches!(
            serde_json::from_str::<Message>(version),
            Ok(Message::Version { lazer: None, .. })
        ));
    }

    #[test]
    fn open_command_escapes_the_path() {
        let line = open_command(Path::new(r"C:\Users\x\osu"));
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["data_dir"], r"C:\Users\x\osu");
    }
}
