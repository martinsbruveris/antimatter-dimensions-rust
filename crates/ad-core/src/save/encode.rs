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
const AUTOBUYER_MODE_BUY_MAX: i64 = 100;

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
    // The one-time `ipOffline` upgrade (offline-only effect) shares the set.
    if state.ip_offline_bought {
        owned_upgrades.push("ipOffline");
    }
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
    // Wall-clock save-creation timestamp — a passthrough; the template's
    // constant only stands in when the state carries none (a fresh
    // `GameState::new()` the embedding backend hasn't stamped).
    if state.records.game_created_time_ms > 0.0 {
        records["gameCreatedTime"] = json!(state.records.game_created_time_ms);
    }
    records["thisInfinity"]["time"] = json!(state.records.this_infinity.time_ms);
    records["thisInfinity"]["realTime"] =
        json!(state.records.this_infinity.real_time_ms);
    records["thisInfinity"]["lastBuyTime"] =
        json!(state.records.this_infinity.last_buy_time_ms);
    records["thisInfinity"]["maxAM"] = decimal(&state.records.this_infinity.max_am);
    records["thisInfinity"]["bestIPmin"] =
        decimal(&state.records.this_infinity.best_ip_min);
    records["thisInfinity"]["bestIPminVal"] =
        decimal(&state.records.this_infinity.best_ip_min_val);
    records["bestInfinity"]["time"] = json!(state.records.best_infinity.time_ms);
    records["bestInfinity"]["realTime"] =
        json!(state.records.best_infinity.real_time_ms);
    records["bestInfinity"]["bestIPminEternity"] =
        decimal(&state.records.best_infinity.best_ip_min_eternity);
    // The recent-infinities ring, as the original's 5-tuples (the challenge-name
    // slot is unmodelled → "").
    records["recentInfinities"] = json!(state
        .records
        .recent_infinities
        .iter()
        .map(|r| json!([
            r.time_ms,
            r.real_time_ms,
            r.ip.to_string(),
            r.infinities.to_string(),
            ""
        ]))
        .collect::<Vec<_>>());
    records["thisEternity"]["time"] = json!(state.records.this_eternity.time_ms);
    records["thisEternity"]["realTime"] =
        json!(state.records.this_eternity.real_time_ms);
    records["thisEternity"]["maxAM"] = decimal(&state.records.this_eternity.max_am);
    records["thisEternity"]["maxIP"] = decimal(&state.records.this_eternity.max_ip);
    records["thisEternity"]["bestEPmin"] =
        decimal(&state.records.this_eternity.best_ep_min);
    records["thisEternity"]["bestEPminVal"] =
        decimal(&state.records.this_eternity.best_ep_min_val);
    records["thisEternity"]["bestInfinitiesPerMs"] =
        decimal(&state.records.this_eternity.best_infinities_per_ms);
    records["thisEternity"]["bestIPMsWithoutMaxAll"] =
        decimal(&state.records.this_eternity.best_ip_ms_without_max_all);
    records["bestEternity"]["time"] = json!(state.records.best_eternity.time_ms);
    records["bestEternity"]["realTime"] =
        json!(state.records.best_eternity.real_time_ms);
    records["bestEternity"]["bestEPminReality"] =
        decimal(&state.records.best_eternity.best_ep_min_reality);
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
        .chain(14..=15)
        .filter(|&id| state.dilation_upgrade_bought(id))
        .collect::<Vec<_>>());
    for (i, count) in state.dilation.rebuyables.iter().enumerate() {
        dilation["rebuyables"][(i + 1).to_string()] = json!(count);
    }
    for (i, count) in state.dilation.pelle_rebuyables.iter().enumerate() {
        dilation["rebuyables"][(i + 11).to_string()] = json!(count);
    }
    dilation["lastEP"] = decimal(&state.dilation.last_ep);

    // Eternity Upgrades (a Set of numeric ids) + the rebuyable EP multiplier.
    player["eternityUpgrades"] = json!(crate::ALL_ETERNITY_UPGRADES
        .iter()
        .filter(|u| state.eternity_upgrade_bought(**u))
        .map(|u| u.id())
        .collect::<Vec<_>>());
    player["epmultUpgrades"] = json!(state.epmult_upgrades);
    player["IPMultPurchases"] = json!(state.ip_mult_purchases);
    player["partInfinitied"] = json!(state.part_infinitied);
    player["partSimulatedReality"] = json!(state.part_simulated_reality);
    player["ic2Count"] = json!(state.ic2_count);
    // Infinity Challenge record times.
    player["challenge"]["infinity"]["bestTimes"] = json!(state.ic_best_times_ms);
    player["challenge"]["normal"]["bestTimes"] = json!(state.nc_best_times_ms);
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
    for kind in [
        crate::glyphs::GlyphType::Power,
        crate::glyphs::GlyphType::Infinity,
        crate::glyphs::GlyphType::Replication,
        crate::glyphs::GlyphType::Time,
        crate::glyphs::GlyphType::Dilation,
        crate::glyphs::GlyphType::Effarig,
        crate::glyphs::GlyphType::Reality,
    ] {
        let i = kind.sacrifice_index().unwrap();
        reality["glyphs"]["sac"][kind.save_id()] = json!(state.reality.glyphs.sac[i]);
    }
    reality["glyphs"]["createdRealityGlyph"] =
        json!(state.reality.glyphs.created_reality_glyph);
    // The auto-glyph filter.
    {
        let f = &state.reality.glyphs.filter;
        let filter = &mut reality["glyphs"]["filter"];
        filter["select"] = json!(f.select);
        filter["trash"] = json!(f.trash);
        filter["simple"] = json!(f.simple);
        for (name, kind) in [
            "time",
            "dilation",
            "replication",
            "infinity",
            "power",
            "effarig",
        ]
        .iter()
        .zip([
            crate::glyphs::GlyphType::Time,
            crate::glyphs::GlyphType::Dilation,
            crate::glyphs::GlyphType::Replication,
            crate::glyphs::GlyphType::Infinity,
            crate::glyphs::GlyphType::Power,
            crate::glyphs::GlyphType::Effarig,
        ]) {
            let i = crate::glyphs::GlyphFilter::type_index(kind).unwrap();
            let cfg = &f.types[i];
            let entry = &mut filter["types"][*name];
            entry["rarity"] = json!(cfg.rarity);
            entry["score"] = json!(cfg.score);
            entry["effectCount"] = json!(cfg.effect_count);
            entry["specifiedMask"] = json!(cfg.specified_mask);
            entry["effectScores"] = json!(cfg.effect_scores);
        }
    }
    // The Glyph-undo stack (dilation studies/upgrades as `toBitmask` numbers).
    reality["glyphs"]["undo"] = json!(state
        .reality
        .glyphs
        .undo
        .iter()
        .map(|u| {
            let mut rebuyables = serde_json::Map::new();
            for (i, count) in u.dilation_rebuyables.iter().enumerate() {
                rebuyables.insert((i + 1).to_string(), json!(count));
            }
            let studies_mask = u
                .dilation_studies
                .iter()
                .fold(0u64, |bits, &id| bits | (1 << id));
            serde_json::json!({
                "targetSlot": u.target_slot,
                "am": u.am.to_string(),
                "ip": u.ip.to_string(),
                "ep": u.ep.to_string(),
                "tt": u.tt.to_string(),
                "ecs": u.ecs,
                "thisInfinityTime": u.this_infinity_time,
                "thisInfinityRealTime": u.this_infinity_real_time,
                "thisEternityTime": u.this_eternity_time,
                "thisEternityRealTime": u.this_eternity_real_time,
                "thisRealityTime": u.this_reality_time,
                "thisRealityRealTime": u.this_reality_real_time,
                "storedTime": u.stored_time,
                "dilationStudies": studies_mask,
                "dilationUpgrades": u.dilation_upgrades,
                "dilationRebuyables": rebuyables,
                "tp": u.tp.to_string(),
                "dt": u.dt.to_string(),
            })
        })
        .collect::<Vec<_>>());
    reality["glyphs"]["protectedRows"] = json!(state.reality.glyphs.protected_rows);
    reality["upgradeBits"] = json!(state.reality.upgrade_bits);
    reality["upgReqs"] = json!(state.reality.upg_reqs);
    reality["reqLock"]["reality"] = json!(state.reality.req_lock);
    reality["respec"] = json!(state.reality.respec);
    reality["achTimer"] = json!(state.reality.ach_timer);
    reality["autoEC"] = json!(state.reality.auto_ec);
    reality["lastAutoEC"] = json!(state.reality.last_auto_ec);
    reality["autoAchieve"] = json!(state.reality.auto_achieve);
    reality["gainedAutoAchievements"] = json!(state.reality.gained_auto_achievements);
    write_automator(&mut reality["automator"], state);
    let records = &mut player["records"];
    records["thisReality"]["time"] = json!(state.records.this_reality.time_ms);
    records["thisReality"]["realTime"] = json!(state.records.this_reality.real_time_ms);
    records["thisReality"]["maxAM"] = decimal(&state.records.this_reality.max_am);
    records["thisReality"]["maxIP"] = decimal(&state.records.this_reality.max_ip);
    records["thisReality"]["maxEP"] = decimal(&state.records.this_reality.max_ep);
    records["thisReality"]["maxReplicanti"] =
        decimal(&state.records.this_reality.max_replicanti);
    records["thisReality"]["maxDT"] = decimal(&state.records.this_reality.max_dt);
    records["thisReality"]["bestEternitiesPerMs"] =
        decimal(&state.records.this_reality.best_eternities_per_ms);
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
    player["requirementChecks"]["eternity"]["onlyAD8"] =
        json!(state.requirement_checks.eternity_only_ad8);
    player["requirementChecks"]["eternity"]["onlyAD1"] =
        json!(state.requirement_checks.eternity_only_ad1);
    player["requirementChecks"]["eternity"]["noAD1"] =
        json!(state.requirement_checks.eternity_no_ad1);
    player["requirementChecks"]["reality"]["noAM"] =
        json!(state.requirement_checks.reality_no_am);
    player["requirementChecks"]["infinity"]["maxAll"] =
        json!(state.requirement_checks.infinity_max_all);
    player["requirementChecks"]["infinity"]["noAD8"] =
        json!(state.requirement_checks.infinity_no_ad8);
    player["requirementChecks"]["infinity"]["noSacrifice"] =
        json!(state.requirement_checks.infinity_no_sacrifice);
    player["requirementChecks"]["reality"]["noInfinities"] =
        json!(state.requirement_checks.reality_no_infinities);
    player["requirementChecks"]["reality"]["noEternities"] =
        json!(state.requirement_checks.reality_no_eternities);
    player["requirementChecks"]["reality"]["maxGlyphs"] =
        json!(state.requirement_checks.reality_max_glyphs);
    player["requirementChecks"]["reality"]["maxStudies"] =
        json!(state.requirement_checks.reality_max_studies);
    player["requirementChecks"]["reality"]["maxID1"] =
        decimal(&state.requirement_checks.reality_max_id1);
    player["requirementChecks"]["reality"]["noPurchasedTT"] =
        json!(state.requirement_checks.reality_no_purchased_tt);
    player["requirementChecks"]["reality"]["noTriads"] =
        json!(state.requirement_checks.reality_no_triads);
    player["requirementChecks"]["reality"]["slowestBH"] =
        json!(state.requirement_checks.reality_slowest_bh);
    player["postC4Tier"] = json!(state.post_c4_tier);

    // Celestials (Phase 7). Modelled sub-fields are overwritten in place; the
    // unmodelled ones (quote bits, glyph weights, Ra/Laitela/Pelle) stay at
    // their template defaults.
    {
        let cel = &state.celestials;
        let teresa = &mut player["celestials"]["teresa"];
        teresa["pouredAmount"] = json!(cel.teresa.poured_amount);
        teresa["unlockBits"] = json!(cel.teresa.unlock_bits);
        teresa["run"] = json!(cel.teresa.run);
        teresa["bestRunAM"] = decimal(&cel.teresa.best_run_am);
        teresa["lastRepeatedMachines"] = decimal(&cel.teresa.last_repeated_machines);
        teresa["perkShop"] = json!(cel.teresa.perk_shop);

        let effarig = &mut player["celestials"]["effarig"];
        effarig["relicShards"] = json!(cel.effarig.relic_shards);
        for (i, key) in ["ep", "repl", "dt", "eternities"].iter().enumerate() {
            effarig["glyphWeights"][*key] = json!(cel.effarig.glyph_weights[i]);
        }
        effarig["unlockBits"] = json!(cel.effarig.unlock_bits);
        effarig["run"] = json!(cel.effarig.run);

        let enslaved = &mut player["celestials"]["enslaved"];
        enslaved["isStoring"] = json!(cel.enslaved.is_storing);
        enslaved["stored"] = json!(cel.enslaved.stored);
        enslaved["isStoringReal"] = json!(cel.enslaved.is_storing_real);
        enslaved["storedReal"] = json!(cel.enslaved.stored_real);
        enslaved["run"] = json!(cel.enslaved.run);
        enslaved["completed"] = json!(cel.enslaved.completed);
        enslaved["tesseracts"] = json!(cel.enslaved.tesseracts);
        enslaved["autoStoreReal"] = json!(cel.enslaved.auto_store_real);
        enslaved["isAutoReleasing"] = json!(cel.enslaved.is_auto_releasing);
        // Pack the unlock bitset back into the `unlocks` id array.
        let unlocks: Vec<u8> = (0..2u8)
            .filter(|id| cel.enslaved.unlock_bits & (1u32 << id) != 0)
            .collect();
        enslaved["unlocks"] = json!(unlocks);

        let v = &mut player["celestials"]["v"];
        v["unlockBits"] = json!(cel.v.unlock_bits);
        v["run"] = json!(cel.v.run);
        v["runUnlocks"] = json!(cel.v.run_unlocks);
        v["goalReductionSteps"] = json!(cel.v.goal_reduction_steps);
        v["STSpent"] = json!(cel.v.st_spent);
        v["runRecords"] = json!(cel.v.run_records);

        // Ra (Feature 7.5): pets, unlocks, charged set, alchemy, refinement.
        let ra = &cel.ra;
        let ra_json = &mut player["celestials"]["ra"];
        for (key, i) in [
            ("teresa", crate::celestials::ra::PET_TERESA),
            ("effarig", crate::celestials::ra::PET_EFFARIG),
            ("enslaved", crate::celestials::ra::PET_ENSLAVED),
            ("v", crate::celestials::ra::PET_V),
        ] {
            let p = &ra.pets[i];
            let pet = &mut ra_json["pets"][key];
            pet["level"] = json!(p.level);
            pet["memories"] = json!(p.memories);
            pet["memoryChunks"] = json!(p.memory_chunks);
            pet["memoryUpgrades"] = json!(p.memory_upgrades);
            pet["chunkUpgrades"] = json!(p.chunk_upgrades);
        }
        ra_json["unlockBits"] = json!(ra.unlock_bits);
        ra_json["run"] = json!(ra.run);
        ra_json["disCharge"] = json!(ra.dis_charge);
        ra_json["peakGamespeed"] = json!(ra.peak_gamespeed);
        ra_json["momentumTime"] = json!(ra.momentum_time);
        ra_json["charged"] = json!((0..16u32)
            .filter(|id| ra.charged & (1u16 << id) != 0)
            .collect::<Vec<_>>());
        ra_json["petWithRemembrance"] = json!(match ra.pet_with_remembrance {
            0 => "teresa",
            1 => "effarig",
            2 => "enslaved",
            3 => "v",
            _ => "",
        });
        ra_json["alchemy"] = json!(ra
            .alchemy
            .iter()
            .map(|a| json!({ "amount": a.amount, "reaction": a.reaction }))
            .collect::<Vec<_>>());
        ra_json["highestRefinementValue"] = json!({
            "power": ra.highest_refinement_value[0],
            "infinity": ra.highest_refinement_value[1],
            "time": ra.highest_refinement_value[2],
            "replication": ra.highest_refinement_value[3],
            "dilation": ra.highest_refinement_value[4],
            "effarig": ra.highest_refinement_value[5],
        });

        // Lai'tela (Feature 7.6).
        let l = &cel.laitela;
        let lj = &mut player["celestials"]["laitela"];
        lj["darkMatter"] = decimal(&l.dark_matter);
        lj["maxDarkMatter"] = decimal(&l.max_dark_matter);
        lj["darkEnergy"] = json!(l.dark_energy);
        lj["singularities"] = json!(l.singularities);
        lj["singularityCapIncreases"] = json!(l.singularity_cap_increases);
        lj["darkMatterMult"] = json!(l.dark_matter_mult);
        lj["run"] = json!(l.run);
        lj["entropy"] = json!(l.entropy);
        lj["thisCompletion"] = json!(l.this_completion);
        lj["fastestCompletion"] = json!(l.fastest_completion);
        lj["difficultyTier"] = json!(l.difficulty_tier);
        lj["dimensions"] = json!(l
            .dimensions
            .iter()
            .map(|d| json!({
                "amount": d.amount.to_string(),
                "intervalUpgrades": d.interval_upgrades,
                "powerDMUpgrades": d.power_dm_upgrades,
                "powerDEUpgrades": d.power_de_upgrades,
                "timeSinceLastUpdate": d.time_since_last_update,
                "ascensionCount": d.ascension_count,
            }))
            .collect::<Vec<_>>());
    }

    // Imaginary Upgrades (Feature 6.4-late / 7.6).
    player["reality"]["imaginaryUpgradeBits"] =
        json!(state.reality.imaginary_upgrade_bits);
    player["reality"]["imaginaryUpgReqs"] = json!(state.reality.imaginary_upg_reqs);
    player["reality"]["imaginaryMachines"] =
        json!(state.reality.imaginary_machines.to_f64());
    player["reality"]["iMCap"] = json!(state.reality.im_cap);
    player["reality"]["imaginaryRebuyables"] = json!(state
        .reality
        .imaginary_rebuyables
        .iter()
        .enumerate()
        .map(|(i, &n)| ((i + 1).to_string(), n))
        .collect::<std::collections::HashMap<_, _>>());

    // Pelle (Feature 7.7).
    {
        let p = &state.celestials.pelle;
        let pj = &mut player["celestials"]["pelle"];
        pj["doomed"] = json!(p.doomed);
        pj["remnants"] = json!(p.remnants);
        pj["realityShards"] = json!(p.reality_shards.to_string());
        pj["records"] = json!({
            "totalAntimatter": p.records.total_antimatter.to_string(),
            "totalInfinityPoints": p.records.total_infinity_points.to_string(),
            "totalEternityPoints": p.records.total_eternity_points.to_string(),
        });
        pj["upgrades"] = json!((0..23u32)
            .filter(|id| p.upgrades & (1u32 << id) != 0)
            .collect::<Vec<_>>());
        pj["progressBits"] = json!(p.progress_bits);
        let rb_keys = [
            "antimatterDimensionMult",
            "timeSpeedMult",
            "glyphLevels",
            "infConversion",
            "galaxyPower",
        ];
        let gg_keys = [
            "galaxyGeneratorAdditive",
            "galaxyGeneratorMultiplicative",
            "galaxyGeneratorAntimatterMult",
            "galaxyGeneratorIPMult",
            "galaxyGeneratorEPMult",
        ];
        let mut rebuyables = serde_json::Map::new();
        for (i, k) in rb_keys.iter().enumerate() {
            rebuyables.insert(k.to_string(), json!(p.rebuyables[i]));
        }
        for (i, k) in gg_keys.iter().enumerate() {
            rebuyables.insert(k.to_string(), json!(p.gg_rebuyables[i]));
        }
        pj["rebuyables"] = serde_json::Value::Object(rebuyables);
        for (i, key) in ["vacuum", "decay", "chaos", "recursion", "paradox"]
            .iter()
            .enumerate()
        {
            let r = &p.rifts[i];
            let rift = &mut pj["rifts"][*key];
            // Chaos stores `fill` as a number; the others as a string.
            rift["fill"] = if i == 2 {
                json!(r.fill.to_f64())
            } else {
                json!(r.fill.to_string())
            };
            rift["active"] = json!(r.active);
            rift["reducedTo"] = json!(r.reduced_to);
            if i == 1 {
                rift["percentageSpent"] = json!(r.percentage_spent);
            }
        }
        let g = &p.galaxy_generator;
        pj["galaxyGenerator"] = json!({
            "unlocked": g.unlocked,
            "spentGalaxies": g.spent_galaxies,
            "generatedGalaxies": g.generated_galaxies,
            "phase": g.phase,
            "sacrificeActive": g.sacrifice_active,
        });
    }
    player["isGameEnd"] = json!(state.is_game_end);

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
    player["blackHoleNegative"] = json!(state.black_holes.negative);
    player["blackHoleAutoPauseMode"] = json!(state.black_holes.auto_pause_mode);
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
    rep["timer"] = json!(state.replicanti.timer_ms);

    // Options.
    let options = &mut player["options"];
    options["hotkeys"] = json!(state.options.hotkeys);
    options["retryChallenge"] = json!(state.options.retry_challenge);
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
    options["statTabResources"] = json!(state.options.stat_tab_resources);

    // The Past Prestige Runs expand/collapse flags (capitalized layer keys).
    player["shownRuns"]["Infinity"] = json!(state.shown_runs.infinity);
    player["shownRuns"]["Eternity"] = json!(state.shown_runs.eternity);
    player["shownRuns"]["Reality"] = json!(state.shown_runs.reality);

    // Autobuyers. We write the flags/modes plus the interval-upgrade state
    // (interval + IP cost, Feature 2.6), the AD-only "Buys max" bulk multiplier,
    // and `lastTick` — the JS absolute-timestamp timer phase, reconstructed from
    // our elapsed-time `timer_ms` as `realTimePlayed - timer_ms` (the inverse of
    // the load conversion).
    let real_time = state.records.real_time_played_ms;
    let last_tick = |timer_ms: f64| json!(real_time - timer_ms);
    player["auto"]["autobuyersOn"] = json!(state.autobuyers.enabled);
    player["auto"]["antimatterDims"]["isActive"] =
        json!(state.autobuyers.ad_group_active);
    for (tier, ab) in state.autobuyers.dimensions.iter().enumerate() {
        let entry = &mut player["auto"]["antimatterDims"]["all"][tier];
        entry["isActive"] = json!(ab.is_active);
        entry["isBought"] = json!(ab.is_bought);
        entry["mode"] = json!(mode_to_raw(ab.mode));
        entry["interval"] = json!(ab.interval_ms);
        entry["cost"] = json!(ab.cost);
        entry["bulk"] = json!(ab.bulk);
        entry["lastTick"] = last_tick(ab.timer_ms);
    }
    let tickspeed = &mut player["auto"]["tickspeed"];
    tickspeed["isActive"] = json!(state.autobuyers.tickspeed.is_active);
    tickspeed["isBought"] = json!(state.autobuyers.tickspeed.is_bought);
    // The Tickspeed autobuyer's "Buys max" is `AUTOBUYER_MODE.BUY_MAX` (100), not
    // the AD `BUY_10` (10) that `mode_to_raw` emits.
    tickspeed["mode"] = json!(match state.autobuyers.tickspeed.mode {
        AutobuyerMode::BuyMax => AUTOBUYER_MODE_BUY_MAX,
        AutobuyerMode::BuySingle => AUTOBUYER_MODE_BUY_SINGLE,
    });
    tickspeed["interval"] = json!(state.autobuyers.tickspeed.interval_ms);
    tickspeed["cost"] = json!(state.autobuyers.tickspeed.cost);
    tickspeed["lastTick"] = last_tick(state.autobuyers.tickspeed.timer_ms);
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
        entry["lastTick"] = last_tick(ab.timer_ms);
    }
    // Dim Boost / Galaxy limit config.
    let dbc = &state.autobuyers.dim_boost_config;
    let db = &mut player["auto"]["dimBoost"];
    db["limitDimBoosts"] = json!(dbc.limit_dim_boosts);
    db["maxDimBoosts"] = json!(dbc.max_dim_boosts);
    db["limitUntilGalaxies"] = json!(dbc.limit_until_galaxies);
    db["galaxies"] = json!(dbc.until_galaxies);
    db["buyMaxInterval"] = json!(dbc.buy_max_interval);
    let gc = &state.autobuyers.galaxy_config;
    let g = &mut player["auto"]["galaxy"];
    g["limitGalaxies"] = json!(gc.limit_galaxies);
    g["maxGalaxies"] = json!(gc.max_galaxies);
    g["buyMax"] = json!(gc.buy_max);
    g["buyMaxInterval"] = json!(gc.buy_max_interval);
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
    player["auto"]["ipMultBuyer"]["isActive"] =
        json!(state.autobuyers.ip_mult_buyer_active);
    player["auto"]["sacrifice"]["isActive"] = json!(state.autobuyers.sacrifice_active);
    player["auto"]["sacrifice"]["multiplier"] =
        decimal(&state.autobuyers.sacrifice_multiplier);
    // The milestone autobuyers: Infinity Dimensions (11–18 Eternities),
    // Replicanti upgrades (50/60/80), and the Replicanti Galaxy toggle (3).
    player["auto"]["infinityDims"]["isActive"] =
        json!(state.autobuyers.infinity_dims_group_active);
    for (i, ab) in state.autobuyers.infinity_dims.iter().enumerate() {
        let entry = &mut player["auto"]["infinityDims"]["all"][i];
        entry["isActive"] = json!(ab.is_active);
        entry["lastTick"] = last_tick(ab.timer_ms);
    }
    player["auto"]["replicantiUpgrades"]["isActive"] =
        json!(state.autobuyers.replicanti_upgrades_group_active);
    for (i, ab) in state.autobuyers.replicanti_upgrades.iter().enumerate() {
        let entry = &mut player["auto"]["replicantiUpgrades"]["all"][i];
        entry["isActive"] = json!(ab.is_active);
        entry["lastTick"] = last_tick(ab.timer_ms);
    }
    player["auto"]["replicantiGalaxies"]["isActive"] =
        json!(state.autobuyers.replicanti_galaxies_active);
    player["auto"]["timeDims"]["isActive"] =
        json!(state.autobuyers.time_dims_group_active);
    for (i, ab) in state.autobuyers.time_dims.iter().enumerate() {
        let entry = &mut player["auto"]["timeDims"]["all"][i];
        entry["isActive"] = json!(ab.is_active);
        entry["lastTick"] = last_tick(ab.timer_ms);
    }
    player["auto"]["epMultBuyer"]["isActive"] =
        json!(state.autobuyers.ep_mult_buyer_active);
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
            for tier in 0..8 {
                assert_eq!(
                    reloaded.autobuyers.dimensions[tier].bulk,
                    state.autobuyers.dimensions[tier].bulk
                );
            }
            assert_eq!(reloaded.options, state.options);
        }
    }

    #[test]
    fn statistics_fields_round_trip() {
        // The Statistics-tab passthroughs survive decode → encode → decode:
        // gameCreatedTime, statTabResources, and the shownRuns flags.
        let mut state = decode_save(SAMPLE_SAVE.trim()).unwrap();
        state.records.game_created_time_ms = 1_650_000_000_123.0;
        state.options.stat_tab_resources = 2;
        state.shown_runs.eternity = false;
        let reloaded = decode_save(&encode_save(&state, 1_700_000_000_000)).unwrap();
        assert_eq!(reloaded.records.game_created_time_ms, 1_650_000_000_123.0);
        assert_eq!(reloaded.options.stat_tab_resources, 2);
        assert!(reloaded.shown_runs.infinity);
        assert!(!reloaded.shown_runs.eternity);
        assert!(reloaded.shown_runs.reality);
    }

    #[test]
    fn ad_autobuyer_bulk_round_trips() {
        // A non-default bulk survives decode → encode → decode.
        let mut state = decode_save(SAMPLE_SAVE.trim()).unwrap();
        state.autobuyers.dimensions[0].bulk = 64;
        state.autobuyers.dimensions[3].bulk = 512;
        let reloaded = decode_save(&encode_save(&state, 1_700_000_000_000)).unwrap();
        assert_eq!(reloaded.autobuyers.dimensions[0].bulk, 64);
        assert_eq!(reloaded.autobuyers.dimensions[3].bulk, 512);
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
        state.reality.glyphs.sac = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
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
        assert_eq!(
            reloaded.reality.glyphs.sac,
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
        );
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
