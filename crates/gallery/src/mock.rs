use ui::model::SongView;

const TITLES: &[&str] = &[
    "Pixel Galaxy",
    "夜に駆ける",
    "Shippaisaku Shoujo",
    "あなたの特別になりたい",
    "you are my curse",
    "FREEDOM DiVE↓",
    "A Very Long Song Title That Does Not Fit In A Single Line Of The List (TV Size) [Extended Mix]",
    "とても長い曲名のサンプルです、一行に収まらない場合は末尾が省略されます ～アレンジ版～",
    "Magical",
    "Sparkle",
    "Biscuit Funk",
    "ブルーアーカイブ Theme",
];

const ARTISTS: &[&str] = &[
    "Snail's House",
    "YOASOBI",
    "Kairiki bear feat. Hatsune Miku",
    "Kotoha",
    "xi",
    "豚乙女 feat. ななひら",
    "Camellia",
];

/// Deterministic mock library for the gallery; every 97th song has a missing audio file.
pub fn songs(count: usize) -> Vec<SongView> {
    (0..count)
        .map(|i| SongView {
            title: format!("{} {}", TITLES[i % TITLES.len()], i + 1),
            artist: ARTISTS[(i * 7 + i / 3) % ARTISTS.len()].to_owned(),
            bpm: 90.0 + ((i * 37) % 160) as f32,
            length_secs: 90 + ((i * 53) % 240) as u32,
            missing: i % 97 == 96,
        })
        .collect()
}
