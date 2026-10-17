//! UI text. The strings live in `locales/*.toml`, embedded in the binary.
//! Keys are dotted table paths (`settings.language`); `{name}` in a string is a placeholder for `tr`.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

type Strings = HashMap<String, String>;

const EN: &str = include_str!("../locales/en.toml");
const JA: &str = include_str!("../locales/ja.toml");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En = 0,
    Ja = 1,
}

static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

impl Lang {
    /// From the settings dropdown: 0 follows the OS, 1 is English, 2 is Japanese.
    pub fn from_setting(index: usize) -> Lang {
        match index {
            1 => Lang::En,
            2 => Lang::Ja,
            _ => system(),
        }
    }
}

pub fn set_language(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn language() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Ja as u8 {
        Lang::Ja
    } else {
        Lang::En
    }
}

/// The text for `key` in the current language, falling back to English, then to the key itself.
pub fn t(key: &str) -> &'static str {
    let (en, ja) = tables();
    let table = if language() == Lang::Ja { ja } else { en };
    table.get(key).or_else(|| en.get(key)).map_or_else(
        || Box::leak(key.to_owned().into_boxed_str()) as &str,
        |s| s.as_str(),
    )
}

/// `t` with `{name}` placeholders filled in.
pub fn tr(key: &str, args: &[(&str, &str)]) -> String {
    let mut text = t(key).to_owned();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

fn tables() -> &'static (Strings, Strings) {
    static TABLES: OnceLock<(Strings, Strings)> = OnceLock::new();
    TABLES.get_or_init(|| (parse(EN), parse(JA)))
}

fn parse(source: &str) -> Strings {
    let table: toml::Table = source.parse().expect("locale file is valid TOML");
    let mut out = Strings::new();
    flatten("", &table, &mut out);
    out
}

fn flatten(prefix: &str, table: &toml::Table, out: &mut Strings) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::String(s) => {
                out.insert(path, s.clone());
            }
            toml::Value::Table(inner) => flatten(&path, inner, out),
            _ => {}
        }
    }
}

#[cfg(windows)]
fn system() -> Lang {
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    // SAFETY: the buffer is LOCALE_NAME_MAX_LENGTH (85) wide, as the call requires.
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    let name = String::from_utf16_lossy(&buf[..(len.max(1) as usize - 1)]);
    from_locale(&name)
}

#[cfg(not(windows))]
fn system() -> Lang {
    let name = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    from_locale(&name)
}

fn from_locale(name: &str) -> Lang {
    if name.to_ascii_lowercase().starts_with("ja") {
        Lang::Ja
    } else {
        Lang::En
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_english_key_has_a_japanese_text() {
        let (en, ja) = tables();
        let missing: Vec<_> = en.keys().filter(|k| !ja.contains_key(*k)).collect();
        assert!(missing.is_empty(), "missing in ja.toml: {missing:?}");
    }

    #[test]
    fn placeholders_match_between_languages() {
        let (en, ja) = tables();
        for (key, text) in en {
            let count = |s: &str| s.matches('{').count();
            assert_eq!(count(text), count(&ja[key]), "{key}");
        }
    }

    #[test]
    fn placeholders_are_filled_and_unknown_keys_fall_back_to_the_key() {
        assert_eq!(from_locale("ja-JP"), Lang::Ja);
        assert_eq!(from_locale("en-US"), Lang::En);
        assert_eq!(t("no.such.key"), "no.such.key");
        assert_eq!(tr("toast.added_to", &[("name", "Mix")]), "Added to Mix");
    }
}
