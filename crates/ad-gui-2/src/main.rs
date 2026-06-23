use std::sync::Mutex;

use ad_core::{Decimal, GameState};
use serde::Serialize;
use tauri::State;

/// Serializable view of a single dimension tier for the frontend.
#[derive(Serialize)]
struct DimensionView {
    amount: String,
    bought: u64,
    bought_mod_10: u64,
    multiplier: String,
    production_per_sec: String,
    cost: String,
    cost_until_10: String,
    can_buy: bool,
    can_buy_10: bool,
    rate_percent: f64,
}

/// Serializable game view sent to the frontend each frame.
#[derive(Serialize)]
struct GameView {
    antimatter: String,
    antimatter_per_sec: String,
    tickspeed_cost: String,
    tickspeed_bought: u64,
    tickspeed_effect: String,
    tickspeed_purchase_multiplier: f64,
    dimensions: Vec<DimensionView>,
    unlocked_dimensions: usize,
    dim_boosts: u32,
    dim_boost_req_tier: usize,
    dim_boost_req_amount: u64,
    can_dim_boost: bool,
    galaxies: u32,
    galaxy_requirement: u64,
    can_buy_galaxy: bool,
    sacrifice_unlocked: bool,
    can_sacrifice: bool,
    sacrifice_multiplier: String,
    sacrifice_multiplier_if_sacrificed: String,
    can_buy_tickspeed: bool,
}

fn build_game_view(game: &GameState) -> GameView {
    let unlocked = game.unlocked_dimensions();
    let mut dimensions = Vec::with_capacity(8);

    for tier in 0..8 {
        let amount = &game.dimensions[tier].amount;
        let bought = game.dimensions[tier].bought;
        let mult = game.dimension_multiplier(tier);
        let production = game.dimension_production_per_second(tier);
        let cost = game.dimension_cost(tier);
        let cost_until_10 = game.dimension_cost_until_10(tier);

        let rate_percent = if tier < 7 && *amount > Decimal::ZERO {
            (production / *amount).to_f64() * 100.0
        } else {
            0.0
        };

        dimensions.push(DimensionView {
            amount: format_decimal(amount),
            bought,
            bought_mod_10: bought % 10,
            multiplier: format_decimal(&mult),
            production_per_sec: format_decimal(&production),
            cost: format_decimal(&cost),
            cost_until_10: format_decimal(&cost_until_10),
            can_buy: game.antimatter >= cost,
            can_buy_10: game.antimatter >= cost_until_10,
            rate_percent,
        });
    }

    let (req_tier, req_amount) = game.dim_boost_requirement();
    let tickspeed_cost = &game.tickspeed.cost;

    GameView {
        antimatter: format_decimal(&game.antimatter),
        antimatter_per_sec: format_decimal(&game.antimatter_per_second()),
        tickspeed_cost: format_decimal(tickspeed_cost),
        tickspeed_bought: game.tickspeed.bought,
        tickspeed_effect: format_decimal(&game.tickspeed_effect()),
        tickspeed_purchase_multiplier: game.tickspeed_purchase_multiplier(),
        dimensions,
        unlocked_dimensions: unlocked,
        dim_boosts: game.dim_boosts,
        dim_boost_req_tier: req_tier,
        dim_boost_req_amount: req_amount,
        can_dim_boost: game.can_dim_boost(),
        galaxies: game.galaxies,
        galaxy_requirement: game.galaxy_requirement(),
        can_buy_galaxy: game.can_buy_galaxy(),
        sacrifice_unlocked: game.sacrifice_unlocked,
        can_sacrifice: game.can_sacrifice(),
        sacrifice_multiplier: format_decimal(&game.sacrifice_multiplier()),
        sacrifice_multiplier_if_sacrificed: format_decimal(
            &game.sacrifice_multiplier_if_sacrificed(),
        ),
        can_buy_tickspeed: game.antimatter >= *tickspeed_cost,
    }
}

#[tauri::command]
fn tick_and_get_state(dt_ms: f64, state: State<'_, Mutex<GameState>>) -> GameView {
    let mut game = state.lock().unwrap();
    game.tick(dt_ms);
    build_game_view(&game)
}

#[tauri::command]
fn buy_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_dimension(tier);
}

#[tauri::command]
fn buy_until_10(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_until_10_dimension(tier);
}

#[tauri::command]
fn buy_tickspeed(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_tickspeed();
}

#[tauri::command]
fn buy_max_tickspeed(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_max_tickspeed();
}

#[tauri::command]
fn buy_dim_boost(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_dim_boost();
}

#[tauri::command]
fn buy_galaxy(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_galaxy();
}

#[tauri::command]
fn sacrifice(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.sacrifice();
}

#[tauri::command]
fn max_all(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.max_all();
}

/// Format a Decimal for display (matches the original game's
/// notation).
fn format_decimal(val: &Decimal) -> String {
    let f = val.to_f64();
    if f == 0.0 {
        return "0".to_string();
    }
    if f < 1000.0 {
        format!("{:.2}", f)
    } else if f < 1e9 {
        format_with_commas(f)
    } else {
        let exp = f.log10().floor() as i64;
        let mantissa = f / 10_f64.powi(exp as i32);
        if mantissa >= 9.995 {
            format!("1.00e{}", exp + 1)
        } else {
            format!("{:.2}e{}", mantissa, exp)
        }
    }
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(GameState::new()))
        .invoke_handler(tauri::generate_handler![
            tick_and_get_state,
            buy_dimension,
            buy_until_10,
            buy_tickspeed,
            buy_max_tickspeed,
            buy_dim_boost,
            buy_galaxy,
            sacrifice,
            max_all,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}

fn main() {
    run();
}
