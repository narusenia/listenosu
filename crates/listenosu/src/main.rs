mod app;
mod backdrop_loader;
mod catalog;
mod config;
mod listing;
mod side;

use eframe::egui;
use library::store::AppDirs;
use ui::Theme;

use app::{App, FULL_SIZE};
use config::Config;

fn main() -> eframe::Result {
    let dirs = AppDirs::from_env();
    let config = dirs.as_ref().map(Config::load).unwrap_or_default();
    eframe::run_native(
        "listenosu",
        eframe::NativeOptions {
            viewport: viewport(&config),
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(App::new(cc, dirs, config)))),
    )
}

/// Opens where the window was last closed, in the mode it was in.
fn viewport(config: &Config) -> egui::ViewportBuilder {
    let shape = Theme::lazer().shape;
    let (default_size, min, saved) = if config.mini {
        (shape.mini_size, shape.mini_size, config.mini_window)
    } else {
        (FULL_SIZE, shape.min_size, config.full_window)
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(saved.map_or(default_size, |g| g.size))
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
