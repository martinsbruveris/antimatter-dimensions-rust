//! The Tauri command layer: every `#[tauri::command]` the webview can invoke,
//! plus their payload structs and parsing helpers. Commands are thin — they
//! lock the managed `GameState`, call one engine method (or a `SaveManager`
//! operation), and return a fresh snapshot where the frontend needs one. View
//! building lives in `views.rs`; game rules live in `ad-core`.

use std::sync::Mutex;

use ad_core::autobuyers::{AutoRealityMode, PrestigeAutobuyerMode};
use ad_core::save::{decode_save_with_last_update, encode_save};
use ad_core::{
    offline_plan as core_offline_plan, AutobuyerTarget, BreakInfinityRebuyable,
    BreakInfinityUpgrade, Decimal, GameState, InfinityUpgrade, INFINITY_DIMENSION_COUNT,
};
use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::persistence::{now_ms, SaveManager};
use crate::views::*;

// Command-return payload structs (constructed here, not part of GameView).
/// Serializable summary of one save slot for the "Choose save" modal.
#[derive(Serialize)]
pub(crate) struct SlotMetaView {
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
pub(crate) struct BackupMetaView {
    id: u8,
    /// Whether the backup slot holds a save.
    exists: bool,
    /// The backup's antimatter (only meaningful when `exists`).
    antimatter: Num,
    /// When the backup was written (epoch ms), or `null` when empty. The frontend
    /// subtracts this from its live clock so "Last saved … ago" ticks in real time.
    last_backup_ms: Option<i64>,
}

/// One event-log entry, shipped by `get_automator_events`.
#[derive(Serialize)]
pub(crate) struct AutomatorEventView {
    message: String,
    line: u32,
    this_reality_ms: f64,
    play_time_ms: f64,
    timegap_ms: f64,
}

/// A compile error for the editor gutter / error panel.
#[derive(Serialize)]
pub(crate) struct AutomatorErrorView {
    line: u32,
    info: String,
    tip: String,
}

#[tauri::command]
pub fn tick_and_get_state(
    dt_ms: f64,
    repeats: u32,
    state: State<'_, Mutex<GameState>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    game.ticks(dt_ms, repeats);
    // Drain the Automator's queued `notify` toasts into this frame's view.
    let notifications =
        std::mem::take(&mut game.automator.runtime.pending_notifications);
    let mut view = build_game_view(&game);
    view.automator.notifications = notifications;
    view
}

/// Replays `game_ms` of accumulated offline game-time (already speed-scaled by
/// the caller) at the resolution set by `offline_ticks`, returning the new view.
/// The all-at-once path, used for sub-threshold catch-ups where no progress modal
/// is shown. See `docs/design/2026-06-30-offline-progress.md`.
#[tauri::command]
pub fn simulate_offline(
    game_ms: f64,
    offline_ticks: u32,
    state: State<'_, Mutex<GameState>>,
) -> GameView {
    let mut game = state.lock().unwrap();
    game.simulate_offline(game_ms, offline_ticks);
    build_game_view(&game)
}

/// The engine's offline replay plan for `game_ms`: the total discrete tick count
/// and per-tick size (ms). The frontend splits `ticks` into batches, running
/// `tick_size_ms`-sized ticks itself to drive the offline catch-up progress bar.
#[derive(Serialize)]
pub(crate) struct OfflinePlan {
    ticks: u32,
    tick_size_ms: f64,
}

/// Returns the offline replay plan for `game_ms` at the chosen resolution, so the
/// GUI can run the catch-up in progress-bar-sized chunks (the budget policy stays
/// in the engine; the pacing lives in the webview).
#[tauri::command]
pub fn offline_plan(game_ms: f64, offline_ticks: u32) -> OfflinePlan {
    let (ticks, tick_size_ms) = core_offline_plan(game_ms, offline_ticks);
    OfflinePlan {
        ticks,
        tick_size_ms,
    }
}

/// Returns the current game view without advancing the engine. Used at startup to
/// seed the first snapshot before running any offline catch-up.
#[tauri::command]
pub fn get_state(state: State<'_, Mutex<GameState>>) -> GameView {
    let game = state.lock().unwrap();
    build_game_view(&game)
}

/// The offline gap (ms) detected at startup from the loaded save's `lastUpdate`,
/// awaiting replay. Consumed once by the frontend via [`take_pending_offline`];
/// zero when there is nothing to catch up.
#[derive(Default)]
pub struct PendingOffline(pub Mutex<f64>);

/// Returns the startup offline gap (ms) once, then clears it, so a reload of the
/// webview doesn't replay the same interval twice.
#[tauri::command]
pub fn take_pending_offline(pending: State<'_, PendingOffline>) -> f64 {
    let mut ms = pending.0.lock().unwrap();
    std::mem::take(&mut *ms)
}

/// A freshly loaded/imported state paired with the offline gap (ms) to replay.
/// The frontend installs `view`, then runs the catch-up over `offline_ms`.
#[derive(Serialize)]
pub(crate) struct LoadResult {
    view: GameView,
    offline_ms: f64,
}

#[tauri::command]
pub fn buy_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_dimension(tier);
}

#[tauri::command]
pub fn buy_until_10(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_until_10_dimension(tier);
}

#[tauri::command]
pub fn buy_tickspeed(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_tickspeed();
}

#[tauri::command]
pub fn buy_max_tickspeed(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_max_tickspeed();
}

#[tauri::command]
pub fn buy_dim_boost(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_dim_boost();
}

#[tauri::command]
pub fn buy_galaxy(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.buy_galaxy();
}

#[tauri::command]
pub fn sacrifice(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.sacrifice();
}

#[tauri::command]
pub fn max_all(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.max_all();
}

#[tauri::command]
pub fn big_crunch(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.big_crunch();
}

#[tauri::command]
pub fn buy_dilation_study(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_dilation_study(id);
}

#[tauri::command]
pub fn buy_dilation_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_dilation_upgrade(id);
}

#[tauri::command]
pub fn toggle_dilation(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if game.dilation.active {
        game.exit_dilation();
    } else {
        game.start_dilated_eternity();
    }
}

#[tauri::command]
pub fn buy_eternity_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    if let Some(upgrade) = ad_core::EternityUpgrade::from_id(id) {
        state.lock().unwrap().buy_eternity_upgrade(upgrade);
    }
}

#[tauri::command]
pub fn buy_ep_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ep_mult();
}

#[tauri::command]
pub fn buy_max_ep_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_ep_mult();
}

#[tauri::command]
pub fn buy_ec_study(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ec_study(id);
}

#[tauri::command]
pub fn start_eternity_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().start_eternity_challenge(id);
}

#[tauri::command]
pub fn exit_eternity_challenge(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().exit_eternity_challenge();
}

#[tauri::command]
pub fn buy_time_study(id: u16, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_time_study(id);
}

#[tauri::command]
pub fn buy_time_theorem(currency: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match currency.as_str() {
        "am" => {
            game.buy_tt_with_am();
        }
        "ip" => {
            game.buy_tt_with_ip();
        }
        "ep" => {
            game.buy_tt_with_ep();
        }
        _ => {}
    }
}

#[tauri::command]
pub fn buy_max_time_theorems(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_time_theorems();
}

#[tauri::command]
pub fn set_respec(respec: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().set_respec(respec);
}

#[tauri::command]
pub fn buy_time_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().buy_time_dimension(tier);
    }
}

#[tauri::command]
pub fn buy_max_time_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().buy_max_time_dimension(tier);
    }
}

#[tauri::command]
pub fn max_all_time_dimensions(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().max_all_time_dimensions();
}

#[tauri::command]
pub fn eternity(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.eternity();
}

#[tauri::command]
pub fn break_infinity(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.break_infinity();
}

#[tauri::command]
pub fn do_reality(
    choice: Option<usize>,
    sacrifice: bool,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.reality_with_glyph_choice(choice, sacrifice);
}

#[tauri::command]
pub fn reset_reality(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.reset_reality();
}

#[tauri::command]
pub fn equip_glyph(id: u32, slot: Option<u32>, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    // No slot given: use the first empty active slot.
    let slot = slot.or_else(|| {
        (0..game.glyph_active_slot_count() as u32)
            .find(|&s| !game.reality.glyphs.active.iter().any(|g| g.idx == s))
    });
    if let Some(slot) = slot {
        game.equip_glyph(id, slot);
    }
}

#[tauri::command]
pub fn sacrifice_glyph(id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.sacrifice_glyph(id);
}

#[tauri::command]
pub fn move_glyph(id: u32, slot: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.move_glyph_to_slot(id, slot);
}

#[tauri::command]
pub fn set_glyph_respec(respec: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().reality.respec = respec;
}

#[tauri::command]
pub fn buy_perk(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_perk(id);
}

#[tauri::command]
pub fn buy_reality_rebuyable(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_reality_rebuyable(id);
}

#[tauri::command]
pub fn buy_reality_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_reality_upgrade(id);
}

// --- Celestials (Phase 7) --------------------------------------------------

/// Pour RM into Teresa for `diff_ms` of real time (the Teresa tab's pour
/// button, held down; the frontend passes the frame delta).
#[tauri::command]
pub fn teresa_pour_rm(diff_ms: f64, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().teresa_pour_rm(diff_ms);
}

/// Reset Teresa's pour-rate timer (the pour button was released).
#[tauri::command]
pub fn teresa_stop_pouring(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().teresa_stop_pouring();
}

/// Buy one level of a Teresa Perk-Shop rebuyable by id (0–3).
#[tauri::command]
pub fn buy_perk_shop(id: usize, state: State<'_, Mutex<GameState>>) {
    if let Some(&entry) = ad_core::celestials::teresa::PERK_SHOP_ENTRIES
        .iter()
        .find(|e| e.id == id)
    {
        state.lock().unwrap().buy_perk_shop(entry);
    }
}

/// Buy an Effarig Relic-Shard unlock by id.
#[tauri::command]
pub fn effarig_buy_unlock(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().effarig_buy_unlock(id);
}

/// Toggle Enslaved's game-time storage (charge/uncharge the Black Hole).
#[tauri::command]
pub fn toggle_store_game_time(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().toggle_store_game_time();
}

/// Discharge Enslaved's stored game time (a burst on the next tick).
#[tauri::command]
pub fn enslaved_release(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().enslaved_release_stored_time();
}

/// Buy an Enslaved unlock by id (0 softcap / 1 run) with stored game time.
#[tauri::command]
pub fn buy_enslaved_unlock(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_enslaved_unlock(id);
}

/// Buy a Tesseract (the Infinity-Point cost is a threshold, not spent).
#[tauri::command]
pub fn buy_tesseract(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_tesseract();
}

/// Unlock V (the celestial) once all six main conditions are met.
#[tauri::command]
pub fn v_unlock_celestial(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().v_unlock_celestial();
}

/// Enter a celestial's Reality (`teresa`/`effarig`/`enslaved`/`v`).
#[tauri::command]
pub fn start_celestial_reality(celestial: String, state: State<'_, Mutex<GameState>>) {
    let cel = match celestial.as_str() {
        "teresa" => ad_core::Celestial::Teresa,
        "effarig" => ad_core::Celestial::Effarig,
        "enslaved" => ad_core::Celestial::Enslaved,
        "v" => ad_core::Celestial::V,
        "ra" => ad_core::Celestial::Ra,
        _ => return,
    };
    state.lock().unwrap().start_celestial_reality(cel);
}

#[tauri::command]
pub fn ra_level_up(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_level_up_max(pet);
}

#[tauri::command]
pub fn ra_buy_memory_upgrade(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_purchase_memory_upgrade(pet);
}

#[tauri::command]
pub fn ra_buy_chunk_upgrade(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_purchase_chunk_upgrade(pet);
}

#[tauri::command]
pub fn ra_set_remembrance(pet: i8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_set_remembrance(pet);
}

#[tauri::command]
pub fn alchemy_toggle_reaction(id: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().alchemy_toggle_reaction(id);
}

#[tauri::command]
pub fn dmd_buy_upgrade(tier: usize, kind: u8, state: State<'_, Mutex<GameState>>) {
    let mut g = state.lock().unwrap();
    match kind {
        0 => g.dmd_buy_interval(tier),
        1 => g.dmd_buy_power_dm(tier),
        2 => g.dmd_buy_power_de(tier),
        _ => false,
    };
}

#[tauri::command]
pub fn dmd_ascend(tier: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().dmd_ascend(tier);
}

#[tauri::command]
pub fn laitela_max_all_dmd(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().laitela_max_all_dmd();
}

#[tauri::command]
pub fn laitela_annihilate(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().annihilate(false);
}

#[tauri::command]
pub fn laitela_condense_singularity(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().condense_singularity();
}

#[tauri::command]
pub fn laitela_set_continuum(on: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().set_continuum(on);
}

#[tauri::command]
pub fn laitela_change_singularity_cap(
    increase: bool,
    state: State<'_, Mutex<GameState>>,
) {
    let mut g = state.lock().unwrap();
    if increase {
        g.singularity_increase_cap();
    } else {
        g.singularity_decrease_cap();
    }
}

#[tauri::command]
pub fn buy_imaginary_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_imaginary_upgrade(id);
}

#[tauri::command]
pub fn buy_imaginary_rebuyable(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_imaginary_rebuyable(id);
}

#[tauri::command]
pub fn doom_reality(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().doom_reality();
}

#[tauri::command]
pub fn pelle_armageddon(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().armageddon(true);
}

#[tauri::command]
pub fn pelle_toggle_rift(rift: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().pelle_toggle_rift(rift);
}

#[tauri::command]
pub fn buy_pelle_upgrade(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_pelle_upgrade(id);
}

#[tauri::command]
pub fn buy_pelle_rebuyable(id: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_pelle_rebuyable(id);
}

#[tauri::command]
pub fn pelle_start_sacrifice(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().galaxy_generator_start_sacrifice();
}

#[tauri::command]
pub fn unlock_black_hole(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().unlock_black_hole();
}

#[tauri::command]
pub fn buy_black_hole_upgrade(
    hole: usize,
    kind: u8,
    state: State<'_, Mutex<GameState>>,
) {
    state.lock().unwrap().buy_black_hole_upgrade(hole, kind);
}

/// Set the Black-Hole inversion strength as the slider exponent (0..=300 →
/// `10^-x`).
#[tauri::command]
pub fn set_black_hole_negative(exponent: f64, state: State<'_, Mutex<GameState>>) {
    let negative = 10f64.powf(-exponent.clamp(0.0, 300.0));
    state.lock().unwrap().set_black_hole_negative(negative);
}

/// Set the Black-Hole auto-pause mode (0 never / 1 before BH1 / 2 before BH2).
#[tauri::command]
pub fn set_black_hole_auto_pause(mode: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().black_holes.auto_pause_mode = mode.min(2);
}

#[tauri::command]
pub fn toggle_black_hole_pause(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().toggle_black_hole_pause();
}

/// Buy a one-time Break Infinity Upgrade by its original save id (e.g.
/// "totalMult"). An unrecognized id is a no-op.
#[tauri::command]
pub fn buy_break_infinity_upgrade(id: String, state: State<'_, Mutex<GameState>>) {
    if let Some(upgrade) = BreakInfinityUpgrade::from_save_id(&id) {
        state.lock().unwrap().buy_break_infinity_upgrade(upgrade);
    }
}

/// Buy one level of a rebuyable Break Infinity Upgrade by index (0/1/2).
#[tauri::command]
pub fn buy_break_infinity_rebuyable(id: usize, state: State<'_, Mutex<GameState>>) {
    let upgrade = match id {
        0 => BreakInfinityRebuyable::TickspeedCostMult,
        1 => BreakInfinityRebuyable::DimCostMult,
        2 => BreakInfinityRebuyable::IpGen,
        _ => return,
    };
    state.lock().unwrap().buy_break_infinity_rebuyable(upgrade);
}

/// Buy one purchase (10 IDs) of the given Infinity Dimension tier (0-indexed), or
/// unlock it if locked.
#[tauri::command]
pub fn buy_infinity_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < INFINITY_DIMENSION_COUNT {
        state.lock().unwrap().buy_infinity_dimension(tier);
    }
}

/// Buy-max a single Infinity Dimension tier (0-indexed).
#[tauri::command]
pub fn buy_max_infinity_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < INFINITY_DIMENSION_COUNT {
        state.lock().unwrap().buy_max_infinity_dimension(tier);
    }
}

/// Buy-max all Infinity Dimensions.
#[tauri::command]
pub fn buy_max_all_infinity_dimensions(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_all_infinity_dimensions();
}

/// Unlock Replicanti (spends 1e140 IP); a no-op if already unlocked or unaffordable.
#[tauri::command]
pub fn unlock_replicanti(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().unlock_replicanti();
}

/// Acknowledge the tab-notification badge for `key` (`tabKey + subtabKey`);
/// called by the frontend when the player opens that tab.
#[tauri::command]
pub fn tab_notification_seen(key: String, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().tab_notification_seen(&key);
}

/// Buy one Replicanti chance upgrade (`+1%`).
#[tauri::command]
pub fn buy_replicanti_chance(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_chance();
}

/// Buy one Replicanti interval upgrade (`×0.9`).
#[tauri::command]
pub fn buy_replicanti_interval(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_interval();
}

/// Buy one Replicanti max-galaxies upgrade (`+1`).
#[tauri::command]
pub fn buy_replicanti_galaxy_cap(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_galaxy_cap();
}

/// Buy a Replicanti Galaxy (resets Replicanti; a no-op unless at the cap and below
/// the bought-galaxy cap).
#[tauri::command]
pub fn buy_replicanti_galaxy(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_galaxy();
}

/// Buy an Infinity Upgrade by its original save id (e.g. "timeMult"). An
/// unrecognized id is a no-op.
#[tauri::command]
pub fn buy_infinity_upgrade(id: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if let Some(upgrade) = InfinityUpgrade::from_save_id(&id) {
        game.buy_infinity_upgrade(upgrade);
    }
}

/// Buy a single ×2 IP-multiplier (`ipMult`) purchase.
#[tauri::command]
pub fn buy_ip_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ip_mult();
}

/// Buy as many `ipMult` purchases as affordable (`buyMax`).
#[tauri::command]
pub fn buy_max_ip_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_ip_mult();
}

/// Buy the one-time `ipOffline` Infinity Upgrade.
#[tauri::command]
pub fn buy_ip_offline(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ip_offline();
}

/// Toggle the IP-multiplier autobuyer (`Autobuyer.ipMult.isActive`).
#[tauri::command]
pub fn set_ip_mult_autobuyer(active: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().autobuyers.ip_mult_buyer_active = active;
}

/// Undo the last equipped glyph (Teresa's undo unlock).
#[tauri::command]
pub fn undo_glyph(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().undo_glyph();
}

/// Set Effarig's glyph-level factor weights (ep/repl/dt/eternities). Values
/// are clamped to 0..=100; the caller keeps the sum at 100 like the original
/// slider group.
#[tauri::command]
pub fn set_glyph_weights(weights: Vec<f64>, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    for (i, w) in weights.iter().take(4).enumerate() {
        game.celestials.effarig.glyph_weights[i] = w.clamp(0.0, 100.0);
    }
}

/// Create a Reality Glyph from the reality Alchemy resource.
#[tauri::command]
pub fn create_reality_glyph(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().create_reality_glyph();
}

/// Set the glyph filter's selection / rejection modes and the shared
/// effect-count threshold.
#[tauri::command]
pub fn set_glyph_filter_modes(
    select: u8,
    trash: u8,
    simple: u32,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    let filter = &mut game.reality.glyphs.filter;
    if select <= 6 {
        filter.select = select;
    }
    if trash <= 2 {
        filter.trash = trash;
    }
    filter.simple = simple;
}

/// Set one type's glyph-filter config (thresholds, required-effect mask,
/// per-effect scores).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn set_glyph_filter_type(
    kind: String,
    rarity: f64,
    score: f64,
    effect_count: u32,
    specified_mask: u32,
    effect_scores: Vec<f64>,
    state: State<'_, Mutex<GameState>>,
) {
    let Some(kind) = ad_core::GlyphType::from_save_id(&kind) else {
        return;
    };
    let Some(index) = ad_core::glyphs::GlyphFilter::type_index(kind) else {
        return;
    };
    let mut game = state.lock().unwrap();
    let cfg = &mut game.reality.glyphs.filter.types[index];
    cfg.rarity = rarity.clamp(0.0, 100.0);
    cfg.score = score;
    cfg.effect_count = effect_count.min(7);
    cfg.specified_mask = specified_mask;
    for (i, v) in effect_scores
        .iter()
        .take(cfg.effect_scores.len())
        .enumerate()
    {
        cfg.effect_scores[i] = *v;
    }
}

/// The lump-sum offline currency award (`ipOffline`), fired once by the
/// frontend's chunked offline replay before its first chunk. The all-at-once
/// `simulate_offline` path applies it engine-side instead.
#[tauri::command]
pub fn offline_currency_gain(away_ms: f64, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().offline_currency_gain(away_ms);
}

/// Start Normal Challenge `id` (2..=12); a no-op if it can't be started.
#[tauri::command]
pub fn start_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.start_challenge(id);
}

/// Exit the current challenge (Normal or Infinity; a no-op if none is running).
#[tauri::command]
pub fn exit_challenge(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.exit_challenge();
}

/// Start Infinity Challenge `id` (1..=8); a no-op if it can't be started.
#[tauri::command]
pub fn start_infinity_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.start_infinity_challenge(id);
}

/// Resets the current save slot to a fresh state (the "HARD RESET" option) and
/// persists it to disk.
#[tauri::command]
pub fn hard_reset(
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
pub fn unlock_ad_autobuyer(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.unlock_ad_autobuyer(tier);
}

#[tauri::command]
pub fn toggle_ad_autobuyer(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_ad_autobuyer(tier);
}

#[tauri::command]
pub fn toggle_ad_autobuyer_mode(tier: usize, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_ad_autobuyer_mode(tier);
}

#[tauri::command]
pub fn unlock_tickspeed_autobuyer(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.unlock_tickspeed_autobuyer();
}

#[tauri::command]
pub fn toggle_tickspeed_autobuyer(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_tickspeed_autobuyer();
}

#[tauri::command]
pub fn toggle_autobuyers(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.toggle_autobuyers();
}

#[tauri::command]
pub fn set_all_autobuyers_active(active: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.set_all_autobuyers_active(active);
}

/// Parse the frontend's string autobuyer handle (`ad0`..`ad7`, `tickspeed`,
/// `dimBoost`, `galaxy`, `bigCrunch`) into an [`AutobuyerTarget`].
fn parse_autobuyer_target(target: &str) -> Option<AutobuyerTarget> {
    match target {
        "tickspeed" => Some(AutobuyerTarget::Tickspeed),
        "dimBoost" => Some(AutobuyerTarget::DimBoost),
        "galaxy" => Some(AutobuyerTarget::Galaxy),
        "bigCrunch" => Some(AutobuyerTarget::BigCrunch),
        other => other
            .strip_prefix("ad")
            .and_then(|n| n.parse::<usize>().ok())
            .filter(|&tier| tier < 8)
            .map(AutobuyerTarget::AdTier),
    }
}

#[tauri::command]
pub fn upgrade_autobuyer_interval(target: String, state: State<'_, Mutex<GameState>>) {
    if let Some(target) = parse_autobuyer_target(&target) {
        state.lock().unwrap().upgrade_autobuyer_interval(target);
    }
}

/// Double an AD autobuyer's "Buys max" bulk (`upgradeBulk`), once its interval
/// is maxed.
#[tauri::command]
pub fn upgrade_ad_autobuyer_bulk(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().upgrade_ad_autobuyer_bulk(tier);
    }
}

/// Toggle one of the milestone autobuyers (or its group flag). `kind` is
/// "infinityDims" / "replicantiUpgrades" / "replicantiGalaxy"; `index` selects
/// the entry, or `None` toggles the group flag.
#[tauri::command]
pub fn toggle_milestone_autobuyer(
    kind: String,
    index: Option<usize>,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    let a = &mut game.autobuyers;
    match (kind.as_str(), index) {
        ("infinityDims", Some(i)) if i < 8 => {
            a.infinity_dims[i].is_active = !a.infinity_dims[i].is_active;
        }
        ("infinityDims", None) => {
            a.infinity_dims_group_active = !a.infinity_dims_group_active;
        }
        ("replicantiUpgrades", Some(i)) if i < 3 => {
            a.replicanti_upgrades[i].is_active = !a.replicanti_upgrades[i].is_active;
        }
        ("replicantiUpgrades", None) => {
            a.replicanti_upgrades_group_active = !a.replicanti_upgrades_group_active;
        }
        ("replicantiGalaxy", _) => {
            a.replicanti_galaxies_active = !a.replicanti_galaxies_active;
        }
        ("timeDims", Some(i)) if i < 8 => {
            a.time_dims[i].is_active = !a.time_dims[i].is_active;
        }
        ("timeDims", None) => {
            a.time_dims_group_active = !a.time_dims_group_active;
        }
        ("epMult", _) => {
            a.ep_mult_buyer_active = !a.ep_mult_buyer_active;
        }
        _ => {}
    }
}

#[tauri::command]
pub fn toggle_autobuyer(target: String, state: State<'_, Mutex<GameState>>) {
    // The Eternity / Reality autobuyers have no `AutobuyerTarget` (no
    // interval-upgrade machinery); they get their own toggles.
    match target.as_str() {
        "eternity" => state.lock().unwrap().toggle_eternity_autobuyer(),
        "reality" => state.lock().unwrap().toggle_reality_autobuyer(),
        other => {
            if let Some(target) = parse_autobuyer_target(other) {
                state.lock().unwrap().toggle_autobuyer_active(target);
            }
        }
    }
}

/// Parse an autobuyer-input string the way the original `AutobuyerInput`'s
/// decimal parser does: plain / scientific ("2.5e30"), logarithm ("e30"), and
/// mixed-scientific ("2.33e41.2") forms, commas stripped.
fn parse_decimal_input(input: &str) -> Option<Decimal> {
    let s: String = input.chars().filter(|&c| c != ',').collect();
    if s.is_empty() {
        return None;
    }
    // Logarithm form: e<float>.
    if let Some(exp) = s.strip_prefix('e') {
        if !exp.is_empty() && exp.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Some(Decimal::pow10(exp.parse::<f64>().ok()?));
        }
        return None;
    }
    let valid_num =
        |t: &str| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.');
    match s.split_once(['e', 'E']) {
        None => {
            if !valid_num(&s) {
                return None;
            }
            Some(Decimal::from_float(s.parse::<f64>().ok()?))
        }
        Some((mantissa, exponent)) => {
            if !valid_num(mantissa) || !valid_num(exponent) {
                return None;
            }
            let m = mantissa.parse::<f64>().ok()?;
            if m <= 0.0 {
                return None;
            }
            // Mixed-scientific exponents ("2.33e41.2") fold into pow10.
            let e = exponent.parse::<f64>().ok()?;
            Some(Decimal::pow10(m.log10() + e))
        }
    }
}

#[tauri::command]
pub fn set_prestige_autobuyer_mode(
    target: String,
    mode: String,
    state: State<'_, Mutex<GameState>>,
) {
    let mode = match mode.as_str() {
        "amount" => PrestigeAutobuyerMode::Amount,
        "time" => PrestigeAutobuyerMode::Time,
        "xHighest" => PrestigeAutobuyerMode::XHighest,
        _ => return,
    };
    let mut game = state.lock().unwrap();
    match target.as_str() {
        "bigCrunch" => {
            game.set_big_crunch_autobuyer_mode(mode);
        }
        "eternity" => {
            game.set_eternity_autobuyer_mode(mode);
        }
        _ => {}
    }
}

/// Set the value input of the Big Crunch / Eternity autobuyer for its current
/// mode. Returns false for unparseable input (the box shows invalid state).
#[tauri::command]
pub fn set_prestige_autobuyer_value(
    target: String,
    value: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    let Some(value) = parse_decimal_input(&value) else {
        return false;
    };
    let mut game = state.lock().unwrap();
    match target.as_str() {
        "bigCrunch" => game.set_big_crunch_autobuyer_value(value),
        "eternity" => game.set_eternity_autobuyer_value(value),
        _ => return false,
    }
    true
}

#[tauri::command]
pub fn toggle_autobuyer_dynamic_amount(
    target: String,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    match target.as_str() {
        "bigCrunch" => game.toggle_big_crunch_dynamic_amount(),
        "eternity" => game.toggle_eternity_dynamic_amount(),
        _ => {}
    }
}

#[tauri::command]
pub fn set_reality_autobuyer_mode(mode: String, state: State<'_, Mutex<GameState>>) {
    let mode = match mode.as_str() {
        "rm" => AutoRealityMode::Rm,
        "glyph" => AutoRealityMode::Glyph,
        "either" => AutoRealityMode::Either,
        "both" => AutoRealityMode::Both,
        "time" => AutoRealityMode::Time,
        _ => return,
    };
    state.lock().unwrap().set_reality_autobuyer_mode(mode);
}

/// Set one of the Reality autobuyer's targets ("rm" decimal, "glyph" int,
/// "time" float). Returns false for unparseable input.
#[tauri::command]
pub fn set_reality_autobuyer_value(
    property: String,
    value: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    let mut game = state.lock().unwrap();
    match property.as_str() {
        "rm" => {
            let Some(value) = parse_decimal_input(&value) else {
                return false;
            };
            game.set_reality_autobuyer_rm(value);
        }
        "glyph" => {
            let Ok(value) = value.trim().parse::<u32>() else {
                return false;
            };
            game.set_reality_autobuyer_glyph(value);
        }
        "time" => {
            let Ok(value) = value.trim().parse::<f64>() else {
                return false;
            };
            game.set_reality_autobuyer_time(value);
        }
        _ => return false,
    }
    true
}

// --- Automator (Feature 6.6 Stage D) ------------------------------------------

/// The play button: pause / resume / start the editor's script.
#[tauri::command]
pub fn automator_play(script_id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_play(script_id);
}

#[tauri::command]
pub fn automator_stop(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_stop();
}

/// Rewind: restart the running script from the top.
#[tauri::command]
pub fn automator_rewind(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_restart();
}

/// Single-step one command (starting the editor's script when off).
#[tauri::command]
pub fn automator_step(script_id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_step_once(Some(script_id));
}

/// Toggle one of the controls-bar settings: "repeat" / "forceRestart" /
/// "followExecution".
#[tauri::command]
pub fn automator_toggle_setting(setting: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match setting.as_str() {
        "repeat" => game.automator_toggle_repeat(),
        "forceRestart" => game.automator_toggle_force_restart(),
        "followExecution" => game.automator_toggle_follow_execution(),
        _ => {}
    }
}

#[tauri::command]
pub fn automator_select_script(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_select_editor_script(id);
}

/// Create a fresh script and open it in the editor. Returns its id (None at
/// the 20-script cap).
#[tauri::command]
pub fn automator_new_script(state: State<'_, Mutex<GameState>>) -> Option<u32> {
    let mut game = state.lock().unwrap();
    let id = game.automator_new_script()?;
    game.automator_select_editor_script(id);
    Some(id)
}

#[tauri::command]
pub fn automator_rename_script(
    id: u32,
    name: String,
    state: State<'_, Mutex<GameState>>,
) {
    state.lock().unwrap().automator_rename_script(id, &name);
}

#[tauri::command]
pub fn automator_delete_script(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_delete_script(id);
}

/// The stored content of one script (the editor loads it when switching).
#[tauri::command]
pub fn get_automator_script(id: u32, state: State<'_, Mutex<GameState>>) -> String {
    state
        .lock()
        .unwrap()
        .automator
        .scripts
        .get(&id)
        .map(|s| s.content.clone())
        .unwrap_or_default()
}

/// Result of saving script content: whether it persisted (character limits)
/// and the compile errors of the *typed* content either way.
#[derive(Serialize)]
pub(crate) struct AutomatorSaveResult {
    saved: bool,
    errors: Vec<AutomatorErrorView>,
}

/// Save the editor's content (stops the script when it is the running one)
/// and recompile for the error panel/gutter.
#[tauri::command]
pub fn save_automator_script(
    id: u32,
    content: String,
    state: State<'_, Mutex<GameState>>,
) -> AutomatorSaveResult {
    let mut game = state.lock().unwrap();
    let saved = game.automator_save_script(id, &content);
    AutomatorSaveResult {
        saved,
        errors: compile_errors(&game, &content),
    }
}

/// Compile errors for a script's stored content (initial editor mount).
#[tauri::command]
pub fn get_automator_errors(
    id: u32,
    state: State<'_, Mutex<GameState>>,
) -> Vec<AutomatorErrorView> {
    let game = state.lock().unwrap();
    let content = game
        .automator
        .scripts
        .get(&id)
        .map(|s| s.content.clone())
        .unwrap_or_default();
    compile_errors(&game, &content)
}

fn compile_errors(game: &GameState, content: &str) -> Vec<AutomatorErrorView> {
    game.compile_automator_script(content)
        .errors
        .into_iter()
        .map(|e| AutomatorErrorView {
            line: e.line,
            info: e.info,
            tip: e.tip,
        })
        .collect()
}

#[tauri::command]
pub fn automator_set_constant(
    name: String,
    value: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state.lock().unwrap().automator_set_constant(&name, &value)
}

#[tauri::command]
pub fn automator_rename_constant(
    old_name: String,
    new_name: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state
        .lock()
        .unwrap()
        .automator_rename_constant(&old_name, &new_name)
}

#[tauri::command]
pub fn automator_delete_constant(name: String, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_delete_constant(&name);
}

#[tauri::command]
pub fn automator_set_info_pane(pane: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator.current_info_pane = pane.min(7);
}

/// The event log plus the current play-time clock (for relative timestamps).
#[derive(Serialize)]
pub(crate) struct AutomatorEventLogView {
    now_play_time_ms: f64,
    events: Vec<AutomatorEventView>,
}

#[tauri::command]
pub fn get_automator_events(
    state: State<'_, Mutex<GameState>>,
) -> AutomatorEventLogView {
    let game = state.lock().unwrap();
    AutomatorEventLogView {
        now_play_time_ms: game.records.real_time_played_ms,
        events: game
            .automator
            .runtime
            .events
            .iter()
            .map(|e| AutomatorEventView {
                message: e.message.clone(),
                line: e.line,
                this_reality_ms: e.this_reality_ms,
                play_time_ms: e.play_time_ms,
                timegap_ms: e.timegap_ms,
            })
            .collect(),
    }
}

#[tauri::command]
pub fn automator_clear_log(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_clear_event_log();
}

/// Event-log display options ("newestFirst" / "clearOnReality" /
/// "clearOnRestart" booleans; "timestampType" 0–4).
#[tauri::command]
pub fn set_automator_event_option(
    option: String,
    value: i64,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    let opts = &mut game.options.automator_events;
    match option.as_str() {
        "newestFirst" => opts.newest_first = value != 0,
        "timestampType" => opts.timestamp_type = value.clamp(0, 4) as u8,
        "clearOnReality" => opts.clear_on_reality = value != 0,
        "clearOnRestart" => opts.clear_on_restart = value != 0,
        _ => {}
    }
}

/// Blockify a script's stored content for the block editor.
#[derive(Serialize)]
pub(crate) struct BlockifyView {
    blocks: Vec<ad_core::automator::blocks::BlockData>,
    lost_lines: usize,
}

#[tauri::command]
pub fn automator_blockify(id: u32, state: State<'_, Mutex<GameState>>) -> BlockifyView {
    let game = state.lock().unwrap();
    let content = game
        .automator
        .scripts
        .get(&id)
        .map(|s| s.content.clone())
        .unwrap_or_default();
    let result = game.automator_blockify(&content);
    BlockifyView {
        blocks: result.blocks,
        lost_lines: result.lost_lines,
    }
}

/// Blockify arbitrary script text (block-mode template creation).
#[tauri::command]
pub fn automator_blockify_text(
    content: String,
    state: State<'_, Mutex<GameState>>,
) -> BlockifyView {
    let result = state.lock().unwrap().automator_blockify(&content);
    BlockifyView {
        blocks: result.blocks,
        lost_lines: result.lost_lines,
    }
}

/// Switch the editor flavor ("text"/"block"); content conversion happens
/// frontend-side before the call.
#[tauri::command]
pub fn automator_set_editor_type(block: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_set_editor_type(block);
}

/// Template generation inputs, as typed in the modal (numbers arrive as
/// strings and are parsed like the autobuyer inputs).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct TemplateParamsIn {
    tree_studies: String,
    tree_nowait: bool,
    final_ep: String,
    crunches_per_eternity: String,
    eternities: String,
    infinities: String,
    is_banked: bool,
    ec: String,
    completions: String,
    auto_inf_mode: String,
    auto_inf_value: String,
    auto_eter_mode: String,
    auto_eter_value: String,
}

impl Default for TemplateParamsIn {
    fn default() -> Self {
        Self {
            tree_studies: String::new(),
            tree_nowait: false,
            final_ep: "0".into(),
            crunches_per_eternity: "1".into(),
            eternities: "0".into(),
            infinities: "0".into(),
            is_banked: false,
            ec: "1".into(),
            completions: "1".into(),
            auto_inf_mode: "mult".into(),
            auto_inf_value: "1".into(),
            auto_eter_mode: "mult".into(),
            auto_eter_value: "1".into(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct TemplateView {
    script: String,
    warnings: Vec<String>,
}

#[tauri::command]
pub fn automator_template(
    name: String,
    params: TemplateParamsIn,
    state: State<'_, Mutex<GameState>>,
) -> Option<TemplateView> {
    let dec = |s: &str| parse_decimal_input(s).unwrap_or(Decimal::ZERO);
    let int = |s: &str| s.trim().parse::<u32>().unwrap_or(0);
    let engine_params = ad_core::automator::templates::TemplateParams {
        tree_studies: params.tree_studies.clone(),
        tree_nowait: params.tree_nowait,
        final_ep: dec(&params.final_ep),
        crunches_per_eternity: int(&params.crunches_per_eternity).max(1),
        eternities: dec(&params.eternities),
        infinities: dec(&params.infinities),
        is_banked: params.is_banked,
        ec: int(&params.ec).min(255) as u8,
        completions: int(&params.completions),
        auto_inf_mode: params.auto_inf_mode.clone(),
        auto_inf_value: dec(&params.auto_inf_value),
        auto_eter_mode: params.auto_eter_mode.clone(),
        auto_eter_value: dec(&params.auto_eter_value),
    };
    let game = state.lock().unwrap();
    let t = game.automator_template(&name, &engine_params)?;
    Some(TemplateView {
        script: t.script,
        warnings: t.warnings,
    })
}

/// Export one script as encoded text (None for a blank script).
#[tauri::command]
pub fn automator_export_script(
    id: u32,
    state: State<'_, Mutex<GameState>>,
) -> Option<String> {
    state.lock().unwrap().automator_export_script(id)
}

/// Export one script plus the presets/constants it references.
#[tauri::command]
pub fn automator_export_full(
    id: u32,
    state: State<'_, Mutex<GameState>>,
) -> Option<String> {
    state.lock().unwrap().automator_export_full_data(id)
}

/// What an import string contains, for the modal preview.
#[derive(Serialize)]
pub(crate) struct ImportPreview {
    name: String,
    content: String,
    /// (1-based slot, name, studies) for a full-data import.
    presets: Vec<(usize, String, String)>,
    constants: Vec<(String, String)>,
    is_full_data: bool,
    /// Whether the script has compilation errors (`hasCompilationErrors`).
    has_errors: bool,
}

#[tauri::command]
pub fn automator_import_preview(
    raw: String,
    state: State<'_, Mutex<GameState>>,
) -> Option<ImportPreview> {
    use ad_core::automator::transfer;
    let game = state.lock().unwrap();
    if let Some(parsed) = transfer::parse_script_contents(&raw) {
        let has_errors = !compile_errors(&game, &parsed.content).is_empty();
        return Some(ImportPreview {
            name: parsed.name,
            content: parsed.content,
            presets: Vec::new(),
            constants: Vec::new(),
            is_full_data: false,
            has_errors,
        });
    }
    let parsed = transfer::parse_full_script_data(&raw)?;
    let has_errors = !compile_errors(&game, &parsed.content).is_empty();
    Some(ImportPreview {
        name: parsed.name,
        content: parsed.content,
        presets: parsed
            .presets
            .into_iter()
            .map(|(slot, name, studies)| (slot + 1, name, studies))
            .collect(),
        constants: parsed.constants,
        is_full_data: true,
        has_errors,
    })
}

#[tauri::command]
pub fn automator_import(
    raw: String,
    ignore_presets: bool,
    ignore_constants: bool,
    state: State<'_, Mutex<GameState>>,
) -> Option<u32> {
    state
        .lock()
        .unwrap()
        .automator_import(&raw, ignore_presets, ignore_constants)
}

/// The presets/constants a script references (the data-transfer page).
#[derive(Serialize)]
pub(crate) struct ScriptDataInfo {
    /// (1-based slot, name, studies).
    presets: Vec<(usize, String, String)>,
    constants: Vec<(String, String)>,
}

#[tauri::command]
pub fn automator_script_data_info(
    id: u32,
    state: State<'_, Mutex<GameState>>,
) -> ScriptDataInfo {
    let game = state.lock().unwrap();
    ScriptDataInfo {
        presets: game
            .automator_used_presets(id)
            .into_iter()
            .map(|slot| {
                let p = &game.study_presets[slot];
                (slot + 1, p.name.clone(), p.studies.clone())
            })
            .collect(),
        constants: game
            .automator_used_constants(id)
            .into_iter()
            .map(|name| {
                let value = game.automator.constants[&name].clone();
                (name, value)
            })
            .collect(),
    }
}

#[tauri::command]
pub fn study_preset_save(slot: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().save_study_preset(slot);
}

#[tauri::command]
pub fn study_preset_load(slot: usize, respec: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if respec {
        game.respec_and_load_study_preset(slot);
    } else {
        game.load_study_preset(slot);
    }
}

#[tauri::command]
pub fn study_preset_rename(
    slot: usize,
    name: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state.lock().unwrap().set_study_preset_name(slot, &name)
}

#[tauri::command]
pub fn study_preset_edit(
    slot: usize,
    studies: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state
        .lock()
        .unwrap()
        .set_study_preset_studies(slot, &studies)
}

/// The current tree as an import string (template modal "Current Tree").
#[tauri::command]
pub fn study_tree_export(state: State<'_, Mutex<GameState>>) -> String {
    state.lock().unwrap().study_tree_export_string()
}

/// `TimeStudyTree.isValidImportString` for template-input validation.
#[tauri::command]
pub fn study_tree_is_valid(text: String) -> bool {
    ad_core::time_studies::is_valid_study_import(&text)
}

#[tauri::command]
pub fn set_hotkeys(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.hotkeys = enabled;
}

#[tauri::command]
pub fn set_update_rate(rate: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_update_rate(rate);
}

/// Set which resource pair the Past Prestige Runs tables show
/// (`statTabResources`, clamped to 0–3).
#[tauri::command]
pub fn set_stat_tab_resources(value: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.stat_tab_resources = value.min(3);
}

/// Flip one Past Prestige Runs table's expand/collapse flag. `layer` is
/// "infinity" / "eternity" / "reality"; unknown names are ignored.
#[tauri::command]
pub fn toggle_shown_runs(layer: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match layer.as_str() {
        "infinity" => game.shown_runs.infinity = !game.shown_runs.infinity,
        "eternity" => game.shown_runs.eternity = !game.shown_runs.eternity,
        "reality" => game.shown_runs.reality = !game.shown_runs.reality,
        _ => {}
    }
}

#[tauri::command]
pub fn set_notation(notation: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_notation(&notation);
}

#[tauri::command]
pub fn set_offline_ticks(ticks: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_offline_ticks(ticks);
}

#[tauri::command]
pub fn set_notation_digits(
    comma: u32,
    notation: u32,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.options.set_notation_digits(comma, notation);
}

/// Flip a single per-action confirmation toggle (original
/// `player.options.confirmations.*`). `kind` is the camelCase action name
/// (`dimensionBoost`, `antimatterGalaxy`, `sacrifice`, `bigCrunch`); an unknown
/// name is ignored by the engine.
#[tauri::command]
pub fn set_confirmation(
    kind: String,
    enabled: bool,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.options.set_confirmation(&kind, enabled);
}

/// Flip a single animation toggle (original `player.options.animations.*`).
/// `kind` is the camelCase name (`bigCrunch`); an unknown name is ignored.
#[tauri::command]
pub fn set_animation(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_animation(&kind, enabled);
}

/// Flip a single info-display hint toggle (original
/// `player.options.showHintText.*`). `kind` is the camelCase name
/// (`showPercentage`, `achievements`, `achievementUnlockStates`, `challenges`).
#[tauri::command]
pub fn set_hint_text(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_hint_text(&kind, enabled);
}

/// Flip a single away-progress display toggle (original
/// `player.options.awayProgress.*`). `kind` is the camelCase resource name.
#[tauri::command]
pub fn set_away_progress(
    kind: String,
    enabled: bool,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.options.set_away_progress(&kind, enabled);
}

/// Toggles the relative prestige-gain text coloring (original
/// `headerTextColored`).
#[tauri::command]
pub fn set_header_text_colored(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.header_text_colored = enabled;
}

/// Toggles "Automatically retry challenges" (original `retryChallenge`): when on,
/// crunching inside an antimatter challenge re-enters it instead of exiting.
#[tauri::command]
pub fn set_retry_challenge(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.retry_challenge = enabled;
}

/// Sets the sidebar resource (original `sidebarResourceID`; 0 = latest).
#[tauri::command]
pub fn set_sidebar_resource(id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_sidebar_resource(id);
}

/// Toggles a top-level tab's hidden bit (original tab ids; the current-tab and
/// non-hidable guards live in the frontend, which knows the open tab).
#[tauri::command]
pub fn toggle_tab_visibility(tab_id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.toggle_tab_visibility(tab_id);
}

/// Clears a top-level tab's hidden bit (used when unhiding a tab whose subtabs
/// were all hidden).
#[tauri::command]
pub fn unhide_tab(tab_id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.unhide_tab(tab_id);
}

/// Toggles a subtab's hidden bit (original tab/subtab ids).
#[tauri::command]
pub fn toggle_subtab_visibility(
    tab_id: u32,
    subtab_id: u32,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.options.toggle_subtab_visibility(tab_id, subtab_id);
}

/// Unhides every tab and subtab (the Modify Visible Tabs modal's "Show all
/// tabs" button).
#[tauri::command]
pub fn show_all_tabs(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.show_all_tabs();
}

/// Sets the autosave cadence in milliseconds (original `autosaveInterval`); the
/// engine clamps to the 10–60 s slider range.
#[tauri::command]
pub fn set_autosave_interval(interval: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_autosave_interval(interval);
}

/// Toggles the header "time since save" indicator (original `showTimeSinceSave`).
#[tauri::command]
pub fn set_show_time_since_save(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.show_time_since_save = enabled;
}

/// Sets the custom save-file name (original `saveFileName`); the engine sanitizes
/// it (alphanumerics/space/hyphen, capped at 16 chars).
#[tauri::command]
pub fn set_save_file_name(name: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_save_file_name(&name);
}

/// Exports the current game state as an AD-compatible save string (for
/// clipboard copy). The frontend shows a toast on success.
#[tauri::command]
pub fn export_save(state: State<'_, Mutex<GameState>>) -> String {
    let game = state.lock().unwrap();
    encode_save(&game, now_ms())
}

/// Imports a save from a text string (pasted by the user). Decodes and
/// validates, then swaps the engine state. Returns the new `GameView` on
/// success or an error message on failure.
#[tauri::command]
pub fn import_save(
    text: String,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<LoadResult, String> {
    let now = now_ms();
    let (new_state, last_update) =
        decode_save_with_last_update(text.trim()).map_err(|e| e.to_string())?;
    let mut game = state.lock().unwrap();
    *game = new_state;
    // Mirror the original: an import fires an immediate save so it persists.
    let mut save = save.lock().unwrap();
    let _ = save.save_root(&game, now);
    Ok(LoadResult {
        view: build_game_view(&game),
        offline_ms: offline_gap_ms(last_update, now),
    })
}

/// The offline gap (ms) between a save's `lastUpdate` and now, clamped at 0.
/// `None`/future timestamps yield 0 (nothing to replay).
fn offline_gap_ms(last_update: Option<i64>, now_ms: i64) -> f64 {
    match last_update {
        Some(t) => (now_ms - t).max(0) as f64,
        None => 0.0,
    }
}

/// Exports the current save to a file chosen via a native "Save As" dialog. The
/// engine-owned custom save-file name (`options.saveFileName`) is used as the
/// default filename suggestion.
#[tauri::command]
pub async fn export_save_to_file(
    app: tauri::AppHandle,
    state: State<'_, Mutex<GameState>>,
) -> Result<(), String> {
    let (save_str, save_file_name) = {
        let game = state.lock().unwrap();
        (
            encode_save(&game, now_ms()),
            game.options.save_file_name.clone(),
        )
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
pub async fn import_save_from_file(
    app: tauri::AppHandle,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<LoadResult, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Text files", &["txt"])
        .blocking_pick_file();

    match path {
        Some(file_path) => {
            let now = now_ms();
            let contents = std::fs::read_to_string(file_path.as_path().unwrap())
                .map_err(|e| format!("Failed to read file: {e}"))?;
            let (new_state, last_update) = decode_save_with_last_update(contents.trim())
                .map_err(|e| e.to_string())?;
            let mut game = state.lock().unwrap();
            *game = new_state;
            let mut save = save.lock().unwrap();
            let _ = save.save_root(&game, now);
            Ok(LoadResult {
                view: build_game_view(&game),
                offline_ms: offline_gap_ms(last_update, now),
            })
        }
        None => Err("Cancelled".to_string()),
    }
}

/// Manually saves the current game state to disk (the "Save game" button, the
/// autosave loop, and the Ctrl/Cmd+S shortcut all route here).
#[tauri::command]
pub fn save_game(
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) {
    let game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    let _ = save.save_root(&game, now_ms());
}

/// Switches the active save slot, persisting the current slot first and loading
/// the target (or a fresh game for an empty slot). Returns the new view.
#[tauri::command]
pub fn switch_save_slot(
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
pub fn get_save_slots(
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
pub fn trigger_backup(
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
pub fn get_backups(save: State<'_, Mutex<SaveManager>>) -> Vec<BackupMetaView> {
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
pub fn load_backup(
    slot: u8,
    state: State<'_, Mutex<GameState>>,
    save: State<'_, Mutex<SaveManager>>,
) -> Result<LoadResult, String> {
    let now = now_ms();
    let mut game = state.lock().unwrap();
    let mut save = save.lock().unwrap();
    let (loaded, last_update) = save.load_backup(slot, &game.clone(), now)?;
    *game = loaded;
    Ok(LoadResult {
        view: build_game_view(&game),
        offline_ms: offline_gap_ms(last_update, now),
    })
}

/// Exports every populated backup of the current save slot as a single
/// backup-bundle file via a native "Save As" dialog (§2.4).
#[tauri::command]
pub async fn export_backups_to_file(
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
pub async fn import_backups_from_file(
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

/// Constructs a fresh game for the running app.
///
/// In debug builds this applies developer conveniences (skip the tutorial,
/// disable confirmation modals) to speed up iteration. These are gated behind
/// `#[cfg(debug_assertions)]`, so any release build (`--release`, packaged
/// bundles) is compiled without them and always starts from the production
/// defaults in `GameState::new()`. There is nothing to remember to revert.
pub fn fresh_game() -> GameState {
    #[allow(unused_mut)]
    let mut game = GameState::new();
    // Stamp the save-creation wall-clock time (the engine avoids wall clocks,
    // so `GameState::new()` leaves it 0). Shown on the Statistics tab.
    game.records.game_created_time_ms = now_ms() as f64;
    #[cfg(debug_assertions)]
    {
        game.tutorial_state = 5;
        game.tutorial_active = false;
        game.options.confirmations.dimension_boost = false;
        game.options.confirmations.antimatter_galaxy = false;
        game.options.confirmations.sacrifice = false;
        game.options.confirmations.big_crunch = true;
    }
    game
}
