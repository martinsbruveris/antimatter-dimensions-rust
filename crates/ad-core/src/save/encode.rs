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
    encode_pipeline(&build_player_json(state, now_ms))
}

/// Overlays `state` onto a fresh copy of the template and serializes it to JSON.
fn build_player_json(state: &GameState, now_ms: i64) -> String {
    let mut player: Value = serde_json::from_str(DEFAULT_PLAYER_TEMPLATE)
        .expect("vendored default_player.json is valid JSON");

    overlay(&mut player, state, now_ms);

    serde_json::to_string(&player).expect("player Value always serializes")
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
    // §4.3 inverse: carry the Infinity-unlocked flag via `break`. We don't model
    // an infinity count, so `infinities` is left at the template's "0".
    player["break"] = json!(state.infinity_unlocked);
    // Stamp the save time so the game computes ~0 offline progress on import.
    player["lastUpdate"] = json!(now_ms);

    // Antimatter dimensions. `costBumps` stays 0 (template); we only model
    // `amount` and `bought`.
    for (tier, dim) in state.dimensions.iter().enumerate() {
        let entry = &mut player["dimensions"]["antimatter"][tier];
        entry["amount"] = decimal(&dim.amount);
        entry["bought"] = json!(dim.bought);
    }

    // Options.
    let options = &mut player["options"];
    options["hotkeys"] = json!(state.options.hotkeys);
    options["updateRate"] = json!(state.options.update_rate);
    options["notation"] = json!(state.options.notation);
    options["notationDigits"]["comma"] = json!(state.options.notation_digits_comma);
    options["notationDigits"]["notation"] =
        json!(state.options.notation_digits_notation);

    // Autobuyers. Intervals/timers are the original's derived state — we leave the
    // template's values and only write the flags/modes we model (§4.4).
    player["auto"]["autobuyersOn"] = json!(state.autobuyers.enabled);
    for (tier, ab) in state.autobuyers.dimensions.iter().enumerate() {
        let entry = &mut player["auto"]["antimatterDims"]["all"][tier];
        entry["isActive"] = json!(ab.is_active);
        entry["isBought"] = json!(ab.is_bought);
        entry["mode"] = json!(mode_to_raw(ab.mode));
    }
    let tickspeed = &mut player["auto"]["tickspeed"];
    tickspeed["isActive"] = json!(state.autobuyers.tickspeed.is_active);
    tickspeed["isBought"] = json!(state.autobuyers.tickspeed.is_bought);
    tickspeed["mode"] = json!(mode_to_raw(state.autobuyers.tickspeed.mode));
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
    fn overlays_modelled_state_changes() {
        // Mutate a freshly-loaded state, then confirm the change survives a
        // round-trip (i.e. we actually overlay, not just echo the template).
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.galaxies = 7;
        state.dim_boosts = 3;
        state.infinity_unlocked = true;
        state.dimensions[2].bought = 42;
        state.options.set_notation("Engineering");

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.galaxies, 7);
        assert_eq!(reloaded.dim_boosts, 3);
        assert!(reloaded.infinity_unlocked);
        assert_eq!(reloaded.dimensions[2].bought, 42);
        assert_eq!(reloaded.options.notation, "Engineering");
        // `break` reflects the Infinity-unlocked flag in the raw JSON.
        let player: Value =
            serde_json::from_str(&decode_pipeline(&encode_save(&state, 0)).unwrap())
                .unwrap();
        assert_eq!(player["break"], true);
    }
}
