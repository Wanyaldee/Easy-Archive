fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Easy Archive",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    status: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.centered_and_justified(|ui| {
            let text = if self.status.is_empty() {
                "ここにファイル/フォルダをドラッグ&ドロップしてください"
            } else {
                &self.status
            };
            ui.label(text);
        });
    }
}
