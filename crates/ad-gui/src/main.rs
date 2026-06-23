use std::time::Instant;

use ad_core::{Decimal, GameState};
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

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.heading("Antimatter Dimensions (Rust)");
        ui.label(format!("Antimatter: {:.2}", self.game.antimatter));
        ui.label(format!(
            "Tickspeed: {:.2} ms (effect: {:.3}x)",
            self.game.current_tickspeed_ms(),
            self.game.tickspeed_effect()
        ));
        ui.separator();
    }

    fn render_dimensions(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Antimatter Dimensions")
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);

        let unlocked = self.game.unlocked_dimensions();
        for tier in 0..unlocked {
            ui.horizontal(|ui| {
                let amount = self.game.dimensions[tier].amount;
                let mult = self.game.dimension_multiplier(tier);
                let production = self.game.dimension_production_per_second(tier);

                ui.label(format!(
                    "AD{}: {:.2} [{:.2}x] ({:.2}/s)",
                    tier + 1,
                    amount,
                    mult,
                    production
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!(
                            "Buy ({})",
                            format_cost(self.game.dimension_cost(tier))
                        ))
                        .clicked()
                    {
                        self.game.buy_dimension(tier);
                    }
                });
            });
        }

        // Show locked dimensions greyed out
        for tier in unlocked..8 {
            ui.label(format!("AD{}: 🔒 Locked", tier + 1));
        }
    }

    fn render_tickspeed(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!(
                "Tickspeed: {:.2} ms",
                self.game.current_tickspeed_ms()
            ));
            if ui
                .button(format!(
                    "Buy Tickspeed ({})",
                    format_cost(self.game.tickspeed.cost)
                ))
                .clicked()
            {
                self.game.buy_tickspeed();
            }
            if ui.button("Buy Max").clicked() {
                self.game.buy_max_tickspeed();
            }
        });
    }

    fn render_prestige_buttons(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("Prestige Actions").strong().size(14.0));
        ui.add_space(4.0);

        // Dimension Boost
        ui.horizontal(|ui| {
            let (req_tier, req_amount) = self.game.dim_boost_requirement();
            let can_boost = self.game.can_dim_boost();
            let label = format!(
                "Dimension Boost ({} boosts) — requires {} of AD{}",
                self.game.dim_boosts,
                req_amount,
                req_tier + 1
            );
            ui.label(&label);
            ui.add_enabled_ui(can_boost, |ui| {
                if ui.button("Boost!").clicked() {
                    self.game.buy_dim_boost();
                }
            });
        });

        // Antimatter Galaxy
        ui.horizontal(|ui| {
            let req = self.game.galaxy_requirement();
            let can_galaxy = self.game.can_buy_galaxy();
            let label = format!(
                "Antimatter Galaxy ({} galaxies) — requires {} bought AD8",
                self.game.galaxies, req
            );
            ui.label(&label);
            ui.add_enabled_ui(can_galaxy, |ui| {
                if ui.button("Galaxy!").clicked() {
                    self.game.buy_galaxy();
                }
            });
        });

        // Sacrifice
        if self.game.sacrifice_unlocked {
            ui.horizontal(|ui| {
                let mult = self.game.sacrifice_multiplier();
                let next_mult = self.game.sacrifice_multiplier_if_sacrificed();
                let can_sacrifice = self.game.can_sacrifice();
                ui.label(format!(
                    "Sacrifice (current: {:.2}x → next: {:.2}x)",
                    mult, next_mult
                ));
                ui.add_enabled_ui(can_sacrifice, |ui| {
                    if ui.button("Sacrifice!").clicked() {
                        self.game.sacrifice();
                    }
                });
            });
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
            self.render_header(ui);
            self.render_dimensions(ui);
            self.render_tickspeed(ui);
            self.render_prestige_buttons(ui);
        });

        // Request continuous repaint (~30fps)
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// Format a Decimal cost for display (compact notation).
fn format_cost(cost: Decimal) -> String {
    let f = cost.to_f64();
    if f < 1e6 {
        format!("{:.0}", f)
    } else {
        format!("{:.2e}", f)
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Antimatter Dimensions",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
