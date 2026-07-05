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

use crate::autobuyers::{AutoRealityMode, AutobuyerMode, PrestigeAutobuyerMode};
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
    // Eternity currency. `eternity_unlocked` is derived on load (eternities/EP).
    player["eternityPoints"] = decimal(&state.eternity_points);
    player["eternities"] = decimal(&state.eternities);
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
    records["thisInfinity"]["bestIPmin"] =
        decimal(&state.records.this_infinity.best_ip_min);
    records["thisInfinity"]["bestIPminVal"] =
        decimal(&state.records.this_infinity.best_ip_min_val);
    records["bestInfinity"]["time"] = json!(state.records.best_infinity.time_ms);
    records["bestInfinity"]["realTime"] =
        json!(state.records.best_infinity.real_time_ms);
    records["thisEternity"]["time"] = json!(state.records.this_eternity.time_ms);
    records["thisEternity"]["realTime"] =
        json!(state.records.this_eternity.real_time_ms);
    records["thisEternity"]["maxAM"] = decimal(&state.records.this_eternity.max_am);
    records["thisEternity"]["maxIP"] = decimal(&state.records.this_eternity.max_ip);
    records["thisEternity"]["bestEPmin"] =
        decimal(&state.records.this_eternity.best_ep_min);
    records["thisEternity"]["bestEPminVal"] =
        decimal(&state.records.this_eternity.best_ep_min_val);
    records["bestEternity"]["time"] = json!(state.records.best_eternity.time_ms);
    records["bestEternity"]["realTime"] =
        json!(state.records.best_eternity.real_time_ms);
    // Achievement bitmask, written back verbatim (one int per row).
    player["achievementBits"] = json!(state.achievement_bits);
    // Tab notification badges: the badged keys (a Set serialized as an array,
    // ours in BTreeSet order) + the triggered-notification bits.
    player["tabNotifications"] = json!(state.tab_notifications);
    player["triggeredTabNotificationBits"] =
        json!(state.triggered_tab_notification_bits);
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

    // Time Studies + Time Theorems.
    let ts = &mut player["timestudy"];
    ts["theorem"] = decimal(&state.time_theorems);
    ts["maxTheorem"] = decimal(&state.max_theorem);
    ts["amBought"] = json!(state.tt_am_bought);
    ts["ipBought"] = json!(state.tt_ip_bought);
    ts["epBought"] = json!(state.tt_ep_bought);
    ts["studies"] = json!(state.studies);
    ts["presets"] = json!(state
        .study_presets
        .iter()
        .map(|p| json!({ "name": p.name, "studies": p.studies }))
        .collect::<Vec<_>>());
    player["respec"] = json!(state.respec);
    player["infinitiesBanked"] = decimal(&state.infinities_banked);
    // EC state: the held study slot and the completion-count map.
    player["challenge"]["eternity"]["unlocked"] =
        json!(state.eternity_challenge_unlocked);
    player["challenge"]["eternity"]["current"] = json!(state.eternity_challenge_current);
    player["challenge"]["eternity"]["requirementBits"] =
        json!(state.ec_requirement_bits);
    player["eterc8ids"] = json!(state.eterc8_ids);
    player["eterc8repl"] = json!(state.eterc8_repl);
    // Time Dilation.
    let dilation = &mut player["dilation"];
    dilation["studies"] = json!(state.dilation.studies);
    dilation["active"] = json!(state.dilation.active);
    dilation["tachyonParticles"] = decimal(&state.dilation.tachyon_particles);
    dilation["dilatedTime"] = decimal(&state.dilation.dilated_time);
    dilation["nextThreshold"] = decimal(&state.dilation.next_threshold);
    dilation["baseTachyonGalaxies"] = json!(state.dilation.base_tachyon_galaxies);
    dilation["totalTachyonGalaxies"] = json!(state.dilation.total_tachyon_galaxies);
    dilation["upgrades"] = json!((4u8..=10)
        .filter(|&id| state.dilation_upgrade_bought(id))
        .collect::<Vec<_>>());
    for (i, count) in state.dilation.rebuyables.iter().enumerate() {
        dilation["rebuyables"][(i + 1).to_string()] = json!(count);
    }
    dilation["lastEP"] = decimal(&state.dilation.last_ep);

    // Eternity Upgrades (a Set of numeric ids) + the rebuyable EP multiplier.
    player["eternityUpgrades"] = json!(crate::ALL_ETERNITY_UPGRADES
        .iter()
        .filter(|u| state.eternity_upgrade_bought(**u))
        .map(|u| u.id())
        .collect::<Vec<_>>());
    player["epmultUpgrades"] = json!(state.epmult_upgrades);
    // Infinity Challenge record times.
    player["challenge"]["infinity"]["bestTimes"] = json!(state.ic_best_times_ms);
    let mut ec_map = serde_json::Map::new();
    for (i, &count) in state.eternity_challenges.iter().enumerate() {
        if count > 0 {
            ec_map.insert(format!("eterc{}", i + 1), json!(count));
        }
    }
    player["eternityChalls"] = Value::Object(ec_map);
    // The recent-eternities ring, as the original's 6-tuples (challenge text
    // and TT-gain slots are unmodelled → "" and "0").
    player["records"]["recentEternities"] = json!(state
        .records
        .recent_eternities
        .iter()
        .map(|r| {
            json!([
                r.time_ms,
                r.real_time_ms,
                r.ep.to_string(),
                r.eternities.to_string(),
                "",
                "0"
            ])
        })
        .collect::<Vec<_>>());

    // Reality: the root realities count, `player.reality`, the reality
    // records, and the requirement-check flags.
    player["realities"] = json!(state.reality.realities);
    let reality = &mut player["reality"];
    reality["realityMachines"] = decimal(&state.reality.machines);
    reality["maxRM"] = decimal(&state.reality.max_rm);
    reality["perkPoints"] = json!(state.reality.perk_points);
    reality["perks"] = json!(state.reality.perks.iter().collect::<Vec<_>>());
    reality["seed"] = json!(state.reality.seed);
    reality["initialSeed"] = json!(state.reality.initial_seed);
    reality["secondGaussian"] = json!(state.reality.second_gaussian);
    for (i, count) in state.reality.rebuyables.iter().enumerate() {
        reality["rebuyables"][(i + 1).to_string()] = json!(count);
    }
    let glyph_json = |g: &crate::glyphs::Glyph| {
        json!({
            "id": g.id,
            "idx": g.idx,
            "type": g.kind.save_id(),
            "strength": g.strength,
            "level": g.level,
            "rawLevel": g.raw_level,
            "effects": g.effects,
        })
    };
    reality["glyphs"]["active"] = json!(state
        .reality
        .glyphs
        .active
        .iter()
        .map(glyph_json)
        .collect::<Vec<_>>());
    reality["glyphs"]["inventory"] = json!(state
        .reality
        .glyphs
        .inventory
        .iter()
        .map(glyph_json)
        .collect::<Vec<_>>());
    for (i, kind) in crate::glyphs::BASIC_GLYPH_TYPES.iter().enumerate() {
        reality["glyphs"]["sac"][kind.save_id()] = json!(state.reality.glyphs.sac[i]);
    }
    reality["glyphs"]["protectedRows"] = json!(state.reality.glyphs.protected_rows);
    reality["upgradeBits"] = json!(state.reality.upgrade_bits);
    reality["upgReqs"] = json!(state.reality.upg_reqs);
    reality["reqLock"]["reality"] = json!(state.reality.req_lock);
    reality["respec"] = json!(state.reality.respec);
    reality["achTimer"] = json!(state.reality.ach_timer);
    reality["autoAchieve"] = json!(state.reality.auto_achieve);
    reality["gainedAutoAchievements"] = json!(state.reality.gained_auto_achievements);
    write_automator(&mut reality["automator"], state);
    let records = &mut player["records"];
    records["thisReality"]["time"] = json!(state.records.this_reality.time_ms);
    records["thisReality"]["realTime"] = json!(state.records.this_reality.real_time_ms);
    records["thisReality"]["maxEP"] = decimal(&state.records.this_reality.max_ep);
    records["thisReality"]["maxReplicanti"] =
        decimal(&state.records.this_reality.max_replicanti);
    records["thisReality"]["maxDT"] = decimal(&state.records.this_reality.max_dt);
    records["bestReality"]["time"] = json!(state.records.best_reality.time_ms);
    records["bestReality"]["realTime"] = json!(state.records.best_reality.real_time_ms);
    records["bestReality"]["RMmin"] = decimal(&state.records.best_reality.rm_min);
    records["bestReality"]["glyphLevel"] = json!(state.records.best_reality.glyph_level);
    records["bestReality"]["bestEP"] = decimal(&state.records.best_reality.best_ep);
    records["bestReality"]["glyphStrength"] =
        json!(state.records.best_reality.glyph_strength);
    records["recentRealities"] = json!(state
        .records
        .recent_realities
        .iter()
        .map(|r| {
            json!([
                r.time_ms,
                r.real_time_ms,
                r.rm.to_string(),
                r.reality_count,
                "",
                0
            ])
        })
        .collect::<Vec<_>>());
    player["requirementChecks"]["eternity"]["noRG"] =
        json!(state.requirement_checks.eternity_no_rg);
    player["requirementChecks"]["reality"]["noInfinities"] =
        json!(state.requirement_checks.reality_no_infinities);
    player["requirementChecks"]["reality"]["noEternities"] =
        json!(state.requirement_checks.reality_no_eternities);
    player["requirementChecks"]["reality"]["maxGlyphs"] =
        json!(state.requirement_checks.reality_max_glyphs);

    // Black Holes.
    player["blackHole"] = json!(state
        .black_holes
        .holes
        .iter()
        .enumerate()
        .map(|(id, h)| {
            json!({
                "id": id,
                "unlocked": h.unlocked,
                "active": h.active,
                "phase": h.phase,
                "activations": h.activations,
                "intervalUpgrades": h.interval_upgrades,
                "powerUpgrades": h.power_upgrades,
                "durationUpgrades": h.duration_upgrades,
            })
        })
        .collect::<Vec<_>>());
    player["blackHolePause"] = json!(state.black_holes.paused);
    player["blackHolePauseTime"] = json!(state.black_holes.pause_time_ms);
    player["records"]["timePlayedAtBHUnlock"] =
        json!(state.records.time_played_at_bh_unlock_ms);

    // Time Dimensions + Time Shards + free tickspeed upgrades.
    player["timeShards"] = decimal(&state.time_shards);
    player["totalTickGained"] = json!(state.total_tick_gained);
    for (tier, d) in state.time_dimensions.iter().enumerate() {
        let entry = &mut player["dimensions"]["time"][tier];
        entry["amount"] = decimal(&d.amount);
        entry["bought"] = json!(d.bought);
        entry["cost"] = decimal(&d.cost);
    }

    // Infinity Dimensions + Infinity Power.
    player["infinityPower"] = decimal(&state.infinity_power);
    for (tier, d) in state.infinity_dimensions.iter().enumerate() {
        let entry = &mut player["dimensions"]["infinity"][tier];
        entry["amount"] = decimal(&d.amount);
        entry["cost"] = decimal(&d.cost);
        entry["baseAmount"] = json!(d.base_amount);
        entry["isUnlocked"] = json!(d.is_unlocked);
    }

    // Replicanti. The sub-interval `timer` is transient (not modelled here) and
    // `galCost` is derived from the bought-galaxy cap.
    let gal_cost = state.replicanti_galaxy_cost();
    let rep = &mut player["replicanti"];
    rep["unl"] = json!(state.replicanti.unlocked);
    rep["amount"] = decimal(&state.replicanti.amount);
    rep["chance"] = json!(state.replicanti.chance);
    rep["chanceCost"] = decimal(&state.replicanti.chance_cost);
    rep["interval"] = json!(state.replicanti.interval_ms);
    rep["intervalCost"] = decimal(&state.replicanti.interval_cost);
    rep["boughtGalaxyCap"] = json!(state.replicanti.galaxy_cap);
    rep["galaxies"] = json!(state.replicanti.galaxies);
    rep["galCost"] = decimal(&gal_cost);

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
    confirmations["eternity"] = json!(state.options.confirmations.eternity);
    confirmations["dilation"] = json!(state.options.confirmations.dilation);
    confirmations["switchAutomatorMode"] =
        json!(state.options.confirmations.switch_automator_mode);
    options["animations"]["bigCrunch"] = json!(state.options.animations.big_crunch);
    let hints = &mut options["showHintText"];
    hints["showPercentage"] = json!(state.options.show_hint_text.show_percentage);
    hints["achievements"] = json!(state.options.show_hint_text.achievements);
    hints["achievementUnlockStates"] =
        json!(state.options.show_hint_text.achievement_unlock_states);
    hints["challenges"] = json!(state.options.show_hint_text.challenges);
    let away = &mut options["awayProgress"];
    away["antimatter"] = json!(state.options.away_progress.antimatter);
    away["dimensionBoosts"] = json!(state.options.away_progress.dimension_boosts);
    away["antimatterGalaxies"] = json!(state.options.away_progress.antimatter_galaxies);
    away["infinities"] = json!(state.options.away_progress.infinities);
    away["infinityPoints"] = json!(state.options.away_progress.infinity_points);
    away["replicanti"] = json!(state.options.away_progress.replicanti);
    away["replicantiGalaxies"] = json!(state.options.away_progress.replicanti_galaxies);
    options["headerTextColored"] = json!(state.options.header_text_colored);
    options["sidebarResourceID"] = json!(state.options.sidebar_resource_id);
    options["hiddenTabBits"] = json!(state.options.hidden_tab_bits);
    options["hiddenSubtabBits"] = json!(state.options.hidden_subtab_bits);
    let ae = &mut options["automatorEvents"];
    ae["newestFirst"] = json!(state.options.automator_events.newest_first);
    ae["timestampType"] = json!(state.options.automator_events.timestamp_type);
    ae["maxEntries"] = json!(state.options.automator_events.max_entries);
    ae["clearOnReality"] = json!(state.options.automator_events.clear_on_reality);
    ae["clearOnRestart"] = json!(state.options.automator_events.clear_on_restart);

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
    // interval-upgrade state. Dim Boost/Galaxy limit config stays at the
    // template default.
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
    // Big Crunch goal settings (post-break modes).
    let s = &state.autobuyers.big_crunch_settings;
    let entry = &mut player["auto"]["bigCrunch"];
    entry["mode"] = json!(prestige_goal_mode_to_raw(s.mode));
    entry["amount"] = decimal(&s.amount);
    entry["increaseWithMult"] = json!(s.increase_with_mult);
    entry["time"] = json!(s.time);
    entry["xHighest"] = decimal(&s.x_highest);
    // Eternity autobuyer.
    let ab = &state.autobuyers.eternity;
    let entry = &mut player["auto"]["eternity"];
    entry["isActive"] = json!(ab.is_active);
    entry["mode"] = json!(prestige_goal_mode_to_raw(ab.settings.mode));
    entry["amount"] = decimal(&ab.settings.amount);
    entry["increaseWithMult"] = json!(ab.settings.increase_with_mult);
    entry["time"] = json!(ab.settings.time);
    entry["xHighest"] = decimal(&ab.settings.x_highest);
    // Reality autobuyer (the Effarig `shard` target stays at the template's 0).
    let ab = &state.autobuyers.reality;
    let entry = &mut player["auto"]["reality"];
    entry["isActive"] = json!(ab.is_active);
    entry["mode"] = json!(match ab.mode {
        AutoRealityMode::Rm => 0,
        AutoRealityMode::Glyph => 1,
        AutoRealityMode::Either => 2,
        AutoRealityMode::Both => 3,
        AutoRealityMode::Time => 4,
    });
    entry["rm"] = decimal(&ab.rm);
    entry["glyph"] = json!(ab.glyph);
    entry["time"] = json!(ab.time);
}

/// A `Decimal` as the JSON string the original stores (`Decimal::toJSON =
/// toString`).
fn decimal(value: &Decimal) -> Value {
    Value::String(value.to_string())
}

/// `player.reality.automator`: scripts, constants, editor + run state
/// (Feature 6.6 Stage B).
fn write_automator(automator: &mut Value, state: &GameState) {
    use crate::automator::{AutomatorEditorType, AutomatorMode, CommandStateData};

    let auto = &state.automator;
    automator["forceUnlock"] = json!(state.reality.automator_force_unlock);
    automator["scripts"] = Value::Object(
        auto.scripts
            .iter()
            .map(|(id, script)| {
                (
                    id.to_string(),
                    json!({ "id": id, "name": script.name, "content": script.content }),
                )
            })
            .collect(),
    );
    automator["constants"] = Value::Object(
        auto.constants
            .iter()
            .map(|(name, value)| (name.clone(), json!(value)))
            .collect(),
    );
    automator["constantSortOrder"] = json!(auto.constant_sort_order);
    automator["type"] = json!(match auto.editor_type {
        AutomatorEditorType::Text => 0,
        AutomatorEditorType::Block => 1,
    });
    automator["currentInfoPane"] = json!(auto.current_info_pane);
    automator["execTimer"] = json!(auto.exec_timer);
    let s = &mut automator["state"];
    s["mode"] = json!(match auto.state.mode {
        AutomatorMode::Pause => 1,
        AutomatorMode::Run => 2,
        AutomatorMode::SingleStep => 3,
    });
    s["topLevelScript"] = json!(auto.state.top_level_script);
    s["editorScript"] = json!(auto.state.editor_script);
    s["repeat"] = json!(auto.state.repeat);
    s["forceRestart"] = json!(auto.state.force_restart);
    s["followExecution"] = json!(auto.state.follow_execution);
    s["stack"] = json!(auto
        .state
        .stack
        .iter()
        .map(|entry| {
            let command_state = match &entry.command_state {
                None => Value::Null,
                Some(CommandStateData::Pause { time_ms }) => {
                    json!({ "timeMs": time_ms })
                }
                Some(CommandStateData::PrestigeLevel { level }) => {
                    json!({ "prestigeLevel": level })
                }
                Some(CommandStateData::IfEntered {
                    advance_on_pop,
                    if_end_line,
                }) => json!({
                    "advanceOnPop": advance_on_pop,
                    "ifEndLine": if_end_line,
                }),
            };
            json!({ "lineNumber": entry.line_number, "commandState": command_state })
        })
        .collect::<Vec<_>>());
}

/// Maps our [`AutobuyerMode`] back to the original numeric `AUTOBUYER_MODE`.
fn mode_to_raw(mode: AutobuyerMode) -> i64 {
    match mode {
        AutobuyerMode::BuyMax => AUTOBUYER_MODE_BUY_10,
        AutobuyerMode::BuySingle => AUTOBUYER_MODE_BUY_SINGLE,
    }
}

/// Maps a [`PrestigeAutobuyerMode`] back to the original numeric
/// `AUTO_CRUNCH_MODE` / `AUTO_ETERNITY_MODE`.
fn prestige_goal_mode_to_raw(mode: PrestigeAutobuyerMode) -> i64 {
    match mode {
        PrestigeAutobuyerMode::Amount => 0,
        PrestigeAutobuyerMode::Time => 1,
        PrestigeAutobuyerMode::XHighest => 2,
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
    fn prestige_autobuyers_and_presets_round_trip() {
        use crate::autobuyers::{AutoRealityMode, PrestigeAutobuyerMode};

        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.autobuyers.big_crunch_settings.mode = PrestigeAutobuyerMode::XHighest;
        state.autobuyers.big_crunch_settings.amount = Decimal::new(2.5, 30);
        state.autobuyers.big_crunch_settings.increase_with_mult = false;
        state.autobuyers.big_crunch_settings.time = 12.5;
        state.autobuyers.big_crunch_settings.x_highest = Decimal::from_float(3.0);
        state.autobuyers.eternity.is_active = true;
        state.autobuyers.eternity.settings.mode = PrestigeAutobuyerMode::Time;
        state.autobuyers.eternity.settings.amount = Decimal::new(1.0, 100);
        state.autobuyers.eternity.settings.time = 30.0;
        state.autobuyers.reality.is_active = true;
        state.autobuyers.reality.mode = AutoRealityMode::Both;
        state.autobuyers.reality.rm = Decimal::from_float(1e6);
        state.autobuyers.reality.glyph = 5000;
        state.autobuyers.reality.time = 600.0;
        state.study_presets[0] = crate::time_studies::StudyPreset {
            name: "ANTI".into(),
            studies: "11,21,22|0".into(),
        };
        state.study_presets[5] = crate::time_studies::StudyPreset {
            name: String::new(),
            studies: "11-62|4!".into(),
        };
        state.reality.automator_force_unlock = true;

        let reloaded = decode_save(&encode_save(&state, 1_700_000_000_000)).unwrap();
        assert_eq!(
            reloaded.autobuyers.big_crunch_settings.mode,
            PrestigeAutobuyerMode::XHighest
        );
        assert_eq!(
            reloaded.autobuyers.big_crunch_settings.amount,
            Decimal::new(2.5, 30)
        );
        assert!(!reloaded.autobuyers.big_crunch_settings.increase_with_mult);
        assert_eq!(reloaded.autobuyers.big_crunch_settings.time, 12.5);
        assert_eq!(
            reloaded.autobuyers.big_crunch_settings.x_highest,
            Decimal::from_float(3.0)
        );
        assert!(reloaded.autobuyers.eternity.is_active);
        assert_eq!(
            reloaded.autobuyers.eternity.settings.mode,
            PrestigeAutobuyerMode::Time
        );
        assert_eq!(
            reloaded.autobuyers.eternity.settings.amount,
            Decimal::new(1.0, 100)
        );
        assert_eq!(reloaded.autobuyers.eternity.settings.time, 30.0);
        assert!(reloaded.autobuyers.reality.is_active);
        assert_eq!(reloaded.autobuyers.reality.mode, AutoRealityMode::Both);
        assert_eq!(reloaded.autobuyers.reality.rm, Decimal::from_float(1e6));
        assert_eq!(reloaded.autobuyers.reality.glyph, 5000);
        assert_eq!(reloaded.autobuyers.reality.time, 600.0);
        assert_eq!(reloaded.study_presets[0].name, "ANTI");
        assert_eq!(reloaded.study_presets[0].studies, "11,21,22|0");
        assert_eq!(reloaded.study_presets[5].studies, "11-62|4!");
        assert!(reloaded.reality.automator_force_unlock);
    }

    #[test]
    fn automator_data_round_trips() {
        use crate::automator::{
            AutomatorEditorType, AutomatorMode, CommandStateData, StackEntryData,
        };

        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.automator_save_script(1, "pause 10s\neternity");
        state.automator_rename_script(1, "Main");
        let second = state
            .automator_create_script("EC grind", "unlock ec1")
            .unwrap();
        state.automator_set_constant("goal", "1e300");
        state.automator_set_constant("tree", "11,21,22|0");
        state.automator.editor_type = AutomatorEditorType::Block;
        state.automator.current_info_pane = 3;
        state.automator.exec_timer = 123.5;
        state.automator.state.mode = AutomatorMode::Run;
        state.automator.state.top_level_script = second;
        state.automator.state.editor_script = 1;
        state.automator.state.repeat = false;
        state.automator.state.stack = vec![
            StackEntryData {
                line_number: 4,
                command_state: Some(CommandStateData::PrestigeLevel { level: 2 }),
            },
            StackEntryData {
                line_number: 2,
                command_state: Some(CommandStateData::Pause { time_ms: 500.0 }),
            },
        ];

        let mut reloaded = decode_save(&encode_save(&state, 1_700_000_000_000)).unwrap();
        // The runtime is transient (never saved); compare persistent data.
        reloaded.automator.runtime = Default::default();
        state.automator.runtime = Default::default();
        assert_eq!(reloaded.automator, state.automator);

        // The encoded JSON keeps the original's schema (scripts keyed by id
        // string, duplicated id prop, numeric type/mode).
        let json = decode_pipeline(&encode_save(&state, 1_700_000_000_000)).unwrap();
        let player: Value = serde_json::from_str(&json).unwrap();
        let automator = &player["reality"]["automator"];
        assert_eq!(automator["scripts"]["1"]["id"], 1);
        assert_eq!(automator["scripts"]["1"]["name"], "Main");
        assert_eq!(automator["type"], 1);
        assert_eq!(automator["state"]["mode"], 2);
        assert_eq!(automator["state"]["stack"][0]["lineNumber"], 4);
        assert_eq!(
            automator["state"]["stack"][0]["commandState"]["prestigeLevel"],
            2
        );
        assert_eq!(automator["constants"]["goal"], "1e300");
    }

    #[test]
    fn visual_options_round_trip() {
        // The Visual-tab option set (animations / hint text / away progress /
        // header coloring / sidebar resource / hidden tabs) survives
        // encode → decode, keys matching the original schema.
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.options.set_animation("bigCrunch", false);
        state.options.set_hint_text("achievements", false);
        state.options.set_hint_text("showPercentage", false);
        state.options.set_away_progress("replicanti", false);
        state.options.header_text_colored = true;
        state.options.set_sidebar_resource(3);
        state.options.toggle_tab_visibility(5);
        state.options.toggle_subtab_visibility(6, 2);
        state.options.toggle_subtab_visibility(0, 1);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.options, state.options);
        assert!(!reloaded.options.animations.big_crunch);
        assert!(!reloaded.options.show_hint_text.achievements);
        assert!(reloaded.options.show_hint_text.challenges);
        assert!(!reloaded.options.away_progress.replicanti);
        assert!(reloaded.options.header_text_colored);
        assert_eq!(reloaded.options.sidebar_resource_id, 3);
        assert_eq!(reloaded.options.hidden_tab_bits, 1 << 5);
        assert_eq!(reloaded.options.hidden_subtab_bits[6], 1 << 2);
        assert_eq!(reloaded.options.hidden_subtab_bits[0], 1 << 1);

        // The raw JSON uses the original's keys (incl. the capital-ID quirk).
        let player: Value =
            serde_json::from_str(&decode_pipeline(&encode_save(&state, 0)).unwrap())
                .unwrap();
        assert_eq!(player["options"]["sidebarResourceID"], 3);
        assert_eq!(player["options"]["headerTextColored"], true);
        assert_eq!(player["options"]["animations"]["bigCrunch"], false);
        assert_eq!(player["options"]["showHintText"]["showPercentage"], false);
        assert_eq!(player["options"]["awayProgress"]["replicanti"], false);
        assert_eq!(player["options"]["hiddenTabBits"], 1 << 5);
        assert_eq!(
            player["options"]["hiddenSubtabBits"]
                .as_array()
                .unwrap()
                .len(),
            11
        );
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
    fn replicanti_round_trip() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.replicanti.unlocked = true;
        state.replicanti.amount = Decimal::new(1.0, 250);
        state.replicanti.chance = 0.23;
        state.replicanti.chance_cost = Decimal::new(1.0, 180);
        state.replicanti.interval_ms = 729.0;
        state.replicanti.interval_cost = Decimal::new(1.0, 160);
        state.replicanti.galaxies = 7;
        state.replicanti.galaxy_cap = 12;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert!(reloaded.replicanti.unlocked);
        assert_eq!(reloaded.replicanti.amount, Decimal::new(1.0, 250));
        assert_eq!(reloaded.replicanti.chance, 0.23);
        assert_eq!(reloaded.replicanti.chance_cost, Decimal::new(1.0, 180));
        assert_eq!(reloaded.replicanti.interval_ms, 729.0);
        assert_eq!(reloaded.replicanti.interval_cost, Decimal::new(1.0, 160));
        assert_eq!(reloaded.replicanti.galaxies, 7);
        assert_eq!(reloaded.replicanti.galaxy_cap, 12);
    }

    #[test]
    fn tab_notifications_round_trip() {
        // Badged keys and triggered bits survive encode → decode — including a
        // key we never render and a bit past our modelled ids.
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state
            .tab_notifications
            .insert("challengesnormal".to_string());
        state
            .tab_notifications
            .insert("eternitystudies".to_string());
        state.triggered_tab_notification_bits = (1 << 0) | (1 << 12) | (1 << 7);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.tab_notifications, state.tab_notifications);
        assert_eq!(
            reloaded.triggered_tab_notification_bits,
            (1 << 0) | (1 << 12) | (1 << 7)
        );
    }

    #[test]
    fn eternity_state_round_trips() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.eternity_points = Decimal::new(1.5, 10);
        state.eternities = Decimal::from_float(7.0);
        state.records.this_eternity.time_ms = 123_000.0;
        state.records.this_eternity.real_time_ms = 124_000.0;
        state.records.this_eternity.max_ip = Decimal::new(1.0, 200);
        state.records.this_eternity.best_ep_min = Decimal::from_float(42.0);
        state.records.this_eternity.best_ep_min_val = Decimal::from_float(84.0);
        state.records.best_eternity.time_ms = 60_000.0;
        state.records.this_infinity.best_ip_min = Decimal::new(1.0, 5);
        state.options.set_confirmation("eternity", false);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.eternity_points, Decimal::new(1.5, 10));
        assert_eq!(reloaded.eternities, Decimal::from_float(7.0));
        // Eternity-unlocked is derived from the eternities count.
        assert!(reloaded.eternity_unlocked);
        assert_eq!(reloaded.records.this_eternity.time_ms, 123_000.0);
        assert_eq!(reloaded.records.this_eternity.real_time_ms, 124_000.0);
        assert_eq!(
            reloaded.records.this_eternity.max_ip,
            Decimal::new(1.0, 200)
        );
        assert_eq!(
            reloaded.records.this_eternity.best_ep_min,
            Decimal::from_float(42.0)
        );
        assert_eq!(reloaded.records.best_eternity.time_ms, 60_000.0);
        assert_eq!(
            reloaded.records.this_infinity.best_ip_min,
            Decimal::new(1.0, 5)
        );
        assert!(!reloaded.options.confirmations.eternity);

        // A fresh save has no eternity progress → not unlocked.
        let fresh = decode_save(INITIAL_SAVE.trim()).unwrap();
        assert!(!fresh.eternity_unlocked);
    }

    #[test]
    fn dilation_round_trips() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.dilation.studies = vec![1, 2, 3];
        state.dilation.active = true;
        state.dilation.tachyon_particles = Decimal::from_float(123.0);
        state.dilation.dilated_time = Decimal::new(1.5, 7);
        state.dilation.next_threshold = Decimal::from_float(5000.0);
        state.dilation.base_tachyon_galaxies = 4;
        state.dilation.total_tachyon_galaxies = 8.0;
        state.dilation.upgrades = (1 << 4) | (1 << 10);
        state.dilation.rebuyables = [3, 1, 2];
        state.dilation.last_ep = Decimal::new(1.0, 60);
        state.options.set_confirmation("dilation", false);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.dilation.studies, vec![1, 2, 3]);
        assert!(reloaded.dilation.active);
        assert_eq!(
            reloaded.dilation.tachyon_particles,
            Decimal::from_float(123.0)
        );
        assert_eq!(reloaded.dilation.dilated_time, Decimal::new(1.5, 7));
        assert_eq!(reloaded.dilation.base_tachyon_galaxies, 4);
        assert_eq!(reloaded.dilation.total_tachyon_galaxies, 8.0);
        assert!(reloaded.dilation_upgrade_bought(4));
        assert!(reloaded.dilation_upgrade_bought(10));
        assert!(!reloaded.dilation_upgrade_bought(5));
        assert_eq!(reloaded.dilation.rebuyables, [3, 1, 2]);
        assert_eq!(reloaded.dilation.last_ep, Decimal::new(1.0, 60));
        assert!(!reloaded.options.confirmations.dilation);
    }

    #[test]
    fn reality_state_round_trips() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.reality.machines = Decimal::from_float(1234.0);
        state.reality.max_rm = Decimal::from_float(2000.0);
        state.reality.realities = 3;
        state.reality.perk_points = 2.0;
        state.reality.perks = [0u8, 10, 205].into_iter().collect();
        state.reality.seed = -1_234_567.0;
        state.reality.initial_seed = 987_654_321_012.0;
        state.reality.second_gaussian = 0.25;
        state.reality.rebuyables = [1, 0, 2, 0, 3];
        state.reality.upgrade_bits = (1 << 6) | (1 << 19);
        state.reality.upg_reqs = 1 << 6;
        state.reality.req_lock = 1 << 9;
        state.reality.respec = true;
        state.reality.ach_timer = 60_000.0;
        state.reality.auto_achieve = false;
        state.reality.gained_auto_achievements = false;
        state.records.this_reality.time_ms = 5_000.0;
        state.records.this_reality.max_ep = Decimal::new(1.0, 4321);
        state.records.this_reality.max_replicanti = Decimal::new(1.0, 30_000);
        state.records.this_reality.max_dt = Decimal::new(1.0, 12);
        state.records.best_reality.time_ms = 4_000.0;
        state.records.best_reality.rm_min = Decimal::from_float(10.0);
        state.records.best_reality.glyph_level = 42;
        state.records.best_reality.best_ep = Decimal::new(1.0, 4500);
        state.records.best_reality.glyph_strength = 2.5;
        state.records.recent_realities[0] = crate::records::RecentReality {
            time_ms: 5_000.0,
            real_time_ms: 6_000.0,
            rm: Decimal::from_float(100.0),
            reality_count: 1.0,
        };
        state.requirement_checks.eternity_no_rg = false;
        state.requirement_checks.reality_no_infinities = false;
        state.requirement_checks.reality_no_eternities = false;
        state.requirement_checks.reality_max_glyphs = 4;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.reality.machines, Decimal::from_float(1234.0));
        assert_eq!(reloaded.reality.max_rm, Decimal::from_float(2000.0));
        assert_eq!(reloaded.reality.realities, 3);
        assert_eq!(reloaded.reality.perk_points, 2.0);
        assert!(reloaded.perk_bought(0));
        assert!(reloaded.perk_bought(205));
        assert!(!reloaded.perk_bought(11));
        assert_eq!(reloaded.reality.seed, -1_234_567.0);
        assert_eq!(reloaded.reality.initial_seed, 987_654_321_012.0);
        assert_eq!(reloaded.reality.second_gaussian, 0.25);
        assert_eq!(reloaded.reality.rebuyables, [1, 0, 2, 0, 3]);
        assert_eq!(reloaded.reality.upgrade_bits, (1 << 6) | (1 << 19));
        assert_eq!(reloaded.reality.upg_reqs, 1 << 6);
        assert_eq!(reloaded.reality.req_lock, 1 << 9);
        assert!(reloaded.reality.respec);
        assert_eq!(reloaded.reality.ach_timer, 60_000.0);
        assert!(!reloaded.reality.auto_achieve);
        assert!(!reloaded.reality.gained_auto_achievements);
        assert_eq!(reloaded.records.this_reality.time_ms, 5_000.0);
        assert_eq!(
            reloaded.records.this_reality.max_ep,
            Decimal::new(1.0, 4321)
        );
        assert_eq!(
            reloaded.records.this_reality.max_replicanti,
            Decimal::new(1.0, 30_000)
        );
        assert_eq!(reloaded.records.this_reality.max_dt, Decimal::new(1.0, 12));
        assert_eq!(reloaded.records.best_reality.time_ms, 4_000.0);
        assert_eq!(
            reloaded.records.best_reality.rm_min,
            Decimal::from_float(10.0)
        );
        assert_eq!(reloaded.records.best_reality.glyph_level, 42);
        assert_eq!(
            reloaded.records.best_reality.best_ep,
            Decimal::new(1.0, 4500)
        );
        assert_eq!(reloaded.records.best_reality.glyph_strength, 2.5);
        assert_eq!(
            reloaded.records.recent_realities[0].rm,
            Decimal::from_float(100.0)
        );
        assert!(!reloaded.requirement_checks.eternity_no_rg);
        assert!(!reloaded.requirement_checks.reality_no_infinities);
        assert!(!reloaded.requirement_checks.reality_no_eternities);
        assert_eq!(reloaded.requirement_checks.reality_max_glyphs, 4);
    }

    #[test]
    fn glyphs_round_trip() {
        use crate::glyphs::{Glyph, GlyphType};
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.reality.glyphs.active.push(Glyph {
            id: 3,
            idx: 1,
            kind: GlyphType::Time,
            strength: 2.25,
            level: 123,
            raw_level: 130,
            effects: 0b1011,
        });
        state.reality.glyphs.inventory.push(Glyph {
            id: 7,
            idx: 25,
            kind: GlyphType::Companion,
            strength: 1.01,
            level: 1,
            raw_level: 1,
            effects: (1 << 8) | (1 << 9),
        });
        state.reality.glyphs.sac = [1.0, 2.0, 3.0, 4.0, 5.0];
        state.reality.glyphs.protected_rows = 3;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.reality.glyphs.active.len(), 1);
        let g = &reloaded.reality.glyphs.active[0];
        assert_eq!(g.id, 3);
        assert_eq!(g.idx, 1);
        assert_eq!(g.kind, GlyphType::Time);
        assert_eq!(g.strength, 2.25);
        assert_eq!(g.level, 123);
        assert_eq!(g.raw_level, 130);
        assert_eq!(g.effects, 0b1011);
        assert_eq!(reloaded.reality.glyphs.inventory.len(), 1);
        assert_eq!(
            reloaded.reality.glyphs.inventory[0].kind,
            GlyphType::Companion
        );
        assert_eq!(reloaded.reality.glyphs.sac, [1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(reloaded.reality.glyphs.protected_rows, 3);
    }

    #[test]
    fn black_holes_round_trip() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.black_holes.holes[0].unlocked = true;
        state.black_holes.holes[0].active = true;
        state.black_holes.holes[0].phase = 123.5;
        state.black_holes.holes[0].activations = 7;
        state.black_holes.holes[0].interval_upgrades = 3;
        state.black_holes.holes[0].power_upgrades = 2;
        state.black_holes.holes[0].duration_upgrades = 1;
        state.black_holes.holes[1].unlocked = true;
        state.black_holes.paused = true;
        state.black_holes.pause_time_ms = 456.0;
        state.records.time_played_at_bh_unlock_ms = 789.0;

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        let h = &reloaded.black_holes.holes[0];
        assert!(h.unlocked && h.active);
        assert_eq!(h.phase, 123.5);
        assert_eq!(h.activations, 7);
        assert_eq!(
            (h.interval_upgrades, h.power_upgrades, h.duration_upgrades),
            (3, 2, 1)
        );
        assert!(reloaded.black_holes.holes[1].unlocked);
        assert!(reloaded.black_holes.paused);
        assert_eq!(reloaded.black_holes.pause_time_ms, 456.0);
        assert_eq!(reloaded.records.time_played_at_bh_unlock_ms, 789.0);
    }

    #[test]
    fn infinity_challenge_state_round_trips() {
        let mut state = decode_save(INITIAL_SAVE.trim()).unwrap();
        state.infinity_challenge.current = 3;
        state.infinity_challenge.completed = (1 << 1) | (1 << 5);
        state.records.this_eternity.max_am = Decimal::new(1.0, 14000);

        let reloaded = decode_save(&encode_save(&state, 0)).unwrap();
        assert_eq!(reloaded.infinity_challenge.current, 3);
        assert!(reloaded.infinity_challenge_completed(1));
        assert!(reloaded.infinity_challenge_completed(5));
        assert!(!reloaded.infinity_challenge_completed(2));
        assert_eq!(
            reloaded.records.this_eternity.max_am,
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
