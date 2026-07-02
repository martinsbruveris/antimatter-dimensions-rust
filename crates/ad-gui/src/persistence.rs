//! On-disk persistence for the Tauri app (design doc §12).
//!
//! `ad-core`'s save module is pure — no IO, no wall clock. This module is the
//! layer above it that actually touches the filesystem and the clock: it owns the
//! app-data-dir layout, the three save slots, the eight automatic backup slots
//! per save slot, and startup/offline handling. The engine codec
//! ([`ad_core::save`]) does all the (de)serialization; we only orchestrate slots,
//! files, and timing.
//!
//! ## Layout (§12.2)
//!
//! ```text
//! {app_data_dir}/
//! ├── saves.dat            # encoded { current, saves } root (all 3 slots)
//! └── backups/
//!     ├── 0/ … 2/          # one directory per save slot
//!     │   ├── 1.dat … 8.dat   # encoded single-player backups
//! ```
//!
//! Each `.dat` is a `GameSaveSerializer`-encoded string (the same `AAB` format),
//! so any file here can be pasted into the original game's import box. Backup
//! ages are read from file mtimes rather than a separate timing file — simpler
//! than the original's `backupTimes-*` key and adequate for the age display.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ad_core::save::{
    decode_root, decode_save, decode_save_file, decode_save_with_last_update,
    encode_backup_bundle, encode_root, encode_save, BackupSlotSave, ImportedSave,
    RootSave, SAVE_SLOT_COUNT,
};
use ad_core::GameState;

/// The eight automatic backup slots (§11.3), by id. `online` slots are checked
/// on a timer while the app runs; `offline` slots fire once on load if the gap
/// since the last save exceeds their interval; the reserve slot (8) is manual.
pub struct BackupSlotSpec {
    pub id: u8,
    pub interval_ms: i64,
    pub kind: BackupKind,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BackupKind {
    Online,
    Offline,
    Reserve,
}

/// The eight backup slots, matching the original's `AutoBackupSlots`.
pub const BACKUP_SLOTS: [BackupSlotSpec; 8] = [
    BackupSlotSpec {
        id: 1,
        interval_ms: 60_000,
        kind: BackupKind::Online,
    },
    BackupSlotSpec {
        id: 2,
        interval_ms: 300_000,
        kind: BackupKind::Online,
    },
    BackupSlotSpec {
        id: 3,
        interval_ms: 1_200_000,
        kind: BackupKind::Online,
    },
    BackupSlotSpec {
        id: 4,
        interval_ms: 3_600_000,
        kind: BackupKind::Online,
    },
    BackupSlotSpec {
        id: 5,
        interval_ms: 600_000,
        kind: BackupKind::Offline,
    },
    BackupSlotSpec {
        id: 6,
        interval_ms: 3_600_000,
        kind: BackupKind::Offline,
    },
    BackupSlotSpec {
        id: 7,
        interval_ms: 18_000_000,
        kind: BackupKind::Offline,
    },
    BackupSlotSpec {
        id: 8,
        interval_ms: 0,
        kind: BackupKind::Reserve,
    },
];

/// Metadata for one backup slot, for the Backup menu display.
pub struct BackupMeta {
    pub id: u8,
    /// The backed-up antimatter, if the slot is populated (`m`, `e`).
    pub antimatter: Option<(f64, f64)>,
    /// When the backup file was last written (epoch ms, from its mtime). The
    /// frontend subtracts this from its live clock so the "Last saved … ago"
    /// value ticks in real time.
    pub last_backup_ms: Option<i64>,
}

/// Metadata for one save slot, for the "Choose save" modal.
pub struct SlotMeta {
    pub id: usize,
    /// The slot's antimatter, if populated (`m`, `e`).
    pub antimatter: Option<(f64, f64)>,
    /// The slot's custom save-file name (empty if unset / slot empty).
    pub save_file_name: String,
    pub is_current: bool,
}

/// Owns the on-disk save state: the app-data directory, the active slot, and the
/// last-known state of every slot (the current slot is refreshed from the live
/// engine state on every save via [`sync_current`](Self::sync_current)).
pub struct SaveManager {
    dir: PathBuf,
    current: usize,
    /// Last-known state per slot; `slots[current]` is refreshed from the live
    /// engine before any read/write, the others are authoritative on disk.
    slots: [Option<GameState>; SAVE_SLOT_COUNT],
}

impl SaveManager {
    /// Creates a manager rooted at `dir`, ensuring the directory exists.
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self {
            dir,
            current: 0,
            slots: Default::default(),
        }
    }

    fn root_path(&self) -> PathBuf {
        self.dir.join("saves.dat")
    }

    fn backup_path(&self, save_slot: usize, backup_slot: u8) -> PathBuf {
        self.dir
            .join("backups")
            .join(save_slot.to_string())
            .join(format!("{backup_slot}.dat"))
    }

    /// Refreshes `slots[current]` from the live engine state so subsequent reads
    /// and writes see the freshest current-slot data.
    pub fn sync_current(&mut self, live: &GameState) {
        self.slots[self.current] = Some(live.clone());
    }

    /// Loads `saves.dat` at startup, returning the active slot's [`GameState`]
    /// and the **offline gap** (ms since the slot's `lastUpdate`, clamped at 0)
    /// for the caller to replay as offline progress. Populates the slot cache and
    /// the active-slot index, then fires any applicable offline backup (§11.3).
    ///
    /// `fresh` supplies a starting state for an empty active slot (so debug
    /// conveniences apply on a truly first run). `now_ms` is the wall clock.
    pub fn load(&mut self, fresh: GameState, now_ms: i64) -> (GameState, i64) {
        let root = match fs::read_to_string(self.root_path()) {
            Ok(text) => match decode_root(text.trim()) {
                Ok(root) => root,
                // A corrupt root shouldn't wedge startup; begin fresh.
                Err(_) => RootSave::empty(),
            },
            Err(_) => RootSave::empty(),
        };

        self.current = root.current.min(SAVE_SLOT_COUNT - 1);
        self.slots = root.saves;

        let live = self.slots[self.current].clone().unwrap_or(fresh);

        // The offline gap for startup catch-up: how long the active slot sat since
        // its last save. Also drives the offline backup below.
        let gap_ms = match root.last_updates[self.current] {
            Some(last_update) => {
                let gap_ms = now_ms - last_update;
                // Offline backup: if the app was closed for longer than an offline
                // slot's interval, back up the loaded (pre-offline) state into the
                // longest applicable slot (matching `backupOfflineSlots`).
                self.write_offline_backup(&live, gap_ms, now_ms);
                gap_ms
            }
            None => 0,
        };

        (live, gap_ms.max(0))
    }

    /// Assembles the `{ current, saves }` root from the slot cache (with the
    /// current slot taken from `live`) and writes it to `saves.dat`.
    pub fn save_root(&mut self, live: &GameState, now_ms: i64) -> std::io::Result<()> {
        self.sync_current(live);
        let root = RootSave {
            current: self.current,
            saves: self.slots.clone(),
            last_updates: Default::default(),
        };
        let encoded = encode_root(&root, now_ms);
        self.write_atomic(&self.root_path(), &encoded)
    }

    /// Switches the active save slot: persists the current slot, then loads the
    /// target slot (or a fresh state for an empty slot). Returns the new live
    /// state to install into the engine.
    pub fn switch_slot(
        &mut self,
        live: &GameState,
        target: usize,
        fresh: GameState,
        now_ms: i64,
    ) -> GameState {
        let target = target.min(SAVE_SLOT_COUNT - 1);
        self.sync_current(live);
        self.current = target;
        let new_live = self.slots[target].clone().unwrap_or(fresh);
        // Persist so the on-disk `current` and (possibly newly-created) slot stick.
        let _ = self.save_root(&new_live, now_ms);
        new_live
    }

    /// Writes the live state into one backup slot of the current save slot.
    pub fn write_backup(
        &self,
        backup_slot: u8,
        live: &GameState,
        now_ms: i64,
    ) -> std::io::Result<()> {
        let path = self.backup_path(self.current, backup_slot);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.write_atomic(&path, &encode_save(live, now_ms))
    }

    /// Fires the offline backup for a load gap: the single longest offline slot
    /// whose interval is exceeded (§11.3), matching the original's
    /// `backupOfflineSlots` (which uses a strict `offlineTime > interval`).
    fn write_offline_backup(&self, live: &GameState, gap_ms: i64, now_ms: i64) {
        let slot = BACKUP_SLOTS
            .iter()
            .filter(|s| s.kind == BackupKind::Offline && gap_ms > s.interval_ms)
            .max_by_key(|s| s.interval_ms);
        if let Some(slot) = slot {
            let _ = self.write_backup(slot.id, live, now_ms);
        }
    }

    /// Loads a backup slot into a new live state, returning it alongside the
    /// backup's `lastUpdate` (epoch ms, if present) so the caller can replay the
    /// offline gap. Before loading, the current state is written to the reserve
    /// slot (8), matching the original ("your current save is saved into the last
    /// slot any time a backup is loaded").
    pub fn load_backup(
        &mut self,
        backup_slot: u8,
        live: &GameState,
        now_ms: i64,
    ) -> Result<(GameState, Option<i64>), String> {
        let text = fs::read_to_string(self.backup_path(self.current, backup_slot))
            .map_err(|e| format!("Failed to read backup: {e}"))?;
        let (loaded, last_update) =
            decode_save_with_last_update(text.trim()).map_err(|e| e.to_string())?;
        // Save the current state into the reserve slot before replacing it.
        let _ = self.write_backup(8, live, now_ms);
        // Persist the newly-loaded state as the current slot too.
        let _ = self.save_root(&loaded, now_ms);
        Ok((loaded, last_update))
    }

    /// Reads metadata for all three save slots (current taken from `live`).
    pub fn slot_metas(&mut self, live: &GameState) -> Vec<SlotMeta> {
        self.sync_current(live);
        (0..SAVE_SLOT_COUNT)
            .map(|id| SlotMeta {
                id,
                antimatter: self.slots[id].as_ref().map(|s| decimal_me(&s.antimatter)),
                save_file_name: self.slots[id]
                    .as_ref()
                    .map(|s| s.options.save_file_name.clone())
                    .unwrap_or_default(),
                is_current: id == self.current,
            })
            .collect()
    }

    /// Reads metadata for the eight backup slots of the current save slot.
    pub fn backup_metas(&self) -> Vec<BackupMeta> {
        BACKUP_SLOTS
            .iter()
            .map(|spec| {
                let path = self.backup_path(self.current, spec.id);
                let (antimatter, last_backup_ms) = match fs::read_to_string(&path) {
                    Ok(text) => {
                        let am = decode_save(text.trim())
                            .ok()
                            .map(|s| decimal_me(&s.antimatter));
                        (am, file_mtime_ms(&path))
                    }
                    Err(_) => (None, None),
                };
                BackupMeta {
                    id: spec.id,
                    antimatter,
                    last_backup_ms,
                }
            })
            .collect()
    }

    /// Bundles every populated backup slot of the current save slot into a
    /// backup-bundle file string (§2.4), or `None` if there are no backups.
    pub fn export_backups(&self, now_ms: i64) -> Option<String> {
        let mut slots = Vec::new();
        for spec in &BACKUP_SLOTS {
            let path = self.backup_path(self.current, spec.id);
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(state) = decode_save(text.trim()) {
                    slots.push(BackupSlotSave {
                        id: spec.id,
                        backup_timer: 0.0,
                        last_update: None,
                        state,
                    });
                }
            }
        }
        if slots.is_empty() {
            None
        } else {
            Some(encode_backup_bundle(&slots, now_ms))
        }
    }

    /// Imports a backup-bundle (or single-save) file, writing each contained
    /// player into the corresponding backup slot of the current save slot.
    /// Returns how many slots were written.
    pub fn import_backups(&mut self, text: &str, now_ms: i64) -> Result<usize, String> {
        match decode_save_file(text.trim()).map_err(|e| e.to_string())? {
            ImportedSave::Backups(slots) => {
                let mut count = 0;
                for slot in slots {
                    if self.write_backup(slot.id, &slot.state, now_ms).is_ok() {
                        count += 1;
                    }
                }
                Ok(count)
            }
            // A plain single save imported here goes into the reserve slot.
            ImportedSave::Single(state) => {
                self.write_backup(8, &state, now_ms)
                    .map_err(|e| format!("Failed to write backup: {e}"))?;
                Ok(1)
            }
        }
    }

    /// Writes `contents` to `path` via a temp file + rename, so an interrupted
    /// write can't corrupt an existing save.
    fn write_atomic(&self, path: &PathBuf, contents: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, contents)?;
        fs::rename(&tmp, path)
    }
}

/// The current wall-clock time in epoch milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Splits a `Decimal` into its `(mantissa, exponent)` pair for the snapshot's
/// raw-number representation.
fn decimal_me(value: &ad_core::Decimal) -> (f64, f64) {
    (value.mantissa(), value.exponent() as f64)
}

/// The last-modified time of `path` as epoch milliseconds, or `None` if
/// unavailable.
fn file_mtime_ms(path: &PathBuf) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use ad_core::Decimal;

    /// Parses a `Decimal` from a decimal string for the tests.
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// A unique scratch directory for one test's on-disk saves.
    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ad-persist-{tag}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A game state with a recognisable antimatter value.
    fn state_with_am(am: &str) -> GameState {
        let mut g = GameState::new();
        g.antimatter = dec(am);
        g
    }

    #[test]
    fn save_root_then_load_round_trips_current_slot() {
        let dir = temp_dir("root");
        let mut sm = SaveManager::new(dir.clone());
        sm.save_root(&state_with_am("12345"), 1000).unwrap();

        // A fresh manager over the same dir loads the saved current slot.
        let mut sm2 = SaveManager::new(dir);
        let (loaded, _gap) = sm2.load(GameState::new(), 2000);
        assert_eq!(loaded.antimatter, dec("12345"));
    }

    #[test]
    fn switch_slot_preserves_each_slot() {
        let dir = temp_dir("slots");
        let mut sm = SaveManager::new(dir.clone());

        // Slot 0 holds "100"; switch to slot 1 (empty → fresh), give it "200".
        sm.save_root(&state_with_am("100"), 1000).unwrap();
        let live1 = sm.switch_slot(&state_with_am("100"), 1, GameState::new(), 1000);
        assert_eq!(live1.antimatter, GameState::new().antimatter); // fresh
        sm.save_root(&state_with_am("200"), 1000).unwrap();

        // Switching back to slot 0 restores "100"; slot 1 still holds "200".
        let back0 = sm.switch_slot(&state_with_am("200"), 0, GameState::new(), 1000);
        assert_eq!(back0.antimatter, dec("100"));
        let back1 = sm.switch_slot(&back0, 1, GameState::new(), 1000);
        assert_eq!(back1.antimatter, dec("200"));

        // The metadata view reflects both populated slots.
        let metas = sm.slot_metas(&back1);
        assert!(metas[0].antimatter.is_some());
        assert!(metas[1].antimatter.is_some());
        assert!(metas[2].antimatter.is_none());
    }

    #[test]
    fn slot_meta_carries_per_slot_save_file_name() {
        let dir = temp_dir("names");
        let mut sm = SaveManager::new(dir.clone());

        // Name the current slot, save, and reload in a fresh manager.
        let mut named = state_with_am("500");
        named.options.set_save_file_name("My File");
        sm.save_root(&named, 1000).unwrap();

        let mut sm2 = SaveManager::new(dir);
        let (live, _gap) = sm2.load(GameState::new(), 2000);
        let metas = sm2.slot_metas(&live);
        assert_eq!(metas[0].save_file_name, "My File");
        // Empty slots report no name.
        assert_eq!(metas[1].save_file_name, "");
    }

    #[test]
    fn backup_write_and_load_round_trips() {
        let dir = temp_dir("backup");
        let mut sm = SaveManager::new(dir);
        sm.save_root(&state_with_am("42"), 1000).unwrap();

        sm.write_backup(3, &state_with_am("777"), 1000).unwrap();
        let metas = sm.backup_metas();
        assert!(metas
            .iter()
            .find(|m| m.id == 3)
            .unwrap()
            .antimatter
            .is_some());
        assert!(metas
            .iter()
            .find(|m| m.id == 1)
            .unwrap()
            .antimatter
            .is_none());

        // Loading the backup replaces the live state and reserves the old one.
        let (loaded, _last) = sm.load_backup(3, &state_with_am("42"), 1000).unwrap();
        assert_eq!(loaded.antimatter, dec("777"));
        // The reserve slot (8) now holds the pre-load state.
        let reserve = sm.backup_metas();
        assert!(reserve
            .iter()
            .find(|m| m.id == 8)
            .unwrap()
            .antimatter
            .is_some());
    }

    #[test]
    fn backup_bundle_export_import_round_trips() {
        let src = temp_dir("bundle-src");
        let mut a = SaveManager::new(src);
        a.save_root(&state_with_am("1"), 1000).unwrap();
        a.write_backup(1, &state_with_am("111"), 1000).unwrap();
        a.write_backup(5, &state_with_am("555"), 1000).unwrap();

        let bundle = a.export_backups(1000).expect("has backups");

        // Import into a different manager and confirm both slots landed.
        let dst = temp_dir("bundle-dst");
        let mut b = SaveManager::new(dst);
        b.save_root(&state_with_am("2"), 1000).unwrap();
        assert_eq!(b.import_backups(&bundle, 1000).unwrap(), 2);
        let metas = b.backup_metas();
        assert!(metas
            .iter()
            .find(|m| m.id == 1)
            .unwrap()
            .antimatter
            .is_some());
        assert!(metas
            .iter()
            .find(|m| m.id == 5)
            .unwrap()
            .antimatter
            .is_some());
    }

    #[test]
    fn offline_backup_fires_on_long_gap() {
        let dir = temp_dir("offline");
        let mut sm = SaveManager::new(dir.clone());
        // Save at t = 0.
        sm.save_root(&state_with_am("99"), 0).unwrap();

        // Reopen 20 minutes later: exceeds the 10-min offline slot (5), not the
        // 1-hour one (6), so slot 5 (the longest applicable) is written.
        let mut sm2 = SaveManager::new(dir);
        let (_live, gap) = sm2.load(GameState::new(), 20 * 60_000);
        // The returned offline gap (20 min) is what startup catch-up replays.
        assert_eq!(gap, 20 * 60_000);
        let metas = sm2.backup_metas();
        assert!(metas
            .iter()
            .find(|m| m.id == 5)
            .unwrap()
            .antimatter
            .is_some());
        assert!(metas
            .iter()
            .find(|m| m.id == 6)
            .unwrap()
            .antimatter
            .is_none());
    }
}
