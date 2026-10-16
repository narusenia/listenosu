//! What the song list shows: the view (all, a collection, a playlist), the search text and the order.

use std::collections::HashSet;

use library::Song;
use library::playlist::Playlists;
use library::search::{SortKey, search};
use ui::model::SongView;

use crate::catalog::Catalog;

pub const SORT_OPTIONS: [&str; 6] = [
    "Title",
    "Artist",
    "Date added",
    "Length",
    "BPM",
    "Last played",
];

/// In the order of `SORT_OPTIONS`, with whether the order is newest / biggest first.
const SORTS: [(SortKey, bool); 6] = [
    (SortKey::Title, false),
    (SortKey::Artist, false),
    (SortKey::DateAdded, true),
    (SortKey::Length, false),
    (SortKey::Bpm, false),
    (SortKey::LastPlayed, true),
];

/// "All songs", then one entry per collection, then one per playlist.
pub fn view_options(catalog: &Catalog, playlists: &Playlists) -> Vec<String> {
    let collections = catalog
        .collections
        .iter()
        .map(|c| format!("Collection: {}", c.name));
    let lists = playlists
        .playlists
        .iter()
        .map(|p| format!("Playlist: {}", p.name));
    std::iter::once("All songs".to_owned())
        .chain(collections)
        .chain(lists)
        .collect()
}

/// Indices into `catalog.songs` of the rows to show, in order.
pub fn visible(
    catalog: &Catalog,
    playlists: &Playlists,
    query: &str,
    view: usize,
    sort: usize,
    romanized: bool,
) -> Vec<usize> {
    let (key, descending) = SORTS[sort.min(SORTS.len() - 1)];
    let mut rows = search(&catalog.songs, query, key, descending, romanized);
    if view == 0 {
        return rows;
    }
    let members: HashSet<usize> = if view <= catalog.collections.len() {
        catalog.collection(view - 1).into_iter().collect()
    } else {
        match playlists
            .playlists
            .get(view - 1 - catalog.collections.len())
        {
            Some(playlist) => playlist
                .resolve(&catalog.songs)
                .into_iter()
                .flatten()
                .collect(),
            None => HashSet::new(),
        }
    };
    rows.retain(|i| members.contains(i));
    rows
}

pub fn song_view(song: &Song, missing: bool, romanized: bool) -> SongView {
    SongView {
        title: song.title(romanized).to_owned(),
        artist: song.artist(romanized).to_owned(),
        bpm: song.bpm as f32,
        length_secs: song.length_secs() as u32,
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use library::Collection;

    fn song(hash: &str, title: &str, md5: &str) -> Song {
        Song {
            audio_hash: hash.into(),
            title: title.into(),
            beatmap_md5s: vec![md5.into()],
            ..Song::default()
        }
    }

    fn catalog() -> Catalog {
        let mut catalog = Catalog::open(std::env::temp_dir().join("listenosu-no-such-dir"), None);
        catalog.songs = vec![
            song("aa", "Alpha", "m1"),
            song("bb", "Beta", "m2"),
            song("cc", "Gamma", "m3"),
        ];
        catalog.collections = vec![Collection {
            name: "Favorites".into(),
            beatmap_md5s: vec!["m2".into()],
        }];
        catalog
    }

    #[test]
    fn views_narrow_the_search_to_a_collection_or_a_playlist() {
        let catalog = catalog();
        let mut playlists = Playlists::default();
        let list = playlists.create("Work");
        playlists.playlists[list].add(&catalog.songs[2]);

        let options = view_options(&catalog, &playlists);
        assert_eq!(
            options,
            ["All songs", "Collection: Favorites", "Playlist: Work"]
        );

        let titles = |view, query| -> Vec<String> {
            visible(&catalog, &playlists, query, view, 0, false)
                .into_iter()
                .map(|i| catalog.songs[i].title.clone())
                .collect()
        };
        assert_eq!(titles(0, ""), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(titles(1, ""), ["Beta"]);
        assert_eq!(titles(2, ""), ["Gamma"]);
        assert_eq!(titles(0, "ta"), ["Beta"]);
        assert!(titles(1, "alpha").is_empty());
        assert!(titles(9, "").is_empty(), "a stale view index shows nothing");
    }
}
