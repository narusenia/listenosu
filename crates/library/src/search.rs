use crate::model::Song;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortKey {
    Title,
    Artist,
    #[default]
    DateAdded,
    Length,
    Bpm,
    LastPlayed,
}

/// Indices into `songs` of the songs that match every whitespace-separated term of `text`
/// (case-insensitive, over title, artist, source, tags and mapper, in both scripts),
/// ordered by `sort`. Songs never played always come last when sorting by last played.
pub fn search(
    songs: &[Song],
    text: &str,
    sort: SortKey,
    descending: bool,
    romanized: bool,
) -> Vec<usize> {
    let terms: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
    let mut hits: Vec<usize> = (0..songs.len())
        .filter(|&i| matches(&songs[i], &terms))
        .collect();

    match sort {
        SortKey::Title => hits.sort_by_cached_key(|&i| songs[i].title(romanized).to_lowercase()),
        SortKey::Artist => hits.sort_by_cached_key(|&i| songs[i].artist(romanized).to_lowercase()),
        SortKey::DateAdded => hits.sort_by_key(|&i| songs[i].date_added_ms),
        SortKey::Length => hits.sort_by(|&a, &b| songs[a].length_ms.total_cmp(&songs[b].length_ms)),
        SortKey::Bpm => hits.sort_by(|&a, &b| songs[a].bpm.total_cmp(&songs[b].bpm)),
        SortKey::LastPlayed => hits.sort_by_key(|&i| songs[i].last_played_ms),
    }
    if descending {
        hits.reverse();
    }
    if sort == SortKey::LastPlayed {
        hits.sort_by_key(|&i| songs[i].last_played_ms.is_none()); // stable
    }
    hits
}

fn matches(song: &Song, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = [
        &song.title,
        &song.title_unicode,
        &song.artist,
        &song.artist_unicode,
        &song.source,
        &song.tags,
        &song.author,
    ]
    .map(|field| field.to_lowercase())
    .join("\n");
    terms.iter().all(|term| haystack.contains(term.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(title: &str, artist: &str) -> Song {
        Song {
            title: title.into(),
            artist: artist.into(),
            ..Song::default()
        }
    }

    #[test]
    fn every_term_must_match_in_any_field_ignoring_case() {
        let mut a = song("Freedom Dive", "xi");
        a.tags = "technical stream".into();
        a.author = "Nathan".into();
        let b = song("Freedom", "Someone Else");
        let songs = [a, b];
        assert_eq!(
            search(&songs, "freedom XI", SortKey::Title, false, false),
            [0]
        );
        assert_eq!(
            search(&songs, "stream nathan", SortKey::Title, false, false),
            [0]
        );
        assert_eq!(
            search(&songs, "freedom", SortKey::Title, false, false),
            [1, 0] // "Freedom" sorts before "Freedom Dive"
        );
        assert!(search(&songs, "freedom nobody", SortKey::Title, false, false).is_empty());
        assert_eq!(search(&songs, "   ", SortKey::Title, false, false).len(), 2);
    }

    #[test]
    fn original_language_fields_are_searched_too() {
        let mut s = song("Yoru ni Kakeru", "YOASOBI");
        s.title_unicode = "夜に駆ける".into();
        assert_eq!(search(&[s], "夜に", SortKey::Title, false, false), [0]);
    }

    #[test]
    fn sorts_by_the_displayed_script_and_reverses() {
        let mut a = song("b romaji", "");
        a.title_unicode = "あ".into();
        let b = song("a romaji", "");
        let songs = [a, b];
        assert_eq!(search(&songs, "", SortKey::Title, false, true), [1, 0]);
        assert_eq!(search(&songs, "", SortKey::Title, true, true), [0, 1]);
        // Original script: "a romaji" has no original title, so it sorts by its fallback.
        assert_eq!(search(&songs, "", SortKey::Title, false, false), [1, 0]);
    }

    #[test]
    fn numeric_sorts() {
        let mk = |bpm, length_ms, added| Song {
            bpm,
            length_ms,
            date_added_ms: added,
            ..Song::default()
        };
        let songs = [
            mk(180.0, 100.0, 3),
            mk(120.0, 300.0, 1),
            mk(150.0, 200.0, 2),
        ];
        assert_eq!(search(&songs, "", SortKey::Bpm, false, false), [1, 2, 0]);
        assert_eq!(search(&songs, "", SortKey::Length, true, false), [1, 2, 0]);
        assert_eq!(
            search(&songs, "", SortKey::DateAdded, false, false),
            [1, 2, 0]
        );
    }

    #[test]
    fn never_played_songs_come_last_in_both_directions() {
        let mk = |played| Song {
            last_played_ms: played,
            ..Song::default()
        };
        let songs = [mk(None), mk(Some(10)), mk(Some(20))];
        assert_eq!(
            search(&songs, "", SortKey::LastPlayed, false, false),
            [1, 2, 0]
        );
        assert_eq!(
            search(&songs, "", SortKey::LastPlayed, true, false),
            [2, 1, 0]
        );
    }
}
