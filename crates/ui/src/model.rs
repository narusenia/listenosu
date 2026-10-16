/// Display data for one song. The library crate converts its own model into this,
/// already resolving the original-language / romanized choice for title and artist.
#[derive(Clone, Debug, PartialEq)]
pub struct SongView {
    pub title: String,
    pub artist: String,
    pub bpm: f32,
    pub length_secs: u32,
    /// The audio file could not be found: shown dimmed and skipped on playback.
    pub missing: bool,
}
