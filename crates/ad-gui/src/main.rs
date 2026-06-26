use std::sync::Mutex;

use ad_core::data::constants::{BUY_TEN_MULTIPLIER, DIM_BOOST_MULTIPLIER};
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
    /// Cost of a single purchase.
    single_cost: String,
    /// Cost of buying `max(how_many_can_buy, 1)` (mirrors the JS
    /// `until10Cost`, which is the cost of the actual bulk purchase).
    until_10_cost: String,
    /// How many can be bought right now without exceeding the
    /// current group of 10 (matches JS `dimension.howManyCanBuy`).
    how_many_can_buy: u64,
    can_buy: bool,
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
    buy_ten_multiplier: String,
    dim_boosts: u32,
    dim_boost_power: f64,
    dim_boost_req_tier: usize,
    dim_boost_req_amount: u64,
    can_dim_boost: bool,
    galaxies: u32,
    galaxy_requirement: u64,
    can_buy_galaxy: bool,
    sacrifice_unlocked: bool,
    can_sacrifice: bool,
    sacrifice_multiplier: String,
    /// Boost multiplier the next sacrifice would grant (JS
    /// `Sacrifice.nextBoost`); shown on the sacrifice button.
    next_sacrifice_boost: String,
    /// Why sacrifice is disabled, when it is unlocked but not
    /// currently performable (JS `Sacrifice.disabledCondition`).
    sacrifice_disabled_condition: String,
    can_buy_tickspeed: bool,
    /// Progress towards Infinity in [0, 1] (log-scaled), for the
    /// bottom progress bar.
    infinity_progress: f64,
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

        // How many can be bought without overflowing the current
        // group of 10, capped by what is affordable. Mirrors JS
        // `floor(max(min(am/cost, 10 - boughtBefore10), 0))`.
        let remaining_in_group = 10 - (bought % 10);
        let affordable = if cost > Decimal::ZERO {
            (game.antimatter / cost).to_f64().floor() as u64
        } else {
            remaining_in_group
        };
        let how_many_can_buy = affordable.min(remaining_in_group);

        // Cost of the bulk purchase actually shown on the button.
        let bulk_count = how_many_can_buy.max(1);
        let until_10_cost = cost * Decimal::from_float(bulk_count as f64);

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
            single_cost: format_decimal(&cost),
            until_10_cost: format_decimal(&until_10_cost),
            how_many_can_buy,
            can_buy: game.antimatter >= cost,
            rate_percent,
        });
    }

    let (req_tier, req_amount) = game.dim_boost_requirement();
    let tickspeed_cost = &game.tickspeed.cost;

    // Progress towards Infinity: log10(AM) / log10(f64::MAX),
    // clamped to [0, 1] (matches the JS pLog10-based progress bar).
    let log10_max = f64::MAX.log10();
    let am_plog10 = if game.antimatter > Decimal::ONE {
        game.antimatter.log10()
    } else {
        0.0
    };
    let infinity_progress = (am_plog10 / log10_max).clamp(0.0, 1.0);

    GameView {
        antimatter: format_decimal(&game.antimatter),
        antimatter_per_sec: format_decimal(&game.antimatter_per_second()),
        tickspeed_cost: format_decimal(tickspeed_cost),
        tickspeed_bought: game.tickspeed.bought,
        tickspeed_effect: format_decimal(&game.tickspeed_effect()),
        tickspeed_purchase_multiplier: game.tickspeed_purchase_multiplier(),
        dimensions,
        unlocked_dimensions: unlocked,
        buy_ten_multiplier: format_decimal(&Decimal::from_float(BUY_TEN_MULTIPLIER)),
        dim_boosts: game.dim_boosts,
        dim_boost_power: DIM_BOOST_MULTIPLIER,
        dim_boost_req_tier: req_tier,
        dim_boost_req_amount: req_amount,
        can_dim_boost: game.can_dim_boost(),
        galaxies: game.galaxies,
        galaxy_requirement: game.galaxy_requirement(),
        can_buy_galaxy: game.can_buy_galaxy(),
        sacrifice_unlocked: game.sacrifice_unlocked(),
        can_sacrifice: game.can_sacrifice(),
        sacrifice_multiplier: format_decimal(&game.sacrifice_multiplier()),
        next_sacrifice_boost: format_decimal(&game.next_sacrifice_boost()),
        sacrifice_disabled_condition: sacrifice_disabled_condition(game),
        can_buy_tickspeed: game.antimatter >= *tickspeed_cost,
        infinity_progress,
    }
}

/// Mirror of the JS `Sacrifice.disabledCondition` for the
/// pre-infinity branches the engine can reach.
fn sacrifice_disabled_condition(game: &GameState) -> String {
    if game.dim_boosts < 5 {
        "Requires 5 Dimension Boosts".to_string()
    } else if game.dimensions[7].amount <= Decimal::ZERO {
        "No 8th Antimatter Dimensions".to_string()
    } else if game.next_sacrifice_boost() <= Decimal::ONE {
        "×1 multiplier".to_string()
    } else {
        "Need to Crunch".to_string()
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
        .plugin(tauri_plugin_opener::init())
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
