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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

impl Repeat {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

/// Playback state as the UI sees it; the player crate owns the real state.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerView {
    pub playing: bool,
    pub position_secs: f32,
    pub length_secs: f32,
    /// 0.0 to 1.0.
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
}

/// Formats seconds as `m:ss`.
pub fn mmss(secs: f32) -> String {
    let total = secs.max(0.0) as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_cycles_through_all_modes() {
        assert_eq!(Repeat::Off.next(), Repeat::All);
        assert_eq!(Repeat::All.next(), Repeat::One);
        assert_eq!(Repeat::One.next(), Repeat::Off);
    }

    #[test]
    fn mmss_formats_and_clamps() {
        assert_eq!(mmss(0.0), "0:00");
        assert_eq!(mmss(75.9), "1:15");
        assert_eq!(mmss(-3.0), "0:00");
    }
}
