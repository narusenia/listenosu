use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;
use library::Song;
use library::playlist::Playlists;
use library::store::AppDirs;
use player::{Event, Player, Repeat, Status};
use ui::Theme;
use ui::backdrop::Backdrop;
use ui::components::notice::{NoticeKind, Toasts};
use ui::components::playbar::PlaybarEvent;
use ui::components::playlist_editor::PlaylistView;
use ui::components::song_rows::RowEvent;
use ui::components::spectrum::SpectrumSmoother;
use ui::components::titlebar::{TitlebarEvent, titlebar};
use ui::model::{PlayerView, Repeat as UiRepeat, SongView};
use ui::screens::about::about_screen;
use ui::screens::first_run::first_run_screen;
use ui::screens::main_screen::{MainInput, MainState, main_screen};
use ui::screens::mini_player::mini_player;
use ui::screens::settings::{SettingsState, settings_screen};

use crate::backdrop_loader::{BackdropLoader, Decoded};
use crate::catalog::Catalog;
use crate::config::{Config, Geometry, LastSession};
use crate::listing::{self, SORT_OPTIONS, song_view};
use crate::side::{Action, Side, SideInput};

pub const FULL_SIZE: [f32; 2] = [1400.0, 860.0];

/// A content size plus the title bar, as the window's inner size.
pub fn window_size(shape: &ui::theme::Shape, content: [f32; 2]) -> [f32; 2] {
    [content[0], content[1] + shape.titlebar_height]
}
/// How long after a change the settings and window position are written.
const SAVE_DELAY: Duration = Duration::from_secs(2);
/// After a seek the engine's position lags by a few hundred ms, so the UI shows the target meanwhile.
const SEEK_HOLD: Duration = Duration::from_millis(600);
/// Frames to wait after a window mode change before trusting the reported window size.
const SETTLE_FRAMES: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Main,
    Settings,
    About,
}

pub struct App {
    theme: Theme,
    theme_applied: bool,
    dirs: Option<AppDirs>,
    config: Config,
    config_dirty_since: Option<Instant>,

    /// None until a lazer folder is found; the first-run page is shown meanwhile.
    catalog: Option<Catalog>,
    searched: Vec<String>,
    playlists: Playlists,
    player: Option<Player<Song>>,
    pending_session: Option<LastSession>,

    page: Page,
    mini: bool,
    settings: SettingsState,
    main: MainState,
    side: Side,
    toasts: Toasts,
    dismissed_problem: Option<String>,

    /// Indices into `catalog.songs`, in the order shown.
    visible: Vec<usize>,
    views: Vec<SongView>,
    view_options: Vec<String>,
    sort_options: Vec<String>,
    playing_row: Option<usize>,
    now_playing: Option<Song>,
    now_view: Option<SongView>,
    queue_dirty: bool,
    playlists_dirty: bool,

    backdrop: Backdrop,
    loader: BackdropLoader,
    smoother: SpectrumSmoother,
    seek_hold: Option<(f32, Instant)>,
    settle: u8,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, dirs: Option<AppDirs>, config: Config) -> Self {
        ui::fonts::install(&cc.egui_ctx);
        let playlists = dirs
            .as_ref()
            .map(|d| Playlists::load(&d.playlists_file()))
            .unwrap_or_default();
        let mut app = Self {
            theme: Theme::lazer(),
            theme_applied: false,
            settings: config.settings_state(),
            main: MainState {
                sort: config.sort,
                ..Default::default()
            },
            mini: config.mini,
            pending_session: config.last_session.clone(),
            dirs,
            config,
            config_dirty_since: None,
            catalog: None,
            searched: Vec::new(),
            playlists,
            player: None,
            page: Page::Main,
            side: Side::default(),
            toasts: Toasts::default(),
            dismissed_problem: None,
            visible: Vec::new(),
            views: Vec::new(),
            view_options: Vec::new(),
            sort_options: SORT_OPTIONS.map(String::from).to_vec(),
            playing_row: None,
            now_playing: None,
            now_view: None,
            queue_dirty: true,
            playlists_dirty: true,
            backdrop: Backdrop::default(),
            loader: BackdropLoader::new(cc.egui_ctx.clone()),
            smoother: SpectrumSmoother::default(),
            seek_hold: None,
            settle: SETTLE_FRAMES,
        };
        app.open_catalog();
        app
    }

    /// Looks for the lazer folder and starts reading the library. Without one, the first-run page shows.
    fn open_catalog(&mut self) {
        let found = if self.config.data_folder.is_empty() {
            library::discover::detect()
        } else {
            library::discover::find_in(&[PathBuf::from(&self.config.data_folder)])
        };
        match found {
            Ok(data_dir) => {
                let cache = self.dirs.as_ref().map(AppDirs::library_cache_file);
                self.catalog = Some(Catalog::open(data_dir.clone(), cache));
                self.start_player(data_dir);
                self.after_library_change();
            }
            Err(searched) => {
                self.searched = searched.iter().map(|p| p.display().to_string()).collect();
            }
        }
    }

    fn start_player(&mut self, data_dir: PathBuf) {
        match Player::new(move |song: &Song| song.audio_path(&data_dir)) {
            Ok(mut player) => {
                player.set_volume(self.config.volume);
                player.set_crossfade(Duration::from_secs_f32(self.config.crossfade_secs));
                player.set_shuffle(self.config.shuffle);
                player.set_repeat(self.config.repeat_mode());
                self.player = Some(player);
            }
            Err(why) => self.toasts.push(
                NoticeKind::Error,
                format!("Audio output is not available: {why}"),
                self.theme.motion.toast_seconds,
            ),
        }
    }

    /// The song list changed (first load, a helper update, a new library): rebuild what depends on it.
    fn after_library_change(&mut self) {
        self.restore_session();
        self.rebuild_list();
        self.queue_dirty = true;
        self.playlists_dirty = true;
    }

    /// Loads the last session's queue paused, once there are songs to build it from.
    fn restore_session(&mut self) {
        let (Some(catalog), Some(player)) = (&self.catalog, &mut self.player) else {
            return;
        };
        if catalog.songs.is_empty() || !player.queue().items().is_empty() {
            return;
        }
        let Some(session) = self.pending_session.take() else {
            return;
        };
        if let Some((songs, index)) = session_songs(&session, &catalog.songs) {
            player.restore(songs, index, session.position_secs);
        }
    }

    fn romanized(&self) -> bool {
        self.config.romanized
    }

    fn rebuild_list(&mut self) {
        let Some(catalog) = &self.catalog else { return };
        self.view_options = listing::view_options(catalog, &self.playlists);
        if self.main.view >= self.view_options.len() {
            self.main.view = 0;
        }
        self.visible = listing::visible(
            catalog,
            &self.playlists,
            &self.main.query,
            self.main.view,
            self.main.sort,
            self.romanized(),
            self.config.group_same,
        );
        self.views = self
            .visible
            .iter()
            .map(|&i| {
                let song = &catalog.songs[i];
                song_view(song, catalog.is_missing(song), self.config.romanized)
            })
            .collect();
        if self
            .main
            .selected
            .is_some_and(|row| row >= self.views.len())
        {
            self.main.selected = None;
        }
        self.update_playing_row();
    }

    fn update_playing_row(&mut self) {
        let (Some(catalog), Some(now)) = (&self.catalog, &self.now_playing) else {
            self.playing_row = None;
            return;
        };
        self.playing_row = self
            .visible
            .iter()
            .position(|&i| catalog.songs[i].audio_hash == now.audio_hash);
    }

    fn selected_song(&self) -> Option<&Song> {
        let catalog = self.catalog.as_ref()?;
        let index = *self.visible.get(self.main.selected?)?;
        catalog.songs.get(index)
    }

    // ---- per-frame work -------------------------------------------------------------------

    fn tick(&mut self, ctx: &egui::Context) {
        if self.catalog.as_mut().is_some_and(Catalog::poll) {
            self.after_library_change();
        }
        let events = self.player.as_mut().map(Player::tick).unwrap_or_default();
        self.handle_player_events(ctx, events);
        if let Some(decoded) = self.loader.poll() {
            self.set_backdrop(ctx, &decoded);
        }
    }

    fn handle_player_events(&mut self, ctx: &egui::Context, events: Vec<Event>) {
        let Some(player) = &self.player else { return };
        // Resolve the queue items first, so the player is not borrowed while the app changes.
        let events: Vec<(Event, Option<Song>)> = events
            .into_iter()
            .map(|event| {
                let song = match event {
                    Event::Started(i) | Event::Skipped(i) => player.queue().items().get(i).cloned(),
                    Event::Finished => None,
                };
                (event, song)
            })
            .collect();
        let mut skipped = Vec::new();
        for (event, song) in events {
            match (event, song) {
                (Event::Started(_), Some(song)) => self.start_song(ctx, song),
                (Event::Skipped(_), Some(song)) => {
                    skipped.push(song.title(self.config.romanized).to_owned());
                }
                (Event::Finished, _) => {
                    self.now_playing = None;
                    self.now_view = None;
                    self.playing_row = None;
                }
                _ => {}
            }
        }
        let seconds = self.theme.motion.toast_seconds;
        match skipped.as_slice() {
            [] => {}
            [one] => self.toasts.push(
                NoticeKind::Warning,
                format!("Skipped (file not found): {one}"),
                seconds,
            ),
            many => self.toasts.push(
                NoticeKind::Warning,
                format!("Skipped {} songs whose files were not found", many.len()),
                seconds,
            ),
        }
    }

    fn start_song(&mut self, ctx: &egui::Context, song: Song) {
        let Some(catalog) = &self.catalog else { return };
        self.now_view = Some(song_view(
            &song,
            catalog.is_missing(&song),
            self.config.romanized,
        ));
        match song.background_path(&catalog.data_dir) {
            Some(path) => {
                if let Some(decoded) = self.loader.request(path) {
                    self.set_backdrop(ctx, &decoded);
                }
            }
            None => {
                self.loader.cancel();
                self.backdrop.clear();
            }
        }
        self.now_playing = Some(song);
        self.update_playing_row();
    }

    fn set_backdrop(&mut self, ctx: &egui::Context, decoded: &Decoded) {
        self.backdrop.set_image(
            ctx,
            &decoded.rgba,
            decoded.width,
            decoded.height,
            self.theme.backdrop.blur,
        );
    }

    fn player_view(&mut self) -> (PlayerView, Status) {
        let status = self.player.as_ref().map(Player::status).unwrap_or_default();
        let mut position = status.position as f32;
        if let Some((target, at)) = self.seek_hold {
            if at.elapsed() < SEEK_HOLD {
                position = target;
            } else {
                self.seek_hold = None;
            }
        }
        let view = PlayerView {
            playing: status.index.is_some() && !status.paused,
            position_secs: position,
            length_secs: status.length as f32,
            volume: self.config.volume,
            shuffle: self.config.shuffle,
            repeat: match self.config.repeat_mode() {
                Repeat::Off => UiRepeat::Off,
                Repeat::All => UiRepeat::All,
                Repeat::One => UiRepeat::One,
            },
        };
        (view, status)
    }

    /// Smoothed bar levels. Nothing is analyzed while the window is minimized or the setting is off.
    fn spectrum(&mut self, ctx: &egui::Context, dt: f32) -> Vec<f32> {
        let visible =
            self.config.spectrum && !ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let bars = self.theme.spectrum.bars;
        let raw = match &mut self.player {
            Some(player) => fit(&player.spectrum(visible), bars),
            None => vec![0.0; bars],
        };
        self.smoother
            .update(&raw, dt, self.theme.spectrum.decay_per_second)
            .to_vec()
    }

    // ---- actions --------------------------------------------------------------------------

    fn play_row(&mut self, row: usize) {
        let Some(catalog) = &self.catalog else { return };
        let songs: Vec<Song> = self
            .visible
            .iter()
            .map(|&i| catalog.songs[i].clone())
            .collect();
        if let Some(player) = &mut self.player {
            player.play(songs, row);
            self.queue_dirty = true;
        }
        self.main.selected = Some(row);
    }

    fn apply_playbar(&mut self, event: PlaybarEvent, status: Status) {
        if self.player.is_none() {
            return;
        }
        if event.toggle_play {
            if status.index.is_some() {
                self.with_player(Player::toggle);
            } else if !self.visible.is_empty() {
                self.play_row(self.main.selected.unwrap_or(0));
                return;
            }
        }
        if event.toggle_shuffle {
            self.config.shuffle = !self.config.shuffle;
            let on = self.config.shuffle;
            self.with_player(|p| p.set_shuffle(on));
            self.touch_config();
        }
        if event.cycle_repeat {
            let next = match self.config.repeat_mode() {
                Repeat::Off => Repeat::All,
                Repeat::All => Repeat::One,
                Repeat::One => Repeat::Off,
            };
            self.config.set_repeat_mode(next);
            self.with_player(|p| p.set_repeat(next));
            self.touch_config();
        }
        if let Some(secs) = event.seek_secs {
            self.with_player(|p| p.seek(f64::from(secs)));
            self.seek_hold = Some((secs, Instant::now()));
        }
        if let Some(volume) = event.volume {
            self.config.volume = volume;
            self.with_player(|p| p.set_volume(volume));
            self.touch_config();
        }
        if event.next {
            self.with_player(Player::next);
        }
        if event.prev {
            self.with_player(Player::previous);
        }
    }

    fn with_player(&mut self, f: impl FnOnce(&mut Player<Song>)) {
        if let Some(player) = &mut self.player {
            f(player);
        }
    }

    fn apply_side(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::DismissBanner => {
                    self.dismissed_problem = self.catalog.as_ref().and_then(|c| c.problem.clone());
                }
                Action::ClearQueue => {
                    if let Some(player) = &mut self.player {
                        player.clear();
                        self.now_playing = None;
                        self.now_view = None;
                        self.playing_row = None;
                        self.queue_dirty = true;
                    }
                }
                Action::AddToQueue => {
                    if let (Some(song), Some(player)) =
                        (self.selected_song().cloned(), &mut self.player)
                    {
                        player.enqueue(song);
                        self.queue_dirty = true;
                    }
                }
                Action::QueueRows(rows) => self.apply_queue_rows(rows),
                Action::AddToPlaylist(index) => self.add_to_playlist(index),
                Action::Playlist(event) => self.apply_playlist_event(event),
            }
        }
    }

    fn apply_queue_rows(&mut self, rows: RowEvent) {
        let Some(player) = &mut self.player else {
            return;
        };
        let len = player.queue().items().len();
        if let Some(index) = rows.play {
            player.jump(index);
        }
        if let Some(index) = rows.remove {
            player.remove(index);
            self.queue_dirty = true;
        }
        if let Some(index) = rows.move_up.filter(|&i| i > 0) {
            player.swap(index, index - 1);
            self.queue_dirty = true;
        }
        if let Some(index) = rows.move_down.filter(|&i| i + 1 < len) {
            player.swap(index, index + 1);
            self.queue_dirty = true;
        }
    }

    fn add_to_playlist(&mut self, index: usize) {
        let Some(song) = self.selected_song().cloned() else {
            return;
        };
        let Some(playlist) = self.playlists.playlists.get_mut(index) else {
            return;
        };
        let (added, name) = (playlist.add(&song), playlist.name.clone());
        let text = if added {
            format!("Added to {name}")
        } else {
            format!("Already in {name}")
        };
        self.toasts
            .push(NoticeKind::Info, text, self.theme.motion.toast_seconds);
        self.playlists_changed();
    }

    fn apply_playlist_event(&mut self, event: ui::components::playlist_editor::PlaylistEvent) {
        if let Some(index) = event.select {
            self.side.selected_playlist = Some(index);
        }
        if let Some(name) = event.create {
            self.side.selected_playlist = Some(self.playlists.create(&name));
            self.playlists_changed();
        }
        let Some(selected) = self.side.selected_playlist else {
            return;
        };
        if let Some(name) = event.rename {
            self.playlists.rename(selected, &name);
            self.playlists_changed();
        }
        if event.delete {
            self.playlists.delete(selected);
            self.side.selected_playlist = None;
            self.playlists_changed();
            return;
        }
        let rows = event.rows;
        let Some(playlist) = self.playlists.playlists.get_mut(selected) else {
            return;
        };
        if let Some(index) = rows.remove {
            playlist.remove(index);
        }
        if let Some(index) = rows.move_up {
            playlist.move_up(index);
        }
        if let Some(index) = rows.move_down {
            playlist.move_down(index);
        }
        if rows.remove.is_some() || rows.move_up.is_some() || rows.move_down.is_some() {
            self.playlists_changed();
        }
        if let Some(row) = rows.play {
            self.play_playlist(selected, row);
        }
    }

    /// Plays a playlist from `row`. Entries no longer in the library are left out of the queue.
    fn play_playlist(&mut self, playlist: usize, row: usize) {
        let (Some(catalog), Some(list)) = (&self.catalog, self.playlists.playlists.get(playlist))
        else {
            return;
        };
        let resolved = list.resolve(&catalog.songs);
        if resolved.get(row).copied().flatten().is_none() {
            return;
        }
        let songs: Vec<Song> = resolved
            .iter()
            .flatten()
            .map(|&i| catalog.songs[i].clone())
            .collect();
        let start = resolved[..row].iter().flatten().count();
        if let Some(player) = &mut self.player {
            player.play(songs, start);
            self.queue_dirty = true;
        }
    }

    fn playlists_changed(&mut self) {
        self.playlists_dirty = true;
        if let Some(dirs) = &self.dirs {
            // Playlists are the user's own data: a failed save is worth telling them about.
            if self.playlists.save(&dirs.playlists_file()).is_err() {
                self.toasts.push(
                    NoticeKind::Error,
                    "Could not save the playlists.",
                    self.theme.motion.toast_seconds,
                );
            }
        }
        self.rebuild_list();
    }

    fn refresh_side_views(&mut self) {
        let (Some(catalog), Some(player)) = (&self.catalog, &self.player) else {
            return;
        };
        let romanized = self.config.romanized;
        if self.queue_dirty {
            self.queue_dirty = false;
            self.side.queue_views = player
                .queue()
                .items()
                .iter()
                .map(|s| song_view(s, catalog.is_missing(s), romanized))
                .collect();
        }
        if self.playlists_dirty {
            self.playlists_dirty = false;
            self.side.playlist_views = self
                .playlists
                .playlists
                .iter()
                .map(|playlist| PlaylistView {
                    name: playlist.name.clone(),
                    songs: playlist
                        .entries
                        .iter()
                        .zip(playlist.resolve(&catalog.songs))
                        .map(|(entry, found)| match found {
                            Some(i) => {
                                let song = &catalog.songs[i];
                                song_view(song, catalog.is_missing(song), romanized)
                            }
                            // Gone from the library: show the saved copy of its name, dimmed.
                            None => SongView {
                                title: entry.title.clone(),
                                artist: entry.artist.clone(),
                                bpm: 0.0,
                                length_secs: 0,
                                missing: true,
                            },
                        })
                        .collect(),
                })
                .collect();
        }
    }

    // ---- settings, window, persistence ----------------------------------------------------

    fn touch_config(&mut self) {
        self.config_dirty_since.get_or_insert_with(Instant::now);
    }

    fn apply_settings(&mut self, ctx: &egui::Context) {
        let (romanized_before, group_before) = (self.config.romanized, self.config.group_same);
        self.config.apply_settings(&self.settings);
        if let Some(player) = &mut self.player {
            player.set_crossfade(Duration::from_secs_f32(self.config.crossfade_secs));
        }
        if group_before != self.config.group_same {
            self.rebuild_list();
        }
        if romanized_before != self.config.romanized {
            self.rebuild_list();
            self.queue_dirty = true;
            self.playlists_dirty = true;
            if let Some(song) = self.now_playing.clone() {
                self.now_view = self
                    .catalog
                    .as_ref()
                    .map(|c| song_view(&song, c.is_missing(&song), self.config.romanized));
            }
        }
        self.apply_window_level(ctx);
        self.touch_config();
    }

    fn apply_window_level(&self, ctx: &egui::Context) {
        let level = if self.mini && self.config.mini_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    fn apply_titlebar(&mut self, ctx: &egui::Context, bar: TitlebarEvent) {
        use egui::ViewportCommand as Cmd;
        if bar.minimize {
            ctx.send_viewport_cmd(Cmd::Minimized(true));
        }
        if bar.toggle_maximize {
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(Cmd::Maximized(!maximized));
        }
        if bar.close {
            ctx.send_viewport_cmd(Cmd::Close);
        }
        if bar.settings {
            self.page = Page::Settings;
        }
        if bar.about {
            self.page = Page::About;
        }
    }

    fn set_mini(&mut self, ctx: &egui::Context, mini: bool) {
        self.mini = mini;
        self.config.mini = mini;
        let shape = &self.theme.shape;
        let (default_size, min, saved) = if mini {
            (
                window_size(shape, shape.mini_size),
                window_size(shape, shape.mini_size),
                self.config.mini_window,
            )
        } else {
            (
                window_size(shape, FULL_SIZE),
                window_size(shape, shape.min_size),
                self.config.full_window,
            )
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
            saved.map_or(default_size, |g| g.size).into(),
        ));
        if let Some(g) = saved {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(g.position.into()));
        }
        self.apply_window_level(ctx);
        self.settle = SETTLE_FRAMES;
        self.touch_config();
    }

    /// Remembers where the window is, for each mode.
    fn track_window(&mut self, ctx: &egui::Context) {
        if self.settle > 0 {
            self.settle -= 1;
            return;
        }
        let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
        let (Some(inner), Some(outer)) = (inner, outer) else {
            return;
        };
        let now = Geometry {
            position: [outer.min.x, outer.min.y],
            size: [inner.width(), inner.height()],
        };
        let slot = if self.mini {
            &mut self.config.mini_window
        } else {
            &mut self.config.full_window
        };
        if *slot != Some(now) {
            *slot = Some(now);
            self.touch_config();
        }
    }

    fn session(&self) -> Option<LastSession> {
        let player = self.player.as_ref()?;
        let queue = player.queue();
        let index = queue.current()?;
        Some(LastSession {
            audio_hashes: queue.items().iter().map(|s| s.audio_hash.clone()).collect(),
            index,
            position_secs: player.status().position,
        })
    }

    fn save_config(&mut self, with_session: bool) {
        if with_session {
            self.config.last_session = self.session().or(self.config.last_session.take());
        }
        if let Some(dirs) = &self.dirs {
            self.config.save(dirs);
        }
        self.config_dirty_since = None;
    }

    fn maybe_save(&mut self) {
        if self
            .config_dirty_since
            .is_some_and(|since| since.elapsed() >= SAVE_DELAY)
        {
            self.save_config(false);
        }
    }

    // ---- pages ----------------------------------------------------------------------------

    fn show_first_run(&mut self, ui: &mut egui::Ui) {
        let event = first_run_screen(ui, &self.theme, &self.searched);
        if event.choose_folder {
            self.toasts.push(
                NoticeKind::Info,
                "Choosing a folder is not available yet. Set the folder in settings.json.",
                self.theme.motion.toast_seconds,
            );
        }
        if event.retry {
            self.open_catalog();
        }
    }

    fn show_page(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let fill = self.theme.colors.base;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(fill))
            .show(ui, |ui| match self.page {
                Page::Settings => {
                    let before = self.settings.clone();
                    let event = settings_screen(ui, &self.theme, &mut self.settings);
                    if event.choose_folder {
                        self.toasts.push(
                            NoticeKind::Info,
                            "Choosing a folder is not available yet. Set the folder in settings.json.",
                            self.theme.motion.toast_seconds,
                        );
                    }
                    if before != self.settings {
                        self.apply_settings(ctx);
                    }
                    if event.back {
                        self.page = Page::Main;
                    }
                }
                Page::About => {
                    if about_screen(ui, &self.theme, env!("CARGO_PKG_VERSION")) {
                        self.page = Page::Main;
                    }
                }
                Page::Main => {}
            });
    }

    fn show_mini(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let dt = ctx.input(|i| i.stable_dt);
        let levels = self.spectrum(ctx, dt);
        let (view, status) = self.player_view();
        let event = mini_player(
            ui,
            &self.theme,
            &mut self.backdrop,
            self.now_view.as_ref(),
            &view,
            &levels,
        );
        self.apply_playbar(event.playbar, status);
        if event.expand {
            self.set_mini(ctx, false);
        }
    }

    fn show_main(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let dt = ctx.input(|i| i.stable_dt);
        let levels = self.spectrum(ctx, dt);
        let (view, status) = self.player_view();
        self.refresh_side_views();

        let problem = self
            .catalog
            .as_ref()
            .and_then(|c| c.problem.as_deref())
            .filter(|text| self.dismissed_problem.as_deref() != Some(text));
        let side_input = SideInput {
            queue_current: self.player.as_ref().and_then(|p| p.queue().current()),
            can_add: self.main.selected.is_some(),
            problem,
        };
        let input = MainInput {
            songs: &self.views,
            playing: self.playing_row,
            now_playing: self.now_view.as_ref(),
            player: &view,
            spectrum: &levels,
            view_options: &self.view_options,
            sort_options: &self.sort_options,
        };
        let mut actions = Vec::new();
        let event = main_screen(
            ui,
            &self.theme,
            &mut self.backdrop,
            &mut self.main,
            &input,
            |ui| actions = self.side.show(ui, &self.theme, &side_input),
        );

        if event.list_changed {
            self.config.sort = self.main.sort;
            self.touch_config();
            self.rebuild_list();
        }
        if let Some(row) = event.list.activated {
            self.play_row(row);
        }
        if event.playbar.toggle_mini {
            self.set_mini(ctx, true);
        }
        self.apply_playbar(event.playbar, status);
        self.apply_side(actions);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.theme_applied {
            self.theme.apply(&ctx);
            self.theme_applied = true;
            self.apply_window_level(&ctx);
        }
        self.tick(&ctx);
        self.track_window(&ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_config(true);
        }

        let bar = titlebar(
            ui,
            &self.theme,
            "listenosu",
            !self.mini,
            self.catalog.is_some() && !self.mini,
        );
        self.apply_titlebar(&ctx, bar);

        if self.catalog.is_none() {
            self.show_first_run(ui);
        } else if self.mini {
            self.show_mini(&ctx, ui);
        } else if self.page != Page::Main {
            self.show_page(&ctx, ui);
        } else {
            self.show_main(&ctx, ui);
        }
        self.toasts.show(&ctx, &self.theme);
        self.maybe_save();

        // Playing: animate the spectrum and the clock. Otherwise only wake to pick up helper messages.
        let playing = self.player.as_ref().is_some_and(|p| {
            let status = p.status();
            status.index.is_some() && !status.paused
        });
        ctx.request_repaint_after(Duration::from_millis(if playing { 16 } else { 250 }));
    }
}

/// Spreads or averages `levels` over `n` bars.
fn fit(levels: &[f32], n: usize) -> Vec<f32> {
    if n == 0 || levels.is_empty() {
        return vec![0.0; n];
    }
    (0..n)
        .map(|bar| {
            let from = bar * levels.len() / n;
            let to = ((bar + 1) * levels.len() / n).clamp(from + 1, levels.len());
            levels[from..to].iter().copied().fold(0.0, f32::max)
        })
        .collect()
}

/// The songs of a saved queue that are still in the library, and where the saved index lands among them.
fn session_songs(session: &LastSession, songs: &[Song]) -> Option<(Vec<Song>, usize)> {
    let by_hash: std::collections::HashMap<&str, &Song> =
        songs.iter().map(|s| (s.audio_hash.as_str(), s)).collect();
    let mut kept = Vec::new();
    let mut index = None;
    for (i, hash) in session.audio_hashes.iter().enumerate() {
        if let Some(song) = by_hash.get(hash.as_str()) {
            if i >= session.index && index.is_none() {
                index = Some(kept.len());
            }
            kept.push((*song).clone());
        }
    }
    // The saved track is gone: fall back to the last kept one before it, or the first.
    let index = index.unwrap_or(0).min(kept.len().saturating_sub(1));
    (!kept.is_empty()).then_some((kept, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(hash: &str) -> Song {
        Song {
            audio_hash: hash.into(),
            ..Song::default()
        }
    }

    fn session(hashes: &[&str], index: usize) -> LastSession {
        LastSession {
            audio_hashes: hashes.iter().map(|h| h.to_string()).collect(),
            index,
            position_secs: 0.0,
        }
    }

    #[test]
    fn a_session_keeps_the_songs_still_in_the_library_and_the_current_one() {
        let library = [song("a"), song("c"), song("d")];
        let (songs, index) = session_songs(&session(&["a", "b", "c", "d"], 2), &library).unwrap();
        let hashes: Vec<_> = songs.iter().map(|s| s.audio_hash.as_str()).collect();
        assert_eq!((hashes, index), (vec!["a", "c", "d"], 1));
    }

    #[test]
    fn a_missing_current_song_moves_to_the_next_kept_one() {
        let library = [song("a"), song("c")];
        let (_, index) = session_songs(&session(&["a", "b", "c"], 1), &library).unwrap();
        assert_eq!(index, 1);
    }

    #[test]
    fn a_session_with_nothing_left_is_dropped() {
        assert!(session_songs(&session(&["x"], 0), &[song("a")]).is_none());
    }

    #[test]
    fn fit_keeps_the_loudest_level_of_each_group() {
        assert_eq!(fit(&[0.1, 0.9, 0.2, 0.3], 2), [0.9, 0.3]);
        assert_eq!(fit(&[0.5], 3), [0.5, 0.5, 0.5]);
    }
}
