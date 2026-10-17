//! What the song list shows: the view (all, a collection, a playlist), the search text and the order.

use std::collections::HashSet;

use library::Song;
use library::playlist::Playlists;
use library::search::{SortKey, search};
use ui::i18n::{t, tr};
use ui::model::SongView;

use crate::catalog::Catalog;

/// Labels for `SORTS`, in the same order.
pub fn sort_options() -> Vec<String> {
    [
        "title",
        "artist",
        "date_added",
        "length",
        "bpm",
        "last_played",
    ]
    .map(|key| t(&format!("sort.{key}")).to_owned())
    .to_vec()
}

/// In the order of `sort_options`, with whether the order is newest / biggest first.
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
        .map(|c| tr("main.collection", &[("name", &c.name)]));
    let lists = playlists
        .playlists
        .iter()
        .map(|p| tr("main.playlist", &[("name", &p.name)]));
    std::iter::once(t("main.all_songs").to_owned())
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
    group_same: bool,
) -> Vec<usize> {
    let (key, descending) = SORTS[sort.min(SORTS.len() - 1)];
    let mut rows = search(&catalog.songs, query, key, descending, romanized);
    if view == 0 {
        if group_same {
            group_same_songs(catalog, &mut rows);
        }
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
    // A playlist is the user's own list, so only collections are grouped.
    if group_same && view <= catalog.collections.len() {
        group_same_songs(catalog, &mut rows);
    }
    rows
}

/// Keeps the first row of each title and artist, so the best-sorted variant stands for the group.
/// The original-language spelling is the key either way, so the romanized toggle does not regroup.
fn group_same_songs(catalog: &Catalog, rows: &mut Vec<usize>) {
    let mut seen = HashSet::new();
    rows.retain(|&i| {
        let song = &catalog.songs[i];
        seen.insert((
            song.title(false).trim().to_lowercase(),
            song.artist(false).trim().to_lowercase(),
        ))
    });
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
            visible(&catalog, &playlists, query, view, 0, false, true)
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

    #[test]
    fn songs_with_the_same_title_and_artist_group_into_the_first_sorted_one_unless_opted_out() {
        let mut catalog = catalog();
        catalog.songs = vec![
            Song {
                bpm: 150.0,
                ..song("aa", "Same", "m1")
            },
            Song {
                bpm: 120.0,
                ..song("bb", "Same ", "m2")
            },
            song("cc", "Other", "m3"),
        ];
        catalog.collections.clear();
        let mut playlists = Playlists::default();
        let list = playlists.create("All");
        for s in catalog.songs.clone() {
            playlists.playlists[list].add(&s);
        }
        // Sorted by BPM (key 4), the slower variant comes first and stands for the group.
        let rows = |view, group| visible(&catalog, &playlists, "", view, 4, false, group);
        assert_eq!(rows(0, true), [2, 1]);
        assert_eq!(rows(0, false), [2, 1, 0]);
        assert_eq!(rows(1, true).len(), 3, "playlists are not grouped");
    }
}
