mod mock;
mod sections;
mod theme_panel;

use std::path::Path;

use eframe::egui;
use ui::Theme;
use ui::backdrop::Backdrop;
use ui::components::notice::NoticeKind;
use ui::components::playbar::PlaybarEvent;
use ui::components::song_card::{CardState, song_card};
use ui::components::spectrum::SpectrumSmoother;
use ui::model::{PlayerView, Repeat, SongView};
use ui::screens::about::about_screen;
use ui::screens::first_run::first_run_screen;
use ui::screens::main_screen::{MainEvent, MainInput, MainState, main_screen};
use ui::screens::mini_player::mini_player;
use ui::screens::settings::{SettingsState, settings_screen};

const VIEW_OPTIONS: [&str; 3] = ["All songs", "Collection: Favorites", "Playlist: Work BGM"];
const SORT_OPTIONS: [&str; 6] = [
    "Title",
    "Artist",
    "Date added",
    "Length",
    "BPM",
    "Last played",
];

struct Image {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

struct Gallery {
    section: sections::Section,
    extras: sections::Extras,
    theme: Theme,
    applied: Option<Theme>,
    status: String,

    songs: Vec<SongView>,
    visible: Vec<SongView>,
    main: MainState,
    view_options: Vec<String>,
    sort_options: Vec<String>,
    /// Show only the main screen (`cargo run -p gallery -- main`), without the component tabs.
    main_only: bool,
    /// Showing the mini player (`cargo run -p gallery -- mini` starts there).
    mini: bool,
    /// A full-window page opened from the command line: `settings`, `first-run` or `about`.
    page: Option<Page>,
    settings: SettingsState,
    playing: Option<usize>,

    player: PlayerView,
    smoother: SpectrumSmoother,
    clock: f32,

    images: Vec<Image>,
    backdrop: Backdrop,
    backdrop_image: Option<usize>,
    backdrop_blur: f32,
}

impl Gallery {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::fonts::install(&cc.egui_ctx);
        let songs = mock::songs(5000);
        Self {
            section: std::env::args()
                .nth(1)
                .and_then(|arg| sections::Section::from_arg(&arg))
                .unwrap_or_default(),
            main_only: std::env::args().nth(1).as_deref() == Some("main"),
            mini: starts_mini(),
            page: std::env::args().nth(1).as_deref().and_then(Page::from_arg),
            settings: SettingsState {
                data_folder: SAMPLE_FOLDER.to_owned(),
                language: 0,
                romanized: false,
                crossfade_secs: 0.0,
                tray_on_close: true,
                mini_on_top: false,
                spectrum: true,
                hotkeys: false,
                hotkey_play_pause: None,
                discord: false,
                check_updates: true,
                capturing: false,
            },
            extras: sections::Extras::new(&songs),
            theme: Theme::lazer(),
            applied: None,
            status: String::new(),
            visible: songs.clone(),
            songs,
            main: MainState {
                sort: 2,
                ..Default::default()
            },
            view_options: VIEW_OPTIONS.map(String::from).to_vec(),
            sort_options: SORT_OPTIONS.map(String::from).to_vec(),
            playing: None,
            player: PlayerView {
                playing: false,
                position_secs: 0.0,
                length_secs: 0.0,
                volume: 0.7,
                shuffle: false,
                repeat: Repeat::Off,
            },
            smoother: SpectrumSmoother::default(),
            clock: 0.0,
            images: load_images(),
            backdrop: Backdrop::default(),
            backdrop_image: None,
            backdrop_blur: 0.0,
        }
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        if self.applied.as_ref() != Some(&self.theme) {
            self.theme.apply(ctx);
            self.applied = Some(self.theme.clone());
        }
        // The blur is baked into the texture, so a changed blur needs the image set again.
        if self.backdrop_blur != self.theme.backdrop.blur {
            self.backdrop_blur = self.theme.backdrop.blur;
            if let Some(index) = self.backdrop_image {
                self.set_backdrop(ctx, index);
            }
        }
    }

    fn set_backdrop(&mut self, ctx: &egui::Context, index: usize) {
        if let Some(image) = self.images.get(index) {
            self.backdrop.set_image(
                ctx,
                &image.rgba,
                image.width,
                image.height,
                self.theme.backdrop.blur,
            );
            self.backdrop_image = Some(index);
        }
    }

    /// Re-filters and re-sorts the list; the real search lives in the library crate.
    fn refresh_list(&mut self) {
        let query = self.main.query.to_lowercase();
        self.visible = self
            .songs
            .iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query) || s.artist.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        match self.main.sort {
            0 => self.visible.sort_by(|a, b| a.title.cmp(&b.title)),
            1 => self.visible.sort_by(|a, b| a.artist.cmp(&b.artist)),
            3 => self.visible.sort_by_key(|s| s.length_secs),
            4 => self.visible.sort_by(|a, b| a.bpm.total_cmp(&b.bpm)),
            5 => self.visible.reverse(),
            _ => {}
        }
        self.playing = None;
    }

    fn play(&mut self, ctx: &egui::Context, index: usize) {
        let Some(song) = self.visible.get(index) else {
            return;
        };
        self.player.length_secs = song.length_secs as f32;
        self.player.position_secs = 0.0;
        self.player.playing = true;
        self.playing = Some(index);
        self.main.selected = Some(index);
        if !self.images.is_empty() {
            self.set_backdrop(ctx, index % self.images.len());
        }
    }

    fn advance_clock(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt);
        self.clock += dt;
        if !self.player.playing {
            return;
        }
        self.player.position_secs += dt;
        if self.player.position_secs >= self.player.length_secs
            && let Some(index) = self.playing
        {
            self.play(ctx, (index + 1) % self.visible.len());
        }
        ctx.request_repaint();
    }

    /// Fake spectrum input: slow waves that fall off toward the high end.
    fn spectrum_levels(&mut self, dt: f32) -> Vec<f32> {
        let bars = self.theme.spectrum.bars;
        let input: Vec<f32> = (0..bars)
            .map(|i| {
                if !self.player.playing {
                    return 0.0;
                }
                let x = i as f32;
                let wave = ((self.clock * 4.0 + x * 0.35).sin() * 0.5 + 0.5)
                    * ((self.clock * 1.3 + x * 0.11).cos() * 0.5 + 0.5);
                wave * (1.0 - x / bars as f32 * 0.5)
            })
            .collect();
        self.smoother
            .update(&input, dt, self.theme.spectrum.decay_per_second)
            .to_vec()
    }

    fn apply_events(&mut self, ctx: &egui::Context, event: MainEvent) {
        if event.list_changed {
            self.refresh_list();
        }
        if let Some(index) = event.list.activated {
            self.play(ctx, index);
        }
        if event.playbar.toggle_mini {
            self.set_mini(ctx, true);
        }
        self.apply_playbar(ctx, event.playbar);
    }

    fn show_page(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, page: Page) {
        let theme = self.theme.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.colors.base))
            .show(ui, |ui| match page {
                Page::Settings => {
                    let event = settings_screen(ui, &theme, &mut self.settings);
                    if event.choose_folder {
                        self.extras.toasts.push(
                            NoticeKind::Info,
                            "The folder dialog is not part of the gallery.",
                            theme.motion.toast_seconds,
                        );
                    }
                    if event.back {
                        self.page = None;
                    }
                }
                Page::FirstRun => {
                    let searched = [SAMPLE_FOLDER.to_owned()];
                    let event = first_run_screen(ui, &theme, &searched);
                    if event.choose_folder || event.retry {
                        self.page = None;
                    }
                }
                Page::About => {
                    if about_screen(ui, &theme, env!("CARGO_PKG_VERSION")) {
                        self.page = None;
                    }
                }
            });
        self.extras.toasts.show(ctx, &theme);
    }

    fn show_mini(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let levels = self.spectrum_levels(ctx.input(|i| i.stable_dt));
        let song = self.playing.and_then(|i| self.visible.get(i));
        let event = mini_player(
            ui,
            &self.theme,
            &mut self.backdrop,
            song,
            &self.player,
            &levels,
        );
        self.apply_playbar(ctx, event.playbar);
        if event.expand {
            self.set_mini(ctx, false);
        }
    }

    fn set_mini(&mut self, ctx: &egui::Context, mini: bool) {
        self.mini = mini;
        let shape = &self.theme.shape;
        let (size, min) = if mini {
            (shape.mini_size, shape.mini_size)
        } else {
            (FULL_SIZE, shape.min_size)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
    }

    fn apply_playbar(&mut self, ctx: &egui::Context, event: PlaybarEvent) {
        let player = &mut self.player;
        if event.toggle_play {
            player.playing = !player.playing;
        }
        if event.toggle_shuffle {
            player.shuffle = !player.shuffle;
        }
        if event.cycle_repeat {
            player.repeat = player.repeat.next();
        }
        if let Some(secs) = event.seek_secs {
            player.position_secs = secs;
        }
        if let Some(volume) = event.volume {
            player.volume = volume;
        }
        if event.next
            && let Some(index) = self.playing
        {
            self.play(ctx, (index + 1) % self.visible.len());
        }
        if event.prev
            && let Some(index) = self.playing
        {
            self.play(ctx, index.saturating_sub(1));
        }
    }

    fn preview(&self, ui: &mut egui::Ui) {
        ui.label(self.theme.heading("listenosu gallery"));
        ui.label("Regular: The quick brown fox jumps over the lazy dog. 0123456789");
        ui.label("日本語の表示確認：作業用BGMを流し聴きする。ひらがな・カタカナ・漢字。");

        ui.add_space(self.theme.shape.padding);
        ui.label(self.theme.heading("Song card"));
        let state = |selected, playing| CardState { selected, playing };
        let samples = [
            ("normal", state(false, false), false),
            ("selected", state(true, false), false),
            ("playing", state(false, true), false),
            ("selected + playing", state(true, true), false),
            ("missing file", state(false, false), true),
        ];
        for (label, state, missing) in samples {
            ui.small(label);
            let mut song = self.songs[0].clone();
            song.missing = missing;
            song_card(ui, &self.theme, &song, state);
        }
        ui.small("long title");
        song_card(ui, &self.theme, &self.songs[6], CardState::default());
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sync_theme(&ctx);
        self.advance_clock(&ctx);

        if self.mini {
            self.show_mini(&ctx, ui);
            return;
        }
        if let Some(page) = self.page {
            self.show_page(&ctx, ui, page);
            return;
        }
        // The tuning panel would take a third of a small window, so `main` leaves it out.
        if !self.main_only {
            egui::Panel::left("theme_panel")
                .default_size(300.0)
                .show(ui, |ui| {
                    theme_panel::show(ui, &mut self.theme, &mut self.status)
                });
        }
        let dt = ctx.input(|i| i.stable_dt);
        let levels = self.spectrum_levels(dt);
        let theme = self.theme.clone();
        let visible = std::mem::take(&mut self.visible);
        let mut state = std::mem::take(&mut self.main);
        let (view_options, sort_options) = (self.view_options.clone(), self.sort_options.clone());
        let player = self.player.clone();
        let mut backdrop = std::mem::take(&mut self.backdrop);
        let input = MainInput {
            songs: &visible,
            playing: self.playing,
            player: &player,
            spectrum: &levels,
            view_options: &view_options,
            sort_options: &sort_options,
        };
        let main_only = self.main_only;
        let event = main_screen(ui, &theme, &mut backdrop, &mut state, &input, |ui| {
            if main_only {
                return;
            }
            ui.horizontal(|ui| {
                for section in sections::Section::ALL {
                    ui.selectable_value(&mut self.section, section, section.label());
                }
            });
            let size = egui::vec2(
                ui.available_width(),
                ui.available_height() - theme.spectrum.height,
            );
            ui.allocate_ui(size, |ui| match self.section {
                sections::Section::Components => {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| self.preview(ui));
                }
                sections::Section::Queue => self.extras.queue(ui, &theme),
                sections::Section::Playlists => self.extras.playlists(ui, &theme),
                sections::Section::Notices => self.extras.notices(ui, &theme),
            });
        });
        self.backdrop = backdrop;
        self.visible = visible;
        self.main = state;
        self.apply_events(&ctx, event);
        self.extras.toasts.show(&ctx, &self.theme);
    }
}

/// Test images for the backdrop: every PNG in `assets/gallery/`.
fn load_images() -> Vec<Image> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/gallery");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|path| image::open(path).ok())
        .map(|image| {
            let rgba = image.to_rgba8();
            Image {
                width: rgba.width() as usize,
                height: rgba.height() as usize,
                rgba: rgba.into_raw(),
            }
        })
        .collect()
}

const SAMPLE_FOLDER: &str = r"C:\Users\you\AppData\Roaming\osu";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Settings,
    FirstRun,
    About,
}

impl Page {
    fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "settings" => Some(Self::Settings),
            "first-run" => Some(Self::FirstRun),
            "about" => Some(Self::About),
            _ => None,
        }
    }
}

const FULL_SIZE: [f32; 2] = [1400.0, 860.0];

fn starts_mini() -> bool {
    std::env::args().nth(1).as_deref() == Some("mini")
}

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu gallery",
        eframe::NativeOptions {
            viewport: {
                let shape = Theme::lazer().shape;
                let (size, min) = if starts_mini() {
                    (shape.mini_size, shape.mini_size)
                } else {
                    (FULL_SIZE, shape.min_size)
                };
                // e.g. GALLERY_SIZE=820x480 to open at an exact size.
                let size = std::env::var("GALLERY_SIZE")
                    .ok()
                    .and_then(|v| {
                        let (w, h) = v.split_once('x')?;
                        Some([w.parse().ok()?, h.parse().ok()?])
                    })
                    .unwrap_or(size);
                egui::ViewportBuilder::default()
                    .with_inner_size(size)
                    .with_min_inner_size(min)
            },
            ..Default::default()
        },
        Box::new(|cc| {
            let mut gallery = Gallery::new(cc);
            // Start with the long title playing, to check truncation without clicking.
            if gallery.mini || gallery.main_only {
                gallery.play(&cc.egui_ctx, 6);
            }
            // e.g. GALLERY_SCALE=2 to check a high-DPI layout.
            if let Some(scale) = std::env::var("GALLERY_SCALE")
                .ok()
                .and_then(|v| v.parse().ok())
            {
                cc.egui_ctx.set_pixels_per_point(scale);
            }
            Ok(Box::new(gallery))
        }),
    )
}
