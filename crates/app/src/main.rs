mod app;
mod audio;
mod keys;
mod keyview;
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

struct Shot {
    path: String,
    frames: u32,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

struct Flip {
    app: app::App,
    shot: Option<Shot>,
}

impl Flip {
    fn capture(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        shot.frames += 1;
        if shot.frames == 20 {
            let path = shot.path.clone();
            let done = shot.done.clone();
            let ctx2 = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    let [w, h] = image.size;
                    let mut out = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                    out.extend_from_slice(&(w as u16).to_le_bytes());
                    out.extend_from_slice(&(h as u16).to_le_bytes());
                    out.extend_from_slice(&[32, 0x28]);
                    for p in &image.pixels {
                        let [r, g, b, a] = p.to_srgba_unmultiplied();
                        out.extend_from_slice(&[b, g, r, a]);
                    }
                    let _ = std::fs::write(&path, out);
                    done.store(true, std::sync::atomic::Ordering::Relaxed);
                    ctx2.request_repaint();
                }),
            ));
        }
        if shot.done.load(std::sync::atomic::Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(FRAME);
    }
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
        self.capture(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn on_exit(&mut self) {
        if self.shot.is_some() {
            return;
        }
        self.app.settings.save();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("flip")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([550.0, 350.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_has_shadow(false),
        ..Default::default()
    };
    eframe::run_native(
        "flip",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let shot = std::env::var("FLIP_SHOT").ok().map(|path| Shot {
                path,
                frames: 0,
                done: Default::default(),
            });
            let mut app = app::App::new(cc.egui_ctx.clone());
            if let Ok(name) = std::env::var("FLIP_MOOD")
                && let Some(i) = flip_engine::mood::find(&name.to_uppercase())
            {
                app.load_mood(i);
                app.mood_beat();
            }
            Ok(Box::new(Flip { app, shot }))
        }),
    )
}
