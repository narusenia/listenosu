struct Gallery;

impl eframe::App for Gallery {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("listenosu gallery");
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "listenosu gallery",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(Gallery))),
    )
}
