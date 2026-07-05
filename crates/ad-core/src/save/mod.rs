//! Antimatter Dimensions save (de)serialization.
//!
//! This module is the engine-side codec for AD's save-string format. It is pure
//! and deterministic — no IO, no wall clock (timestamps are passed in by the
//! caller) — so it stays inside `ad-core`'s IO-free boundary and is unit-testable
//! headless.
//!
//! Layering (built up over the save/load phases):
//! - [`codec`] — the byte pipeline that converts between a raw JSON string and
//!   the encoded save string (deflate + base64 + AD's character-safe cleanup and
//!   magic markers). This is phase 2: JSON ⇄ save string only.
//! - *(later)* DTO + `from_save_dto` (read path) and a vendored `defaultStart`
//!   template + overlay (write path), which sit on top of [`codec`].

mod bundle;
pub(crate) mod codec;
mod dto;
mod encode;

pub use bundle::{
    decode_root, decode_save_file, encode_backup_bundle, encode_root, BackupSlotSave,
    ImportedSave, RootSave, SAVE_SLOT_COUNT,
};
pub use codec::{decode_pipeline, encode_pipeline};
pub use dto::PlayerDTO;
pub use encode::{encode_save, to_player_value};

use std::fmt;

use crate::state::GameState;

/// Decodes an AD save string straight into a [`GameState`].
///
/// Runs the [`decode_pipeline`] (save string → JSON), parses the modelled subset
/// of the `player` schema ([`PlayerDTO`], ignoring unmodelled keys but requiring
/// the fields we do model), and maps it in via [`GameState::from_save_dto`].
/// Fields past our frontier are reset to defaults; derived state is rebuilt. The
/// load is strict — a missing modelled field, an out-of-range option, an
/// unexpected array length, or an unrecognized autobuyer mode all error rather
/// than being silently guessed (the sole exception is an unmodelled notation
/// name, which is ignored).
pub fn decode_save(save: &str) -> Result<GameState, SaveError> {
    let json = decode_pipeline(save)?;
    let dto: PlayerDTO = serde_json::from_str(&json)?;
    GameState::from_save_dto(&dto)
}

/// Decodes an AD save string into a [`GameState`] and its `player.lastUpdate`
/// timestamp (epoch ms), if present.
///
/// Same strict decode as [`decode_save`], but also reads the raw `lastUpdate`
/// field so the caller (the persistence layer) can compute the offline gap when
/// loading or importing a save. `None` when the save carries no `lastUpdate`.
pub fn decode_save_with_last_update(
    save: &str,
) -> Result<(GameState, Option<i64>), SaveError> {
    let json = decode_pipeline(save)?;
    let value: serde_json::Value = serde_json::from_str(&json)?;
    let last_update = value.get("lastUpdate").and_then(serde_json::Value::as_i64);
    let state = from_player_value(&value)?;
    Ok((state, last_update))
}

/// Maps an already-parsed `player` JSON [`Value`](serde_json::Value) into a
/// [`GameState`].
///
/// This is the [`decode_save`] tail without the byte pipeline — used by the
/// bundle decoders ([`decode_root`], [`decode_save_file`]), which parse a
/// multi-player JSON document once and then map each contained `player` object
/// through here. Applies the same strict validation as [`decode_save`].
pub fn from_player_value(value: &serde_json::Value) -> Result<GameState, SaveError> {
    let dto: PlayerDTO = serde_json::from_value(value.clone())?;
    GameState::from_save_dto(&dto)
}

/// An error produced while decoding a save string.
#[derive(Debug)]
pub enum SaveError {
    /// The string does not start with the AD savefile magic prefix. We only
    /// support the current `AAB` format; legacy (pre-Reality, `atob`-only) saves
    /// are rejected here rather than supported.
    UnrecognizedFormat,
    /// The magic prefix was present but the 3-char version marker is not the
    /// supported `AAB` (an older pre-`AAB` save or a newer format).
    UnsupportedVersion,
    /// The trailing `EndOfSavefile` marker is missing — the save is truncated or
    /// corrupt (a valid `AAB` save always ends with it).
    MissingEndMarker,
    /// A localStorage-root save (`saves.dat`) had neither the `{ current, saves }`
    /// wrapper nor a bare-`player` shape, so it can't be interpreted as a root.
    MissingSavesWrapper,
    /// The base64 payload could not be decoded (after reversing the cleanup).
    Base64(base64::DecodeError),
    /// The zlib (deflate) stream could not be inflated.
    Inflate(std::io::Error),
    /// The inflated bytes were not valid UTF-8 JSON text.
    Utf8(std::string::FromUtf8Error),
    /// The JSON text could not be parsed into the `player` DTO. This also covers
    /// a modelled field being absent (serde "missing field"): we require the
    /// fields we model rather than silently substituting a default.
    Json(serde_json::Error),
    /// An autobuyer's saved `mode` was not a recognized `AUTOBUYER_MODE`
    /// (`1` = single, `10` = buy-10/max). Holds the offending value.
    InvalidAutobuyerMode(i64),
    /// A modelled numeric option was outside its accepted range. We reject such a
    /// value rather than silently clamping it.
    OptionOutOfRange {
        field: &'static str,
        value: u32,
        min: u32,
        max: u32,
    },
    /// A fixed-length array in the save (the 8 antimatter dimensions or their
    /// autobuyers) had an unexpected length, signalling a format we don't expect.
    UnexpectedArrayLength {
        field: &'static str,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::UnrecognizedFormat => f.write_str(
                "not an Antimatter Dimensions save: missing the savefile format prefix",
            ),
            SaveError::UnsupportedVersion => f.write_str(
                "unsupported savefile version: only the current \"AAB\" format is supported",
            ),
            SaveError::MissingEndMarker => f.write_str(
                "corrupt or truncated save: missing the trailing EndOfSavefile marker",
            ),
            SaveError::MissingSavesWrapper => f.write_str(
                "not a root save: missing the { current, saves } wrapper",
            ),
            SaveError::Base64(e) => write!(f, "base64 decode failed: {e}"),
            SaveError::Inflate(e) => write!(f, "zlib inflate failed: {e}"),
            SaveError::Utf8(e) => write!(f, "save payload was not valid UTF-8: {e}"),
            SaveError::Json(e) => write!(f, "save JSON could not be parsed: {e}"),
            SaveError::InvalidAutobuyerMode(m) => {
                write!(f, "invalid autobuyer mode {m}: expected 1 (single) or 10 (buy-10)")
            }
            SaveError::OptionOutOfRange {
                field,
                value,
                min,
                max,
            } => write!(
                f,
                "option `{field}` value {value} is outside the supported range {min}..={max}"
            ),
            SaveError::UnexpectedArrayLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "`{field}` had {found} entries, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::UnrecognizedFormat => None,
            SaveError::UnsupportedVersion => None,
            SaveError::MissingEndMarker => None,
            SaveError::MissingSavesWrapper => None,
            SaveError::Base64(e) => Some(e),
            SaveError::Inflate(e) => Some(e),
            SaveError::Utf8(e) => Some(e),
            SaveError::Json(e) => Some(e),
            SaveError::InvalidAutobuyerMode(_) => None,
            SaveError::OptionOutOfRange { .. } => None,
            SaveError::UnexpectedArrayLength { .. } => None,
        }
    }
}

impl From<base64::DecodeError> for SaveError {
    fn from(e: base64::DecodeError) -> Self {
        SaveError::Base64(e)
    }
}

impl From<std::string::FromUtf8Error> for SaveError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        SaveError::Utf8(e)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Json(e)
    }
}
