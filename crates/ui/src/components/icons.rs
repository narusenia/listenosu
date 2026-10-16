use egui::{Color32, Pos2, Response, Sense, Shape, Stroke, Vec2, pos2, vec2};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Prev,
    Next,
}

/// Round icon button. Glyphs are painted as shapes so they do not depend on any font.
pub fn icon_button(ui: &mut egui::Ui, theme: &Theme, icon: Icon, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered(),
        theme.motion.hover_seconds,
    );
    let c = &theme.colors;
    let painter = ui.painter();
    painter.circle_filled(
        rect.center(),
        size / 2.0,
        Color32::TRANSPARENT.lerp_to_gamma(c.surface_hover, hover),
    );

    let half = size * 0.2;
    let center = rect.center();
    let color = c.text;
    match icon {
        Icon::Play => triangle(painter, center, half, 1.0, color),
        Icon::Pause => {
            for dx in [-half * 0.55, half * 0.55] {
                let bar = egui::Rect::from_center_size(
                    center + vec2(dx, 0.0),
                    vec2(half * 0.7, half * 2.0),
                );
                painter.rect_filled(bar, 1.0, color);
            }
        }
        Icon::Prev | Icon::Next => {
            let direction = if icon == Icon::Next { 1.0 } else { -1.0 };
            triangle(
                painter,
                center + vec2(-direction * half * 0.3, 0.0),
                half,
                direction,
                color,
            );
            let bar = egui::Rect::from_center_size(
                center + vec2(direction * half * 0.9, 0.0),
                vec2(half * 0.4, half * 2.0),
            );
            painter.rect_filled(bar, 1.0, color);
        }
    }
    response
}

/// Isosceles triangle pointing right (`direction` = 1.0) or left (-1.0).
fn triangle(painter: &egui::Painter, center: Pos2, half: f32, direction: f32, color: Color32) {
    let points = vec![
        pos2(center.x + direction * half, center.y),
        pos2(center.x - direction * half * 0.8, center.y - half),
        pos2(center.x - direction * half * 0.8, center.y + half),
    ];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}
