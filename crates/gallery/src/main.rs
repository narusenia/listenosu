use eframe::egui;

struct Gallery;

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("listenosu gallery");
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu gallery",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(Gallery))),
    )
}
