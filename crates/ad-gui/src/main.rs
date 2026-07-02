mod persistence;

use std::path::PathBuf;
use std::sync::Mutex;

use ad_core::data::constants::{
    BIG_CRUNCH_THRESHOLD, BUY_TEN_MULTIPLIER, DIM_BOOST_MULTIPLIER,
};
use ad_core::save::{decode_save, encode_save};
use ad_core::{AutobuyerMode, Decimal, GameState};
use serde::Serialize;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

use persistence::{now_ms, SaveManager};

/// Ordinal names for the eight antimatter dimensions, used to label the
/// per-dimension autobuyers (mirrors `AntimatterDimension(tier).shortDisplayName`).
const DIMENSION_ORDINALS: [&str; 8] =
    ["1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"];

/// A raw number for the frontend to format: `mantissa × 10^exponent`.
///
/// Formatting moved to the webview (the `ad-format` WASM module), so the snapshot
/// ships raw numbers instead of pre-baked strings — see
/// `design-docs/2026-06-25-number-formatting.md` (Option C). The exponent is the
/// `Decimal`'s `i64` widened to `f64`, which is exact for every in-game magnitude.
#[derive(Serialize)]
struct Num {
    m: f64,
    e: f64,
}

/// Build a [`Num`] from a `Decimal` for the snapshot.
fn num(value: &Decimal) -> Num {
    Num {
        m: value.mantissa(),
        e: value.exponent() as f64,
    }
}

/// Serializable view of a single dimension tier for the frontend.
#[derive(Serialize)]
struct DimensionView {
    amount: Num,
    bought: u64,
    bought_mod_10: u64,
    multiplier: Num,
    production_per_sec: Num,
    /// Cost of a single purchase.
    single_cost: Num,
    /// Cost of buying `max(how_many_can_buy, 1)` (mirrors the JS
    /// `until10Cost`, which is the cost of the actual bulk purchase).
    until_10_cost: Num,
    /// How many can be bought right now without exceeding the
    /// current group of 10 (matches JS `dimension.howManyCanBuy`).
    how_many_can_buy: u64,
    can_buy: bool,
    rate_percent: f64,
    /// Whether this tier can be purchased (band + previous tier owned). Gates
    /// the buy button and drives the dimmed `c-dim-row--not-reached` style.
    available_for_purchase: bool,
    /// Whether the row is shown at all (progressive reveal).
    shown: bool,
}

/// Serializable view of a single autobuyer (AD tier or tickspeed).
#[derive(Serialize)]
struct AutobuyerView {
    /// Display name, e.g. "1st Dimension Autobuyer" / "Tickspeed Autobuyer".
    name: String,
    /// Whether the slow version is unlocked (full row vs. purchase box).
    is_bought: bool,
    /// Whether the antimatter requirement is met (purchase box enabled).
    can_unlock: bool,
    /// Requirement amount shown on the purchase box.
    requirement: Num,
    /// Interval between purchases, formatted in seconds (e.g. "0.50").
    interval_seconds: String,
    /// Whether the autobuyer is toggled on.
    is_active: bool,
    /// Current purchase mode: "single" or "max".
    mode: String,
    /// Whether the mode can be changed. Pre-Infinity the tickspeed autobuyer is
    /// locked to single (the toggle needs a completed challenge).
    can_change_mode: bool,
}

/// Serializable view of the whole autobuyers tab.
#[derive(Serialize)]
struct AutobuyersView {
    /// Whether the Automation tab (Autobuyers subtab) is unlocked.
    tab_unlocked: bool,
    /// Global on/off switch (JS `autobuyersOn`).
    enabled: bool,
    /// The eight antimatter dimension autobuyers (index 0 = 1st Dimension).
    dimensions: Vec<AutobuyerView>,
    /// The tickspeed autobuyer.
    tickspeed: AutobuyerView,
}

/// Serializable game view sent to the frontend each frame.
#[derive(Serialize)]
struct GameView {
    antimatter: Num,
    antimatter_per_sec: Num,
    tickspeed_cost: Num,
    tickspeed_bought: u64,
    tickspeed_effect: Num,
    tickspeed_purchase_multiplier: f64,
    /// Whether Tickspeed is unlocked (JS `Tickspeed.isUnlocked`).
    tickspeed_unlocked: bool,
    dimensions: Vec<DimensionView>,
    unlocked_dimensions: usize,
    buy_ten_multiplier: Num,
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
    sacrifice_multiplier: Num,
    /// Boost multiplier the next sacrifice would grant (JS
    /// `Sacrifice.nextBoost`); shown on the sacrifice button.
    next_sacrifice_boost: Num,
    /// Why sacrifice is disabled, when it is unlocked but not
    /// currently performable (JS `Sacrifice.disabledCondition`).
    sacrifice_disabled_condition: String,
    can_buy_tickspeed: bool,
    /// Progress towards Infinity in [0, 1] (log-scaled), for the
    /// bottom progress bar.
    infinity_progress: f64,
    /// Whether antimatter has reached the Big Crunch threshold, so the
    /// Big Crunch screen should replace the normal game view.
    can_big_crunch: bool,
    /// Whether the player has performed at least one Big Crunch (JS
    /// `PlayerProgress.infinityUnlocked()`). Persists across crunches.
    infinity_unlocked: bool,
    /// Autobuyer tab state (unlock progress, per-autobuyer status).
    autobuyers: AutobuyersView,
    /// Player options (UI/UX preferences), surfaced for the options tabs.
    options: OptionsView,
    /// Sorted ids of unlocked normal achievements; drives the Achievements tab
    /// and the unlock-toast diff in the frontend.
    unlocked_achievements: Vec<u16>,
    /// Global achievement-power multiplier (shown as the tab's boost).
    achievement_power: Num,
    /// Current tutorial-highlight step (`player.tutorialState`); components
    /// compare against their own step id to decide whether to glow.
    tutorial_state: u8,
    /// Whether the current tutorial step's highlight is active
    /// (`player.tutorialActive`).
    tutorial_active: bool,
}

/// Serializable view of the player options the frontend reads/writes.
#[derive(Serialize)]
struct OptionsView {
    hotkeys: bool,
    update_rate: u32,
    /// Active notation name; the frontend passes it to the WASM formatter.
    notation: String,
    /// Exponent-notation digit thresholds (comma / in-notation), passed to the
    /// WASM formatter and shown on the Exponent Notation modal's sliders.
    notation_digits_comma: u32,
    notation_digits_notation: u32,
    /// Offline replay resolution (original `offlineTicks`); drives the Gameplay
    /// tab slider and the Offline-mode replay budget.
    offline_ticks: u32,
    /// Autosave cadence in milliseconds (original `autosaveInterval`); drives the
    /// Saving-tab slider and the frontend autosave loop.
    autosave_interval: u32,
    /// Whether the header shows the elapsed time since the last save (original
    /// `showTimeSinceSave`).
    show_time_since_save: bool,
    /// Custom save-file name (original `saveFileName`); shown per slot in the
    /// "Choose save" modal and used as the default export filename.
    save_file_name: String,
    /// Per-action confirmation toggles; the action handlers branch on these to
    /// decide whether to show the explanatory modal.
    confirmations: ConfirmationsView,
}

/// Serializable view of the per-action confirmation toggles.
#[derive(Serialize)]
struct ConfirmationsView {
    dimension_boost: bool,
    antimatter_galaxy: bool,
    sacrifice: bool,
    big_crunch: bool,
}

/// Build the serializable view for one autobuyer.
fn build_autobuyer_view(
    autobuyer: &ad_core::Autobuyer,
    name: String,
    requirement: Decimal,
    can_unlock: bool,
    can_change_mode: bool,
) -> AutobuyerView {
    AutobuyerView {
        name,
        is_bought: autobuyer.is_bought,
        can_unlock,
        requirement: num(&requirement),
        interval_seconds: format!("{:.2}", autobuyer.interval_ms / 1000.0),
        is_active: autobuyer.is_active,
        mode: match autobuyer.mode {
            AutobuyerMode::BuySingle => "single".to_string(),
            AutobuyerMode::BuyMax => "max".to_string(),
        },
        can_change_mode,
    }
}

fn build_autobuyers_view(game: &GameState) -> AutobuyersView {
    let dimensions = (0..8)
        .map(|tier| {
            build_autobuyer_view(
                &game.autobuyers.dimensions[tier],
                format!("{} Dimension Autobuyer", DIMENSION_ORDINALS[tier]),
                GameState::ad_autobuyer_requirement(tier),
                game.can_unlock_ad_autobuyer(tier),
                // AD autobuyer mode ("Buys singles"/"Buys max") is always
                // changeable, even pre-Infinity.
                true,
            )
        })
        .collect();

    let tickspeed = build_autobuyer_view(
        &game.autobuyers.tickspeed,
        "Tickspeed Autobuyer".to_string(),
        GameState::tickspeed_autobuyer_requirement(),
        game.can_unlock_tickspeed_autobuyer(),
        // Pre-Infinity the tickspeed autobuyer is locked to single.
        false,
    );

    AutobuyersView {
        tab_unlocked: game.autobuyer_tab_unlocked(),
        enabled: game.autobuyers.enabled,
        dimensions,
        tickspeed,
    }
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

        // Per-second growth rate shown as "+X%/s", mirroring the JS
        // `AntimatterDimension.rateOfChange`: this tier is fed by the dimension
        // *above* it, so the rate is `production(tier+1) × 10 / max(amount, 1)`
        // (the 8th dimension, tier 7, has nothing above it → 0). Game-speed
        // scaling (`getGameSpeedupForDisplay`) is 1 pre-Infinity.
        let rate_percent = if tier < 7 && *amount > Decimal::ZERO {
            let to_gain = game.dimension_production_per_second(tier + 1);
            let denom = if *amount > Decimal::ONE {
                *amount
            } else {
                Decimal::ONE
            };
            (to_gain * Decimal::from_float(10.0) / denom).to_f64()
        } else {
            0.0
        };

        dimensions.push(DimensionView {
            amount: num(amount),
            bought,
            bought_mod_10: bought % 10,
            multiplier: num(&mult),
            production_per_sec: num(&production),
            single_cost: num(&cost),
            until_10_cost: num(&until_10_cost),
            how_many_can_buy,
            can_buy: game.antimatter >= cost,
            rate_percent,
            available_for_purchase: game.dim_available_for_purchase(tier),
            shown: game.dim_is_shown(tier) || *amount > Decimal::ZERO,
        });
    }

    let (req_tier, req_amount) = game.dim_boost_requirement();
    let tickspeed_cost = &game.tickspeed.cost;

    // Progress towards Infinity: log10(AM) / log10(BIG_CRUNCH_THRESHOLD),
    // clamped to [0, 1]. Keyed off the Big Crunch threshold (where antimatter
    // is capped) so progress hits 100% exactly at the cap.
    let log10_threshold = BIG_CRUNCH_THRESHOLD.log10();
    let am_plog10 = if game.antimatter > Decimal::ONE {
        game.antimatter.log10()
    } else {
        0.0
    };
    let infinity_progress = (am_plog10 / log10_threshold).clamp(0.0, 1.0);

    GameView {
        antimatter: num(&game.antimatter),
        antimatter_per_sec: num(&game.antimatter_per_second()),
        tickspeed_cost: num(tickspeed_cost),
        tickspeed_bought: game.tickspeed.bought,
        tickspeed_effect: num(&game.tickspeed_effect()),
        tickspeed_purchase_multiplier: game.tickspeed_purchase_multiplier(),
        tickspeed_unlocked: game.tickspeed_unlocked(),
        dimensions,
        unlocked_dimensions: unlocked,
        buy_ten_multiplier: num(&Decimal::from_float(BUY_TEN_MULTIPLIER)),
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
        sacrifice_multiplier: num(&game.sacrifice_multiplier()),
        next_sacrifice_boost: num(&game.next_sacrifice_boost()),
        sacrifice_disabled_condition: sacrifice_disabled_condition(game),
        can_buy_tickspeed: game.antimatter >= *tickspeed_cost,
        infinity_progress,
        can_big_crunch: game.can_big_crunch(),
        infinity_unlocked: game.infinity_unlocked,
        autobuyers: build_autobuyers_view(game),
        unlocked_achievements: game.unlocked_achievement_ids(),
        achievement_power: num(&game.achievement_power()),
        tutorial_state: game.tutorial_state,
        tutorial_active: game.tutorial_active,
        options: OptionsView {
            hotkeys: game.options.hotkeys,
            update_rate: game.options.update_rate,
            notation: game.options.notation.clone(),
            notation_digits_comma: game.options.notation_digits_comma,
            notation_digits_notation: game.options.notation_digits_notation,
            offline_ticks: game.options.offline_ticks,
            autosave_interval: game.options.autosave_interval,
            show_time_since_save: game.options.show_time_since_save,
            save_file_name: game.options.save_file_name.clone(),
            confirmations: ConfirmationsView {
                dimension_boost: game.options.confirmations.dimension_boost,
                antimatter_galaxy: game.options.confirmations.antimatter_galaxy,
                sacrifice: game.options.confirmations.sacrifice,
                big_crunch: game.options.confirmations.big_crunch,
            },
        },
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
fn tick_and_get_state(
    dt_ms: f64,
    repeats: u32,
    state: State<'_, Mutex<GameState>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    game.ticks(dt_ms, repeats);
    build_game_view(&game)
}

/// Replays `game_ms` of accumulated offline game-time (already speed-scaled by
/// the caller) at the resolution set by `offline_ticks`, returning the new view.
/// Used by the Offline-mode button when it is switched off. See
/// `design-docs/2026-06-30-offline-progress.md`.
#[tauri::command]
fn simulate_offline(
    game_ms: f64,
    offline_ticks: u32,
    state: State<'_, Mutex<GameState>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    game.simulate_offline(game_ms, offline_ticks);
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

#[tauri::command]
fn big_crunch(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.big_crunch();
}

/// Resets the current save slot to a fresh state (the "HARD RESET" option) and
/// persists it to disk.
#[tauri::command]
fn hard_reset(
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    *game = fresh_game();
    let mut save = save.lock().unwrap();
    let _ = save.save_root(&game, now_ms());
    build_game_view(&game)
}

#[tauri::command]
fn unlock_ad_autobuyer(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.unlock_ad_autobuyer(tier);
}

#[tauri::command]
fn toggle_ad_autobuyer(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_ad_autobuyer(tier);
}

#[tauri::command]
fn toggle_ad_autobuyer_mode(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_ad_autobuyer_mode(tier);
}

#[tauri::command]
fn unlock_tickspeed_autobuyer(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.unlock_tickspeed_autobuyer();
}

#[tauri::command]
fn toggle_tickspeed_autobuyer(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_tickspeed_autobuyer();
}

#[tauri::command]
fn toggle_autobuyers(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_autobuyers();
}

#[tauri::command]
fn set_all_autobuyers_active(active: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.set_all_autobuyers_active(active);
}

#[tauri::command]
fn set_hotkeys(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.hotkeys = enabled;
}

#[tauri::command]
fn set_update_rate(rate: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_update_rate(rate);
}

#[tauri::command]
fn set_notation(notation: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_notation(&notation);
}

#[tauri::command]
fn set_offline_ticks(ticks: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_offline_ticks(ticks);
}

#[tauri::command]
fn set_notation_digits(comma: u32, notation: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_notation_digits(comma, notation);
}

/// Flip a single per-action confirmation toggle (original
/// `player.options.confirmations.*`). `kind` is the camelCase action name
/// (`dimensionBoost`, `antimatterGalaxy`, `sacrifice`, `bigCrunch`); an unknown
/// name is ignored by the engine.
#[tauri::command]
fn set_confirmation(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_confirmation(&kind, enabled);
}

/// Sets the autosave cadence in milliseconds (original `autosaveInterval`); the
/// engine clamps to the 10–60 s slider range.
#[tauri::command]
fn set_autosave_interval(interval: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_autosave_interval(interval);
}

/// Toggles the header "time since save" indicator (original `showTimeSinceSave`).
#[tauri::command]
fn set_show_time_since_save(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.show_time_since_save = enabled;
}

/// Sets the custom save-file name (original `saveFileName`); the engine sanitizes
/// it (alphanumerics/space/hyphen, capped at 16 chars).
#[tauri::command]
fn set_save_file_name(name: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_save_file_name(&name);
}

/// Exports the current game state as an AD-compatible save string (for
/// clipboard copy). The frontend shows a toast on success.
#[tauri::command]
fn export_save(state: State<'_, Mutex<GameState>>) -> String {
    let game = state.lock().unwrap();
    encode_save(&game, now_ms())
}

/// Imports a save from a text string (pasted by the user). Decodes and
/// validates, then swaps the engine state. Returns the new `GameView` on
/// success or an error message on failure.
#[tauri::command]
fn import_save(
    text: String,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<GameView, String> {
    let new_state = decode_save(text.trim()).map_err(|e| e.to_string())?;
    let mut game = state.lock().unwrap();
    *game = new_state;
    // Mirror the original: an import fires an immediate save so it persists.
    let mut save = save.lock().unwrap();
    let _ = save.save_root(&game, now_ms());
    Ok(build_game_view(&game))
}

/// Exports the current save to a file chosen via a native "Save As" dialog. The
/// engine-owned custom save-file name (`options.saveFileName`) is used as the
/// default filename suggestion.
#[tauri::command]
async fn export_save_to_file(
    app: tauri::AppHandle,
    state: State<'_, Mutex<GameState>>,
) -> Result<(), String> {
    let (save_str, save_file_name) = {
        let game = state.lock().unwrap();
        (encode_save(&game, now_ms()), game.options.save_file_name.clone())
    };

    let default_name = if save_file_name.is_empty() {
        "Antimatter Dimensions Save".to_string()
    } else {
        save_file_name
    };

    let path = app
        .dialog()
        .file()
        .set_file_name(format!("{default_name}.txt"))
        .add_filter("Text files", &["txt"])
        .blocking_save_file();

    match path {
        Some(file_path) => std::fs::write(file_path.as_path().unwrap(), save_str)
            .map_err(|e| format!("Failed to write file: {e}")),
        None => Err("Cancelled".to_string()),
    }
}

/// Imports a save from a file chosen via a native "Open" dialog.
#[tauri::command]
async fn import_save_from_file(
    app: tauri::AppHandle,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<GameView, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Text files", &["txt"])
        .blocking_pick_file();

    match path {
        Some(file_path) => {
            let contents = std::fs::read_to_string(file_path.as_path().unwrap())
                .map_err(|e| format!("Failed to read file: {e}"))?;
            let new_state = decode_save(contents.trim()).map_err(|e| e.to_string())?;
            let mut game = state.lock().unwrap();
            *game = new_state;
            let mut save = save.lock().unwrap();
            let _ = save.save_root(&game, now_ms());
            Ok(build_game_view(&game))
        }
        None => Err("Cancelled".to_string()),
    }
}

/// Serializable summary of one save slot for the "Choose save" modal.
#[derive(Serialize)]
struct SlotMetaView {
    id: usize,
    /// Whether the slot holds a save.
    exists: bool,
    /// The slot's antimatter (only meaningful when `exists`).
    antimatter: Num,
    /// The slot's custom save-file name (empty if unset), shown per the original
    /// "Choose save" modal.
    save_file_name: String,
    /// Whether this is the active slot.
    is_current: bool,
}

/// Serializable summary of one automatic backup slot for the Backup menu.
#[derive(Serialize)]
struct BackupMetaView {
    id: u8,
    /// Whether the backup slot holds a save.
    exists: bool,
    /// The backup's antimatter (only meaningful when `exists`).
    antimatter: Num,
    /// When the backup was written (epoch ms), or `null` when empty. The frontend
    /// subtracts this from its live clock so "Last saved … ago" ticks in real time.
    last_backup_ms: Option<i64>,
}

/// Manually saves the current game state to disk (the "Save game" button, the
/// autosave loop, and the Ctrl/Cmd+S shortcut all route here).
#[tauri::command]
fn save_game(state: State<'_, Mutex<GameState>>, save: State<'_, Mutex<SaveManager>>) {
    let game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    let _ = save.save_root(&game, now_ms());
}

/// Switches the active save slot, persisting the current slot first and loading
/// the target (or a fresh game for an empty slot). Returns the new view.
#[tauri::command]
fn switch_save_slot(
    index: usize,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    *game = save.switch_slot(&game.clone(), index, fresh_game(), now_ms());
    build_game_view(&game)
}

/// Returns per-slot summaries for the "Choose save" modal.
#[tauri::command]
fn get_save_slots(
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Vec<SlotMetaView> {
    let game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    save.slot_metas(&game)
        .into_iter()
        .map(|m| SlotMetaView {
            id: m.id,
            exists: m.antimatter.is_some(),
            antimatter: me_to_num(m.antimatter),
            save_file_name: m.save_file_name,
            is_current: m.is_current,
        })
        .collect()
}

/// Writes the current state into a backup slot (online backups from the loop,
/// and the manual reserve slot).
#[tauri::command]
fn trigger_backup(
    slot: u8,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) {
    let game = state.lock().unwrap();
    let save = save.lock().unwrap();
    let _ = save.write_backup(slot, &game, now_ms());
}

/// Returns per-backup-slot summaries for the Backup menu.
#[tauri::command]
fn get_backups(save: State<'_, Mutex<SaveManager>>) -> Vec<BackupMetaView> {
    let save = save.lock().unwrap();
    save.backup_metas()
        .into_iter()
        .map(|m| BackupMetaView {
            id: m.id,
            exists: m.antimatter.is_some(),
            antimatter: me_to_num(m.antimatter),
            last_backup_ms: m.last_backup_ms,
        })
        .collect()
}

/// Loads a backup slot into the running game (saving the current state to the
/// reserve slot first). Returns the new view.
#[tauri::command]
fn load_backup(
    slot: u8,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<GameView, String> {
    let mut game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    let loaded = save.load_backup(slot, &game.clone(), now_ms())?;
    *game = loaded;
    Ok(build_game_view(&game))
}

/// Exports every populated backup of the current save slot as a single
/// backup-bundle file via a native "Save As" dialog (§2.4).
#[tauri::command]
async fn export_backups_to_file(
    app: tauri::AppHandle,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<(), String> {
    let bundle = {
        let save = save.lock().unwrap();
        save.export_backups(now_ms())
    };
    let bundle = bundle.ok_or_else(|| "No backups to export".to_string())?;

    let path = app
        .dialog()
        .file()
        .set_file_name("Antimatter Dimensions Backups.txt")
        .add_filter("Text files", &["txt"])
        .blocking_save_file();

    match path {
        Some(file_path) => std::fs::write(file_path.as_path().unwrap(), bundle)
            .map_err(|e| format!("Failed to write file: {e}")),
        None => Err("Cancelled".to_string()),
    }
}

/// Imports a backup-bundle file into the current save slot's backup slots via a
/// native "Open" dialog (§2.4). Returns how many slots were written.
#[tauri::command]
async fn import_backups_from_file(
    app: tauri::AppHandle,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<usize, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Text files", &["txt"])
        .blocking_pick_file();

    match path {
        Some(file_path) => {
            let contents = std::fs::read_to_string(file_path.as_path().unwrap())
                .map_err(|e| format!("Failed to read file: {e}"))?;
            let mut save = save.lock().unwrap();
            save.import_backups(&contents, now_ms())
        }
        None => Err("Cancelled".to_string()),
    }
}

/// Converts an optional `(mantissa, exponent)` pair into the snapshot's [`Num`].
/// An empty slot maps to zero, which the frontend hides behind the `exists` flag.
fn me_to_num(me: Option<(f64, f64)>) -> Num {
    match me {
        Some((m, e)) => Num { m, e },
        None => Num { m: 0.0, e: 0.0 },
    }
}

/// Constructs a fresh game for the running app.
///
/// In debug builds this applies developer conveniences (skip the tutorial,
/// disable confirmation modals) to speed up iteration. These are gated behind
/// `#[cfg(debug_assertions)]`, so any release build (`--release`, packaged
/// bundles) is compiled without them and always starts from the production
/// defaults in `GameState::new()`. There is nothing to remember to revert.
fn fresh_game() -> GameState {
    #[allow(unused_mut)]
    let mut game = GameState::new();
    #[cfg(debug_assertions)]
    {
        game.tutorial_state = 5;
        game.tutorial_active = false;
        game.options.confirmations.dimension_boost = false;
        game.options.confirmations.antimatter_galaxy = false;
        game.options.confirmations.sacrifice = false;
        game.options.confirmations.big_crunch = false;
    }
    game
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(fresh_game()))
        .setup(|app| {
            // Resolve the OS app-data dir (§12.1), load the on-disk root save into
            // the running game, and install the SaveManager. A missing/corrupt
            // save just starts fresh.
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let mut manager = SaveManager::new(dir);
            let loaded = manager.load(fresh_game(), now_ms());
            *app.state::<Mutex<GameState>>().lock().unwrap() = loaded;
            app.manage(Mutex::new(manager));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tick_and_get_state,
            simulate_offline,
            buy_dimension,
            buy_until_10,
            buy_tickspeed,
            buy_max_tickspeed,
            buy_dim_boost,
            buy_galaxy,
            sacrifice,
            max_all,
            big_crunch,
            hard_reset,
            unlock_ad_autobuyer,
            toggle_ad_autobuyer,
            toggle_ad_autobuyer_mode,
            unlock_tickspeed_autobuyer,
            toggle_tickspeed_autobuyer,
            toggle_autobuyers,
            set_all_autobuyers_active,
            set_hotkeys,
            set_update_rate,
            set_notation,
            set_notation_digits,
            set_offline_ticks,
            set_confirmation,
            set_autosave_interval,
            set_show_time_since_save,
            set_save_file_name,
            export_save,
            import_save,
            export_save_to_file,
            import_save_from_file,
            save_game,
            switch_save_slot,
            get_save_slots,
            trigger_backup,
            get_backups,
            load_backup,
            export_backups_to_file,
            import_backups_from_file,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_carries_raw_mantissa_and_exponent() {
        // The snapshot ships raw numbers; the webview's WASM formatter renders
        // them. `num` must read mantissa/exponent straight off the Decimal,
        // which stays exact past f64's range (~1.8e308).
        let n = num(&Decimal::new(1.0, 500));
        assert_eq!(n.m, 1.0);
        assert_eq!(n.e, 500.0);

        let z = num(&Decimal::ZERO);
        assert_eq!(z.m, 0.0);
        assert_eq!(z.e, 0.0);
    }

    #[test]
    fn autobuyers_view_reflects_unlock_state() {
        let mut game = GameState::new();

        // Fresh game: tab locked, nothing unlockable yet.
        let view = build_autobuyers_view(&game);
        assert!(!view.tab_unlocked);
        assert!(view.enabled);
        assert_eq!(view.dimensions.len(), 8);
        assert_eq!(view.dimensions[0].name, "1st Dimension Autobuyer");
        assert_eq!(view.dimensions[0].requirement.m, 1.0);
        assert_eq!(view.dimensions[0].requirement.e, 40.0);
        assert_eq!(view.dimensions[0].interval_seconds, "0.50");
        assert_eq!(view.dimensions[0].mode, "max");
        assert!(!view.dimensions[0].can_unlock);
        assert!(!view.dimensions[0].is_bought);
        // Tickspeed mode is locked pre-Infinity.
        assert!(!view.tickspeed.can_change_mode);
        assert_eq!(view.tickspeed.requirement.m, 1.0);
        assert_eq!(view.tickspeed.requirement.e, 140.0);

        // Past 1e40: tab unlocks and the 1st AD autobuyer becomes unlockable.
        game.total_antimatter = Decimal::new(1.0, 40);
        let view = build_autobuyers_view(&game);
        assert!(view.tab_unlocked);
        assert!(view.dimensions[0].can_unlock);
        assert!(!view.dimensions[1].can_unlock);

        // After unlocking, the entry reports as bought.
        game.unlock_ad_autobuyer(0);
        let view = build_autobuyers_view(&game);
        assert!(view.dimensions[0].is_bought);
    }
}
