//! The **write path**: produce a complete, original-game-loadable save from a
//! [`GameState`].
//!
//! Requirement (2) of the save design is that a save we write must be a
//! *complete, valid* `player` object the real game accepts — even though we only
//! model a slice of the game. We satisfy this by **templating** (§5): we start
//! from a baked copy of a real fresh-start `player` (every field, every empty
//! `Set`-as-`[]`, `version: 25`, the full `options`/`auto`/`records` trees) and
//! overlay only our modelled fields onto it (the inverse of the §2.3 mapping
//! table). The original game fills in nothing — the object is already complete —
//! and runs no migrations (`version: 25`).
//!
//! The template, [`DEFAULT_PLAYER_TEMPLATE`], is vendored in `default_player.json`.
//! It is a fresh-start save decoded from `tests/fixtures/ad_initial_save.txt`
//! (serializer format `AAB`, `player.version` 25). Per §10 this is regenerated
//! manually; to refresh it, decode a new fresh-start save from the pinned game build
//! and overwrite the file.
//!
//! [`encode_save`] is pure and deterministic: the only time-varying input,
//! `lastUpdate`, is a caller-supplied timestamp, so `ad-core` stays free of the
//! wall clock.

use break_infinity::Decimal;
use serde_json::{json, Value};

use crate::autobuyers::AutobuyerMode;
use crate::break_infinity_upgrades::ALL_BREAK_INFINITY_UPGRADES;
use crate::infinity_upgrades::ALL_INFINITY_UPGRADES;
use crate::save::codec::encode_pipeline;
use crate::state::GameState;

/// A complete, valid `player` object (fresh start, `version: 25`) used as the
/// overlay base. See the module docs for provenance and regeneration.
const DEFAULT_PLAYER_TEMPLATE: &str = include_str!("default_player.json");

/// The original `AUTOBUYER_MODE` numeric values.
const AUTOBUYER_MODE_BUY_SINGLE: i64 = 1;
const AUTOBUYER_MODE_BUY_10: i64 = 10;

/// Encodes a [`GameState`] into an AD save string the original game can import.
///
/// `now_ms` is the wall-clock time (epoch milliseconds) written to
/// `player.lastUpdate`; passing the real import time avoids spurious offline
/// progress when the save is loaded. The engine itself never reads the clock.
pub fn encode_save(state: &GameState, now_ms: i64) -> String {
    let player = to_player_value(state, now_ms);
    encode_pipeline(
        &serde_json::to_string(&player).expect("player Value always serializes"),
    )
}

/// Overlays `state` onto a fresh copy of the template, returning the complete
/// `player` JSON [`Value`].
///
/// This is the shared building block for both a single-player save
/// ([`encode_save`]) and the multi-player bundles (the localStorage-root and
/// backup-bundle shapes in [`super::bundle`]), which assemble several of these
/// player objects into one JSON document before running the byte pipeline.
pub fn to_player_value(state: &GameState, now_ms: i64) -> Value {
    let mut player: Value = serde_json::from_str(DEFAULT_PLAYER_TEMPLATE)
        .expect("vendored default_player.json is valid JSON");

    overlay(&mut player, state, now_ms);

    player
}

/// Writes our modelled fields onto the complete template `player` object,
/// replacing values in place (never removing keys, so the object stays complete).
fn overlay(player: &mut Value, state: &GameState, now_ms: i64) {
    // Scalars / Decimals (Decimals as JSON strings, matching break_infinity.js).
    player["antimatter"] = decimal(&state.antimatter);
    player["records"]["totalAntimatter"] = decimal(&state.total_antimatter);
    player["sacrificed"] = decimal(&state.sacrificed);
    player["dimensionBoosts"] = json!(state.dim_boosts);
    player["galaxies"] = json!(state.galaxies);
    player["totalTickBought"] = json!(state.tickspeed.bought);
    // `break` carries the Break-Infinity flag. `infinity_unlocked` is not stored
    // separately — the load derives it from break / infinities / IP.
    player["break"] = json!(state.broke_infinity);
    // Infinity currency (Decimal strings, matching the save schema).
    player["infinityPoints"] = decimal(&state.infinity_points);
    player["infinities"] = decimal(&state.infinities);
    // Infinity Upgrades + one-time Break Infinity Upgrades share the id set:
    // write the owned ids from both, plus the ipGen fractional accumulator and the
    // rebuyable Break Infinity Upgrade counts.
    let mut owned_upgrades: Vec<&str> = ALL_INFINITY_UPGRADES
        .iter()
        .filter(|u| state.infinity_upgrade_bought(**u))
        .map(|u| u.save_id())
        .collect();
    owned_upgrades.extend(
        ALL_BREAK_INFINITY_UPGRADES
            .iter()
            .filter(|u| state.break_infinity_upgrade_bought(**u))
            .map(|u| u.save_id()),
    );
    player["infinityUpgrades"] = json!(owned_upgrades);
    player["infinityRebuyables"] = json!(state.infinity_rebuyables);
    player["partInfinityPoint"] = json!(state.part_infinity_point);
    // Normal- and infinity-challenge run state.
    player["challenge"]["normal"]["current"] = json!(state.challenge.current);
    player["challenge"]["normal"]["completedBits"] = json!(state.challenge.completed);
    player["challenge"]["infinity"]["current"] = json!(state.infinity_challenge.current);
    player["challenge"]["infinity"]["completedBits"] =
        json!(state.infinity_challenge.completed);
    // Per-run challenge accumulators: NC8's running sacrifice product, NC2's
    // production factor (a plain number), NC3's 1st-dim multiplier, and NC11
    // normal matter (Decimal strings).
    player["chall8TotalSacrifice"] = decimal(&state.chall8_total_sacrifice);
    player["chall2Pow"] = json!(state.chall2_pow);
    player["chall3Pow"] = decimal(&state.chall3_pow);
    player["matter"] = decimal(&state.matter);

    // Time / infinity records. `records.totalAntimatter` is written above; here we
    // add the time and infinity-timing slice.
    let records = &mut player["records"];
    records["totalTimePlayed"] = json!(state.records.total_time_played_ms);
    records["realTimePlayed"] = json!(state.records.real_time_played_ms);
    records["thisInfinity"]["time"] = json!(state.records.this_infinity.time_ms);
    records["thisInfinity"]["realTime"] =
        json!(state.records.this_infinity.real_time_ms);
    records["thisInfinity"]["maxAM"] = decimal(&state.records.this_infinity.max_am);
    records["bestInfinity"]["time"] = json!(state.records.best_infinity.time_ms);
    records["bestInfinity"]["realTime"] =
        json!(state.records.best_infinity.real_time_ms);
    records["thisEternity"]["maxAM"] = decimal(&state.records.max_am_this_eternity);
    // Achievement bitmask, written back verbatim (one int per row).
    player["achievementBits"] = json!(state.achievement_bits);
    // Tutorial-highlight progress (at the player root, not under options).
    player["tutorialState"] = json!(state.tutorial_state);
    player["tutorialActive"] = json!(state.tutorial_active);
    // Stamp the save time so the game computes ~0 offline progress on import.
    player["lastUpdate"] = json!(now_ms);

    // Antimatter dimensions: amount, purchase count, and NC9 cost bumps.
    for (tier, dim) in state.dimensions.iter().enumerate() {
        let entry = &mut player["dimensions"]["antimatter"][tier];
        entry["amount"] = decimal(&dim.amount);
        entry["bought"] = json!(dim.bought);
        entry["costBumps"] = json!(dim.cost_bumps);
    }
    // NC9 tickspeed cost bumps.
    player["chall9TickspeedCostBumps"] = json!(state.tickspeed.cost_bumps);

    // Infinity Dimensions + Infinity Power.
    player["infinityPower"] = decimal(&state.infinity_power);
    for (tier, d) in state.infinity_dimensions.iter().enumerate() {
        let entry = &mut player["dimensions"]["infinity"][tier];
        entry["amount"] = decimal(&d.amount);
        entry["cost"] = decimal(&d.cost);
        entry["baseAmount"] = json!(d.base_amount);
        entry["isUnlocked"] = json!(d.is_unlocked);
    }

    // Options.
    let options = &mut player["options"];
    options["hotkeys"] = json!(state.options.hotkeys);
    options["updateRate"] = json!(state.options.update_rate);
    options["notation"] = json!(state.options.notation);
    options["notationDigits"]["comma"] = json!(state.options.notation_digits_comma);
    options["notationDigits"]["notation"] =
        json!(state.options.notation_digits_notation);
    options["offlineTicks"] = json!(state.options.offline_ticks);
    options["autosaveInterval"] = json!(state.options.autosave_interval);
    options["showTimeSinceSave"] = json!(state.options.show_time_since_save);
    options["saveFileName"] = json!(state.options.save_file_name);
    let confirmations = &mut options["confirmations"];
    confirmations["dimensionBoost"] = json!(state.options.confirmations.dimension_boost);
    confirmations["antimatterGalaxy"] =
        json!(state.options.confirmations.antimatter_galaxy);
    confirmations["sacrifice"] = json!(state.options.confirmations.sacrifice);
    confirmations["bigCrunch"] = json!(state.options.confirmations.big_crunch);

    // Autobuyers. `lastTick`/`bulk` stay the template's derived state; we write the
    // flags/modes plus the interval-upgrade state (interval + IP cost, Feature 2.6).
    player["auto"]["autobuyersOn"] = json!(state.autobuyers.enabled);
    for (tier, ab) in state.autobuyers.dimensions.iter().enumerate() {
        let entry = &mut player["auto"]["antimatterDims"]["all"][tier];
        entry["isActive"] = json!(ab.is_active);
        entry["isBought"] = json!(ab.is_bought);
        entry["mode"] = json!(mode_to_raw(ab.mode));
        entry["interval"] = json!(ab.interval_ms);
        entry["cost"] = json!(ab.cost);
    }
    let tickspeed = &mut player["auto"]["tickspeed"];
    tickspeed["isActive"] = json!(state.autobuyers.tickspeed.is_active);
    tickspeed["isBought"] = json!(state.autobuyers.tickspeed.is_bought);
    tickspeed["mode"] = json!(mode_to_raw(state.autobuyers.tickspeed.mode));
    tickspeed["interval"] = json!(state.autobuyers.tickspeed.interval_ms);
    tickspeed["cost"] = json!(state.autobuyers.tickspeed.cost);
    // Prestige autobuyers (Dim Boost / Galaxy / Big Crunch): active flag +
    // interval-upgrade state. Their limit/mode config stays at the template default.
    for (key, ab) in [
        ("dimBoost", &state.autobuyers.dim_boost),
        ("galaxy", &state.autobuyers.galaxy),
        ("bigCrunch", &state.autobuyers.big_crunch),
    ] {
        let entry = &mut player["auto"][key];
        entry["isActive"] = json!(ab.is_active);
        entry["interval"] = json!(ab.interval_ms);
        entry["cost"] = json!(ab.cost);
    }
}

/// A `Decimal` as the JSON string the original stores (`Decimal::toJSON =
/// toString`).
fn decimal(value: &Decimal) -> Value {
    Value::String(value.to_string())
}

/// Maps our [`AutobuyerMode`] back to the original numeric `AUTOBUYER_MODE`.
fn mode_to_raw(mode: AutobuyerMode) -> i64 {
    match mode {
        AutobuyerMode::BuyMax => AUTOBUYER_MODE_BUY_10,
        AutobuyerMode::BuySingle => AUTOBUYER_MODE_BUY_SINGLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infinity_upgrades::InfinityUpgrade;
    use crate::save::{decode_pipeline, decode_save};

    const INITIAL_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_initial_save.txt"
    ));
    const SAMPLE_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_sample_save.txt"
    ));

    #[test]
    fn template_is_complete_valid_player() {
        // The vendored template must parse and be a fresh, migration-free player.
        let player: Value = serde_json::from_str(DEFAULT_PLAYER_TEMPLATE).unwrap();
        assert_eq!(player["version"], 25);
        assert_eq!(
            player["dimensions"]["antimatter"].as_array().unwrap().len(),
            8
        );
        assert_eq!(
            player["auto"]["antimatterDims"]["all"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
    }

    #[test]
    fn produced_save_is_well_formed_and_complete() {
        let state = decode_save(SAMPLE_SAVE.trim()).unwrap();
        let encoded = encode_save(&state, 1_700_000_000_000);

        // It must decode back to JSON via the standard pipeline...
        let json = decode_pipeline(&encoded).unwrap();
        let player: Value = serde_json::from_str(&json).unwrap();

        // ...stay a complete player (template keys preserved)...
        assert_eq!(player["version"], 25);
        assert!(player.as_object().unwrap().len() > 60);

        // ...with our overlaid fields and the supplied timestamp.
        assert_eq!(player["antimatter"], "16613773273375400000");
        assert_eq!(player["galaxies"], 1);
        assert_eq!(player["totalTickBought"], 12);
        assert_eq!(player["dimensions"]["antimatter"][0]["bought"], 50);
        assert_eq!(player["lastUpdate"], 1_700_000_000_000_i64);
        // Decimals are written as JSON strings.
        assert!(player["records"]["totalAntimatter"].is_string());
    }

    #[test]
    fn round_trips_through_our_codec() {
        // decode → encode → decode reproduces every modelled field.
        for fixture in [INITIAL_SAVE, SAMPLE_SAVE] {
            let state = decode_save(fixture.trim()).unwrap();
            let reloaded = decode_save(&encode_save(&state, 1_700_000_000_000)).unwrap();

            assert_eq!(reloaded.antimatter, state.antimatter);
            assert_eq!(reloaded.total_antimatter, state.total_antimatter);
            assert_eq!(reloaded.sacrificed, state.sacrificed);
            assert_eq!(reloaded.dim_boosts, state.dim_boosts);
            assert_eq!(reloaded.galaxies, state.galaxies);
            assert_eq!(reloaded.tickspeed.bought, state.tickspeed.bought);
            assert_eq!(reloaded.infinity_unlocked, state.infinity_unlocked);
            assert_eq!(reloaded.infinity_points, state.infinity_points);
            assert_eq!(reloaded.infinities, state.infinities);
            assert_eq!(reloaded.infinity_upgrades, state.infinity_upgrades);
            assert_eq!(reloaded.part_infinity_point, state.part_infinity_point);
            assert_eq!(reloaded.challenge, state.challenge);
            assert_eq!(reloaded.records, state.records);
            for tier in 0..8 {
                assert_eq!(
                    reloaded.dimensions[tier].amount,
                    state.dimensions[tier].amount
                );
                assert_eq!(
                    reloaded.dimensions[tier].bought,
                    state.dimensions[tier].bought
                );
            }
            assert_eq!(reloaded.autobuyers.enabled, state.autobuyers.enabled);
            assert_eq!(reloaded.options, state.options);
        }
    }

    #[test]
    fn challenge_accumulators_round_trip() {
        // The per-run challenge accumulators survive encode → decode: chall2Pow
        // as a plain number, chall3Pow / matter / chall8TotalSacrifice as Decimals.
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.chall2_pow = 0.375;
        state.chall3_pow = Decimal::from_float(1234.5);
        state.matter = Decimal::new(1.0, 200);
        state.chall8_total_sacrifice = Decimal::new(2.5, 50);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.chall2_pow, 0.375);
        assert_eq!(reloaded.chall3_pow, Decimal::from_float(1234.5));
        assert_eq!(reloaded.matter, Decimal::new(1.0, 200));
        assert_eq!(reloaded.chall8_total_sacrifice, Decimal::new(2.5, 50));
    }

    #[test]
    fn infinity_dimensions_round_trip() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.infinity_power = Decimal::new(1.0, 50);
        state.infinity_dimensions[0].is_unlocked = true;
        state.infinity_dimensions[0].base_amount = 40;
        state.infinity_dimensions[0].amount = Decimal::from_float(1234.5);
        state.infinity_dimensions[0].cost = Decimal::new(1.0, 20);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.infinity_power, Decimal::new(1.0, 50));
        assert!(reloaded.infinity_dimensions[0].is_unlocked);
        assert_eq!(reloaded.infinity_dimensions[0].base_amount, 40);
        assert_eq!(
            reloaded.infinity_dimensions[0].amount,
            Decimal::from_float(1234.5)
        );
        assert_eq!(reloaded.infinity_dimensions[0].cost, Decimal::new(1.0, 20));
    }

    #[test]
    fn infinity_challenge_state_round_trips() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.infinity_challenge.current = 3;
        state.infinity_challenge.completed = (1 << 1) | (1 << 5);
        state.records.max_am_this_eternity = Decimal::new(1.0, 14000);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.infinity_challenge.current, 3);
        assert!(reloaded.infinity_challenge_completed(1));
        assert!(reloaded.infinity_challenge_completed(5));
        assert!(!reloaded.infinity_challenge_completed(2));
        assert_eq!(
            reloaded.records.max_am_this_eternity,
            Decimal::new(1.0, 14000)
        );
    }

    #[test]
    fn break_infinity_upgrades_round_trip() {
        // The one-time Break Infinity Upgrades (sharing the `infinityUpgrades`
        // array with the Infinity Upgrades) and the 3 rebuyable counts round-trip.
        use crate::{BreakInfinityUpgrade, InfinityUpgrade};
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.broke_infinity = true;
        state.infinity_upgrades = InfinityUpgrade::TotalTimeMult.bit();
        state.break_infinity_upgrades = BreakInfinityUpgrade::TotalAmMult.bit()
            | BreakInfinityUpgrade::GalaxyBoost.bit()
            | BreakInfinityUpgrade::AutobuyerSpeed.bit();
        state.infinity_rebuyables = [3, 1, 5];

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        // Both upgrade sets survive the shared array without cross-contamination.
        assert_eq!(reloaded.infinity_upgrades, state.infinity_upgrades);
        assert_eq!(
            reloaded.break_infinity_upgrades,
            state.break_infinity_upgrades
        );
        assert!(
            reloaded.break_infinity_upgrade_bought(BreakInfinityUpgrade::GalaxyBoost)
        );
        assert!(
            !reloaded.break_infinity_upgrade_bought(BreakInfinityUpgrade::CurrentAmMult)
        );
        assert_eq!(reloaded.infinity_rebuyables, [3, 1, 5]);
    }

    #[test]
    fn autobuyer_interval_upgrades_round_trip() {
        // Interval-upgrade state (interval + IP cost) survives encode → decode,
        // for both AD tiers and the tickspeed autobuyer.
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.autobuyers.dimensions[0].interval_ms = 108.0;
        state.autobuyers.dimensions[0].cost = 8.0;
        state.autobuyers.tickspeed.interval_ms = 180.0;
        state.autobuyers.tickspeed.cost = 4.0;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.autobuyers.dimensions[0].interval_ms, 108.0);
        assert_eq!(reloaded.autobuyers.dimensions[0].cost, 8.0);
        assert_eq!(reloaded.autobuyers.tickspeed.interval_ms, 180.0);
        assert_eq!(reloaded.autobuyers.tickspeed.cost, 4.0);
    }

    #[test]
    fn prestige_autobuyers_round_trip() {
        // The Dim Boost / Galaxy / Big Crunch autobuyers' active flag +
        // interval-upgrade state survive encode → decode.
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.autobuyers.dim_boost.interval_ms = 2400.0;
        state.autobuyers.dim_boost.cost = 4.0;
        state.autobuyers.dim_boost.is_active = false;
        state.autobuyers.galaxy.interval_ms = 12000.0;
        state.autobuyers.big_crunch.interval_ms = 100.0;
        state.autobuyers.big_crunch.cost = 32768.0;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.autobuyers.dim_boost.interval_ms, 2400.0);
        assert_eq!(reloaded.autobuyers.dim_boost.cost, 4.0);
        assert!(!reloaded.autobuyers.dim_boost.is_active);
        assert_eq!(reloaded.autobuyers.galaxy.interval_ms, 12000.0);
        assert_eq!(reloaded.autobuyers.big_crunch.interval_ms, 100.0);
        assert_eq!(reloaded.autobuyers.big_crunch.cost, 32768.0);
    }

    #[test]
    fn cost_bumps_round_trip() {
        // NC9 cost bumps survive encode → decode for dimensions and tickspeed,
        // and the derived tickspeed cost reflects bought + bumps.
        use crate::state::TickspeedState;
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.dimensions[2].cost_bumps = 3;
        state.dimensions[5].cost_bumps = 1;
        state.tickspeed = TickspeedState::with_bought_and_bumps(12, 4);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.dimensions[2].cost_bumps, 3);
        assert_eq!(reloaded.dimensions[5].cost_bumps, 1);
        assert_eq!(reloaded.tickspeed.cost_bumps, 4);
        assert_eq!(reloaded.tickspeed.bought, 12);
        assert_eq!(
            reloaded.tickspeed.cost,
            TickspeedState::with_bought_and_bumps(12, 4).cost
        );
    }

    #[test]
    fn overlays_modelled_state_changes() {
        // Mutate a freshly-loaded state, then confirm the change survives a
        // round-trip (i.e. we actually overlay, not just echo the template).
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.galaxies = 7;
        state.dim_boosts = 3;
        state.broke_infinity = true;
        state.infinity_points = Decimal::from_float(5.0);
        state.infinities = Decimal::from_float(3.0);
        // Own a couple of Infinity Upgrades (with their column prereqs) + a partial
        // ipGen accumulator.
        state.infinity_upgrades = InfinityUpgrade::TotalTimeMult.bit()
            | InfinityUpgrade::Dim18Mult.bit()
            | InfinityUpgrade::Buy10Mult.bit();
        state.part_infinity_point = 0.42;
        // In challenge 3, with challenges 1 and 2 completed.
        state.challenge.current = 3;
        state.challenge.completed = (1 << 1) | (1 << 2);
        state.records.total_time_played_ms = 123_456.0;
        state.records.this_infinity.time_ms = 7_890.0;
        state.records.this_infinity.max_am = Decimal::new(1.0, 250);
        state.records.best_infinity.time_ms = 42_000.0;
        state.dimensions[2].bought = 42;
        state.options.set_notation("Engineering");
        // Unlock a couple of achievements (18 → bits[0] bit 7; 21 → bits[1] bit 0).
        state.unlock_achievement(18);
        state.unlock_achievement(21);
        // Tutorial progress (player root): advanced to TICKSPEED, glow cleared.
        state.tutorial_state = 2;
        state.tutorial_active = false;
        // Disable one confirmation; the others stay on.
        state.options.set_confirmation("sacrifice", false);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.galaxies, 7);
        assert_eq!(reloaded.dim_boosts, 3);
        // `break` round-trips as broke_infinity; infinity_unlocked derives from it
        // (and from the infinities/IP we set).
        assert!(reloaded.broke_infinity);
        assert!(reloaded.infinity_unlocked);
        assert_eq!(reloaded.infinity_points, Decimal::from_float(5.0));
        assert_eq!(reloaded.infinities, Decimal::from_float(3.0));
        assert_eq!(reloaded.infinity_upgrades, state.infinity_upgrades);
        assert!(reloaded.infinity_upgrade_bought(InfinityUpgrade::TotalTimeMult));
        assert!(reloaded.infinity_upgrade_bought(InfinityUpgrade::Buy10Mult));
        assert_eq!(reloaded.part_infinity_point, 0.42);
        assert_eq!(reloaded.challenge.current, 3);
        assert!(reloaded.challenge_completed(1));
        assert!(reloaded.challenge_completed(2));
        assert!(!reloaded.challenge_completed(3));
        assert_eq!(reloaded.records.total_time_played_ms, 123_456.0);
        assert_eq!(reloaded.records.this_infinity.time_ms, 7_890.0);
        assert_eq!(
            reloaded.records.this_infinity.max_am,
            Decimal::new(1.0, 250)
        );
        assert_eq!(reloaded.records.best_infinity.time_ms, 42_000.0);
        assert_eq!(reloaded.dimensions[2].bought, 42);
        assert_eq!(reloaded.options.notation, "Engineering");
        // Tutorial fields survive the round-trip.
        assert_eq!(reloaded.tutorial_state, 2);
        assert!(!reloaded.tutorial_active);
        // The disabled confirmation round-trips; the rest remain on.
        assert!(!reloaded.options.confirmations.sacrifice);
        assert!(reloaded.options.confirmations.dimension_boost);
        assert!(reloaded.options.confirmations.big_crunch);
        // Achievement bits survive the round-trip verbatim.
        assert_eq!(reloaded.achievement_bits, state.achievement_bits);
        assert!(reloaded.achievement_unlocked(18));
        assert!(reloaded.achievement_unlocked(21));
        // `break` reflects the Infinity-unlocked flag in the raw JSON.
        let player: Value =
            serde_json::from_str(&decode_pipeline(&encode_save(&state, 0)).unwrap())
                .unwrap();
        assert_eq!(player["break"], true);
    }
}
