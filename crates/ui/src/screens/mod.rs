use crate::theme::Theme;

pub mod about;
pub mod first_run;
pub mod main_screen;
pub mod mini_player;
pub mod settings;

/// Page title with a Back button on the right. Returns true when Back was clicked.
fn header(ui: &mut egui::Ui, theme: &Theme, title: &str) -> bool {
    let mut back = false;
    ui.horizontal(|ui| {
        crate::components::controls::fix_row_height(ui);
        ui.label(theme.heading(title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            back = ui.button("Back").clicked();
        });
    });
    ui.add_space(theme.shape.padding);
    back
}
