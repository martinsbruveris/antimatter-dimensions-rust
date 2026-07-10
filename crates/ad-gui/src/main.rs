mod persistence;

use std::path::PathBuf;
use std::sync::Mutex;

use ad_core::autobuyers::{AutoRealityMode, PrestigeAutobuyerMode};
use ad_core::data::constants::{
    BIG_CRUNCH_THRESHOLD, BUY_TEN_MULTIPLIER, DIM_BOOST_MULTIPLIER,
};
use ad_core::save::{decode_save_with_last_update, encode_save};
use ad_core::{
    offline_plan as core_offline_plan, AutobuyerMode, AutobuyerTarget,
    BreakInfinityRebuyable, BreakInfinityUpgrade, Decimal, GameState, InfinityUpgrade,
    ALL_BREAK_INFINITY_REBUYABLES, ALL_BREAK_INFINITY_UPGRADES, ALL_INFINITY_UPGRADES,
    INFINITY_CHALLENGE_COUNT, INFINITY_DIMENSION_COUNT, NORMAL_CHALLENGE_COUNT,
    REPLICANTI_UNLOCK_COST,
};
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
/// `docs/design/2026-06-25-number-formatting.md` (Option C). The exponent is the
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
    /// Whether it runs at all: antimatter-unlocked (`is_bought`) or challenge-
    /// unlocked (`can_be_upgraded`). Drives the row-vs-locked display for the
    /// prestige autobuyers (which have no antimatter path).
    is_unlocked: bool,
    /// Whether this autobuyer has an antimatter "slow version" (AD/Tickspeed);
    /// the prestige autobuyers (Dim Boost / Galaxy / Big Crunch) do not.
    antimatter_unlockable: bool,
    /// The Normal Challenge whose completion unlocks/upgrades this autobuyer
    /// (shown in the locked hint, e.g. "Complete Normal Challenge 10").
    unlock_challenge: u8,
    /// Whether the antimatter requirement is met (purchase box enabled).
    can_unlock: bool,
    /// Requirement amount shown on the purchase box.
    requirement: Num,
    /// Interval between purchases, formatted in seconds (e.g. "0.50").
    interval_seconds: String,
    /// Whether the interval can be reduced with IP (its challenge is completed).
    can_be_upgraded: bool,
    /// Whether the interval is already at the 100 ms floor.
    has_maxed_interval: bool,
    /// IP cost of the next interval upgrade.
    upgrade_cost: Num,
    /// Whether that upgrade is affordable right now (upgradeable, not maxed, and
    /// enough IP).
    can_afford_upgrade: bool,
    /// Whether the autobuyer is toggled on.
    is_active: bool,
    /// Current purchase mode: "single" or "max".
    mode: String,
    /// Whether the mode can be changed. Pre-Infinity the tickspeed autobuyer is
    /// locked to single (the toggle needs a completed challenge).
    can_change_mode: bool,
    /// "Buys max" bulk multiplier (AD autobuyers only; 1 elsewhere).
    bulk: u32,
    /// Whether the bulk is at its 512 cap (`hasMaxedBulk`).
    has_maxed_bulk: bool,
    /// Whether Achievement 61 lifted the bulk to unlimited (`hasUnlimitedBulk`).
    has_unlimited_bulk: bool,
    /// Whether the next bulk doubling is affordable (interval maxed, bulk not).
    can_afford_bulk_upgrade: bool,
}

/// Serializable view of a single Infinity Upgrade for the grid.
#[derive(Serialize)]
struct InfinityUpgradeView {
    /// Original save id (e.g. "timeMult"); the frontend keys its description/layout
    /// table on this and passes it back to `buy_infinity_upgrade`.
    id: String,
    /// Whether it is owned (lit tile).
    is_bought: bool,
    /// Whether it can be bought now (column prerequisite + enough IP).
    can_be_bought: bool,
    /// IP cost.
    cost: Num,
    /// Numeric effect value (for the tiles whose effect shows a `×value`); the
    /// frontend ignores it for constant/description-only tiles.
    effect: Num,
}

/// Serializable view of the Infinity Upgrades bottom row — the `ipMult`
/// rebuyable and the one-time `ipOffline`, both unlocked by Achievement 41.
#[derive(Serialize)]
struct InfinityUpgradesBottomRowView {
    /// Whether the row is shown at all (Achievement 41 unlocked).
    unlocked: bool,
    /// `ipMult` purchases so far.
    ip_mult_purchases: u32,
    /// Next `ipMult` cost.
    ip_mult_cost: Num,
    /// Current `ipMult` effect (`2^purchases`).
    ip_mult_effect: Num,
    /// Whether `ipMult` hit its 1e6M cost cap (displays as bought).
    ip_mult_capped: bool,
    /// Whether the next `ipMult` purchase can happen now.
    ip_mult_can_be_bought: bool,
    /// Whether `ipOffline` is owned.
    ip_offline_bought: bool,
    /// Whether `ipOffline` can be bought now.
    ip_offline_can_be_bought: bool,
    /// `ipOffline`'s 1000-IP cost.
    ip_offline_cost: Num,
    /// `ipOffline`'s displayed effect (IP per offline minute).
    ip_offline_effect_per_min: Num,
    /// The IP-mult autobuyer (1-Eternity milestone): unlocked + active flags.
    autobuyer_unlocked: bool,
    autobuyer_active: bool,
}

/// Serializable view of a single Normal Challenge tile.
#[derive(Serialize)]
struct ChallengeView {
    /// Challenge id (1..=12).
    id: u8,
    /// Whether it is unlocked (tab open + enough Infinities).
    is_unlocked: bool,
    /// Whether it is the challenge currently running.
    is_running: bool,
    /// Whether it has been completed.
    is_completed: bool,
}

/// Serializable view of a single Infinity Challenge tile.
#[derive(Serialize)]
struct InfinityChallengeView {
    /// Challenge id (1..=8).
    id: u8,
    /// Whether it is unlocked (peak antimatter this eternity ≥ its unlockAM).
    is_unlocked: bool,
    /// Whether it is the challenge currently running.
    is_running: bool,
    /// Whether it has been completed.
    is_completed: bool,
}

/// Serializable view of the Infinity Dimensions tab.
#[derive(Serialize)]
struct InfinityDimensionsView {
    /// Whether the tab is available (Infinity broken, or ID1 already unlocked).
    unlocked: bool,
    /// Current Infinity Power.
    power: Num,
    /// The `^7` multiplier Infinity Power gives to all Antimatter Dimensions.
    power_mult: Num,
    /// The 8 tiers (index 0 = 1st Infinity Dimension).
    dimensions: Vec<InfinityDimensionView>,
}

/// Serializable view of one Infinity Dimension row.
#[derive(Serialize)]
struct InfinityDimensionView {
    /// 0-indexed tier.
    tier: usize,
    amount: Num,
    multiplier: Num,
    cost: Num,
    is_unlocked: bool,
    /// Whether a purchase is possible now (unlocked, affordable, not capped).
    can_be_bought: bool,
    /// Whether it can be unlocked now (still locked, requirements met).
    can_unlock: bool,
    /// Whether it is at its purchase cap.
    is_capped: bool,
}

/// Serializable view of the Break Infinity tab (12 upgrades).
#[derive(Serialize)]
struct BreakInfinityView {
    /// Whether Infinity is broken (the tab/upgrades are active).
    unlocked: bool,
    /// The 9 one-time upgrades, in enum order.
    upgrades: Vec<BreakUpgradeView>,
    /// The 3 rebuyable upgrades, in index order.
    rebuyables: Vec<BreakRebuyableView>,
}

/// A single one-time Break Infinity Upgrade.
#[derive(Serialize)]
struct BreakUpgradeView {
    /// Original save id (the frontend keys its description on this).
    id: String,
    cost: Num,
    is_bought: bool,
    can_be_bought: bool,
}

/// A single rebuyable Break Infinity Upgrade.
#[derive(Serialize)]
struct BreakRebuyableView {
    /// Index 0/1/2 (tickspeedCostMult / dimCostMult / ipGen).
    id: usize,
    cost: Num,
    count: u32,
    max: u32,
    can_be_bought: bool,
}

/// Serializable view of the Replicanti tab.
#[derive(Serialize)]
struct ReplicantiView {
    /// Whether Replicanti are unlocked (else the tab shows the unlock button).
    unlocked: bool,
    /// IP cost of the unlock (1e140), and whether it's affordable.
    unlock_cost: Num,
    can_unlock: bool,
    /// Current amount and its `×` multiplier to all Infinity Dimensions.
    amount: Num,
    mult: Num,
    /// Reproduction chance (a fraction 0…1) and interval (ms).
    chance: f64,
    interval_ms: f64,
    /// Chance upgrade (`+1%`): next cost, capped at 100%, affordability.
    chance_cost: Num,
    chance_capped: bool,
    can_buy_chance: bool,
    /// Interval upgrade (`×0.9`, floor 50 ms).
    interval_cost: Num,
    interval_capped: bool,
    can_buy_interval: bool,
    /// Max-galaxies upgrade (`+1`): value, next cost, affordability.
    galaxy_cap: u32,
    galaxy_cost: Num,
    can_buy_galaxy_cap: bool,
    /// Replicanti Galaxies made, and whether one can be bought now (amount at cap,
    /// below the cap). The galaxy button shows once the cap is ≥ 1.
    galaxies: u32,
    can_buy_galaxy: bool,
    can_see_galaxy_button: bool,
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
    /// The Dimension Boost autobuyer (unlocked by completing NC10).
    dim_boost: AutobuyerView,
    /// The Antimatter Galaxy autobuyer (unlocked by completing NC11).
    galaxy: AutobuyerView,
    /// The Big Crunch (Infinity) autobuyer (unlocked by completing NC12).
    big_crunch: AutobuyerView,
    /// The Big Crunch autobuyer's post-break goal settings.
    big_crunch_settings: PrestigeGoalView,
    /// The Eternity autobuyer (100-Eternities milestone).
    eternity: EternityAutobuyerView,
    /// The Reality autobuyer (Reality Upgrade 25).
    reality: RealityAutobuyerView,
    /// The 8 Infinity Dimension autobuyers (milestones 11–18 Eternities).
    infinity_dims: MilestoneAutobuyerGroupView,
    /// The 3 Replicanti-upgrade autobuyers (milestones 50/60/80).
    replicanti_upgrades: MilestoneAutobuyerGroupView,
    /// The Replicanti Galaxy autobuyer (milestone 3): unlocked + active.
    replicanti_galaxy_unlocked: bool,
    replicanti_galaxy_active: bool,
    /// The 8 Time Dimension autobuyers (Reality Upgrade 13).
    time_dims: MilestoneAutobuyerGroupView,
    /// The EP-multiplier autobuyer (Reality Upgrade 13): unlocked + active.
    ep_mult_unlocked: bool,
    ep_mult_active: bool,
}

/// A milestone-autobuyer group (ID / Replicanti-upgrade): the group toggle plus
/// each entry's name, unlock state, and active flag.
#[derive(Serialize)]
struct MilestoneAutobuyerGroupView {
    /// Whether any entry is unlocked (the box renders at all).
    any_unlocked: bool,
    /// The group `isActive` toggle.
    group_active: bool,
    entries: Vec<MilestoneAutobuyerEntryView>,
}

#[derive(Serialize)]
struct MilestoneAutobuyerEntryView {
    /// Short display name (e.g. "1st" or "Chance").
    name: String,
    is_unlocked: bool,
    is_active: bool,
}

/// The goal settings of the Big Crunch / Eternity autobuyers (mode dropdown +
/// value input + "Dynamic amount" checkbox).
#[derive(Serialize)]
struct PrestigeGoalView {
    /// Current mode: "amount" / "time" / "xHighest".
    mode: String,
    /// Whether the Time / X-highest modes are available (`hasAdditionalModes`:
    /// the `bigCrunchModes` milestone, resp. Reality Upgrade 13).
    has_modes: bool,
    amount: Num,
    time: f64,
    x_highest: Num,
    increase_with_mult: bool,
}

/// The Eternity autobuyer's row.
#[derive(Serialize)]
struct EternityAutobuyerView {
    is_unlocked: bool,
    is_active: bool,
    settings: PrestigeGoalView,
}

/// The Reality autobuyer's row.
#[derive(Serialize)]
struct RealityAutobuyerView {
    is_unlocked: bool,
    is_active: bool,
    /// "rm" / "glyph" / "either" / "both" / "time".
    mode: String,
    rm: Num,
    glyph: u32,
    time: f64,
}

/// One row of a Past Prestige Runs table: `[time, realTime, currency, count]`
/// (the engine's recent-run rings; the original tuples' trailing entries —
/// challenge name, TT/glyph extras — are not modelled).
#[derive(Serialize)]
struct RecentRunView {
    time_ms: f64,
    real_time_ms: f64,
    currency: Num,
    count: Num,
}

/// The Past Prestige Runs expand/collapse flags (`player.shownRuns`).
#[derive(Serialize)]
struct ShownRunsView {
    infinity: bool,
    eternity: bool,
    reality: bool,
}

/// The Statistics tab (main page + Challenge records + Past Prestige Runs).
/// Read-only records, shipped display-ready; section visibility comes from the
/// top-level `infinity_unlocked` / `eternity_unlocked` / `reality.unlocked`.
#[derive(Serialize)]
struct StatisticsView {
    total_antimatter: Num,
    real_time_played_ms: f64,
    /// Game time; its line is shown once Reality is unlocked.
    total_time_played_ms: f64,
    /// Wall-clock save-creation timestamp (0 = unknown; line hidden).
    game_created_time_ms: f64,
    // Infinity block.
    infinities: Num,
    infinities_banked: Num,
    /// `999999999999` sentinel = "no fastest Infinity yet".
    best_infinity_time_ms: f64,
    this_infinity_time_ms: f64,
    this_infinity_real_time_ms: f64,
    /// Best IP/min this Eternity (`bestInfinity.bestIPminEternity`).
    best_ip_min: Num,
    // Eternity block.
    eternities: Num,
    /// Banked Infinities an Eternity would grant now (Ach 131 + TS191).
    projected_banked: Num,
    /// `projected_banked / clampMin(33, thisEternity time) × 60000`
    /// (real time once Reality is unlocked, like the original).
    banked_rate_per_min: Num,
    best_eternity_time_ms: f64,
    this_eternity_time_ms: f64,
    this_eternity_real_time_ms: f64,
    /// Best EP/min this Reality (`bestEternity.bestEPminReality`).
    best_ep_min: Num,
    // Reality block.
    realities: u32,
    best_reality_time_ms: f64,
    best_reality_real_time_ms: f64,
    this_reality_time_ms: f64,
    this_reality_real_time_ms: f64,
    /// Best RM/min (`bestReality.RMmin`).
    best_rm_min: Num,
    /// Best Glyph rarity in percent (`strengthToRarity`, clamped ≥ 0).
    best_glyph_rarity: f64,
    is_doomed: bool,
    // Challenge records.
    nc_best_times_ms: Vec<f64>,
    ic_best_times_ms: Vec<f64>,
    // Past Prestige Runs (each 10 entries, newest first).
    recent_infinities: Vec<RecentRunView>,
    recent_eternities: Vec<RecentRunView>,
    recent_realities: Vec<RecentRunView>,
    shown_runs: ShownRunsView,
}

/// Serializable game view sent to the frontend each frame.
#[derive(Serialize)]
struct GameView {
    antimatter: Num,
    antimatter_per_sec: Num,
    /// Current Infinity Points (shown in the Infinity tab header once unlocked).
    /// `infinities` and the per-crunch IP gain live in the engine snapshot
    /// (`ObservedState`) but aren't surfaced here yet — they gain consumers with
    /// the Statistics tab and the post-break crunch modal (Feature 2.3).
    infinity_points: Num,
    /// Number of Infinities performed (shown on locked challenge tiles as X/Y).
    infinities: Num,
    /// IP a Big Crunch would grant right now (the post-break header button).
    gained_infinity_points: Num,
    /// The current antimatter crunch goal (an IC's own goal while one runs,
    /// else 1.8e308), for the header button's "Reach X antimatter" line.
    infinity_goal: Num,
    /// Real time in the current infinity (ms), for the button's IP/min line.
    this_infinity_real_time_ms: f64,
    /// Peak IP/min this infinity + the gain when that peak was set.
    best_ip_min: Num,
    best_ip_min_val: Num,
    /// Current Eternity Points (header readout once Eternity is unlocked).
    eternity_points: Num,
    /// Number of Eternities performed (drives milestones).
    eternities: Num,
    /// Whether the player has performed at least one Eternity.
    eternity_unlocked: bool,
    /// Whether the Eternity goal (1.8e308 peak IP) is met.
    can_eternity: bool,
    /// The IP goal for an Eternity (the "Reach X IP" line).
    eternity_goal: Num,
    /// EP an Eternity would grant right now.
    gained_eternity_points: Num,
    /// Real time in the current eternity (ms), for the button's EP/min line.
    this_eternity_real_time_ms: f64,
    /// Peak EP/min this eternity + the gain when that peak was set.
    best_ep_min: Num,
    best_ep_min_val: Num,
    /// The Eternity Milestones (threshold order), for the Milestones subtab.
    eternity_milestones: Vec<EternityMilestoneView>,
    /// Time Dimensions tab state (8 tiers + Time Shards / free tickspeed).
    time_dimensions: TimeDimensionsView,
    /// Time Studies tab state (TT + the study tree).
    time_studies: TimeStudiesView,
    /// The 12 Eternity Challenges (tab tiles + tree EC nodes).
    eternity_challenges: Vec<EternityChallengeView>,
    /// Eternity Upgrades tab state (6 upgrades + the EP multiplier).
    eternity_upgrades: EternityUpgradesView,
    /// Time Dilation tab state.
    dilation: DilationView,
    /// The Reality layer (header button, Glyphs tab, Reality modal).
    reality: RealityView,
    /// Whether the EC subtab is available (a study held or any completion).
    eternity_challenges_unlocked: bool,
    /// The 16 Infinity Upgrades (grid order), for the Infinity Upgrades tab.
    infinity_upgrades: Vec<InfinityUpgradeView>,
    /// The Achievement-41 bottom row (`ipMult` rebuyable + `ipOffline`).
    infinity_upgrades_bottom_row: InfinityUpgradesBottomRowView,
    /// Whether the Challenges tab is available (post first Infinity).
    challenges_unlocked: bool,
    /// The 12 Normal Challenges, for the Challenges tab.
    challenges: Vec<ChallengeView>,
    /// Whether any Infinity Challenge is unlocked (the IC subtab gate).
    infinity_challenges_unlocked: bool,
    /// The 8 Infinity Challenges, for the Challenges tab.
    infinity_challenges: Vec<InfinityChallengeView>,
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
    /// Whether Infinity has been broken (`player.break`): antimatter may exceed
    /// `1e308` and the IP formula scales.
    broke_infinity: bool,
    /// Fastest infinity by game time (ms); `f64::MAX` before the first crunch.
    /// Drives the original's post-crunch "early game" tab change (> 60 s).
    best_infinity_time_ms: f64,
    /// Whether the Break Infinity button should be offered (the Big Crunch
    /// autobuyer's interval is at the 100 ms floor).
    break_infinity_unlockable: bool,
    /// Whether the `1e308` Big-Crunch goal is still in force (pre-break, or inside
    /// a challenge). When false the full-screen crunch view is suppressed so
    /// post-break play continues in the normal tab view.
    has_big_crunch_goal: bool,
    /// Autobuyer tab state (unlock progress, per-autobuyer status).
    autobuyers: AutobuyersView,
    /// Break Infinity tab state (the 12 upgrades).
    break_infinity: BreakInfinityView,
    /// Infinity Dimensions tab state (8 tiers + Infinity Power).
    infinity_dimensions: InfinityDimensionsView,
    /// Replicanti tab state (unlock, amount/mult, 3 upgrades, galaxies).
    replicanti: ReplicantiView,
    /// Player options (UI/UX preferences), surfaced for the options tabs.
    options: OptionsView,
    /// Sorted ids of unlocked normal achievements; drives the Achievements tab
    /// and the unlock-toast diff in the frontend.
    unlocked_achievements: Vec<u16>,
    /// Global achievement-power multiplier (shown as the tab's boost).
    achievement_power: Num,
    /// Tabs/subtabs currently showing the yellow `!` badge, as concatenated
    /// `tabKey + subtabKey` strings; the sidebar matches them against its
    /// entries. Cleared per key via the `tab_notification_seen` command.
    tab_notifications: Vec<String>,
    /// Current tutorial-highlight step (`player.tutorialState`); components
    /// compare against their own step id to decide whether to glow.
    tutorial_state: u8,
    /// Whether the current tutorial step's highlight is active
    /// (`player.tutorialActive`).
    tutorial_active: bool,
    /// Automator tab state (Feature 6.6 Stage D).
    automator: AutomatorTabView,
    /// Celestials tab state (Phase 7).
    celestials: CelestialsView,
    /// Statistics tab state (records + challenge times + recent runs).
    statistics: StatisticsView,
}

/// Serializable view of the Automator tab.
#[derive(Serialize)]
struct AutomatorTabView {
    /// 100 AP reached (or force-unlocked).
    unlocked: bool,
    /// The stack is non-empty (a paused Automator is still on).
    is_on: bool,
    is_running: bool,
    /// "pause" / "run" / "singleStep".
    mode: String,
    /// 1-based line of the current command (0 when off).
    current_line: u32,
    /// The running (or last-run) script id and the editor's script id.
    top_level_script: u32,
    editor_script: u32,
    repeat: bool,
    force_restart: bool,
    follow_execution: bool,
    /// The previous run finished on its own (`hasJustCompleted`).
    just_completed: bool,
    /// Milliseconds per command.
    interval_ms: f64,
    /// All scripts (id order) for the dropdown.
    scripts: Vec<AutomatorScriptEntry>,
    running_script_name: String,
    /// Stored character counts (the live editor buffer refines the current
    /// count frontend-side between saves).
    current_script_chars: usize,
    total_script_chars: usize,
    /// Constants in display order, for the define panel.
    constants: Vec<AutomatorConstantView>,
    /// Which docs pane is open (`currentInfoPane`, 0–7).
    current_info_pane: u8,
    /// The editor flavor: "text" or "block".
    editor_type: String,
    /// `notify` toasts queued since the last tick (drained by the frontend).
    notifications: Vec<String>,
    /// Event-log length (the log itself is fetched on demand).
    event_count: usize,
    /// Event-log display options (`options.automatorEvents`).
    event_options: AutomatorEventOptionsView,
    /// AP progress breakdown; only populated while locked (the points page).
    points: Option<AutomatorPointsView>,
}

#[derive(Serialize)]
struct AutomatorScriptEntry {
    id: u32,
    name: String,
}

#[derive(Serialize)]
struct AutomatorConstantView {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct AutomatorEventOptionsView {
    newest_first: bool,
    timestamp_type: u8,
    max_entries: u32,
    clear_on_reality: bool,
    clear_on_restart: bool,
}

/// The locked-tab AP page (`AutomatorPointsList`): totals plus each source.
/// Perk/upgrade display text lives frontend-side, keyed by id.
#[derive(Serialize)]
struct AutomatorPointsView {
    total: u32,
    threshold: u32,
    from_perks: u32,
    from_upgrades: u32,
    perks: Vec<ApSourceView>,
    upgrades: Vec<ApSourceView>,
    /// "Reality Count" and "Black Hole" (`otherAutomatorPoints`).
    other: Vec<ApOtherSourceView>,
}

#[derive(Serialize)]
struct ApSourceView {
    id: u8,
    ap: u32,
    bought: bool,
}

#[derive(Serialize)]
struct ApOtherSourceView {
    name: &'static str,
    ap: u32,
}

/// One event-log entry, shipped by `get_automator_events`.
#[derive(Serialize)]
struct AutomatorEventView {
    message: String,
    line: u32,
    this_reality_ms: f64,
    play_time_ms: f64,
    timegap_ms: f64,
}

/// A compile error for the editor gutter / error panel.
#[derive(Serialize)]
struct AutomatorErrorView {
    line: u32,
    info: String,
    tip: String,
}

/// Serializable view of the Time Dimensions tab.
#[derive(Serialize)]
struct TimeDimensionsView {
    /// Current Time Shards.
    time_shards: Num,
    /// Free Tickspeed upgrades gained from shards.
    total_tick_gained: u64,
    /// Shard total the next free Tickspeed upgrade needs.
    next_shards: Num,
    /// Shard-requirement multiplier per upgrade (1.33; 1.25 with TS171).
    mult_to_next: f64,
    /// Upgrade count where requirements start growing faster (300000).
    softcap: f64,
    /// TD1 production (the "you are getting X Time Shards per second" line).
    shards_per_second: Num,
    /// The 8 tiers (index 0 = 1st Time Dimension).
    dimensions: Vec<TimeDimensionView>,
}

/// Serializable view of one Time Dimension row.
#[derive(Serialize)]
struct TimeDimensionView {
    /// 0-indexed tier.
    tier: usize,
    amount: Num,
    multiplier: Num,
    bought: u64,
    cost: Num,
    /// Tiers 1–4 pre-dilation; 5–8 stay locked until Phase 5.
    is_unlocked: bool,
    /// Unlocked + affordable (buy button enabled).
    available_for_purchase: bool,
    /// Per-second growth (+X%/s) from the tier above.
    rate_percent: f64,
}

/// Serializable view of the Time Studies tab.
#[derive(Serialize)]
struct TimeStudiesView {
    /// Unspent Time Theorems.
    theorems: Num,
    /// Whether TT can be bought at all (a 1st Time Dimension is owned).
    can_buy_tt: bool,
    /// Next TT cost per currency + affordability.
    am_cost: Num,
    ip_cost: Num,
    ep_cost: Num,
    can_afford_am: bool,
    can_afford_ip: bool,
    can_afford_ep: bool,
    /// Whether the next Eternity respecs the tree.
    respec: bool,
    /// Per-study state, in catalogue order.
    studies: Vec<TimeStudyView>,
    /// The EC whose unlock study is held (0 = none; Feature 4.5).
    ec_unlocked: u8,
    /// The six study presets (save/load buttons above the TT shop).
    presets: Vec<StudyPresetView>,
    /// Whether an Eternity is available (enables "Respec and Load").
    can_eternity: bool,
}

/// One study preset slot.
#[derive(Serialize)]
struct StudyPresetView {
    name: String,
    studies: String,
}

/// Serializable view of one time study node.
#[derive(Serialize)]
struct TimeStudyView {
    id: u16,
    cost: f64,
    is_bought: bool,
    can_buy: bool,
}

/// Serializable view of the Time Dilation tab.
#[derive(Serialize)]
struct DilationView {
    /// Whether Dilation is unlocked (study 1).
    unlocked: bool,
    /// Whether a dilated Eternity is running.
    active: bool,
    tachyon_particles: Num,
    dilated_time: Num,
    /// DT gained per second.
    dt_per_second: Num,
    /// DT needed for the next Tachyon Galaxy.
    next_threshold: Num,
    base_tachyon_galaxies: u32,
    total_tachyon_galaxies: f64,
    /// TGs granted per threshold (2 with doubleGalaxies below 500 base).
    tachyon_galaxy_gain: u32,
    /// TP an exit right now would leave the player with vs. gain.
    tachyon_gain: Num,
    /// The 3 rebuyables + 7 one-time upgrades, plus the Pelle-only 11–15.
    upgrades: Vec<DilationUpgradeView>,
    /// Whether the Pelle-only upgrades (11–15) are shown (Doomed + the Paradox
    /// rift's first milestone).
    pelle_upgrades_unlocked: bool,
    /// The 5 dilation studies (tree nodes): per-study state.
    studies: Vec<DilationStudyView>,
}

/// Serializable view of one Dilation Upgrade tile.
#[derive(Serialize)]
struct DilationUpgradeView {
    /// Original id (1–15; 1–3 and 11–13 rebuyable).
    id: u8,
    cost: Num,
    is_rebuyable: bool,
    /// Rebuyable purchase count.
    count: u32,
    is_bought: bool,
    is_capped: bool,
    can_buy: bool,
}

/// Serializable view of one dilation study (ids 1–5).
#[derive(Serialize)]
struct DilationStudyView {
    id: u8,
    cost: f64,
    is_bought: bool,
    can_buy: bool,
}

/// Serializable view of one glyph (equipped, inventory, or a Reality choice).
#[derive(Serialize)]
struct GlyphView {
    id: u32,
    idx: u32,
    /// Type id ("power" … "companion").
    kind: String,
    strength: f64,
    level: u32,
    /// Effect bitmask (generated bits 0–19; companion bits 8/9).
    effects: u32,
    /// Per-effect values for the tooltip, in display order.
    effect_values: Vec<GlyphEffectValueView>,
    /// Sacrifice value (0 before RU19 / for the companion).
    sacrifice_value: f64,
}

/// One effect entry on a glyph (bit + this glyph's value).
#[derive(Serialize)]
struct GlyphEffectValueView {
    bit: u8,
    value: Num,
}

/// One *combined* active glyph effect (the Current Glyph Effects panel).
#[derive(Serialize)]
struct ActiveGlyphEffectView {
    bit: u8,
    value: Num,
    /// Whether a softcap reduced the combined value.
    capped: bool,
}

/// Serializable view of the Reality layer (header button + Glyphs tab).
#[derive(Serialize)]
struct RealityView {
    /// Whether the player has ever realitied.
    unlocked: bool,
    /// Whether the Reality study (dilation study 6) is bought.
    has_reality_study: bool,
    /// `isRealityAvailable()`.
    is_available: bool,
    realities: u32,
    machines: Num,
    perk_points: f64,
    /// RM a Reality would grant right now.
    gained_rm: Num,
    /// The glyph level a Reality would grant + its unfloored value.
    glyph_level: u32,
    glyph_level_exact: f64,
    /// EP needed for the next whole RM (the button's "Next at X EP").
    next_machine_ep: Num,
    /// Real time in this reality (minutes), for the RM/min readout.
    reality_time_minutes: f64,
    /// Best glyph level ever attained (modal comparison).
    best_glyph_level: u32,
    /// Whether the next Reality unequips glyphs.
    respec: bool,
    /// Glyph choices offered (1 pre-START, 4 after).
    choice_count: usize,
    /// The upcoming glyph choice(s) (RNG-stable preview).
    upcoming_glyphs: Vec<GlyphView>,
    /// Equipped glyphs + slot count.
    active_glyphs: Vec<GlyphView>,
    active_slot_count: usize,
    /// Inventory glyphs (sparse; `idx` = slot in the 120-slot grid).
    inventory_glyphs: Vec<GlyphView>,
    protected_rows: u32,
    free_inventory_space: u32,
    /// Whether glyph sacrifice is unlocked (RU19).
    can_sacrifice: bool,
    /// Sacrifice totals per basic type (BASIC_GLYPH_TYPES order).
    sac_totals: Vec<f64>,
    /// The five sacrifice-effect values (power/infinity/time/replication/
    /// dilation order; power+replication are integer delays).
    sac_effects: Vec<f64>,
    /// Combined active glyph effects, in the panel's display order.
    active_effects: Vec<ActiveGlyphEffectView>,
    /// The glyph filter (Effarig unlock): unlocked flag + settings.
    filter_unlocked: bool,
    filter: GlyphFilterView,
    /// Glyph undo (Teresa unlock): availability + stack depth.
    undo_unlocked: bool,
    can_undo: bool,
    undo_depth: usize,
    /// Reality-glyph creation (the reality Alchemy resource).
    can_create_reality_glyph: bool,
    reality_glyph_level: u32,
    /// Effarig's glyph-level weight adjuster: unlocked + the 4 weights
    /// (ep/repl/dt/eternities, summing 100).
    weights_unlocked: bool,
    glyph_weights: Vec<f64>,
    /// Bought perk ids + the currently purchasable ones (Feature 6.3).
    perks_bought: Vec<u8>,
    perks_buyable: Vec<u8>,
    /// The 5 rebuyable + 20 one-time Reality Upgrades (Feature 6.4).
    rebuyables: Vec<RealityRebuyableView>,
    upgrades: Vec<RealityUpgradeView>,
    /// The Black Holes (Feature 6.5).
    black_holes: BlackHolesView,
}

/// The auto-glyph filter settings (select/trash/simple + per-type configs,
/// keyed by type name).
#[derive(Serialize)]
struct GlyphFilterView {
    select: u8,
    trash: u8,
    simple: u32,
    types: Vec<GlyphFilterTypeView>,
}

#[derive(Serialize)]
struct GlyphFilterTypeView {
    /// Type name ("time" … "effarig").
    kind: String,
    rarity: f64,
    score: f64,
    effect_count: u32,
    specified_mask: u32,
    effect_scores: Vec<f64>,
    /// The type's lowest effect bit (for labelling the score/mask entries).
    bit_offset: u8,
}

#[derive(Serialize)]
struct RealityRebuyableView {
    id: u8,
    count: u32,
    cost: Num,
    can_buy: bool,
}

#[derive(Serialize)]
struct RealityUpgradeView {
    id: u8,
    cost: Num,
    is_bought: bool,
    /// Whether the unlock requirement has been met (`upgReqs`).
    req_met: bool,
    can_buy: bool,
}

#[derive(Serialize)]
struct BlackHolesView {
    unlocked: bool,
    can_unlock: bool,
    paused: bool,
    /// Inversion: slider availability, current strength, active state.
    negative_unlocked: bool,
    negative: f64,
    is_inverted: bool,
    /// Auto-pause mode (0 never / 1 before BH1 / 2 before BH2).
    auto_pause_mode: u8,
    holes: Vec<BlackHoleView>,
}

#[derive(Serialize)]
struct BlackHoleView {
    unlocked: bool,
    /// Effectively active right now (charge + lower holes + pause).
    is_active: bool,
    /// This hole's own charged flag.
    charged: bool,
    is_permanent: bool,
    phase: f64,
    interval: f64,
    power: f64,
    duration: f64,
    activations: u32,
    /// Upgrade costs: [interval, power, duration] + affordability.
    upgrade_costs: Vec<Num>,
    can_buy_upgrades: Vec<bool>,
}

/// Serializable view of the Eternity Upgrades tab.
#[derive(Serialize)]
struct EternityUpgradesView {
    /// The six one-time upgrades (grid order).
    upgrades: Vec<EternityUpgradeView>,
    /// The rebuyable ×5 EP multiplier.
    ep_mult_purchases: u32,
    ep_mult_cost: Num,
    ep_mult_effect: Num,
    can_buy_ep_mult: bool,
}

/// Serializable view of one Eternity Upgrade tile.
#[derive(Serialize)]
struct EternityUpgradeView {
    /// Original id (1–6); the frontend keys descriptions on it.
    id: u8,
    cost: Num,
    is_bought: bool,
    can_buy: bool,
    /// Current effect value (a multiplier).
    effect: Num,
}

/// Serializable view of one Eternity Challenge (tiles + tree nodes).
#[derive(Serialize)]
struct EternityChallengeView {
    /// Challenge id (1..=12).
    id: u8,
    /// Completions so far (0..=5).
    completions: u8,
    /// Whether this EC is currently running.
    is_running: bool,
    /// Whether its unlock study is currently held.
    is_unlocked: bool,
    /// Whether the unlock study can be bought right now.
    can_unlock: bool,
    /// TT cost of the unlock study.
    study_cost: f64,
    /// The current IP goal.
    goal: Num,
    /// Secondary unlock requirement (current/required), absent for EC11/12.
    secondary_current: Option<Num>,
    secondary_required: Option<Num>,
}

/// Serializable view of one Eternity Milestone (grid order = threshold order).
#[derive(Serialize)]
struct EternityMilestoneView {
    /// Original config key (frontend keys its reward text on this).
    id: &'static str,
    /// Eternities required.
    eternities: u64,
    /// Whether the milestone is reached.
    is_reached: bool,
}

/// Serializable view of the player options the frontend reads/writes.
#[derive(Serialize)]
struct OptionsView {
    hotkeys: bool,
    /// "Automatically retry challenges" toggle (original `retryChallenge`):
    /// crunching inside an antimatter challenge re-enters it.
    retry_challenge: bool,
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
    /// Animation toggles (Animation Options modal; only Big Crunch so far).
    animations: AnimationsView,
    /// Info-display hint toggles (Info Display Options modal).
    show_hint_text: ShowHintTextView,
    /// Away-progress display toggles (Away Progress Options modal + the
    /// "While you were away" summary).
    away_progress: AwayProgressView,
    /// Relative prestige-gain text coloring (original `headerTextColored`).
    header_text_colored: bool,
    /// Sidebar resource id (original `sidebarResourceID`; 0 = latest resource).
    sidebar_resource_id: u32,
    /// Hidden top-level tabs bitmask (original `hiddenTabBits`, original ids).
    hidden_tab_bits: u32,
    /// Per-tab hidden-subtab bitmasks (original `hiddenSubtabBits`, 11 entries).
    hidden_subtab_bits: Vec<u32>,
    /// Which resource pair the Past Prestige Runs tables show (original
    /// `statTabResources`, 0–3).
    stat_tab_resources: u8,
}

/// Serializable view of the per-action confirmation toggles.
#[derive(Serialize)]
struct ConfirmationsView {
    dimension_boost: bool,
    antimatter_galaxy: bool,
    sacrifice: bool,
    big_crunch: bool,
    eternity: bool,
    switch_automator_mode: bool,
    dilation: bool,
}

/// Serializable view of the animation toggles (modelled subset).
#[derive(Serialize)]
struct AnimationsView {
    big_crunch: bool,
}

/// Serializable view of the info-display hint toggles.
#[derive(Serialize)]
struct ShowHintTextView {
    show_percentage: bool,
    achievements: bool,
    achievement_unlock_states: bool,
    challenges: bool,
}

/// Serializable view of the away-progress display toggles.
#[derive(Serialize)]
struct AwayProgressView {
    antimatter: bool,
    dimension_boosts: bool,
    antimatter_galaxies: bool,
    infinities: bool,
    infinity_points: bool,
    replicanti: bool,
    replicanti_galaxies: bool,
}

/// The Celestials tab (Phase 7). One sub-view per implemented celestial; the
/// `unlocked` gate mirrors `celestials_unlocked`.
#[derive(Serialize)]
struct CelestialsView {
    unlocked: bool,
    teresa: TeresaView,
    effarig: EffarigView,
    enslaved: EnslavedView,
    v: VView,
    ra: RaView,
    laitela: LaitelaView,
    pelle: PelleView,
}

/// Pelle subtab (Feature 7.7).
#[derive(Serialize)]
struct PelleView {
    unlocked: bool,
    doomed: bool,
    remnants: f64,
    remnants_gain: f64,
    can_armageddon: bool,
    reality_shards: Num,
    reality_shard_per_second: Num,
    is_game_end: bool,
    game_end_progress: f64,
    galaxy_generator_unlocked: bool,
    galaxy_generator_galaxies: f64,
    rifts: Vec<RiftView>,
    rebuyables: Vec<PelleRebuyableView>,
    upgrades: Vec<PelleUpgradeView>,
}

#[derive(Serialize)]
struct RiftView {
    id: usize,
    unlocked: bool,
    active: bool,
    percentage: f64,
    milestones: Vec<bool>,
}

#[derive(Serialize)]
struct PelleRebuyableView {
    id: usize,
    cost: Num,
    count: u32,
    cap: u32,
}

#[derive(Serialize)]
struct PelleUpgradeView {
    id: u32,
    cost: Num,
    bought: bool,
}

/// Lai'tela subtab (Feature 7.6).
#[derive(Serialize)]
struct LaitelaView {
    unlocked: bool,
    is_running: bool,
    can_start_run: bool,
    dark_matter: Num,
    max_dark_matter: Num,
    dark_energy: Num,
    singularities: Num,
    singularity_cap: Num,
    singularity_cap_increases: u32,
    singularities_gained: Num,
    can_condense: bool,
    dark_matter_mult: f64,
    dark_matter_mult_gain: f64,
    can_annihilate: bool,
    annihilation_unlocked: bool,
    continuum_unlocked: bool,
    continuum_active: bool,
    difficulty_tier: u32,
    max_allowed_dimension: u32,
    entropy: f64,
    reality_reward: f64,
    dimensions: Vec<DmdView>,
    milestones: Vec<MilestoneView>,
    imaginary_machines: Num,
    im_cap: Num,
    imaginary_upgrades: Vec<ImaginaryUpgradeView>,
    imaginary_rebuyables: Vec<ImaginaryRebuyableView>,
}

#[derive(Serialize)]
struct DmdView {
    tier: usize,
    unlocked: bool,
    amount: Num,
    interval_ms: f64,
    interval_cost: Num,
    power_dm: Num,
    power_dm_cost: Num,
    power_de: Num,
    power_de_cost: Num,
    can_ascend: bool,
    ascensions: u32,
}

#[derive(Serialize)]
struct MilestoneView {
    id: usize,
    unlocked: bool,
    completions: u32,
    start: f64,
}

#[derive(Serialize)]
struct ImaginaryUpgradeView {
    id: u8,
    cost: Num,
    bought: bool,
    available: bool,
}

#[derive(Serialize)]
struct ImaginaryRebuyableView {
    id: u8,
    cost: Num,
    count: u32,
}

/// Ra subtab (Feature 7.5).
#[derive(Serialize)]
struct RaView {
    unlocked: bool,
    is_running: bool,
    can_start_run: bool,
    total_pet_level: u32,
    remembrance_unlocked: bool,
    total_charges: u32,
    charges_used: u32,
    alchemy_unlocked: bool,
    pets: Vec<RaPetView>,
    unlocks: Vec<RaUnlockView>,
    alchemy: Vec<AlchemyResourceView>,
}

#[derive(Serialize)]
struct RaPetView {
    id: u8,
    unlocked: bool,
    level: u32,
    memories: Num,
    memory_chunks: Num,
    required_memories: Num,
    memory_upgrade_cost: Num,
    chunk_upgrade_cost: Num,
    memory_upgrade_capped: bool,
    chunk_upgrade_capped: bool,
    has_remembrance: bool,
}

#[derive(Serialize)]
struct RaUnlockView {
    id: u8,
    pet: u8,
    level: u32,
    unlocked: bool,
}

#[derive(Serialize)]
struct AlchemyResourceView {
    id: u8,
    unlocked: bool,
    unlocked_at: u32,
    is_base: bool,
    amount: Num,
    cap: Num,
    reaction_active: bool,
}

/// V subtab (Feature 7.4).
#[derive(Serialize)]
struct VView {
    /// Whether the V tab is available (V unlocked, or the chain reached it).
    unlocked: bool,
    /// Whether V (the celestial) is unlocked.
    is_v_unlocked: bool,
    /// Whether all six main conditions are met (the "unlock V" prompt).
    can_unlock: bool,
    /// The six main-unlock progress fractions [0, 1].
    main_progress: Vec<f64>,
    is_running: bool,
    can_start_run: bool,
    space_theorems: u32,
    available_st: u32,
    achievements: Vec<VAchievementView>,
    rewards: Vec<VRewardView>,
}

#[derive(Serialize)]
struct VAchievementView {
    id: u8,
    completions: u32,
    tiers: u32,
    current_value: f64,
    next_goal: f64,
    condition_met: bool,
    is_hard: bool,
}

#[derive(Serialize)]
struct VRewardView {
    id: u8,
    st_required: u32,
    unlocked: bool,
}

/// Enslaved — The Nameless Ones subtab (Feature 7.3). Times are in ms (the
/// frontend renders them with `timeDisplayShort`).
#[derive(Serialize)]
struct EnslavedView {
    /// Whether Enslaved is available (Effarig's Eternity stage done).
    unlocked: bool,
    /// Stored game time (ms) and real time (ms).
    stored: f64,
    stored_real: f64,
    /// Whether game-time storage is active / toggleable.
    is_storing_game_time: bool,
    can_modify_game_time_storage: bool,
    /// Whether a discharge is possible now.
    can_release: bool,
    /// Whether Enslaved's Reality is unlocked / running / startable / completed.
    run_unlocked: bool,
    is_running: bool,
    can_start_run: bool,
    completed: bool,
    /// Whether the run unlock's glyph requirement (level 5000 / rarity 100) is met.
    run_requirement_met: bool,
    /// The two unlocks (softcap id 0, run id 1).
    unlocks: Vec<EnslavedUnlockView>,
}

#[derive(Serialize)]
struct EnslavedUnlockView {
    id: u8,
    price_ms: f64,
    owned: bool,
    can_buy: bool,
}

/// Effarig subtab (Feature 7.2).
#[derive(Serialize)]
struct EffarigView {
    /// Whether Effarig is unlocked at all (Teresa's `effarig` threshold).
    unlocked: bool,
    relic_shards: Num,
    shards_gained: Num,
    /// Current stage (1 Infinity / 2 Eternity / 3 Reality / 4 Completed).
    current_stage: u8,
    glyph_level_cap: u32,
    run_unlocked: bool,
    is_running: bool,
    can_start_run: bool,
    /// Relic-Shard purchasable unlocks (adjuster/glyphFilter/setSaves/run).
    shop_unlocks: Vec<EffarigUnlockView>,
    /// The three stage unlocks (infinity/eternity/reality).
    stage_unlocks: Vec<EffarigStageView>,
}

#[derive(Serialize)]
struct EffarigUnlockView {
    id: u8,
    cost: Num,
    unlocked: bool,
    can_buy: bool,
}

#[derive(Serialize)]
struct EffarigStageView {
    id: u8,
    unlocked: bool,
}

/// Teresa subtab (Feature 7.1).
#[derive(Serialize)]
struct TeresaView {
    /// RM poured into Teresa (`pouredAmount`).
    poured_amount: Num,
    /// Current Reality Machines (the pour source).
    reality_machines: Num,
    /// The RM-gain multiplier from the pool (`rmMultiplier`).
    rm_multiplier: f64,
    /// The live glyph-sacrifice multiplier (`runRewardMultiplier`).
    run_reward_multiplier: f64,
    /// Pour-bar fill in [0, 1] (`fill`).
    fill: f64,
    /// Fill the pool *could* reach with all current RM (`possibleFill`).
    possible_fill: f64,
    /// Whether Teresa's Reality is unlocked / running / startable.
    run_unlocked: bool,
    is_running: bool,
    can_start_run: bool,
    /// Whether the Perk Shop is unlocked.
    shop_unlocked: bool,
    /// Unspent Perk Points (the Perk-Shop currency).
    perk_points: f64,
    /// The 6 threshold unlocks, in save-id order.
    unlocks: Vec<TeresaUnlockView>,
    /// The 4 Perk-Shop rebuyables (modelled subset).
    perk_shop: Vec<PerkShopView>,
}

#[derive(Serialize)]
struct TeresaUnlockView {
    id: u8,
    price: Num,
    unlocked: bool,
}

#[derive(Serialize)]
struct PerkShopView {
    id: usize,
    cost: Num,
    effect: Num,
    bought: u32,
    capped: bool,
    can_buy: bool,
}

/// Build the Celestials tab view.
fn build_celestials_view(game: &GameState) -> CelestialsView {
    let f = |x: f64| num(&Decimal::from_float(x));
    let unlocks = ad_core::celestials::teresa::TERESA_UNLOCKS
        .iter()
        .map(|u| TeresaUnlockView {
            id: u.id,
            price: f(u.price),
            unlocked: game.celestials.teresa.unlock_bought(u.id),
        })
        .collect();
    let perk_shop = ad_core::celestials::teresa::PERK_SHOP_ENTRIES
        .iter()
        .map(|&e| PerkShopView {
            id: e.id,
            cost: f(game.perk_shop_cost(e)),
            effect: f(game.perk_shop_effect(e)),
            bought: game.perk_shop_bought(e),
            capped: game.perk_shop_capped(e),
            can_buy: game.perk_shop_can_buy(e),
        })
        .collect();
    let teresa = TeresaView {
        poured_amount: f(game.celestials.teresa.poured_amount),
        reality_machines: num(&game.reality.machines),
        rm_multiplier: game.teresa_rm_multiplier(),
        run_reward_multiplier: game.teresa_run_reward_multiplier(),
        fill: game.teresa_fill(),
        // `possibleFill = min(log10(RM + poured)/24, 1)`.
        possible_fill: ((game.reality.machines
            + Decimal::from_float(game.celestials.teresa.poured_amount))
        .pos_log10()
            / 24.0)
            .clamp(0.0, 1.0),
        run_unlocked: game.teresa_run_unlocked(),
        is_running: game.celestials.teresa.run,
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::Teresa),
        shop_unlocked: game.teresa_shop_unlocked(),
        perk_points: game.reality.perk_points,
        unlocks,
        perk_shop,
    };
    CelestialsView {
        unlocked: game.celestials_unlocked(),
        teresa,
        effarig: build_effarig_view(game),
        enslaved: build_enslaved_view(game),
        v: build_v_view(game),
        ra: build_ra_view(game),
        laitela: build_laitela_view(game),
        pelle: build_pelle_view(game),
    }
}

/// Build the Pelle subtab view.
fn build_pelle_view(game: &GameState) -> PelleView {
    use ad_core::celestials::pelle::RIFT_COUNT;
    let p = &game.celestials.pelle;
    let rifts = (0..RIFT_COUNT)
        .map(|i| RiftView {
            id: i,
            unlocked: game.pelle_rift_unlocked(i),
            active: p.rifts[i].active,
            percentage: game.pelle_rift_percentage(i).clamp(0.0, 1.0),
            milestones: (0..3).map(|m| game.pelle_rift_milestone(i, m)).collect(),
        })
        .collect();
    let rebuyables = (0..5)
        .map(|id| PelleRebuyableView {
            id,
            cost: num(&game.pelle_rebuyable_cost(id)),
            count: p.rebuyables[id],
            cap: GameState::PELLE_REBUYABLE_CAPS[id],
        })
        .collect();
    let upgrades = (0..23u32)
        .map(|id| PelleUpgradeView {
            id,
            cost: num(&game.pelle_upgrade_cost(id)),
            bought: game.pelle_upgrade_bought(id),
        })
        .collect();
    PelleView {
        unlocked: game.pelle_unlocked(),
        doomed: game.is_doomed(),
        remnants: p.remnants,
        remnants_gain: game.remnants_gain(),
        can_armageddon: game.can_armageddon(),
        reality_shards: num(&p.reality_shards),
        reality_shard_per_second: num(&game.reality_shard_gain_per_second()),
        is_game_end: game.is_game_end,
        game_end_progress: game.game_end_state().min(1.0),
        galaxy_generator_unlocked: game.galaxy_generator_unlocked(),
        galaxy_generator_galaxies: game.galaxy_generator_galaxies(),
        rifts,
        rebuyables,
        upgrades,
    }
}

/// Build the Lai'tela subtab view.
fn build_laitela_view(game: &GameState) -> LaitelaView {
    let f = |x: f64| num(&Decimal::from_float(x.min(1e300)));
    let l = &game.celestials.laitela;
    let dimensions = (0..4)
        .map(|tier| DmdView {
            tier,
            unlocked: game.dmd_unlocked(tier),
            amount: num(&l.dimensions[tier].amount),
            interval_ms: game.dmd_interval(tier),
            interval_cost: num(&game.dmd_interval_cost(tier)),
            power_dm: num(&game.dmd_power_dm(tier)),
            power_dm_cost: num(&game.dmd_power_dm_cost(tier)),
            power_de: f(game.dmd_power_de(tier)),
            power_de_cost: num(&game.dmd_power_de_cost(tier)),
            can_ascend: game.dmd_interval(tier) <= 10.0,
            ascensions: l.dimensions[tier].ascension_count,
        })
        .collect();
    let milestones = (0..ad_core::celestials::singularity::MILESTONE_COUNT)
        .map(|id| MilestoneView {
            id,
            unlocked: game.singularity_milestone_unlocked(id),
            completions: game.singularity_milestone_completions(id),
            start: ad_core::celestials::singularity::MILESTONES[id].start,
        })
        .collect();
    let imaginary_upgrades = (11..=25u8)
        .map(|id| ImaginaryUpgradeView {
            id,
            cost: f(game.imaginary_upgrade_cost(id)),
            bought: game.imaginary_upgrade_bought(id),
            available: game.imaginary_upgrade_available(id),
        })
        .collect();
    let imaginary_rebuyables = (1..=10u8)
        .map(|id| ImaginaryRebuyableView {
            id,
            cost: f(game.imaginary_rebuyable_cost(id)),
            count: game.imaginary_rebuyable_count(id),
        })
        .collect();
    LaitelaView {
        unlocked: game.laitela_unlocked(),
        is_running: game.laitela_is_running(),
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::Laitela),
        dark_matter: num(&l.dark_matter),
        max_dark_matter: num(&l.max_dark_matter),
        dark_energy: f(l.dark_energy),
        singularities: f(l.singularities),
        singularity_cap: f(game.singularity_cap()),
        singularity_cap_increases: l.singularity_cap_increases,
        singularities_gained: f(game.singularities_gained()),
        can_condense: game.singularity_cap_reached(),
        dark_matter_mult: l.dark_matter_mult,
        dark_matter_mult_gain: game.dark_matter_mult_gain(),
        can_annihilate: game.can_annihilate(),
        annihilation_unlocked: game.annihilation_unlocked(),
        continuum_unlocked: game.continuum_unlocked(),
        continuum_active: game.continuum_active(),
        difficulty_tier: l.difficulty_tier,
        max_allowed_dimension: game.laitela_max_allowed_dimension(),
        entropy: l.entropy.max(0.0),
        reality_reward: game.laitela_reality_reward(),
        dimensions,
        milestones,
        imaginary_machines: num(&game.reality.imaginary_machines),
        im_cap: f(game.imaginary_machine_cap()),
        imaginary_upgrades,
        imaginary_rebuyables,
    }
}

/// Build the Ra subtab view (pets, unlocks, Alchemy).
fn build_ra_view(game: &GameState) -> RaView {
    use ad_core::celestials::ra::{RA_UNLOCK_COUNT, RA_UNLOCK_REQS};
    let f = |x: f64| num(&Decimal::from_float(x.min(1e300)));
    let pets = (0..ad_core::celestials::ra::PET_COUNT)
        .map(|p| {
            let pet = &game.celestials.ra.pets[p];
            RaPetView {
                id: p as u8,
                unlocked: game.ra_pet_unlocked(p),
                level: game.ra_pet_level(p),
                memories: f(pet.memories),
                memory_chunks: f(pet.memory_chunks),
                required_memories: f(game.ra_required_memories_for_level(pet.level)),
                memory_upgrade_cost: f(game.ra_memory_upgrade_cost(p)),
                chunk_upgrade_cost: f(game.ra_chunk_upgrade_cost(p)),
                memory_upgrade_capped: game.ra_memory_upgrade_capped(p),
                chunk_upgrade_capped: game.ra_chunk_upgrade_capped(p),
                has_remembrance: game.celestials.ra.pet_with_remembrance == p as i8,
            }
        })
        .collect();
    let unlocks = (0..RA_UNLOCK_COUNT)
        .map(|id| {
            let (pet, level) = RA_UNLOCK_REQS[id as usize];
            RaUnlockView {
                id,
                pet: pet as u8,
                level,
                unlocked: game.ra_has_unlock(id),
            }
        })
        .collect();
    let alchemy = (0..ad_core::celestials::alchemy::ALCHEMY_COUNT)
        .map(|id| AlchemyResourceView {
            id: id as u8,
            unlocked: game.alchemy_resource_unlocked(id),
            unlocked_at: game.alchemy_unlocked_at(id),
            is_base: game.alchemy_is_base(id),
            amount: f(game.alchemy_amount(id)),
            cap: f(game.alchemy_cap(id)),
            reaction_active: game.celestials.ra.alchemy[id].reaction,
        })
        .collect();
    RaView {
        unlocked: game.ra_is_unlocked(),
        is_running: game.ra_is_running(),
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::Ra),
        total_pet_level: game.ra_total_pet_level(),
        remembrance_unlocked: game.ra_remembrance_unlocked(),
        total_charges: game.ra_total_charges(),
        charges_used: game.ra_charges_used(),
        alchemy_unlocked: game.alchemy_unlocked(),
        pets,
        unlocks,
        alchemy,
    }
}

/// Build the V subtab view.
fn build_v_view(game: &GameState) -> VView {
    use ad_core::celestials::v::V_UNLOCK_ST_THRESHOLDS;
    let achievements = (0..9usize)
        .map(|id| {
            let (completions, tiers, current_value, next_goal, condition_met, is_hard) =
                game.v_achievement_status(id);
            VAchievementView {
                id: id as u8,
                completions,
                tiers,
                current_value,
                next_goal,
                condition_met,
                is_hard,
            }
        })
        .collect();
    let rewards = V_UNLOCK_ST_THRESHOLDS
        .iter()
        .map(|&(bit, st)| VRewardView {
            id: bit,
            st_required: st,
            unlocked: game.celestials.v.unlock_bought(bit),
        })
        .collect();
    VView {
        unlocked: game.v_tab_available(),
        is_v_unlocked: game.v_celestial_unlocked(),
        can_unlock: game.v_can_unlock_celestial(),
        main_progress: game.v_main_unlock_progress().to_vec(),
        is_running: game.celestials.v.run,
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::V),
        space_theorems: game.v_space_theorems(),
        available_st: game.v_available_space_theorems(),
        achievements,
        rewards,
    }
}

/// Build the Enslaved subtab view.
fn build_enslaved_view(game: &GameState) -> EnslavedView {
    use ad_core::celestials::enslaved::{ENSLAVED_UNLOCK_RUN, ENSLAVED_UNLOCK_SOFTCAP};
    let unlocks = [ENSLAVED_UNLOCK_SOFTCAP, ENSLAVED_UNLOCK_RUN]
        .iter()
        .map(|&id| EnslavedUnlockView {
            id,
            price_ms: GameState::enslaved_unlock_price(id),
            owned: game.celestials.enslaved.unlock_bought(id),
            can_buy: game.can_buy_enslaved_unlock(id),
        })
        .collect();
    EnslavedView {
        unlocked: game.enslaved_unlocked(),
        stored: game.celestials.enslaved.stored,
        stored_real: game.celestials.enslaved.stored_real,
        is_storing_game_time: game.is_storing_game_time(),
        can_modify_game_time_storage: game.can_modify_game_time_storage(),
        can_release: game.celestials.enslaved.stored > 0.0 && !game.ec_running(12),
        run_unlocked: game.enslaved_run_unlocked(),
        is_running: game.celestials.enslaved.run,
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::Enslaved),
        completed: game.celestials.enslaved.completed,
        run_requirement_met: game.enslaved_run_requirement_met(),
        unlocks,
    }
}

/// Build the Effarig subtab view.
fn build_effarig_view(game: &GameState) -> EffarigView {
    use ad_core::celestials::effarig;
    let f = |x: f64| num(&Decimal::from_float(x));
    let shop_unlocks = effarig::EFFARIG_UNLOCK_COSTS
        .iter()
        .map(|&(id, cost)| EffarigUnlockView {
            id,
            cost: f(cost),
            unlocked: game.celestials.effarig.unlock_bought(id),
            can_buy: !game.celestials.effarig.unlock_bought(id)
                && game.celestials.effarig.relic_shards >= cost,
        })
        .collect();
    let stage_unlocks = [
        effarig::EFFARIG_UNLOCK_INFINITY,
        effarig::EFFARIG_UNLOCK_ETERNITY,
        effarig::EFFARIG_UNLOCK_REALITY,
    ]
    .iter()
    .map(|&id| EffarigStageView {
        id,
        unlocked: game.celestials.effarig.unlock_bought(id),
    })
    .collect();
    EffarigView {
        unlocked: game.effarig_unlocked(),
        relic_shards: f(game.celestials.effarig.relic_shards),
        shards_gained: f(game.effarig_shards_gained()),
        current_stage: game.effarig_current_stage() as u8,
        glyph_level_cap: game.effarig_glyph_level_cap(),
        run_unlocked: game.effarig_run_unlocked(),
        is_running: game.celestials.effarig.run,
        can_start_run: game.can_start_celestial_reality(ad_core::Celestial::Effarig),
        shop_unlocks,
        stage_unlocks,
    }
}

/// Build the serializable view for one autobuyer, including its interval-upgrade
/// state (cost / affordability / maxed) derived from `target`.
fn build_autobuyer_view(
    game: &GameState,
    target: AutobuyerTarget,
    name: String,
    requirement: Decimal,
    can_unlock: bool,
    can_change_mode: bool,
    antimatter_unlockable: bool,
) -> AutobuyerView {
    let autobuyer = game.autobuyer(target);
    let can_be_upgraded = game.autobuyer_can_be_upgraded(target);
    let has_maxed_interval = game.autobuyer_has_maxed_interval(target);
    let cost = Decimal::from_float(autobuyer.cost);
    let can_afford_upgrade =
        can_be_upgraded && !has_maxed_interval && game.infinity_points >= cost;
    // Bulk-upgrade state (AD autobuyers only).
    let (bulk, has_maxed_bulk, has_unlimited_bulk, can_afford_bulk_upgrade) =
        if let AutobuyerTarget::AdTier(tier) = target {
            (
                autobuyer.bulk,
                game.ad_autobuyer_has_maxed_bulk(tier),
                game.achievement_unlocked(61),
                has_maxed_interval
                    && !game.ad_autobuyer_has_maxed_bulk(tier)
                    && game.infinity_points >= cost,
            )
        } else {
            (1, false, false, false)
        };

    AutobuyerView {
        name,
        is_bought: autobuyer.is_bought,
        is_unlocked: game.autobuyer_is_unlocked(target),
        antimatter_unlockable,
        unlock_challenge: GameState::autobuyer_challenge(target),
        can_unlock,
        requirement: num(&requirement),
        interval_seconds: format!("{:.2}", autobuyer.interval_ms / 1000.0),
        can_be_upgraded,
        has_maxed_interval,
        upgrade_cost: num(&cost),
        can_afford_upgrade,
        is_active: autobuyer.is_active,
        mode: match autobuyer.mode {
            AutobuyerMode::BuySingle => "single".to_string(),
            AutobuyerMode::BuyMax => "max".to_string(),
        },
        can_change_mode,
        bulk,
        has_maxed_bulk,
        has_unlimited_bulk,
        can_afford_bulk_upgrade,
    }
}

fn build_autobuyers_view(game: &GameState) -> AutobuyersView {
    let dimensions = (0..8)
        .map(|tier| {
            build_autobuyer_view(
                game,
                AutobuyerTarget::AdTier(tier),
                format!("{} Dimension Autobuyer", DIMENSION_ORDINALS[tier]),
                GameState::ad_autobuyer_requirement(tier),
                game.can_unlock_ad_autobuyer(tier),
                // AD autobuyer mode ("Buys singles"/"Buys max") is always
                // changeable, even pre-Infinity.
                true,
                true,
            )
        })
        .collect();

    let tickspeed = build_autobuyer_view(
        game,
        AutobuyerTarget::Tickspeed,
        "Tickspeed Autobuyer".to_string(),
        GameState::tickspeed_autobuyer_requirement(),
        game.can_unlock_tickspeed_autobuyer(),
        // Pre-Infinity the tickspeed autobuyer is locked to single.
        false,
        true,
    );

    // The prestige autobuyers have no antimatter "slow version" (no requirement,
    // no purchase box) and no single/max mode — they unlock by challenge only.
    let prestige = |target, name: &str| {
        build_autobuyer_view(
            game,
            target,
            name.to_string(),
            Decimal::ZERO,
            false,
            false,
            false,
        )
    };

    AutobuyersView {
        tab_unlocked: game.autobuyer_tab_unlocked(),
        enabled: game.autobuyers.enabled,
        dimensions,
        tickspeed,
        dim_boost: prestige(AutobuyerTarget::DimBoost, "Dimension Boost Autobuyer"),
        galaxy: prestige(AutobuyerTarget::Galaxy, "Antimatter Galaxy Autobuyer"),
        big_crunch: prestige(AutobuyerTarget::BigCrunch, "Big Crunch Autobuyer"),
        big_crunch_settings: prestige_goal_view(
            &game.autobuyers.big_crunch_settings,
            game.big_crunch_autobuyer_has_modes(),
        ),
        eternity: EternityAutobuyerView {
            is_unlocked: game.eternity_autobuyer_unlocked(),
            is_active: game.autobuyers.eternity.is_active,
            settings: prestige_goal_view(
                &game.autobuyers.eternity.settings,
                game.eternity_autobuyer_has_modes(),
            ),
        },
        reality: RealityAutobuyerView {
            is_unlocked: game.reality_autobuyer_unlocked(),
            is_active: game.autobuyers.reality.is_active,
            mode: match game.autobuyers.reality.mode {
                AutoRealityMode::Rm => "rm",
                AutoRealityMode::Glyph => "glyph",
                AutoRealityMode::Either => "either",
                AutoRealityMode::Both => "both",
                AutoRealityMode::Time => "time",
            }
            .to_string(),
            rm: num(&game.autobuyers.reality.rm),
            glyph: game.autobuyers.reality.glyph,
            time: game.autobuyers.reality.time,
        },
        infinity_dims: MilestoneAutobuyerGroupView {
            any_unlocked: game.eternity_milestone_reached(11),
            group_active: game.autobuyers.infinity_dims_group_active,
            entries: (0..8)
                .map(|tier| MilestoneAutobuyerEntryView {
                    name: DIMENSION_ORDINALS[tier].to_string(),
                    is_unlocked: game.eternity_milestone_reached(11 + tier as u64),
                    is_active: game.autobuyers.infinity_dims[tier].is_active,
                })
                .collect(),
        },
        replicanti_upgrades: MilestoneAutobuyerGroupView {
            any_unlocked: game.eternity_milestone_reached(50),
            group_active: game.autobuyers.replicanti_upgrades_group_active,
            entries: ["Chance", "Interval", "Max Galaxies"]
                .iter()
                .zip([50u64, 60, 80])
                .enumerate()
                .map(|(id, (&name, milestone))| MilestoneAutobuyerEntryView {
                    name: name.to_string(),
                    is_unlocked: game.eternity_milestone_reached(milestone),
                    is_active: game.autobuyers.replicanti_upgrades[id].is_active,
                })
                .collect(),
        },
        replicanti_galaxy_unlocked: game.eternity_milestone_reached(3),
        replicanti_galaxy_active: game.autobuyers.replicanti_galaxies_active,
        time_dims: MilestoneAutobuyerGroupView {
            any_unlocked: game.reality_upgrade_bought(13),
            group_active: game.autobuyers.time_dims_group_active,
            entries: (0..8)
                .map(|tier| MilestoneAutobuyerEntryView {
                    name: DIMENSION_ORDINALS[tier].to_string(),
                    is_unlocked: game.reality_upgrade_bought(13),
                    is_active: game.autobuyers.time_dims[tier].is_active,
                })
                .collect(),
        },
        ep_mult_unlocked: game.reality_upgrade_bought(13) && !game.is_doomed(),
        ep_mult_active: game.autobuyers.ep_mult_buyer_active,
    }
}

/// Build the mode/threshold view for a prestige-goal autobuyer.
fn prestige_goal_view(
    settings: &ad_core::autobuyers::PrestigeGoalSettings,
    has_modes: bool,
) -> PrestigeGoalView {
    PrestigeGoalView {
        mode: match settings.mode {
            PrestigeAutobuyerMode::Amount => "amount",
            PrestigeAutobuyerMode::Time => "time",
            PrestigeAutobuyerMode::XHighest => "xHighest",
        }
        .to_string(),
        has_modes,
        amount: num(&settings.amount),
        time: settings.time,
        x_highest: num(&settings.x_highest),
        increase_with_mult: settings.increase_with_mult,
    }
}

/// Build the Infinity Upgrades view (grid order = engine's `ALL_INFINITY_UPGRADES`,
/// column-major). The frontend keys its layout/description table on `id`.
fn build_infinity_upgrades_view(game: &GameState) -> Vec<InfinityUpgradeView> {
    ALL_INFINITY_UPGRADES
        .iter()
        .map(|&upgrade| InfinityUpgradeView {
            id: upgrade.save_id().to_string(),
            is_bought: game.infinity_upgrade_bought(upgrade),
            can_be_bought: game.can_buy_infinity_upgrade(upgrade),
            cost: num(&upgrade.cost()),
            effect: num(&game.infinity_upgrade_effect(upgrade)),
        })
        .collect()
}

/// Build the Infinity Upgrades bottom-row view (`ipMult` + `ipOffline`,
/// Achievement 41).
fn build_infinity_upgrades_bottom_row_view(
    game: &GameState,
) -> InfinityUpgradesBottomRowView {
    InfinityUpgradesBottomRowView {
        unlocked: game.achievement_unlocked(41),
        ip_mult_purchases: game.ip_mult_purchases,
        ip_mult_cost: num(&game.ip_mult_cost()),
        ip_mult_effect: num(&game.ip_mult_effect()),
        ip_mult_capped: game.ip_mult_capped(),
        ip_mult_can_be_bought: game.can_buy_ip_mult(),
        ip_offline_bought: game.ip_offline_bought,
        ip_offline_can_be_bought: game.can_buy_ip_offline(),
        ip_offline_cost: num(&game.ip_offline_cost()),
        ip_offline_effect_per_min: num(&game.ip_offline_effect_per_min()),
        autobuyer_unlocked: game.eternity_milestone_reached(1) && !game.is_doomed(),
        autobuyer_active: game.autobuyers.ip_mult_buyer_active,
    }
}

/// Build the Normal Challenges view (all 12, in id order). Static per-challenge
/// display data (name, reward, description, unlock threshold) lives frontend-side.
fn build_challenges_view(game: &GameState) -> Vec<ChallengeView> {
    (1..=NORMAL_CHALLENGE_COUNT)
        .map(|id| ChallengeView {
            id,
            is_unlocked: game.challenge_unlocked(id),
            is_running: game.challenge_running(id),
            is_completed: game.challenge_completed(id),
        })
        .collect()
}

/// Build the Infinity Challenges view (all 8, in id order). Descriptions/goals
/// live frontend-side, keyed on the id.
fn build_infinity_challenges_view(game: &GameState) -> Vec<InfinityChallengeView> {
    (1..=INFINITY_CHALLENGE_COUNT)
        .map(|id| InfinityChallengeView {
            id,
            is_unlocked: game.infinity_challenge_unlocked(id),
            is_running: game.infinity_challenge_running(id),
            is_completed: game.infinity_challenge_completed(id),
        })
        .collect()
}

/// Build the Infinity Dimensions view (8 tiers + Infinity Power).
fn build_infinity_dimensions_view(game: &GameState) -> InfinityDimensionsView {
    let dimensions = (0..INFINITY_DIMENSION_COUNT)
        .map(|tier| {
            let d = &game.infinity_dimensions[tier];
            InfinityDimensionView {
                tier,
                amount: num(&d.amount),
                multiplier: num(&game.id_multiplier(tier)),
                cost: num(&d.cost),
                is_unlocked: d.is_unlocked,
                can_be_bought: game.id_available_for_purchase(tier),
                can_unlock: game.can_unlock_infinity_dimension(tier),
                is_capped: game.id_is_capped(tier),
            }
        })
        .collect();
    InfinityDimensionsView {
        unlocked: game.broke_infinity || game.infinity_dimensions[0].is_unlocked,
        power: num(&game.infinity_power),
        power_mult: num(&game.infinity_power_ad_multiplier()),
        dimensions,
    }
}

/// Build the Replicanti view (unlock, amount + ID multiplier, the 3 IP upgrades,
/// and the Replicanti Galaxy button).
fn build_replicanti_view(game: &GameState) -> ReplicantiView {
    let r = &game.replicanti;
    ReplicantiView {
        unlocked: r.unlocked,
        unlock_cost: num(&REPLICANTI_UNLOCK_COST),
        can_unlock: game.can_unlock_replicanti(),
        amount: num(&r.amount),
        mult: num(&game.replicanti_mult()),
        chance: r.chance,
        interval_ms: r.interval_ms,
        chance_cost: num(&r.chance_cost),
        chance_capped: game.replicanti_chance_capped(),
        can_buy_chance: game.can_buy_replicanti_chance(),
        interval_cost: num(&r.interval_cost),
        interval_capped: game.replicanti_interval_capped(),
        can_buy_interval: game.can_buy_replicanti_interval(),
        galaxy_cap: r.galaxy_cap,
        galaxy_cost: num(&game.replicanti_galaxy_cost()),
        can_buy_galaxy_cap: game.can_buy_replicanti_galaxy_cap(),
        galaxies: r.galaxies,
        can_buy_galaxy: game.can_buy_replicanti_galaxy(),
        can_see_galaxy_button: r.galaxy_cap >= 1,
    }
}

/// Build the Break Infinity view (the 9 one-time + 3 rebuyable upgrades). Static
/// per-upgrade display data (descriptions) lives frontend-side, keyed on the id.
fn build_break_infinity_view(game: &GameState) -> BreakInfinityView {
    let upgrades = ALL_BREAK_INFINITY_UPGRADES
        .iter()
        .map(|&u| BreakUpgradeView {
            id: u.save_id().to_string(),
            cost: num(&u.cost()),
            is_bought: game.break_infinity_upgrade_bought(u),
            can_be_bought: game.can_buy_break_infinity_upgrade(u),
        })
        .collect();
    let rebuyables = ALL_BREAK_INFINITY_REBUYABLES
        .iter()
        .map(|&r| BreakRebuyableView {
            id: r.index(),
            cost: num(&game.break_infinity_rebuyable_cost(r)),
            count: game.break_infinity_rebuyable_count(r),
            max: r.max_upgrades(),
            can_be_bought: game.can_buy_break_infinity_rebuyable(r),
        })
        .collect();
    BreakInfinityView {
        unlocked: game.broke_infinity,
        upgrades,
        rebuyables,
    }
}

fn build_time_dimensions_view(game: &GameState) -> TimeDimensionsView {
    let dimensions = (0..8)
        .map(|tier| {
            let d = &game.time_dimensions[tier];
            let rate_percent = if tier < 7 && d.amount > Decimal::ZERO {
                let to_gain = game.td_production_per_second(tier + 1);
                let denom = d.amount.max(&Decimal::ONE);
                (to_gain * Decimal::from_float(10.0) / denom).to_f64()
            } else {
                0.0
            };
            TimeDimensionView {
                tier,
                amount: num(&d.amount),
                multiplier: num(&game.td_multiplier(tier)),
                bought: d.bought,
                cost: num(&d.cost),
                is_unlocked: game.td_is_unlocked(tier),
                available_for_purchase: game.td_available_for_purchase(tier),
                rate_percent,
            }
        })
        .collect();
    TimeDimensionsView {
        time_shards: num(&game.time_shards),
        total_tick_gained: game.total_tick_gained,
        next_shards: num(&game.next_free_tickspeed_shards()),
        mult_to_next: game.free_tickspeed_mult(),
        softcap: 300_000.0,
        shards_per_second: num(&game.td_production_per_second(0)),
        dimensions,
    }
}

fn build_time_studies_view(game: &GameState) -> TimeStudiesView {
    TimeStudiesView {
        theorems: num(&game.time_theorems),
        can_buy_tt: game.can_buy_time_theorems(),
        am_cost: num(&game.tt_cost_am()),
        ip_cost: num(&game.tt_cost_ip()),
        ep_cost: num(&game.tt_cost_ep()),
        can_afford_am: game.can_buy_time_theorems()
            && game.antimatter >= game.tt_cost_am(),
        can_afford_ip: game.can_buy_time_theorems()
            && game.infinity_points >= game.tt_cost_ip(),
        can_afford_ep: game.can_buy_time_theorems()
            && game.eternity_points >= game.tt_cost_ep(),
        respec: game.respec,
        studies: ad_core::TIME_STUDIES
            .iter()
            .map(|d| TimeStudyView {
                id: d.id,
                cost: d.cost,
                is_bought: game.time_study_bought(d.id),
                can_buy: game.can_buy_time_study(d.id),
            })
            .collect(),
        ec_unlocked: game.eternity_challenge_unlocked,
        presets: game
            .study_presets
            .iter()
            .map(|p| StudyPresetView {
                name: p.name.clone(),
                studies: p.studies.clone(),
            })
            .collect(),
        can_eternity: game.can_eternity(),
    }
}

fn build_dilation_view(game: &GameState) -> DilationView {
    let tachyon_galaxy_gain = if game.dilation_upgrade_bought(4)
        && game.dilation.base_tachyon_galaxies < 500
    {
        2
    } else {
        1
    };
    DilationView {
        unlocked: game.dilation_unlocked(),
        active: game.dilation.active,
        tachyon_particles: num(&game.dilation.tachyon_particles),
        dilated_time: num(&game.dilation.dilated_time),
        dt_per_second: num(&game.dilation_gain_per_second()),
        next_threshold: num(&game.dilation.next_threshold),
        base_tachyon_galaxies: game.dilation.base_tachyon_galaxies,
        total_tachyon_galaxies: game.dilation.total_tachyon_galaxies,
        tachyon_galaxy_gain,
        tachyon_gain: num(&game.tachyon_gain()),
        upgrades: (1u8..=15)
            .map(|id| DilationUpgradeView {
                id,
                cost: num(&game.dilation_upgrade_cost(id)),
                is_rebuyable: id <= 3 || (11..=13).contains(&id),
                count: game.dilation_rebuyable_count(id),
                is_bought: ((4..=10).contains(&id) || id >= 14)
                    && game.dilation_upgrade_bought(id),
                is_capped: game.dilation_rebuyable_capped(id),
                can_buy: game.can_buy_dilation_upgrade(id),
            })
            .collect(),
        pelle_upgrades_unlocked: game.is_doomed()
            && game.pelle_rift_milestone(ad_core::celestials::pelle::RIFT_PARADOX, 0),
        studies: (1u8..=5)
            .map(|id| DilationStudyView {
                id,
                cost: GameState::dilation_study_cost(id),
                is_bought: game.dilation_study_bought(id),
                can_buy: game.can_buy_dilation_study(id),
            })
            .collect(),
    }
}

/// The Current Glyph Effects panel's display order (`glyphEffectsOrder` in
/// CurrentGlyphEffects.vue), restricted to the generated bits.
const GLYPH_EFFECT_DISPLAY_ORDER: [u8; 27] = [
    16, 17, 18, 19, // power
    12, 15, 14, 13, // infinity
    9, 10, 8, 11, // replication
    0, 3, 1, 2, // time
    7, 6, 4, 5, // dilation
    20, 21, 22, 23, 24, 25, 26, // effarig
];

fn build_glyph_view(game: &GameState, glyph: &ad_core::Glyph) -> GlyphView {
    // The companion's effect bits live in the non-generated bit space (its
    // "effects" are flavour text), so it gets no numeric effect values.
    // Reality glyphs also use the non-generated space (bits 4–7); their view
    // bits are offset by 100 so the frontend can key them separately.
    let effect_values = if glyph.kind == ad_core::GlyphType::Companion {
        Vec::new()
    } else if glyph.kind == ad_core::GlyphType::Reality {
        (4u8..=7)
            .filter(|&bit| glyph.effects & (1u32 << bit) != 0)
            .map(|bit| GlyphEffectValueView {
                bit: 100 + bit,
                value: num(&GameState::reality_glyph_single_effect_value(glyph, bit)),
            })
            .collect()
    } else {
        GLYPH_EFFECT_DISPLAY_ORDER
            .iter()
            .filter(|&&bit| glyph.effects & (1u32 << bit) != 0)
            .map(|&bit| GlyphEffectValueView {
                bit,
                value: num(&GameState::glyph_single_effect_value(glyph, bit)),
            })
            .collect()
    };
    GlyphView {
        id: glyph.id,
        idx: glyph.idx,
        kind: glyph.kind.save_id().to_string(),
        strength: glyph.strength,
        level: glyph.level,
        effects: glyph.effects,
        effect_values,
        sacrifice_value: game.glyph_sacrifice_gain(glyph),
    }
}

/// `EPforRM` from RealityButton.vue: the EP needed for a given RM amount
/// (inverse of the RM formula), for the "Next at X EP" readout.
fn ep_for_rm(game: &GameState, rm: &Decimal) -> Decimal {
    if *rm <= Decimal::ONE {
        return Decimal::pow10(4000.0);
    }
    if *rm <= Decimal::from_float(10.0) {
        return Decimal::pow10(4000.0 / 27.0 * (rm.to_f64() + 26.0));
    }
    let mut result = Decimal::pow10(4000.0 * (rm.log10() / 3.0 + 1.0));
    // Pre-first-reality softcap inverse: past 1e6000 the tax quadruples the
    // required log-distance.
    if !game.reality_unlocked() && result >= Decimal::pow10(6000.0) {
        result = Decimal::pow10((result.log10() - 6000.0) * 4.0 + 6000.0);
    }
    result
}

fn build_reality_view(game: &GameState) -> RealityView {
    let is_available = game.is_reality_available();
    let gained_rm = game.gained_reality_machines();
    let glyph_level = game.gained_glyph_level();

    // Combined active effects: union of the equipped basic glyphs' bits.
    let mut active_bits = 0u32;
    for g in &game.reality.glyphs.active {
        if g.kind != ad_core::GlyphType::Companion {
            active_bits |= g.effects;
        }
    }
    let active_effects = GLYPH_EFFECT_DISPLAY_ORDER
        .iter()
        .filter(|&&bit| active_bits & (1u32 << bit) != 0)
        .map(|&bit| {
            let (value, capped) = combined_glyph_effect(game, bit);
            ActiveGlyphEffectView {
                bit,
                value: num(&value),
                capped,
            }
        })
        .collect();

    RealityView {
        unlocked: game.reality_unlocked(),
        has_reality_study: game.dilation_study_bought(6),
        is_available,
        realities: game.reality.realities,
        machines: num(&game.reality.machines),
        perk_points: game.reality.perk_points,
        gained_rm: num(&gained_rm),
        glyph_level: glyph_level.actual_level,
        glyph_level_exact: game.gained_glyph_level_exact(),
        next_machine_ep: num(&ep_for_rm(game, &(gained_rm + Decimal::ONE))),
        reality_time_minutes: game.records.this_reality.real_time_ms / 60_000.0,
        best_glyph_level: game.records.best_reality.glyph_level,
        respec: game.reality.respec,
        choice_count: game.glyph_choice_count(),
        upcoming_glyphs: if is_available {
            game.upcoming_glyphs()
                .iter()
                .map(|g| build_glyph_view(game, g))
                .collect()
        } else {
            Vec::new()
        },
        active_glyphs: game
            .reality
            .glyphs
            .active
            .iter()
            .map(|g| build_glyph_view(game, g))
            .collect(),
        active_slot_count: game.glyph_active_slot_count(),
        inventory_glyphs: game
            .reality
            .glyphs
            .inventory
            .iter()
            .map(|g| build_glyph_view(game, g))
            .collect(),
        protected_rows: game.reality.glyphs.protected_rows,
        free_inventory_space: game.glyph_free_inventory_space(),
        can_sacrifice: game.can_sacrifice_glyphs(),
        sac_totals: game.reality.glyphs.sac.to_vec(),
        // BASIC_GLYPH_TYPES order (power/infinity/replication/time/dilation),
        // matching `sac_totals`.
        sac_effects: vec![
            game.glyph_sac_power_effect() as f64,
            game.glyph_sac_infinity_effect(),
            game.glyph_sac_replication_effect() as f64,
            game.glyph_sac_time_effect(),
            game.glyph_sac_dilation_effect(),
            game.glyph_sac_effarig_effect(),
            game.glyph_sac_reality_effect(),
        ],
        filter_unlocked: game.glyph_filter_unlocked(),
        filter: {
            let f = &game.reality.glyphs.filter;
            let kinds = [
                ad_core::GlyphType::Time,
                ad_core::GlyphType::Dilation,
                ad_core::GlyphType::Replication,
                ad_core::GlyphType::Infinity,
                ad_core::GlyphType::Power,
                ad_core::GlyphType::Effarig,
            ];
            GlyphFilterView {
                select: f.select,
                trash: f.trash,
                simple: f.simple,
                types: kinds
                    .iter()
                    .map(|&kind| {
                        let i = ad_core::glyphs::GlyphFilter::type_index(kind).unwrap();
                        let cfg = &f.types[i];
                        GlyphFilterTypeView {
                            kind: kind.save_id().to_string(),
                            rarity: cfg.rarity,
                            score: cfg.score,
                            effect_count: cfg.effect_count,
                            specified_mask: cfg.specified_mask,
                            effect_scores: cfg.effect_scores.clone(),
                            bit_offset: ad_core::glyphs::GlyphFilter::bit_offset(kind),
                        }
                    })
                    .collect(),
            }
        },
        undo_unlocked: game.glyph_undo_unlocked(),
        can_undo: game.can_undo_glyph(),
        undo_depth: game.reality.glyphs.undo.len(),
        can_create_reality_glyph: game.can_create_reality_glyph(),
        reality_glyph_level: game.reality_glyph_creation_level(),
        weights_unlocked: game
            .celestials
            .effarig
            .unlock_bought(ad_core::celestials::effarig::EFFARIG_UNLOCK_ADJUSTER),
        glyph_weights: game.celestials.effarig.glyph_weights.to_vec(),
        active_effects,
        rebuyables: (1u8..=5)
            .map(|id| RealityRebuyableView {
                id,
                count: game.reality_rebuyable_count(id),
                cost: num(&game.reality_rebuyable_cost(id)),
                can_buy: game.reality.machines >= game.reality_rebuyable_cost(id),
            })
            .collect(),
        upgrades: (6u8..=25)
            .map(|id| RealityUpgradeView {
                id,
                cost: num(&GameState::reality_upgrade_cost(id)),
                is_bought: game.reality_upgrade_bought(id),
                req_met: game.reality_upgrade_req_met(id),
                can_buy: game.can_buy_reality_upgrade(id),
            })
            .collect(),
        black_holes: BlackHolesView {
            unlocked: game.black_holes.holes[0].unlocked,
            can_unlock: game.can_unlock_black_hole(),
            paused: game.black_holes.paused,
            negative_unlocked: game.black_hole_negative_unlocked(),
            negative: game.black_holes.negative,
            is_inverted: game.black_holes_are_negative(),
            auto_pause_mode: game.black_holes.auto_pause_mode,
            holes: (0..2usize)
                .map(|i| BlackHoleView {
                    unlocked: game.black_holes.holes[i].unlocked,
                    is_active: game.black_hole_is_active(i),
                    charged: game.black_holes.holes[i].active,
                    is_permanent: game.black_hole_is_permanent(i),
                    phase: game.black_holes.holes[i].phase,
                    interval: game.black_hole_interval(i),
                    power: game.black_hole_power(i),
                    duration: game.black_hole_duration(i),
                    activations: game.black_holes.holes[i].activations,
                    upgrade_costs: (0u8..3)
                        .map(|k| num(&game.black_hole_upgrade_cost(i, k)))
                        .collect(),
                    can_buy_upgrades: (0u8..3)
                        .map(|k| {
                            game.reality.machines >= game.black_hole_upgrade_cost(i, k)
                        })
                        .collect(),
                })
                .collect(),
        },
        perks_bought: game.reality.perks.iter().copied().collect(),
        perks_buyable: ad_core::PERKS
            .iter()
            .map(|p| p.id)
            .filter(|&id| game.can_buy_perk(id))
            .collect(),
    }
}

/// The combined value of active-glyph effect `bit` plus its softcap flag.
fn combined_glyph_effect(game: &GameState, bit: u8) -> (Decimal, bool) {
    let d = Decimal::from_float;
    match bit {
        0 => (d(game.glyph_effect_timepow()), false),
        1 => (d(game.glyph_effect_timespeed()), false),
        2 => (d(game.glyph_effect_timeetermult()), false),
        3 => (d(game.glyph_effect_time_ep()), false),
        4 => (game.glyph_effect_dilation_dt(), false),
        5 => {
            let value = game.glyph_effect_dilation_galaxy_threshold();
            (d(value), value < 0.4)
        }
        6 => (d(game.glyph_effect_dilation_ttgen()), false),
        7 => (d(game.glyph_effect_dilationpow()), false),
        8 => (game.glyph_effect_replicationspeed(), false),
        9 => (d(game.glyph_effect_replicationpow()), false),
        10 => (d(game.glyph_effect_replicationdtgain()), false),
        11 => {
            let value = game.glyph_effect_replicationglyphlevel_impl();
            (d(value), value > 0.1)
        }
        12 => (d(game.glyph_effect_infinitypow()), false),
        13 => (d(game.glyph_effect_infinityrate()), false),
        14 => (d(game.glyph_effect_infinity_ip()), false),
        15 => (game.glyph_effect_infinityinfmult(), false),
        16 => (d(game.glyph_effect_powerpow()), false),
        17 => (game.glyph_effect_powermult(), false),
        18 => (d(game.glyph_effect_powerdimboost()), false),
        19 => (d(game.glyph_effect_powerbuy10()), false),
        _ => (Decimal::ZERO, false),
    }
}

fn build_eternity_upgrades_view(game: &GameState) -> EternityUpgradesView {
    use ad_core::EternityUpgrade;
    let effect = |u: EternityUpgrade| -> Decimal {
        match u {
            EternityUpgrade::IdMultEp => game.eternity_points + Decimal::ONE,
            EternityUpgrade::IdMultEternities => game.eu2_effect_public(),
            EternityUpgrade::IdMultIcRecords => game.eu3_effect_public(),
            EternityUpgrade::TdMultAchievements => game.achievement_power(),
            EternityUpgrade::TdMultTheorems => game.time_theorems.max(&Decimal::ONE),
            EternityUpgrade::TdMultRealTime => Decimal::from_float(
                (game.records.total_time_played_ms / 86_400_000.0).max(1.0),
            ),
        }
    };
    EternityUpgradesView {
        upgrades: ad_core::ALL_ETERNITY_UPGRADES
            .iter()
            .map(|u| EternityUpgradeView {
                id: u.id(),
                cost: num(&u.cost()),
                is_bought: game.eternity_upgrade_bought(*u),
                can_buy: game.can_buy_eternity_upgrade(*u),
                effect: num(&effect(*u)),
            })
            .collect(),
        ep_mult_purchases: game.epmult_upgrades,
        ep_mult_cost: num(&game.ep_mult_cost()),
        ep_mult_effect: num(&game.ep_mult_effect()),
        can_buy_ep_mult: game.eternity_points >= game.ep_mult_cost(),
    }
}

fn build_eternity_challenges_view(game: &GameState) -> Vec<EternityChallengeView> {
    (1..=12u8)
        .map(|id| {
            let secondary = game.ec_secondary_requirement(id);
            EternityChallengeView {
                id,
                completions: game.eternity_challenge_completions(id),
                is_running: game.ec_running(id),
                is_unlocked: game.eternity_challenge_unlocked == id,
                can_unlock: game.can_buy_ec_study(id),
                study_cost: ad_core::ec_study_cost(id),
                goal: num(&game.ec_current_goal(id)),
                secondary_current: secondary.as_ref().map(|(c, _)| num(c)),
                secondary_required: secondary.as_ref().map(|(_, r)| num(r)),
            }
        })
        .collect()
}

/// The Statistics tab's snapshot (main page + Challenge records + Past
/// Prestige Runs).
fn build_statistics_view(game: &GameState) -> StatisticsView {
    let records = &game.records;
    // The banked-Infinities rate line: `projectedBanked / clampMin(33,
    // thisEternity.time) × 60000` — real time once Reality is unlocked (the
    // original's reality branch recomputes it with `realTime`).
    let projected_banked = game.banked_infinities_gain();
    let bank_divisor_ms = if game.reality_unlocked() {
        records.this_eternity.real_time_ms
    } else {
        records.this_eternity.time_ms
    }
    .max(33.0);
    let banked_rate_per_min =
        projected_banked / Decimal::from_float(bank_divisor_ms / 60_000.0);

    let run_view =
        |time_ms: f64, real_time_ms: f64, currency: &Decimal, count: &Decimal| {
            RecentRunView {
                time_ms,
                real_time_ms,
                currency: num(currency),
                count: num(count),
            }
        };

    StatisticsView {
        total_antimatter: num(&game.total_antimatter),
        real_time_played_ms: records.real_time_played_ms,
        total_time_played_ms: records.total_time_played_ms,
        game_created_time_ms: records.game_created_time_ms,
        infinities: num(&game.infinities),
        infinities_banked: num(&game.infinities_banked),
        best_infinity_time_ms: records.best_infinity.time_ms,
        this_infinity_time_ms: records.this_infinity.time_ms,
        this_infinity_real_time_ms: records.this_infinity.real_time_ms,
        best_ip_min: num(&records.best_infinity.best_ip_min_eternity),
        eternities: num(&game.eternities),
        projected_banked: num(&projected_banked),
        banked_rate_per_min: num(&banked_rate_per_min),
        best_eternity_time_ms: records.best_eternity.time_ms,
        this_eternity_time_ms: records.this_eternity.time_ms,
        this_eternity_real_time_ms: records.this_eternity.real_time_ms,
        best_ep_min: num(&records.best_eternity.best_ep_min_reality),
        realities: game.reality.realities,
        best_reality_time_ms: records.best_reality.time_ms,
        best_reality_real_time_ms: records.best_reality.real_time_ms,
        this_reality_time_ms: records.this_reality.time_ms,
        this_reality_real_time_ms: records.this_reality.real_time_ms,
        best_rm_min: num(&records.best_reality.rm_min),
        best_glyph_rarity: ad_core::strength_to_rarity(
            records.best_reality.glyph_strength,
        )
        .max(0.0),
        is_doomed: game.is_doomed(),
        nc_best_times_ms: game.nc_best_times_ms.to_vec(),
        ic_best_times_ms: game.ic_best_times_ms.to_vec(),
        recent_infinities: records
            .recent_infinities
            .iter()
            .map(|r| run_view(r.time_ms, r.real_time_ms, &r.ip, &r.infinities))
            .collect(),
        recent_eternities: records
            .recent_eternities
            .iter()
            .map(|r| run_view(r.time_ms, r.real_time_ms, &r.ep, &r.eternities))
            .collect(),
        recent_realities: records
            .recent_realities
            .iter()
            .map(|r| {
                run_view(
                    r.time_ms,
                    r.real_time_ms,
                    &r.rm,
                    &Decimal::from_float(r.reality_count),
                )
            })
            .collect(),
        shown_runs: ShownRunsView {
            infinity: game.shown_runs.infinity,
            eternity: game.shown_runs.eternity,
            reality: game.shown_runs.reality,
        },
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
        infinity_points: num(&game.infinity_points),
        infinities: num(&game.infinities),
        gained_infinity_points: num(&game.gained_infinity_points()),
        infinity_goal: num(&game.infinity_goal()),
        this_infinity_real_time_ms: game.records.this_infinity.real_time_ms,
        best_ip_min: num(&game.records.this_infinity.best_ip_min),
        best_ip_min_val: num(&game.records.this_infinity.best_ip_min_val),
        eternity_points: num(&game.eternity_points),
        eternities: num(&game.eternities),
        eternity_unlocked: game.eternity_unlocked,
        can_eternity: game.can_eternity(),
        eternity_goal: num(&game.eternity_goal()),
        gained_eternity_points: num(&game.gained_eternity_points()),
        this_eternity_real_time_ms: game.records.this_eternity.real_time_ms,
        best_ep_min: num(&game.records.this_eternity.best_ep_min),
        best_ep_min_val: num(&game.records.this_eternity.best_ep_min_val),
        time_dimensions: build_time_dimensions_view(game),
        time_studies: build_time_studies_view(game),
        eternity_challenges: build_eternity_challenges_view(game),
        eternity_upgrades: build_eternity_upgrades_view(game),
        dilation: build_dilation_view(game),
        reality: build_reality_view(game),
        eternity_challenges_unlocked: game.eternity_challenge_unlocked != 0
            || (1..=12).any(|id| game.eternity_challenge_completions(id) > 0),
        eternity_milestones: ad_core::ETERNITY_MILESTONES
            .iter()
            .map(|m| EternityMilestoneView {
                id: m.id,
                eternities: m.eternities,
                is_reached: game.eternity_milestone_reached(m.eternities),
            })
            .collect(),
        infinity_upgrades: build_infinity_upgrades_view(game),
        infinity_upgrades_bottom_row: build_infinity_upgrades_bottom_row_view(game),
        challenges_unlocked: game.challenges_unlocked(),
        challenges: build_challenges_view(game),
        infinity_challenges_unlocked: game.infinity_challenges_unlocked(),
        infinity_challenges: build_infinity_challenges_view(game),
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
        broke_infinity: game.broke_infinity,
        best_infinity_time_ms: game.records.best_infinity.time_ms,
        break_infinity_unlockable: game.break_infinity_unlockable(),
        has_big_crunch_goal: !game.broke_infinity || game.any_challenge_running(),
        autobuyers: build_autobuyers_view(game),
        break_infinity: build_break_infinity_view(game),
        infinity_dimensions: build_infinity_dimensions_view(game),
        replicanti: build_replicanti_view(game),
        unlocked_achievements: game.unlocked_achievement_ids(),
        achievement_power: num(&game.achievement_power()),
        tab_notifications: game.tab_notifications.iter().cloned().collect(),
        tutorial_state: game.tutorial_state,
        tutorial_active: game.tutorial_active,
        automator: build_automator_view(game),
        celestials: build_celestials_view(game),
        statistics: build_statistics_view(game),
        options: OptionsView {
            hotkeys: game.options.hotkeys,
            retry_challenge: game.options.retry_challenge,
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
                switch_automator_mode: game.options.confirmations.switch_automator_mode,
                big_crunch: game.options.confirmations.big_crunch,
                eternity: game.options.confirmations.eternity,
                dilation: game.options.confirmations.dilation,
            },
            animations: AnimationsView {
                big_crunch: game.options.animations.big_crunch,
            },
            show_hint_text: ShowHintTextView {
                show_percentage: game.options.show_hint_text.show_percentage,
                achievements: game.options.show_hint_text.achievements,
                achievement_unlock_states: game
                    .options
                    .show_hint_text
                    .achievement_unlock_states,
                challenges: game.options.show_hint_text.challenges,
            },
            away_progress: AwayProgressView {
                antimatter: game.options.away_progress.antimatter,
                dimension_boosts: game.options.away_progress.dimension_boosts,
                antimatter_galaxies: game.options.away_progress.antimatter_galaxies,
                infinities: game.options.away_progress.infinities,
                infinity_points: game.options.away_progress.infinity_points,
                replicanti: game.options.away_progress.replicanti,
                replicanti_galaxies: game.options.away_progress.replicanti_galaxies,
            },
            header_text_colored: game.options.header_text_colored,
            sidebar_resource_id: game.options.sidebar_resource_id,
            hidden_tab_bits: game.options.hidden_tab_bits,
            hidden_subtab_bits: game.options.hidden_subtab_bits.to_vec(),
            stat_tab_resources: game.options.stat_tab_resources,
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
    // Drain the Automator's queued `notify` toasts into this frame's view.
    let notifications =
        std::mem::take(&mut game.automator.runtime.pending_notifications);
    let mut view = build_game_view(&game);
    view.automator.notifications = notifications;
    view
}

/// Build the Automator tab view (`notifications` is filled by the tick
/// command, which drains the queue).
fn build_automator_view(game: &GameState) -> AutomatorTabView {
    use ad_core::automator::AutomatorMode;
    let unlocked = game.automator_unlocked();
    let state = &game.automator.state;
    let editor_script = state.editor_script;
    let current_chars = game
        .automator
        .scripts
        .get(&editor_script)
        .map(|s| s.content.len())
        .unwrap_or(0);
    AutomatorTabView {
        unlocked,
        is_on: game.automator_is_on(),
        is_running: game.automator_is_running(),
        mode: match state.mode {
            AutomatorMode::Pause => "pause",
            AutomatorMode::Run => "run",
            AutomatorMode::SingleStep => "singleStep",
        }
        .to_string(),
        current_line: game.automator_current_line().unwrap_or(0),
        top_level_script: state.top_level_script,
        editor_script,
        repeat: state.repeat,
        force_restart: state.force_restart,
        follow_execution: state.follow_execution,
        just_completed: game.automator.runtime.has_just_completed,
        interval_ms: game.automator_current_interval(),
        scripts: game
            .automator
            .scripts
            .iter()
            .map(|(id, s)| AutomatorScriptEntry {
                id: *id,
                name: s.name.clone(),
            })
            .collect(),
        running_script_name: game
            .automator
            .scripts
            .get(&state.top_level_script)
            .map(|s| s.name.clone())
            .unwrap_or_default(),
        current_script_chars: current_chars,
        total_script_chars: game.automator.total_script_chars(),
        constants: game
            .automator
            .constant_sort_order
            .iter()
            .map(|name| AutomatorConstantView {
                name: name.clone(),
                value: game
                    .automator
                    .constants
                    .get(name)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect(),
        current_info_pane: game.automator.current_info_pane,
        editor_type: match game.automator.editor_type {
            ad_core::automator::AutomatorEditorType::Text => "text",
            ad_core::automator::AutomatorEditorType::Block => "block",
        }
        .to_string(),
        notifications: Vec::new(),
        event_count: game.automator.runtime.events.len(),
        event_options: AutomatorEventOptionsView {
            newest_first: game.options.automator_events.newest_first,
            timestamp_type: game.options.automator_events.timestamp_type,
            max_entries: game.options.automator_events.max_entries,
            clear_on_reality: game.options.automator_events.clear_on_reality,
            clear_on_restart: game.options.automator_events.clear_on_restart,
        },
        points: if unlocked {
            None
        } else {
            Some(build_automator_points_view(game))
        },
    }
}

fn build_automator_points_view(game: &GameState) -> AutomatorPointsView {
    use ad_core::automator_points::{AUTOMATOR_UNLOCK_POINTS, UPGRADE_AUTOMATOR_POINTS};
    AutomatorPointsView {
        total: game.automator_points(),
        threshold: AUTOMATOR_UNLOCK_POINTS,
        from_perks: game.automator_points_from_perks(),
        from_upgrades: game.automator_points_from_upgrades(),
        perks: ad_core::perks::PERKS
            .iter()
            .filter(|p| p.automator_points > 0)
            .map(|p| ApSourceView {
                id: p.id,
                ap: p.automator_points,
                bought: game.perk_bought(p.id),
            })
            .collect(),
        upgrades: UPGRADE_AUTOMATOR_POINTS
            .iter()
            .map(|&(id, ap)| ApSourceView {
                id,
                ap,
                bought: game.reality_upgrade_bought(id),
            })
            .collect(),
        other: vec![
            ApOtherSourceView {
                name: "Reality Count",
                ap: 2 * (game.reality.realities).min(50),
            },
            ApOtherSourceView {
                name: "Black Hole",
                ap: if game.black_holes.holes[0].unlocked {
                    10
                } else {
                    0
                },
            },
        ],
    }
}

/// Replays `game_ms` of accumulated offline game-time (already speed-scaled by
/// the caller) at the resolution set by `offline_ticks`, returning the new view.
/// The all-at-once path, used for sub-threshold catch-ups where no progress modal
/// is shown. See `docs/design/2026-06-30-offline-progress.md`.
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

/// The engine's offline replay plan for `game_ms`: the total discrete tick count
/// and per-tick size (ms). The frontend splits `ticks` into batches, running
/// `tick_size_ms`-sized ticks itself to drive the offline catch-up progress bar.
#[derive(Serialize)]
struct OfflinePlan {
    ticks: u32,
    tick_size_ms: f64,
}

/// Returns the offline replay plan for `game_ms` at the chosen resolution, so the
/// GUI can run the catch-up in progress-bar-sized chunks (the budget policy stays
/// in the engine; the pacing lives in the webview).
#[tauri::command]
fn offline_plan(game_ms: f64, offline_ticks: u32) -> OfflinePlan {
    let (ticks, tick_size_ms) = core_offline_plan(game_ms, offline_ticks);
    OfflinePlan {
        ticks,
        tick_size_ms,
    }
}

/// Returns the current game view without advancing the engine. Used at startup to
/// seed the first snapshot before running any offline catch-up.
#[tauri::command]
fn get_state(state: State<'_, Mutex<GameState>>) -> GameView {
    let game = state.lock().unwrap();
    build_game_view(&game)
}

/// The offline gap (ms) detected at startup from the loaded save's `lastUpdate`,
/// awaiting replay. Consumed once by the frontend via [`take_pending_offline`];
/// zero when there is nothing to catch up.
#[derive(Default)]
struct PendingOffline(Mutex<f64>);

/// Returns the startup offline gap (ms) once, then clears it, so a reload of the
/// webview doesn't replay the same interval twice.
#[tauri::command]
fn take_pending_offline(pending: State<'_, PendingOffline>) -> f64 {
    let mut ms = pending.0.lock().unwrap();
    std::mem::take(&mut *ms)
}

/// A freshly loaded/imported state paired with the offline gap (ms) to replay.
/// The frontend installs `view`, then runs the catch-up over `offline_ms`.
#[derive(Serialize)]
struct LoadResult {
    view: GameView,
    offline_ms: f64,
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

#[tauri::command]
fn buy_dilation_study(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_dilation_study(id);
}

#[tauri::command]
fn buy_dilation_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_dilation_upgrade(id);
}

#[tauri::command]
fn toggle_dilation(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if game.dilation.active {
        game.exit_dilation();
    } else {
        game.start_dilated_eternity();
    }
}

#[tauri::command]
fn buy_eternity_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    if let Some(upgrade) = ad_core::EternityUpgrade::from_id(id) {
        state.lock().unwrap().buy_eternity_upgrade(upgrade);
    }
}

#[tauri::command]
fn buy_ep_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ep_mult();
}

#[tauri::command]
fn buy_max_ep_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_ep_mult();
}

#[tauri::command]
fn buy_ec_study(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ec_study(id);
}

#[tauri::command]
fn start_eternity_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().start_eternity_challenge(id);
}

#[tauri::command]
fn exit_eternity_challenge(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().exit_eternity_challenge();
}

#[tauri::command]
fn buy_time_study(id: u16, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_time_study(id);
}

#[tauri::command]
fn buy_time_theorem(currency: String, state: State<'_, Mutex<GameState>>) {
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
fn buy_max_time_theorems(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_time_theorems();
}

#[tauri::command]
fn set_respec(respec: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().set_respec(respec);
}

#[tauri::command]
fn buy_time_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().buy_time_dimension(tier);
    }
}

#[tauri::command]
fn buy_max_time_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().buy_max_time_dimension(tier);
    }
}

#[tauri::command]
fn max_all_time_dimensions(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().max_all_time_dimensions();
}

#[tauri::command]
fn eternity(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.eternity();
}

#[tauri::command]
fn break_infinity(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.break_infinity();
}

#[tauri::command]
fn do_reality(
    choice: Option<usize>,
    sacrifice: bool,
    state: State<'_, Mutex<GameState>>,
) {
    let mut game = state.lock().unwrap();
    game.reality_with_glyph_choice(choice, sacrifice);
}

#[tauri::command]
fn reset_reality(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.reset_reality();
}

#[tauri::command]
fn equip_glyph(id: u32, slot: Option<u32>, state: State<'_, Mutex<GameState>>) {
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
fn sacrifice_glyph(id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.sacrifice_glyph(id);
}

#[tauri::command]
fn move_glyph(id: u32, slot: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.move_glyph_to_slot(id, slot);
}

#[tauri::command]
fn set_glyph_respec(respec: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().reality.respec = respec;
}

#[tauri::command]
fn buy_perk(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_perk(id);
}

#[tauri::command]
fn buy_reality_rebuyable(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_reality_rebuyable(id);
}

#[tauri::command]
fn buy_reality_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_reality_upgrade(id);
}

// --- Celestials (Phase 7) --------------------------------------------------

/// Pour RM into Teresa for `diff_ms` of real time (the Teresa tab's pour
/// button, held down; the frontend passes the frame delta).
#[tauri::command]
fn teresa_pour_rm(diff_ms: f64, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().teresa_pour_rm(diff_ms);
}

/// Reset Teresa's pour-rate timer (the pour button was released).
#[tauri::command]
fn teresa_stop_pouring(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().teresa_stop_pouring();
}

/// Buy one level of a Teresa Perk-Shop rebuyable by id (0–3).
#[tauri::command]
fn buy_perk_shop(id: usize, state: State<'_, Mutex<GameState>>) {
    if let Some(&entry) = ad_core::celestials::teresa::PERK_SHOP_ENTRIES
        .iter()
        .find(|e| e.id == id)
    {
        state.lock().unwrap().buy_perk_shop(entry);
    }
}

/// Buy an Effarig Relic-Shard unlock by id.
#[tauri::command]
fn effarig_buy_unlock(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().effarig_buy_unlock(id);
}

/// Toggle Enslaved's game-time storage (charge/uncharge the Black Hole).
#[tauri::command]
fn toggle_store_game_time(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().toggle_store_game_time();
}

/// Discharge Enslaved's stored game time (a burst on the next tick).
#[tauri::command]
fn enslaved_release(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().enslaved_release_stored_time();
}

/// Buy an Enslaved unlock by id (0 softcap / 1 run) with stored game time.
#[tauri::command]
fn buy_enslaved_unlock(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_enslaved_unlock(id);
}

/// Unlock V (the celestial) once all six main conditions are met.
#[tauri::command]
fn v_unlock_celestial(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().v_unlock_celestial();
}

/// Enter a celestial's Reality (`teresa`/`effarig`/`enslaved`/`v`).
#[tauri::command]
fn start_celestial_reality(celestial: String, state: State<'_, Mutex<GameState>>) {
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
fn ra_level_up(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_level_up_max(pet);
}

#[tauri::command]
fn ra_buy_memory_upgrade(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_purchase_memory_upgrade(pet);
}

#[tauri::command]
fn ra_buy_chunk_upgrade(pet: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_purchase_chunk_upgrade(pet);
}

#[tauri::command]
fn ra_set_remembrance(pet: i8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().ra_set_remembrance(pet);
}

#[tauri::command]
fn alchemy_toggle_reaction(id: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().alchemy_toggle_reaction(id);
}

#[tauri::command]
fn dmd_buy_upgrade(tier: usize, kind: u8, state: State<'_, Mutex<GameState>>) {
    let mut g = state.lock().unwrap();
    match kind {
        0 => g.dmd_buy_interval(tier),
        1 => g.dmd_buy_power_dm(tier),
        2 => g.dmd_buy_power_de(tier),
        _ => false,
    };
}

#[tauri::command]
fn dmd_ascend(tier: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().dmd_ascend(tier);
}

#[tauri::command]
fn laitela_max_all_dmd(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().laitela_max_all_dmd();
}

#[tauri::command]
fn laitela_annihilate(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().annihilate(false);
}

#[tauri::command]
fn laitela_condense_singularity(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().condense_singularity();
}

#[tauri::command]
fn laitela_set_continuum(on: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().set_continuum(on);
}

#[tauri::command]
fn laitela_change_singularity_cap(increase: bool, state: State<'_, Mutex<GameState>>) {
    let mut g = state.lock().unwrap();
    if increase {
        g.singularity_increase_cap();
    } else {
        g.singularity_decrease_cap();
    }
}

#[tauri::command]
fn buy_imaginary_upgrade(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_imaginary_upgrade(id);
}

#[tauri::command]
fn buy_imaginary_rebuyable(id: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_imaginary_rebuyable(id);
}

#[tauri::command]
fn doom_reality(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().doom_reality();
}

#[tauri::command]
fn pelle_armageddon(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().armageddon(true);
}

#[tauri::command]
fn pelle_toggle_rift(rift: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().pelle_toggle_rift(rift);
}

#[tauri::command]
fn buy_pelle_upgrade(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_pelle_upgrade(id);
}

#[tauri::command]
fn buy_pelle_rebuyable(id: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_pelle_rebuyable(id);
}

#[tauri::command]
fn pelle_start_sacrifice(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().galaxy_generator_start_sacrifice();
}

#[tauri::command]
fn unlock_black_hole(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().unlock_black_hole();
}

#[tauri::command]
fn buy_black_hole_upgrade(hole: usize, kind: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_black_hole_upgrade(hole, kind);
}

/// Set the Black-Hole inversion strength as the slider exponent (0..=300 →
/// `10^-x`).
#[tauri::command]
fn set_black_hole_negative(exponent: f64, state: State<'_, Mutex<GameState>>) {
    let negative = 10f64.powf(-exponent.clamp(0.0, 300.0));
    state.lock().unwrap().set_black_hole_negative(negative);
}

/// Set the Black-Hole auto-pause mode (0 never / 1 before BH1 / 2 before BH2).
#[tauri::command]
fn set_black_hole_auto_pause(mode: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().black_holes.auto_pause_mode = mode.min(2);
}

#[tauri::command]
fn toggle_black_hole_pause(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().toggle_black_hole_pause();
}

/// Buy a one-time Break Infinity Upgrade by its original save id (e.g.
/// "totalMult"). An unrecognized id is a no-op.
#[tauri::command]
fn buy_break_infinity_upgrade(id: String, state: State<'_, Mutex<GameState>>) {
    if let Some(upgrade) = BreakInfinityUpgrade::from_save_id(&id) {
        state.lock().unwrap().buy_break_infinity_upgrade(upgrade);
    }
}

/// Buy one level of a rebuyable Break Infinity Upgrade by index (0/1/2).
#[tauri::command]
fn buy_break_infinity_rebuyable(id: usize, state: State<'_, Mutex<GameState>>) {
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
fn buy_infinity_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < INFINITY_DIMENSION_COUNT {
        state.lock().unwrap().buy_infinity_dimension(tier);
    }
}

/// Buy-max a single Infinity Dimension tier (0-indexed).
#[tauri::command]
fn buy_max_infinity_dimension(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < INFINITY_DIMENSION_COUNT {
        state.lock().unwrap().buy_max_infinity_dimension(tier);
    }
}

/// Buy-max all Infinity Dimensions.
#[tauri::command]
fn buy_max_all_infinity_dimensions(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_all_infinity_dimensions();
}

/// Unlock Replicanti (spends 1e140 IP); a no-op if already unlocked or unaffordable.
#[tauri::command]
fn unlock_replicanti(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().unlock_replicanti();
}

/// Acknowledge the tab-notification badge for `key` (`tabKey + subtabKey`);
/// called by the frontend when the player opens that tab.
#[tauri::command]
fn tab_notification_seen(key: String, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().tab_notification_seen(&key);
}

/// Buy one Replicanti chance upgrade (`+1%`).
#[tauri::command]
fn buy_replicanti_chance(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_chance();
}

/// Buy one Replicanti interval upgrade (`×0.9`).
#[tauri::command]
fn buy_replicanti_interval(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_interval();
}

/// Buy one Replicanti max-galaxies upgrade (`+1`).
#[tauri::command]
fn buy_replicanti_galaxy_cap(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_galaxy_cap();
}

/// Buy a Replicanti Galaxy (resets Replicanti; a no-op unless at the cap and below
/// the bought-galaxy cap).
#[tauri::command]
fn buy_replicanti_galaxy(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_replicanti_galaxy();
}

/// Buy an Infinity Upgrade by its original save id (e.g. "timeMult"). An
/// unrecognized id is a no-op.
#[tauri::command]
fn buy_infinity_upgrade(id: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if let Some(upgrade) = InfinityUpgrade::from_save_id(&id) {
        game.buy_infinity_upgrade(upgrade);
    }
}

/// Buy a single ×2 IP-multiplier (`ipMult`) purchase.
#[tauri::command]
fn buy_ip_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ip_mult();
}

/// Buy as many `ipMult` purchases as affordable (`buyMax`).
#[tauri::command]
fn buy_max_ip_mult(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_max_ip_mult();
}

/// Buy the one-time `ipOffline` Infinity Upgrade.
#[tauri::command]
fn buy_ip_offline(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().buy_ip_offline();
}

/// Toggle the IP-multiplier autobuyer (`Autobuyer.ipMult.isActive`).
#[tauri::command]
fn set_ip_mult_autobuyer(active: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().autobuyers.ip_mult_buyer_active = active;
}

/// Undo the last equipped glyph (Teresa's undo unlock).
#[tauri::command]
fn undo_glyph(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().undo_glyph();
}

/// Set Effarig's glyph-level factor weights (ep/repl/dt/eternities). Values
/// are clamped to 0..=100; the caller keeps the sum at 100 like the original
/// slider group.
#[tauri::command]
fn set_glyph_weights(weights: Vec<f64>, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    for (i, w) in weights.iter().take(4).enumerate() {
        game.celestials.effarig.glyph_weights[i] = w.clamp(0.0, 100.0);
    }
}

/// Create a Reality Glyph from the reality Alchemy resource.
#[tauri::command]
fn create_reality_glyph(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().create_reality_glyph();
}

/// Set the glyph filter's selection / rejection modes and the shared
/// effect-count threshold.
#[tauri::command]
fn set_glyph_filter_modes(
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
fn set_glyph_filter_type(
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
fn offline_currency_gain(away_ms: f64, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().offline_currency_gain(away_ms);
}

/// Start Normal Challenge `id` (2..=12); a no-op if it can't be started.
#[tauri::command]
fn start_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.start_challenge(id);
}

/// Exit the current challenge (Normal or Infinity; a no-op if none is running).
#[tauri::command]
fn exit_challenge(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.exit_challenge();
}

/// Start Infinity Challenge `id` (1..=8); a no-op if it can't be started.
#[tauri::command]
fn start_infinity_challenge(id: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.start_infinity_challenge(id);
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
fn upgrade_autobuyer_interval(target: String, state: State<'_, Mutex<GameState>>) {
    if let Some(target) = parse_autobuyer_target(&target) {
        state.lock().unwrap().upgrade_autobuyer_interval(target);
    }
}

/// Double an AD autobuyer's "Buys max" bulk (`upgradeBulk`), once its interval
/// is maxed.
#[tauri::command]
fn upgrade_ad_autobuyer_bulk(tier: usize, state: State<'_, Mutex<GameState>>) {
    if tier < 8 {
        state.lock().unwrap().upgrade_ad_autobuyer_bulk(tier);
    }
}

/// Toggle one of the milestone autobuyers (or its group flag). `kind` is
/// "infinityDims" / "replicantiUpgrades" / "replicantiGalaxy"; `index` selects
/// the entry, or `None` toggles the group flag.
#[tauri::command]
fn toggle_milestone_autobuyer(
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
fn toggle_autobuyer(target: String, state: State<'_, Mutex<GameState>>) {
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
fn set_prestige_autobuyer_mode(
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
fn set_prestige_autobuyer_value(
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
fn toggle_autobuyer_dynamic_amount(target: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match target.as_str() {
        "bigCrunch" => game.toggle_big_crunch_dynamic_amount(),
        "eternity" => game.toggle_eternity_dynamic_amount(),
        _ => {}
    }
}

#[tauri::command]
fn set_reality_autobuyer_mode(mode: String, state: State<'_, Mutex<GameState>>) {
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
fn set_reality_autobuyer_value(
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
fn automator_play(script_id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_play(script_id);
}

#[tauri::command]
fn automator_stop(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_stop();
}

/// Rewind: restart the running script from the top.
#[tauri::command]
fn automator_rewind(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_restart();
}

/// Single-step one command (starting the editor's script when off).
#[tauri::command]
fn automator_step(script_id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_step_once(Some(script_id));
}

/// Toggle one of the controls-bar settings: "repeat" / "forceRestart" /
/// "followExecution".
#[tauri::command]
fn automator_toggle_setting(setting: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match setting.as_str() {
        "repeat" => game.automator_toggle_repeat(),
        "forceRestart" => game.automator_toggle_force_restart(),
        "followExecution" => game.automator_toggle_follow_execution(),
        _ => {}
    }
}

#[tauri::command]
fn automator_select_script(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_select_editor_script(id);
}

/// Create a fresh script and open it in the editor. Returns its id (None at
/// the 20-script cap).
#[tauri::command]
fn automator_new_script(state: State<'_, Mutex<GameState>>) -> Option<u32> {
    let mut game = state.lock().unwrap();
    let id = game.automator_new_script()?;
    game.automator_select_editor_script(id);
    Some(id)
}

#[tauri::command]
fn automator_rename_script(id: u32, name: String, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_rename_script(id, &name);
}

#[tauri::command]
fn automator_delete_script(id: u32, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_delete_script(id);
}

/// The stored content of one script (the editor loads it when switching).
#[tauri::command]
fn get_automator_script(id: u32, state: State<'_, Mutex<GameState>>) -> String {
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
struct AutomatorSaveResult {
    saved: bool,
    errors: Vec<AutomatorErrorView>,
}

/// Save the editor's content (stops the script when it is the running one)
/// and recompile for the error panel/gutter.
#[tauri::command]
fn save_automator_script(
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
fn get_automator_errors(
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
fn automator_set_constant(
    name: String,
    value: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state.lock().unwrap().automator_set_constant(&name, &value)
}

#[tauri::command]
fn automator_rename_constant(
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
fn automator_delete_constant(name: String, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_delete_constant(&name);
}

#[tauri::command]
fn automator_set_info_pane(pane: u8, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator.current_info_pane = pane.min(7);
}

/// The event log plus the current play-time clock (for relative timestamps).
#[derive(Serialize)]
struct AutomatorEventLogView {
    now_play_time_ms: f64,
    events: Vec<AutomatorEventView>,
}

#[tauri::command]
fn get_automator_events(state: State<'_, Mutex<GameState>>) -> AutomatorEventLogView {
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
fn automator_clear_log(state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_clear_event_log();
}

/// Event-log display options ("newestFirst" / "clearOnReality" /
/// "clearOnRestart" booleans; "timestampType" 0–4).
#[tauri::command]
fn set_automator_event_option(
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
struct BlockifyView {
    blocks: Vec<ad_core::automator::blocks::BlockData>,
    lost_lines: usize,
}

#[tauri::command]
fn automator_blockify(id: u32, state: State<'_, Mutex<GameState>>) -> BlockifyView {
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
fn automator_blockify_text(
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
fn automator_set_editor_type(block: bool, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().automator_set_editor_type(block);
}

/// Template generation inputs, as typed in the modal (numbers arrive as
/// strings and are parsed like the autobuyer inputs).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TemplateParamsIn {
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
struct TemplateView {
    script: String,
    warnings: Vec<String>,
}

#[tauri::command]
fn automator_template(
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
fn automator_export_script(
    id: u32,
    state: State<'_, Mutex<GameState>>,
) -> Option<String> {
    state.lock().unwrap().automator_export_script(id)
}

/// Export one script plus the presets/constants it references.
#[tauri::command]
fn automator_export_full(id: u32, state: State<'_, Mutex<GameState>>) -> Option<String> {
    state.lock().unwrap().automator_export_full_data(id)
}

/// What an import string contains, for the modal preview.
#[derive(Serialize)]
struct ImportPreview {
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
fn automator_import_preview(
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
fn automator_import(
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
struct ScriptDataInfo {
    /// (1-based slot, name, studies).
    presets: Vec<(usize, String, String)>,
    constants: Vec<(String, String)>,
}

#[tauri::command]
fn automator_script_data_info(
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
fn study_preset_save(slot: usize, state: State<'_, Mutex<GameState>>) {
    state.lock().unwrap().save_study_preset(slot);
}

#[tauri::command]
fn study_preset_load(slot: usize, respec: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    if respec {
        game.respec_and_load_study_preset(slot);
    } else {
        game.load_study_preset(slot);
    }
}

#[tauri::command]
fn study_preset_rename(
    slot: usize,
    name: String,
    state: State<'_, Mutex<GameState>>,
) -> bool {
    state.lock().unwrap().set_study_preset_name(slot, &name)
}

#[tauri::command]
fn study_preset_edit(
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
fn study_tree_export(state: State<'_, Mutex<GameState>>) -> String {
    state.lock().unwrap().study_tree_export_string()
}

/// `TimeStudyTree.isValidImportString` for template-input validation.
#[tauri::command]
fn study_tree_is_valid(text: String) -> bool {
    ad_core::time_studies::is_valid_study_import(&text)
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

/// Set which resource pair the Past Prestige Runs tables show
/// (`statTabResources`, clamped to 0–3).
#[tauri::command]
fn set_stat_tab_resources(value: u8, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.stat_tab_resources = value.min(3);
}

/// Flip one Past Prestige Runs table's expand/collapse flag. `layer` is
/// "infinity" / "eternity" / "reality"; unknown names are ignored.
#[tauri::command]
fn toggle_shown_runs(layer: String, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    match layer.as_str() {
        "infinity" => game.shown_runs.infinity = !game.shown_runs.infinity,
        "eternity" => game.shown_runs.eternity = !game.shown_runs.eternity,
        "reality" => game.shown_runs.reality = !game.shown_runs.reality,
        _ => {}
    }
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

/// Flip a single animation toggle (original `player.options.animations.*`).
/// `kind` is the camelCase name (`bigCrunch`); an unknown name is ignored.
#[tauri::command]
fn set_animation(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_animation(&kind, enabled);
}

/// Flip a single info-display hint toggle (original
/// `player.options.showHintText.*`). `kind` is the camelCase name
/// (`showPercentage`, `achievements`, `achievementUnlockStates`, `challenges`).
#[tauri::command]
fn set_hint_text(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_hint_text(&kind, enabled);
}

/// Flip a single away-progress display toggle (original
/// `player.options.awayProgress.*`). `kind` is the camelCase resource name.
#[tauri::command]
fn set_away_progress(kind: String, enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_away_progress(&kind, enabled);
}

/// Toggles the relative prestige-gain text coloring (original
/// `headerTextColored`).
#[tauri::command]
fn set_header_text_colored(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.header_text_colored = enabled;
}

/// Toggles "Automatically retry challenges" (original `retryChallenge`): when on,
/// crunching inside an antimatter challenge re-enters it instead of exiting.
#[tauri::command]
fn set_retry_challenge(enabled: bool, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.retry_challenge = enabled;
}

/// Sets the sidebar resource (original `sidebarResourceID`; 0 = latest).
#[tauri::command]
fn set_sidebar_resource(id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.set_sidebar_resource(id);
}

/// Toggles a top-level tab's hidden bit (original tab ids; the current-tab and
/// non-hidable guards live in the frontend, which knows the open tab).
#[tauri::command]
fn toggle_tab_visibility(tab_id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.toggle_tab_visibility(tab_id);
}

/// Clears a top-level tab's hidden bit (used when unhiding a tab whose subtabs
/// were all hidden).
#[tauri::command]
fn unhide_tab(tab_id: u32, state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.unhide_tab(tab_id);
}

/// Toggles a subtab's hidden bit (original tab/subtab ids).
#[tauri::command]
fn toggle_subtab_visibility(
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
fn show_all_tabs(state: State<'_, Mutex<GameState>>) {
    let mut game = state.lock().unwrap();
    game.options.show_all_tabs();
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
async fn export_save_to_file(
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
async fn import_save_from_file(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(fresh_game()))
        .manage(PendingOffline::default())
        .setup(|app| {
            // Resolve the OS app-data dir (§12.1), load the on-disk root save into
            // the running game, and install the SaveManager. A missing/corrupt
            // save just starts fresh.
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let mut manager = SaveManager::new(dir);
            let (loaded, offline_ms) = manager.load(fresh_game(), now_ms());
            *app.state::<Mutex<GameState>>().lock().unwrap() = loaded;
            // Stash the away-time for the frontend to replay as offline progress
            // once it has mounted (see take_pending_offline).
            *app.state::<PendingOffline>().0.lock().unwrap() = offline_ms as f64;
            app.manage(Mutex::new(manager));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tick_and_get_state,
            simulate_offline,
            offline_plan,
            get_state,
            take_pending_offline,
            buy_dimension,
            buy_until_10,
            buy_tickspeed,
            buy_max_tickspeed,
            buy_dim_boost,
            buy_galaxy,
            sacrifice,
            max_all,
            big_crunch,
            eternity,
            buy_time_dimension,
            buy_max_time_dimension,
            max_all_time_dimensions,
            buy_time_study,
            buy_dilation_study,
            buy_dilation_upgrade,
            toggle_dilation,
            buy_eternity_upgrade,
            buy_ep_mult,
            buy_max_ep_mult,
            buy_ec_study,
            start_eternity_challenge,
            exit_eternity_challenge,
            buy_time_theorem,
            buy_max_time_theorems,
            set_respec,
            break_infinity,
            do_reality,
            reset_reality,
            equip_glyph,
            sacrifice_glyph,
            move_glyph,
            set_glyph_respec,
            buy_perk,
            buy_reality_rebuyable,
            buy_reality_upgrade,
            teresa_pour_rm,
            teresa_stop_pouring,
            buy_perk_shop,
            effarig_buy_unlock,
            toggle_store_game_time,
            enslaved_release,
            buy_enslaved_unlock,
            v_unlock_celestial,
            start_celestial_reality,
            ra_level_up,
            ra_buy_memory_upgrade,
            ra_buy_chunk_upgrade,
            ra_set_remembrance,
            alchemy_toggle_reaction,
            dmd_buy_upgrade,
            dmd_ascend,
            laitela_max_all_dmd,
            laitela_annihilate,
            laitela_condense_singularity,
            laitela_set_continuum,
            laitela_change_singularity_cap,
            buy_imaginary_upgrade,
            buy_imaginary_rebuyable,
            doom_reality,
            pelle_armageddon,
            pelle_toggle_rift,
            buy_pelle_upgrade,
            buy_pelle_rebuyable,
            pelle_start_sacrifice,
            unlock_black_hole,
            buy_black_hole_upgrade,
            toggle_black_hole_pause,
            set_black_hole_negative,
            set_black_hole_auto_pause,
            buy_break_infinity_upgrade,
            buy_break_infinity_rebuyable,
            buy_infinity_dimension,
            buy_max_infinity_dimension,
            buy_max_all_infinity_dimensions,
            unlock_replicanti,
            tab_notification_seen,
            buy_replicanti_chance,
            buy_replicanti_interval,
            buy_replicanti_galaxy_cap,
            buy_replicanti_galaxy,
            buy_infinity_upgrade,
            buy_ip_mult,
            buy_max_ip_mult,
            buy_ip_offline,
            set_ip_mult_autobuyer,
            undo_glyph,
            set_glyph_weights,
            create_reality_glyph,
            set_glyph_filter_modes,
            set_glyph_filter_type,
            offline_currency_gain,
            start_challenge,
            exit_challenge,
            start_infinity_challenge,
            hard_reset,
            unlock_ad_autobuyer,
            toggle_ad_autobuyer,
            toggle_ad_autobuyer_mode,
            unlock_tickspeed_autobuyer,
            toggle_tickspeed_autobuyer,
            toggle_autobuyers,
            set_all_autobuyers_active,
            upgrade_autobuyer_interval,
            upgrade_ad_autobuyer_bulk,
            toggle_milestone_autobuyer,
            toggle_autobuyer,
            set_hotkeys,
            set_update_rate,
            set_stat_tab_resources,
            toggle_shown_runs,
            set_notation,
            set_notation_digits,
            set_offline_ticks,
            set_animation,
            set_hint_text,
            set_away_progress,
            set_header_text_colored,
            set_retry_challenge,
            set_sidebar_resource,
            toggle_tab_visibility,
            unhide_tab,
            toggle_subtab_visibility,
            show_all_tabs,
            set_confirmation,
            set_autosave_interval,
            set_show_time_since_save,
            set_save_file_name,
            export_save,
            import_save,
            export_save_to_file,
            import_save_from_file,
            save_game,
            set_prestige_autobuyer_mode,
            set_prestige_autobuyer_value,
            toggle_autobuyer_dynamic_amount,
            set_reality_autobuyer_mode,
            set_reality_autobuyer_value,
            study_preset_save,
            study_preset_load,
            study_preset_rename,
            study_preset_edit,
            study_tree_export,
            study_tree_is_valid,
            automator_play,
            automator_stop,
            automator_rewind,
            automator_step,
            automator_toggle_setting,
            automator_select_script,
            automator_new_script,
            automator_rename_script,
            automator_delete_script,
            get_automator_script,
            save_automator_script,
            get_automator_errors,
            automator_set_constant,
            automator_rename_constant,
            automator_delete_constant,
            automator_set_info_pane,
            get_automator_events,
            automator_clear_log,
            set_automator_event_option,
            automator_blockify,
            automator_blockify_text,
            automator_set_editor_type,
            automator_template,
            automator_export_script,
            automator_export_full,
            automator_import_preview,
            automator_import,
            automator_script_data_info,
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
    fn statistics_view_reflects_records() {
        let mut game = GameState::new();
        // A fresh game: no runs, sentinel best times, unknown creation time.
        let view = build_statistics_view(&game);
        assert_eq!(view.game_created_time_ms, 0.0);
        assert_eq!(view.nc_best_times_ms.len(), 11);
        assert_eq!(view.ic_best_times_ms.len(), 8);
        assert_eq!(view.recent_infinities.len(), 10);
        assert_eq!(view.recent_infinities[0].time_ms, f64::MAX);
        assert!(view.shown_runs.infinity);
        assert_eq!(view.projected_banked.m, 0.0);

        // With Achievement 131, the projected bank mirrors the engine helper
        // (floor(infinities × 0.05)) and the rate divisor is the clamped
        // this-eternity time.
        game.infinities = Decimal::from_float(1000.0);
        // Unlock Achievement 131 (row 13 / column 1) directly via the bitmask.
        game.achievement_bits[12] |= 1;
        game.records.this_eternity.time_ms = 120_000.0;
        game.records.game_created_time_ms = 1_650_000_000_000.0;
        let view = build_statistics_view(&game);
        assert_eq!(view.projected_banked.m, 5.0);
        assert_eq!(view.projected_banked.e, 1.0);
        // 50 banked / 2 minutes = 25 per minute.
        assert_eq!(view.banked_rate_per_min.m, 2.5);
        assert_eq!(view.banked_rate_per_min.e, 1.0);
        assert_eq!(view.game_created_time_ms, 1_650_000_000_000.0);

        // The whole snapshot (including the statistics view) serializes.
        let json = serde_json::to_string(&build_game_view(&game)).unwrap();
        assert!(json.contains("\"statistics\""));
    }

    #[test]
    fn automator_view_reflects_lock_state() {
        let mut game = GameState::new();
        // Locked: the AP page payload is included.
        let view = build_automator_view(&game);
        assert!(!view.unlocked);
        let points = view.points.expect("locked tab ships AP breakdown");
        assert_eq!(points.threshold, 100);
        assert_eq!(points.perks.len(), 21);
        assert_eq!(points.upgrades.len(), 6);

        // Unlocked: run state + scripts, no AP payload.
        game.reality.automator_force_unlock = true;
        game.automator_save_script(1, "pause 10s");
        game.automator_start(Some(1));
        let view = build_automator_view(&game);
        assert!(view.unlocked);
        assert!(view.points.is_none());
        assert!(view.is_running);
        assert_eq!(view.scripts.len(), 1);
        assert_eq!(view.current_script_chars, "pause 10s".len());
        assert_eq!(view.interval_ms, 500.0);
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
