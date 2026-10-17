use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{
    Activity, ActivityType, Assets, Button, StatusDisplayType, Timestamps,
};
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
    /// 0 when unknown; otherwise Discord shows the time left as a progress bar.
    pub length_secs: f64,
    /// Cover image URL. Discord (and the host of the image) learn which song is playing.
    pub cover_url: Option<&'a str>,
    /// Label and URL of the one button shown under the activity.
    pub button: Option<(&'a str, &'a str)>,
}

/// What was last sent; a new activity is written only when this changes.
#[derive(PartialEq)]
struct Sent {
    title: String,
    artist: String,
    playing: bool,
    /// Start time shown by Discord, in Unix milliseconds.
    start: i64,
    length_secs: u64,
    cover_url: Option<String>,
    button: Option<(String, String)>,
}

/// Discord Rich Presence. Everything is best effort: no Discord, no presence, no error.
pub struct Presence {
    client: DiscordIpcClient,
    connected: bool,
    last_try: Option<Instant>,
    sent: Option<Sent>,
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
        let mut now = Sent {
            title: l.title.to_owned(),
            artist: l.artist.to_owned(),
            playing: l.playing,
            start,
            length_secs: l.length_secs as u64,
            cover_url: l.cover_url.map(str::to_owned),
            button: l
                .button
                .map(|(label, url)| (label.to_owned(), url.to_owned())),
        };
        if let Some(sent) = &self.sent {
            // While paused there is no clock, so the start time does not matter. Within the
            // drift the old start stays, so the comparison below sees no change.
            if !l.playing || (sent.start - start).abs() <= DRIFT_MS {
                now.start = sent.start;
            }
            if *sent == now {
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
            let mut timestamps = Timestamps::new().start(start);
            if l.length_secs > 0.0 {
                timestamps = timestamps.end(start + (l.length_secs * 1000.0) as i64);
            }
            activity = activity.timestamps(timestamps);
        }
        if let Some(url) = l.cover_url {
            activity = activity.assets(Assets::new().large_image(url));
        }
        if let Some((label, url)) = l.button {
            activity = activity.buttons(vec![Button::new(label, url)]);
        }
        if self.client.set_activity(activity).is_ok() {
            now.start = start;
            self.sent = Some(now);
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
