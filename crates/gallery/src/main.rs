mod mock;
mod theme_panel;

use std::path::Path;

use eframe::egui;
use ui::Theme;
use ui::backdrop::Backdrop;
use ui::components::controls::{dropdown, search_bar};
use ui::components::info_panel::info_panel;
use ui::components::playbar::playbar;
use ui::components::song_card::{CardState, song_card};
use ui::components::song_list::song_list;
use ui::components::spectrum::{SpectrumSmoother, spectrum};
use ui::model::{PlayerView, Repeat, SongView};

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
    theme: Theme,
    applied: Option<Theme>,
    status: String,

    songs: Vec<SongView>,
    visible: Vec<SongView>,
    query: String,
    view: usize,
    sort: usize,
    selected: Option<usize>,
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
            theme: Theme::lazer(),
            applied: None,
            status: String::new(),
            visible: songs.clone(),
            songs,
            query: String::new(),
            view: 0,
            sort: 2,
            selected: None,
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
        let query = self.query.to_lowercase();
        self.visible = self
            .songs
            .iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query) || s.artist.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        match self.sort {
            0 => self.visible.sort_by(|a, b| a.title.cmp(&b.title)),
            1 => self.visible.sort_by(|a, b| a.artist.cmp(&b.artist)),
            3 => self.visible.sort_by_key(|s| s.length_secs),
            4 => self.visible.sort_by(|a, b| a.bpm.total_cmp(&b.bpm)),
            5 => self.visible.reverse(),
            _ => {}
        }
        self.selected = None;
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
        self.selected = Some(index);
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

    fn list_panel(&mut self, ui: &mut egui::Ui) {
        let mut changed =
            search_bar(ui, &self.theme, &mut self.query, self.visible.len()).changed();
        let options = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        changed |= dropdown(ui, "view", "View", &mut self.view, &options(&VIEW_OPTIONS));
        changed |= dropdown(ui, "sort", "Sort", &mut self.sort, &options(&SORT_OPTIONS));
        if changed {
            self.refresh_list();
        }

        let event = song_list(ui, &self.theme, &self.visible, self.selected, self.playing);
        if event.clicked.is_some() {
            self.selected = event.clicked;
        }
        if let Some(index) = event.activated {
            self.play(ui.ctx(), index);
        }
    }

    fn preview(&self, ui: &mut egui::Ui) {
        ui.heading("listenosu gallery");
        ui.label("Regular: The quick brown fox jumps over the lazy dog. 0123456789");
        ui.label("日本語の表示確認：作業用BGMを流し聴きする。ひらがな・カタカナ・漢字。");

        ui.add_space(self.theme.shape.padding);
        ui.heading("Info panel");
        if let Some(song) = self.playing.and_then(|i| self.visible.get(i)) {
            info_panel(ui, &self.theme, song);
        } else {
            ui.label("Double-click a song in the list to play it.");
        }

        ui.add_space(self.theme.shape.padding);
        ui.heading("Song card");
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

        egui::Panel::left("theme_panel")
            .default_size(300.0)
            .show(ui, |ui| {
                theme_panel::show(ui, &mut self.theme, &mut self.status)
            });
        egui::Panel::bottom("playbar")
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let event = playbar(ui, &self.theme, &self.player);
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
                    self.play(&ctx, (index + 1) % self.visible.len());
                }
                if event.prev
                    && let Some(index) = self.playing
                {
                    self.play(&ctx, index.saturating_sub(1));
                }
            });
        egui::Panel::right("song_list")
            .default_size(self.theme.shape.list_width)
            .show(ui, |ui| self.list_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.backdrop.paint(ui, ui.max_rect(), &self.theme);

                let levels = self.spectrum_levels(ctx.input(|i| i.stable_dt));
                let spectrum_height = self.theme.spectrum.height;
                let content_height = (ui.available_height() - spectrum_height).max(0.0);
                egui::Frame::NONE
                    .inner_margin(self.theme.shape.padding)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(content_height)
                            .auto_shrink(false)
                            .show(ui, |ui| self.preview(ui));
                    });
                spectrum(ui, &self.theme, &levels, spectrum_height);
            });
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

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu gallery",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 860.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Gallery::new(cc)))),
    )
}
