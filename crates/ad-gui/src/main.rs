use std::time::Instant;

use ad_core::GameState;
use eframe::egui;

struct App {
    game: GameState,
    last_tick: Instant,
}

impl App {
    fn new() -> Self {
        Self {
            game: GameState::new(),
            last_tick: Instant::now(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Game tick
        let now = Instant::now();
        let dt_ms = now.duration_since(self.last_tick).as_secs_f64() * 1000.0;
        self.game.tick(dt_ms);
        self.last_tick = now;

        // UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Antimatter Dimensions (Rust)");
            ui.separator();

            ui.label(format!("Antimatter: {}", self.game.antimatter));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(format!(
                    "AD1: {} (bought: {})",
                    self.game.ad1.amount, self.game.ad1.bought
                ));
                if ui
                    .button(format!("Buy (cost: {})", self.game.ad1.cost))
                    .clicked()
                {
                    self.game.buy_ad1();
                }
            });

            ui.add_space(10.0);
            ui.label(format!(
                "Production: {}/s",
                self.game.ad1.amount
            ));
        });

        // Request continuous repaint (~30fps)
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Antimatter Dimensions",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
