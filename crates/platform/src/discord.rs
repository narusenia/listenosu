use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{Activity, ActivityType, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// Discord is often not running; retry the connection no more often than this.
const RETRY: Duration = Duration::from_secs(15);
/// How far the shown start time may drift from the real one before it is sent again (a seek).
const DRIFT_MS: i64 = 2000;

/// What the presence shows. `None` clears it.
pub struct Listening<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub playing: bool,
    pub position_secs: f64,
}

/// Discord Rich Presence. Everything is best effort: no Discord, no presence, no error.
pub struct Presence {
    client: DiscordIpcClient,
    connected: bool,
    last_try: Option<Instant>,
    /// What was last sent: title, artist, playing, and the start time shown by Discord.
    sent: Option<(String, String, bool, i64)>,
}

impl Presence {
    /// `app_id` is the Application ID from the Discord Developer Portal.
    pub fn new(app_id: &str) -> Self {
        Self {
            client: DiscordIpcClient::new(app_id),
            connected: false,
            last_try: None,
            sent: None,
        }
    }

    /// Cheap to call every frame: Discord is only written to when something changed.
    pub fn update(&mut self, listening: Option<&Listening>) {
        let Some(l) = listening else {
            if self.sent.take().is_some() && self.connected {
                self.connected = self.client.clear_activity().is_ok();
            }
            return;
        };
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let start = now_ms - (l.position_secs * 1000.0) as i64;
        if let Some((title, artist, playing, shown)) = &self.sent {
            // While paused there is no clock, so the start time does not matter.
            let moved = l.playing && (shown - start).abs() > DRIFT_MS;
            if title == l.title && artist == l.artist && *playing == l.playing && !moved {
                return;
            }
        }
        if !self.connect() {
            return;
        }
        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            // The member list shows the song title instead of the app name.
            .status_display_type(StatusDisplayType::Details)
            .details(l.title)
            .state(l.artist);
        if l.playing {
            activity = activity.timestamps(Timestamps::new().start(start));
        }
        if self.client.set_activity(activity).is_ok() {
            self.sent = Some((l.title.to_owned(), l.artist.to_owned(), l.playing, start));
        } else {
            self.connected = false;
        }
    }

    fn connect(&mut self) -> bool {
        if self.connected {
            return true;
        }
        if self.last_try.is_some_and(|t| t.elapsed() < RETRY) {
            return false;
        }
        self.last_try = Some(Instant::now());
        self.connected = self.client.connect().is_ok();
        self.connected
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.client.close();
        }
    }
}
