//! Multi-player save **bundles**: the two JSON shapes that pack several `player`
//! objects into one document sharing the [`codec`](super::codec) byte pipeline.
//!
//! Everything else in [`save`](super) handles a single `player`; these two shapes
//! do not:
//!
//! - **The localStorage root** (§11.1 of the save design): `{ "current": n,
//!   "saves": { "0": player, "1": player|null, "2": player|null } }`. This is our
//!   on-disk `saves.dat`: the active-slot index plus up to three save slots. See
//!   [`RootSave`], [`encode_root`], [`decode_root`].
//! - **The backup-bundle file** (§2.4): `{ "<slotId>": player, …, "time": { … } }`
//!   — the Backup menu's *Export/Import as File* payload, a map of populated
//!   backup slots (ids 1–8) to full players plus a reserved `time` key of timing
//!   metadata. See [`ImportedSave`], [`decode_save_file`], [`encode_backup_bundle`].
//!
//! Like the rest of the module these are pure and deterministic: the only
//! time-varying input is the caller-supplied `now_ms`, so `ad-core` stays free of
//! the wall clock. All filesystem IO, slot management, and wall-clock timing live
//! above this layer in `ad-gui`.

use serde_json::{json, Map, Value};

use super::codec::{decode_pipeline, encode_pipeline};
use super::encode::to_player_value;
use super::{from_player_value, SaveError};
use crate::state::GameState;

/// The number of save slots in the localStorage root (§11.1): slots 0, 1, 2.
pub const SAVE_SLOT_COUNT: usize = 3;

/// The decoded localStorage-root save (`saves.dat`).
///
/// Holds the active-slot index and the up-to-three save slots, each either a
/// full [`GameState`] or empty (`None`). [`last_updates`](Self::last_updates)
/// carries each slot's `player.lastUpdate` so the persistence layer can compute
/// the offline gap on load without re-decoding.
#[derive(Debug)]
pub struct RootSave {
    /// The active save-slot index (0..[`SAVE_SLOT_COUNT`)).
    pub current: usize,
    /// The save slots; `None` is an empty slot.
    pub saves: [Option<GameState>; SAVE_SLOT_COUNT],
    /// `player.lastUpdate` (epoch ms) per slot, for offline-gap detection.
    pub last_updates: [Option<i64>; SAVE_SLOT_COUNT],
}

impl RootSave {
    /// A fresh root with all slots empty and slot 0 active.
    pub fn empty() -> Self {
        Self {
            current: 0,
            saves: Default::default(),
            last_updates: Default::default(),
        }
    }
}

/// Encodes a [`RootSave`] into the on-disk `saves.dat` string (the §11.1
/// `{ current, saves }` shape run through the byte pipeline). Each populated slot
/// is stamped with `now_ms` as its `lastUpdate`; empty slots serialize as `null`.
pub fn encode_root(root: &RootSave, now_ms: i64) -> String {
    let mut saves = Map::new();
    for (i, slot) in root.saves.iter().enumerate() {
        let value = match slot {
            Some(state) => to_player_value(state, now_ms),
            None => Value::Null,
        };
        saves.insert(i.to_string(), value);
    }
    let root_value = json!({ "current": root.current, "saves": saves });
    encode_pipeline(
        &serde_json::to_string(&root_value).expect("root Value always serializes"),
    )
}

/// Decodes an on-disk `saves.dat` string into a [`RootSave`].
///
/// Accepts the wrapped `{ current, saves }` shape we write, and — defensively —
/// a legacy bare `player` (no `saves` key), which the original migrates into slot
/// 0 on load. Each populated slot's `player` is mapped through
/// [`from_player_value`], applying the same strict validation as
/// [`decode_save`](super::decode_save); an out-of-range `current` clamps into
/// range.
pub fn decode_root(save: &str) -> Result<RootSave, SaveError> {
    let json = decode_pipeline(save)?;
    let root: Value = serde_json::from_str(&json)?;

    // Legacy bare player (pre-slots): a top-level `antimatter` and no `saves`
    // wrapper. Load it into slot 0.
    if root.get("saves").is_none() && root.get("antimatter").is_some() {
        let mut out = RootSave::empty();
        out.last_updates[0] = read_last_update(&root);
        out.saves[0] = Some(from_player_value(&root)?);
        return Ok(out);
    }

    let saves = root
        .get("saves")
        .and_then(Value::as_object)
        .ok_or(SaveError::MissingSavesWrapper)?;

    let mut out = RootSave::empty();
    out.current = root
        .get("current")
        .and_then(Value::as_u64)
        .map(|c| (c as usize).min(SAVE_SLOT_COUNT - 1))
        .unwrap_or(0);

    for (slot, entry) in out.saves.iter_mut().enumerate() {
        match saves.get(&slot.to_string()) {
            Some(player) if !player.is_null() => {
                out.last_updates[slot] = read_last_update(player);
                *entry = Some(from_player_value(player)?);
            }
            _ => {}
        }
    }

    Ok(out)
}

/// One populated backup slot from a backup-bundle file (§2.4).
#[derive(Debug)]
pub struct BackupSlotSave {
    /// The `AutoBackupSlots` id (1–8).
    pub id: u8,
    /// The original in-game `backupTimer` value at the time of the backup.
    pub backup_timer: f64,
    /// The backed-up game state.
    pub state: GameState,
    /// `player.lastUpdate` (epoch ms), if present.
    pub last_update: Option<i64>,
}

/// The result of decoding a file through [`decode_save_file`]: the same byte
/// codec can carry either a single `player` (clipboard/file export) or a backup
/// bundle (Backup menu's *Export as File*), distinguished after decoding by the
/// top-level JSON shape (§2.4).
#[derive(Debug)]
pub enum ImportedSave {
    /// A single `player` — a normal save export. Boxed so this variant doesn't
    /// dwarf the small `Backups` handle (a `GameState` is ~0.6 KB).
    Single(Box<GameState>),
    /// A backup bundle — one or more populated backup slots.
    Backups(Vec<BackupSlotSave>),
}

/// Decodes a save/backup **file**, dispatching on the top-level JSON shape.
///
/// A single `player` has `antimatter` at its root; a backup bundle instead has
/// numeric slot-id keys plus a reserved `time` key (§2.4). The single case maps
/// through [`from_player_value`]; the bundle case returns every contained slot
/// (sorted by id) with its `backupTimer` from the `time` metadata, for the caller
/// to write back / choose among.
pub fn decode_save_file(save: &str) -> Result<ImportedSave, SaveError> {
    let json = decode_pipeline(save)?;
    let value: Value = serde_json::from_str(&json)?;

    // A single player carries `antimatter` at the top; a bundle does not (its
    // top-level keys are slot ids + `time`).
    if value.get("antimatter").is_some() {
        return Ok(ImportedSave::Single(Box::new(from_player_value(&value)?)));
    }

    let obj = value
        .as_object()
        .ok_or(SaveError::UnrecognizedFormat)?;
    let times = obj.get("time").and_then(Value::as_object);

    let mut slots = Vec::new();
    for (key, player) in obj {
        if key == "time" || player.is_null() {
            continue;
        }
        // Every non-`time` key is a backup-slot id.
        let id: u8 = key
            .parse()
            .map_err(|_| SaveError::UnrecognizedFormat)?;
        let backup_timer = times
            .and_then(|t| t.get(key))
            .and_then(|meta| meta.get("backupTimer"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        slots.push(BackupSlotSave {
            id,
            backup_timer,
            last_update: read_last_update(player),
            state: from_player_value(player)?,
        });
    }
    slots.sort_by_key(|s| s.id);
    Ok(ImportedSave::Backups(slots))
}

/// Encodes a set of backup slots into a backup-bundle file string (§2.4): a map
/// of `"<id>" -> player` plus a `time` map of `{ backupTimer, date }` per slot.
/// Each player is stamped with `now_ms` as `lastUpdate`, and `now_ms` is used as
/// each slot's `date`.
pub fn encode_backup_bundle(slots: &[BackupSlotSave], now_ms: i64) -> String {
    let mut obj = Map::new();
    let mut time = Map::new();
    for slot in slots {
        obj.insert(slot.id.to_string(), to_player_value(&slot.state, now_ms));
        time.insert(
            slot.id.to_string(),
            json!({ "backupTimer": slot.backup_timer, "date": now_ms }),
        );
    }
    obj.insert("time".to_string(), Value::Object(time));
    encode_pipeline(
        &serde_json::to_string(&Value::Object(obj)).expect("bundle Value always serializes"),
    )
}

/// Reads `player.lastUpdate` (epoch ms) from a `player` JSON value, if present.
fn read_last_update(player: &Value) -> Option<i64> {
    player.get("lastUpdate").and_then(Value::as_i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{decode_save, encode_save};

    const INITIAL_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_initial_save.txt"
    ));
    const SAMPLE_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_sample_save.txt"
    ));

    #[test]
    fn root_round_trips_slots_and_current() {
        let s0 = decode_save(INITIAL_SAVE.trim()).unwrap();
        let s2 = decode_save(SAMPLE_SAVE.trim()).unwrap();
        let root = RootSave {
            current: 2,
            saves: [Some(s0.clone()), None, Some(s2.clone())],
            last_updates: Default::default(),
        };

        let encoded = encode_root(&root, 1_700_000_000_000);
        let decoded = decode_root(&encoded).unwrap();

        assert_eq!(decoded.current, 2);
        assert!(decoded.saves[0].is_some());
        assert!(decoded.saves[1].is_none());
        assert!(decoded.saves[2].is_some());
        assert_eq!(
            decoded.saves[0].as_ref().unwrap().antimatter,
            s0.antimatter
        );
        assert_eq!(
            decoded.saves[2].as_ref().unwrap().galaxies,
            s2.galaxies
        );
        // Populated slots carry the stamped lastUpdate.
        assert_eq!(decoded.last_updates[0], Some(1_700_000_000_000));
        assert_eq!(decoded.last_updates[1], None);
    }

    #[test]
    fn decode_root_accepts_legacy_bare_player() {
        // A pre-slots save is a bare player; it loads into slot 0.
        let single = encode_save(&decode_save(SAMPLE_SAVE.trim()).unwrap(), 42);
        let root = decode_root(&single).unwrap();
        assert_eq!(root.current, 0);
        assert!(root.saves[0].is_some());
        assert!(root.saves[1].is_none());
        assert_eq!(root.last_updates[0], Some(42));
    }

    #[test]
    fn decode_save_file_dispatches_single() {
        let expected = decode_save(INITIAL_SAVE.trim()).unwrap();
        let single = encode_save(&expected, 0);
        match decode_save_file(&single).unwrap() {
            ImportedSave::Single(state) => {
                assert_eq!(state.antimatter, expected.antimatter)
            }
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[test]
    fn backup_bundle_round_trips() {
        let a = decode_save(INITIAL_SAVE.trim()).unwrap();
        let b = decode_save(SAMPLE_SAVE.trim()).unwrap();
        let slots = vec![
            BackupSlotSave {
                id: 1,
                backup_timer: 60.0,
                last_update: None,
                state: a.clone(),
            },
            BackupSlotSave {
                id: 5,
                backup_timer: 600.0,
                last_update: None,
                state: b.clone(),
            },
        ];

        let encoded = encode_backup_bundle(&slots, 1_700_000_000_000);
        match decode_save_file(&encoded).unwrap() {
            ImportedSave::Backups(decoded) => {
                assert_eq!(decoded.len(), 2);
                assert_eq!(decoded[0].id, 1);
                assert_eq!(decoded[0].backup_timer, 60.0);
                assert_eq!(decoded[0].state.antimatter, a.antimatter);
                assert_eq!(decoded[1].id, 5);
                assert_eq!(decoded[1].backup_timer, 600.0);
                assert_eq!(decoded[1].state.galaxies, b.galaxies);
                assert_eq!(decoded[1].last_update, Some(1_700_000_000_000));
            }
            other => panic!("expected Backups, got {other:?}"),
        }
    }
}
