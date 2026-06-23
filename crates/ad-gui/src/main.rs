use std::time::Instant;

use ad_core::{Decimal, GameState};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

/// Colors matching the original game's dark theme.
mod colors {
    use eframe::egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(30, 30, 30);
    pub const PANEL_BG: Color32 = Color32::from_rgb(40, 40, 40);
    pub const TEXT: Color32 = Color32::from_rgb(180, 180, 180);
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(220, 220, 220);
    pub const ANTIMATTER: Color32 = Color32::from_rgb(33, 150, 243);
    pub const GREEN_ACCENT: Color32 = Color32::from_rgb(90, 196, 103);
    pub const GREEN_DARK: Color32 = Color32::from_rgb(18, 122, 32);
    pub const BTN_BG: Color32 = Color32::from_rgb(50, 50, 50);
    pub const BTN_BORDER: Color32 = Color32::from_rgb(18, 122, 32);
    pub const DIM_TEXT: Color32 = Color32::from_rgb(130, 130, 130);
    pub const SEPARATOR: Color32 = Color32::from_rgb(70, 70, 70);
}

/// Dimension tier display names matching the original game.
const DIM_NAMES: [&str; 8] = ["1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"];

struct App {
    game: GameState,
    last_tick: Instant,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        Self::configure_style(ctx);
        Self {
            game: GameState::new(),
            last_tick: Instant::now(),
        }
    }

    fn configure_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 4.0);
        style.spacing.button_padding = Vec2::new(12.0, 4.0);
        ctx.set_style(style);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = colors::BG;
        visuals.window_fill = colors::BG;
        visuals.override_text_color = Some(colors::TEXT);
        visuals.widgets.noninteractive.bg_fill = colors::PANEL_BG;
        visuals.widgets.inactive.bg_fill = colors::BTN_BG;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::BTN_BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_BRIGHT);
        visuals.widgets.hovered.bg_fill = colors::GREEN_DARK;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors::GREEN_ACCENT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.bg_fill = colors::GREEN_ACCENT;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors::GREEN_ACCENT);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ctx.set_visuals(visuals);
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            // "You have X antimatter."
            ui.horizontal_wrapped(|ui| {
                ui.add_space(
                    (ui.available_width()
                        - estimate_header_width(&self.game.antimatter))
                        / 2.0,
                );
                ui.label(
                    RichText::new("You have ")
                        .size(16.0)
                        .color(colors::TEXT_BRIGHT),
                );
                ui.label(
                    RichText::new(format_decimal(&self.game.antimatter))
                        .size(20.0)
                        .color(colors::ANTIMATTER)
                        .strong(),
                );
                ui.label(
                    RichText::new(" antimatter.")
                        .size(16.0)
                        .color(colors::TEXT_BRIGHT),
                );
            });

            ui.add_space(2.0);

            // "You are getting X antimatter per second."
            let per_sec = self.game.antimatter_per_second();
            ui.label(
                RichText::new(format!(
                    "You are getting {} antimatter per second.",
                    format_decimal(&per_sec)
                ))
                .size(13.0)
                .color(colors::TEXT),
            );

            ui.add_space(2.0);

            // Tickspeed info
            let tickspeed_mult = self.game.tickspeed_purchase_multiplier();
            let effect_per_upgrade = 1.0 / tickspeed_mult;
            let tickspeed_per_sec = self.game.tickspeed_effect();
            ui.label(
                RichText::new(format!(
                    "ADs produce {:.3}x faster per Tickspeed \
                     upgrade",
                    effect_per_upgrade
                ))
                .size(12.0)
                .color(colors::DIM_TEXT),
            );
            ui.label(
                RichText::new(format!(
                    "Total Tickspeed: {} / sec",
                    format_decimal(&tickspeed_per_sec)
                ))
                .size(12.0)
                .color(colors::DIM_TEXT),
            );
        });

        ui.add_space(4.0);
        colored_separator(ui);
    }

    fn render_sacrifice_and_max_all(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            // Multiplier info line
            let buy10_mult = 2.0_f64.powi(1); // Always 2x
            let sacrifice_text = if self.game.sacrifice_unlocked {
                format!(
                    " | Dimensional Sacrifice multiplier: {}",
                    format_multiplier(&self.game.sacrifice_multiplier())
                )
            } else {
                String::new()
            };
            ui.label(
                RichText::new(format!(
                    "Buy 10 Dimension purchase multiplier: \
                     {:.2}x{}",
                    buy10_mult, sacrifice_text
                ))
                .size(12.0)
                .color(colors::TEXT),
            );

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                // Center the buttons
                let total_width = if self.game.sacrifice_unlocked {
                    500.0
                } else {
                    120.0
                };
                ui.add_space((ui.available_width() - total_width) / 2.0);

                // Sacrifice button
                if self.game.sacrifice_unlocked {
                    let can_sacrifice = self.game.can_sacrifice();
                    let sacrifice_boost = self.game.sacrifice_multiplier_if_sacrificed();
                    let btn_text = if can_sacrifice {
                        format!(
                            "Dimensional Sacrifice ({})",
                            format_multiplier(&sacrifice_boost)
                        )
                    } else {
                        "Dimensional Sacrifice Disabled \
                         (no dimensions)"
                            .to_string()
                    };

                    let btn = egui::Button::new(
                        RichText::new(btn_text).size(12.0).color(if can_sacrifice {
                            colors::TEXT_BRIGHT
                        } else {
                            colors::DIM_TEXT
                        }),
                    )
                    .min_size(Vec2::new(340.0, 28.0));

                    let response = ui.add_enabled(can_sacrifice, btn);
                    if response.clicked() {
                        self.game.sacrifice();
                    }

                    ui.add_space(8.0);
                }

                // Max All button
                let max_btn = egui::Button::new(
                    RichText::new("Max all (M)")
                        .size(12.0)
                        .color(colors::TEXT_BRIGHT),
                )
                .min_size(Vec2::new(100.0, 28.0));
                if ui.add(max_btn).clicked() {
                    self.game.max_all();
                }
            });
        });

        ui.add_space(4.0);
    }

    fn render_tickspeed_row(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let cost = &self.game.tickspeed.cost;
                let affordable = self.game.antimatter >= *cost;

                // Center
                ui.add_space((ui.available_width() - 420.0) / 2.0);

                let btn_text = format!("Tickspeed Cost: {}", format_decimal(cost));
                let btn = egui::Button::new(RichText::new(btn_text).size(12.0).color(
                    if affordable {
                        colors::TEXT_BRIGHT
                    } else {
                        colors::DIM_TEXT
                    },
                ))
                .min_size(Vec2::new(300.0, 28.0));

                if ui.add_enabled(affordable, btn).clicked() {
                    self.game.buy_tickspeed();
                }

                ui.add_space(4.0);

                let max_btn = egui::Button::new(
                    RichText::new("Buy Max")
                        .size(12.0)
                        .color(colors::TEXT_BRIGHT),
                )
                .min_size(Vec2::new(100.0, 28.0));
                if ui.add(max_btn).clicked() {
                    self.game.buy_max_tickspeed();
                }
            });
        });

        ui.add_space(6.0);
    }

    #[allow(clippy::needless_range_loop)]
    fn render_dimensions(&mut self, ui: &mut egui::Ui) {
        let unlocked = self.game.unlocked_dimensions();

        for tier in 0..8 {
            let is_unlocked = tier < unlocked;

            if !is_unlocked {
                // Show locked row dimmed
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!(
                            "{} Antimatter Dimension",
                            DIM_NAMES[tier]
                        ))
                        .size(13.0)
                        .color(colors::DIM_TEXT.gamma_multiply(0.5)),
                    );
                });
                ui.add_space(2.0);
                continue;
            }

            let amount = self.game.dimensions[tier].amount;
            let bought = self.game.dimensions[tier].bought;
            let bought_before_10 = bought % 10;
            let mult = self.game.dimension_multiplier(tier);
            let production = self.game.dimension_production_per_second(tier);
            let cost_single = self.game.dimension_cost(tier);
            let cost_until_10 = self.game.dimension_cost_until_10(tier);
            let can_buy_single = self.game.antimatter >= cost_single;
            let can_buy_10 = self.game.antimatter >= cost_until_10;

            // Rate of change (only for tiers < 8)
            let rate_text = if tier < 7 && amount > Decimal::ZERO {
                let rate_pct = if amount > Decimal::ZERO {
                    (production / amount).to_f64() * 100.0
                } else {
                    0.0
                };
                if rate_pct > 0.01 {
                    format!(" (+{:.2}%/s)", rate_pct)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            ui.horizontal(|ui| {
                ui.add_space(12.0);

                // Dimension name + multiplier
                let name_text = format!(
                    "{} Antimatter Dimension  {}",
                    DIM_NAMES[tier],
                    format_multiplier(&mult)
                );
                ui.label(
                    RichText::new(name_text)
                        .size(13.0)
                        .color(colors::TEXT_BRIGHT),
                );

                // Amount (bought_before_10)
                let amount_text =
                    format!("{} ({})", format_decimal(&amount), bought_before_10);
                ui.label(RichText::new(amount_text).size(13.0).color(colors::TEXT));

                // Rate of change
                if !rate_text.is_empty() {
                    ui.label(
                        RichText::new(&rate_text).size(11.0).color(colors::DIM_TEXT),
                    );
                }

                // Push buttons to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Buy 10 button (rightmost)
                    let btn10_text =
                        format!("Until 10, Cost: {}", format_decimal(&cost_until_10));
                    let btn10 = egui::Button::new(
                        RichText::new(btn10_text).size(11.0).color(if can_buy_10 {
                            colors::TEXT_BRIGHT
                        } else {
                            colors::DIM_TEXT
                        }),
                    )
                    .min_size(Vec2::new(160.0, 26.0));
                    if ui.add_enabled(can_buy_10, btn10).clicked() {
                        self.game.buy_until_10_dimension(tier);
                    }

                    // Buy 1 button
                    let btn1_text = format!("Cost: {}", format_decimal(&cost_single));
                    let btn1 =
                        egui::Button::new(RichText::new(btn1_text).size(11.0).color(
                            if can_buy_single {
                                colors::TEXT_BRIGHT
                            } else {
                                colors::DIM_TEXT
                            },
                        ))
                        .min_size(Vec2::new(120.0, 26.0));
                    if ui.add_enabled(can_buy_single, btn1).clicked() {
                        self.game.buy_dimension(tier);
                    }
                });
            });
            ui.add_space(2.0);
        }
    }

    fn render_prestige_rows(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        colored_separator(ui);
        ui.add_space(4.0);

        // Dimension Boost row
        let (req_tier, req_amount) = self.game.dim_boost_requirement();
        let can_boost = self.game.can_dim_boost();

        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!(
                    "Dimension Boost ({}):  requires {} {} \
                     Antimatter Dimensions",
                    self.game.dim_boosts, req_amount, DIM_NAMES[req_tier]
                ))
                .size(13.0)
                .color(colors::TEXT),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let unlock_text = if self.game.dim_boosts < 4 {
                    format!(
                        "Reset your Dimensions to unlock \
                             {} Antimatter Dimension",
                        DIM_NAMES[4 + self.game.dim_boosts as usize]
                    )
                } else {
                    "Reset your Dimensions to boost".to_string()
                };
                let btn = egui::Button::new(
                    RichText::new(unlock_text).size(11.0).color(if can_boost {
                        colors::TEXT_BRIGHT
                    } else {
                        colors::DIM_TEXT
                    }),
                )
                .min_size(Vec2::new(220.0, 36.0));
                if ui.add_enabled(can_boost, btn).clicked() {
                    self.game.buy_dim_boost();
                }
            });
        });

        ui.add_space(6.0);

        // Antimatter Galaxy row
        let req = self.game.galaxy_requirement();
        let can_galaxy = self.game.can_buy_galaxy();

        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!(
                    "Antimatter Galaxies ({}):  requires {} 8th \
                     Antimatter Dimensions",
                    self.game.galaxies, req
                ))
                .size(13.0)
                .color(colors::TEXT),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn_text = "Reset your Dimensions and Boosts to \
                         increase Tickspeed power";
                let btn = egui::Button::new(RichText::new(btn_text).size(11.0).color(
                    if can_galaxy {
                        colors::TEXT_BRIGHT
                    } else {
                        colors::DIM_TEXT
                    },
                ))
                .min_size(Vec2::new(220.0, 36.0));
                if ui.add_enabled(can_galaxy, btn).clicked() {
                    self.game.buy_galaxy();
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Game tick
        let now = Instant::now();
        let dt_ms = now.duration_since(self.last_tick).as_secs_f64() * 1000.0;
        self.game.tick(dt_ms);
        self.last_tick = now;

        // Handle keyboard shortcut: M for max all
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            self.game.max_all();
        }

        // UI
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_header(ui);
                self.render_sacrifice_and_max_all(ui);
                self.render_tickspeed_row(ui);
                self.render_dimensions(ui);
                self.render_prestige_rows(ui);
            });
        });

        // Request continuous repaint (~30fps)
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// Draw a colored separator line.
fn colored_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.top();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 20.0, y),
            egui::pos2(rect.right() - 20.0, y),
        ],
        Stroke::new(1.0, colors::SEPARATOR),
    );
    ui.add_space(2.0);
}

/// Estimate the header width for centering purposes.
fn estimate_header_width(antimatter: &Decimal) -> f32 {
    let text = format_decimal(antimatter);
    // Rough estimate: 8px per char at size 16-20
    (text.len() as f32 + 22.0) * 8.0
}

/// Format a Decimal for display using the game's notation.
fn format_decimal(val: &Decimal) -> String {
    let f = val.to_f64();
    if f == 0.0 {
        return "0".to_string();
    }
    if f < 1000.0 {
        format!("{:.2}", f)
    } else if f < 1e9 {
        // Use comma-separated notation
        format_with_commas(f)
    } else {
        // Scientific notation like the original game
        let exp = f.log10().floor() as i64;
        let mantissa = f / 10_f64.powi(exp as i32);
        if mantissa >= 9.995 {
            // Would round to 10.00, bump exponent
            format!("1.00e{}", exp + 1)
        } else {
            format!("{:.2}e{}", mantissa, exp)
        }
    }
}

/// Format a number with comma separators (e.g., 1,234,567).
fn format_with_commas(f: f64) -> String {
    let int_part = f.floor() as u64;
    let s = int_part.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Format a Decimal as a multiplier (e.g., "×1.23e4").
fn format_multiplier(val: &Decimal) -> String {
    format!("×{}", format_decimal(val))
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 650.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Antimatter Dimensions",
        options,
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))),
    )
}
