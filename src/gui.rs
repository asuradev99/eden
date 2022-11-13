use egui;

pub struct Gui {

}

impl Default for Gui {
    fn default() -> Self {
        Self {
           
        }
    }
}


impl Gui {
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                ui.label("Your name: s ");
            });
        });
    }
}