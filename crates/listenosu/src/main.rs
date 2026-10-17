mod app;
mod backdrop_loader;
mod catalog;
mod config;
mod listing;
mod side;

use eframe::egui;
use library::store::AppDirs;
use ui::Theme;

use app::{App, FULL_SIZE, window_size};
use config::Config;

fn main() -> eframe::Result {
    let dirs = AppDirs::from_env();
    let config = dirs.as_ref().map(Config::load).unwrap_or_default();
    ui::i18n::set_language(ui::i18n::Lang::from_setting(config.language));
    eframe::run_native(
        "listenosu!",
        eframe::NativeOptions {
            viewport: viewport(&config),
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(App::new(cc, dirs, config)))),
    )
}

/// A saved size from an older layout may be below today's minimum, which clips the content.
pub fn at_least(size: [f32; 2], min: [f32; 2]) -> [f32; 2] {
    [size[0].max(min[0]), size[1].max(min[1])]
}

/// Opens where the window was last closed, in the mode it was in.
fn viewport(config: &Config) -> egui::ViewportBuilder {
    let shape = Theme::lazer().shape;
    let (default_size, min, saved) = if config.mini {
        let size = window_size(&shape, shape.mini_size);
        (size, size, config.mini_window)
    } else {
        (
            window_size(&shape, FULL_SIZE),
            window_size(&shape, shape.min_size),
            config.full_window,
        )
    };
    // The app draws its own title bar.
    let icon = ui::icon::large();
    let mut viewport = egui::ViewportBuilder::default()
        .with_icon(egui::IconData {
            rgba: icon.rgba,
            width: icon.width,
            height: icon.height,
        })
        .with_decorations(false)
        .with_inner_size(saved.map_or(default_size, |g| at_least(g.size, min)))
        .with_min_inner_size(min);
    // A position from a monitor that is gone would open the window out of reach; the limit is a rough guard.
    if let Some(g) = saved.filter(|g| g.position.iter().all(|v| v.abs() < 20_000.0)) {
        viewport = viewport.with_position(g.position);
    }
    if config.mini && config.mini_on_top {
        viewport = viewport.with_always_on_top();
    }
    viewport
}
