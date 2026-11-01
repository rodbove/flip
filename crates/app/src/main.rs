mod app;
mod audio;
mod keys;
mod midi;
mod params;
mod presets;
mod record;
mod roll;
mod settings;
mod theme;
mod view;

use eframe::egui;

const FRAME: std::time::Duration = std::time::Duration::from_millis(16);
const IDLE: std::time::Duration = std::time::Duration::from_millis(100);

struct Flip {
    app: app::App,
}

impl eframe::App for Flip {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.app.tick(now);
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.app.load_files(dropped);
        }
        keys::handle(&mut self.app, &ctx);
        view::draw(&mut self.app, ui);
        view::setup(&mut self.app, &ctx);
        view::naming(&mut self.app, &ctx);
        ctx.request_repaint_after(if self.app.busy() { FRAME } else { IDLE });
    }

    fn on_exit(&mut self) {
        self.app.settings.save();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("flip")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([660.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "flip",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Flip {
                app: app::App::new(cc.egui_ctx.clone()),
            }))
        }),
    )
}
