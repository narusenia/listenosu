use egui::{
    Align, Color32, Layout, Margin, Panel, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2,
};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitlebarEvent {
    pub minimize: bool,
    pub toggle_maximize: bool,
    pub close: bool,
    /// The page icons: only shown when `show_pages` is set.
    pub settings: bool,
    pub about: bool,
}

/// Title bar for a window without system decorations: dragging the bar moves the window,
/// double-clicking it maximizes, and the three buttons sit on the right.
pub fn titlebar(
    ui: &mut egui::Ui,
    theme: &Theme,
    title: &str,
    can_maximize: bool,
    show_pages: bool,
) -> TitlebarEvent {
    let mut event = TitlebarEvent::default();
    let height = theme.shape.titlebar_height;
    let c = &theme.colors;
    Panel::top("titlebar")
        .frame(egui::Frame::NONE.fill(c.chrome).inner_margin(Margin::ZERO))
        .exact_size(height)
        .show_separator_line(false)
        .show(ui, |ui| {
            let bar = ui.max_rect();
            let drag = ui.interact(bar, ui.make_persistent_id("drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if can_maximize && drag.double_clicked() {
                event.toggle_maximize = true;
            }
            ui.painter().text(
                pos2(bar.left() + theme.shape.padding, bar.center().y),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(theme.text.small),
                c.text_dim,
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(bar)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let button = vec2(height * 1.4, height);
                    event.close = glyph_button(ui, theme, button, Glyph::Close, c.error);
                    if can_maximize {
                        event.toggle_maximize |=
                            glyph_button(ui, theme, button, Glyph::Maximize, c.surface_hover);
                    }
                    event.minimize =
                        glyph_button(ui, theme, button, Glyph::Minimize, c.surface_hover);
                    if show_pages {
                        event.settings =
                            glyph_button(ui, theme, button, Glyph::Settings, c.surface_hover);
                        event.about =
                            glyph_button(ui, theme, button, Glyph::About, c.surface_hover);
                    }
                },
            );
        });
    event
}

#[derive(Clone, Copy)]
enum Glyph {
    Minimize,
    Maximize,
    Close,
    Settings,
    About,
}

fn glyph_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    size: Vec2,
    glyph: Glyph,
    hover: Color32,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let amount = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered(),
        theme.motion.hover_seconds,
    );
    ui.painter()
        .rect_filled(rect, 0.0, Color32::TRANSPARENT.lerp_to_gamma(hover, amount));
    let half = size.y * 0.16;
    let stroke = Stroke::new(1.2, theme.colors.text);
    let c = rect.center();
    match glyph {
        Glyph::Minimize => {
            ui.painter()
                .line_segment([c - vec2(half, 0.0), c + vec2(half, 0.0)], stroke);
        }
        Glyph::Maximize => {
            ui.painter().rect_stroke(
                Rect::from_center_size(c, Vec2::splat(half * 2.0)),
                0.0,
                stroke,
                StrokeKind::Middle,
            );
        }
        Glyph::Settings => {
            // A gear: a ring with eight teeth.
            let painter = ui.painter();
            painter.circle_stroke(c, half * 0.7, stroke);
            for tooth in 0..8 {
                let angle = tooth as f32 * std::f32::consts::FRAC_PI_4;
                let dir = vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * half, c + dir * half * 1.35], stroke);
            }
        }
        Glyph::About => {
            let painter = ui.painter();
            painter.circle_stroke(c, half * 1.25, stroke);
            painter.circle_filled(c - vec2(0.0, half * 0.55), 1.0, theme.colors.text);
            painter.line_segment(
                [c - vec2(0.0, half * 0.15), c + vec2(0.0, half * 0.6)],
                stroke,
            );
        }
        Glyph::Close => {
            ui.painter()
                .line_segment([c - Vec2::splat(half), c + Vec2::splat(half)], stroke);
            ui.painter()
                .line_segment([c + vec2(-half, half), c + vec2(half, -half)], stroke);
        }
    }
    response.clicked()
}
