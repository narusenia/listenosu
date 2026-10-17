//! The themes that ship with the app, and the ones a user drops into a folder.
//! A theme file is TOML in the shape of `Theme`; every missing value falls back to the lazer theme.

use std::path::Path;

use crate::theme::Theme;

const FILES: [&str; 10] = [
    include_str!("../themes/default-dark.toml"),
    include_str!("../themes/default-light.toml"),
    include_str!("../themes/catppuccin-mocha.toml"),
    include_str!("../themes/catppuccin-latte.toml"),
    include_str!("../themes/nord.toml"),
    include_str!("../themes/dracula.toml"),
    include_str!("../themes/tokyo-night.toml"),
    include_str!("../themes/gruvbox-dark.toml"),
    include_str!("../themes/solarized-dark.toml"),
    include_str!("../themes/rose-pine.toml"),
];

/// The lazer theme first, then the bundled ones.
pub fn builtin() -> Vec<Theme> {
    let bundled = FILES
        .iter()
        .map(|source| Theme::from_toml(source).expect("bundled theme is valid"));
    std::iter::once(Theme::lazer()).chain(bundled).collect()
}

/// The built-in themes followed by every valid `*.toml` in `dir`, in file name order.
/// A file that does not parse, or whose name is already taken, is skipped.
pub fn load(dir: Option<&Path>) -> Vec<Theme> {
    let mut themes = builtin();
    let Some(dir) = dir else { return themes };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return themes;
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    for path in paths {
        let theme = std::fs::read_to_string(&path)
            .ok()
            .and_then(|source| Theme::from_toml(&source).ok());
        if let Some(theme) = theme
            && !themes.iter().any(|t| t.name == theme.name)
        {
            themes.push(theme);
        }
    }
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_themes_parse_and_have_unique_names() {
        let themes = builtin();
        assert_eq!(themes.len(), FILES.len() + 1);
        let mut names: Vec<_> = themes.iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), themes.len());
    }

    #[test]
    fn light_themes_tint_the_backdrop_light() {
        let latte = builtin().into_iter().find(|t| t.name == "Catppuccin Latte");
        let latte = latte.unwrap();
        assert!(!latte.dark);
        assert_ne!(latte.backdrop.tint, crate::Theme::lazer().backdrop.tint);
    }

    #[test]
    fn user_themes_are_added_and_bad_or_duplicate_ones_skipped() {
        let dir = std::env::temp_dir().join("listenosu-themes-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.toml"),
            "name = \"Mine\"\n[colors]\nbase = [1, 2, 3, 255]\n",
        )
        .unwrap();
        std::fs::write(dir.join("b.toml"), "name = \"Nord\"\n").unwrap();
        std::fs::write(dir.join("c.toml"), "not toml [").unwrap();
        let themes = load(Some(&dir));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(themes.len(), builtin().len() + 1);
        let mine = themes.last().unwrap();
        assert_eq!(mine.name, "Mine");
        assert_eq!(mine.colors.base, egui::Color32::from_rgb(1, 2, 3));
        // The rest of a partial theme comes from lazer.
        assert_eq!(mine.colors.text, crate::Theme::lazer().colors.text);
    }
}
