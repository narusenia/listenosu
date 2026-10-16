use eframe::egui;

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("listenosu");
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App))),
    )
}
