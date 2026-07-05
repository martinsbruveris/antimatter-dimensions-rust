//! The byte pipeline that converts between a raw JSON string and AD's encoded
//! save string. This is a faithful port of
//! `antimatter-dimensions/src/core/storage/serializer.js` (`GameSaveSerializer`),
//! restricted to the current `AAB` savefile format.
//!
//! Encode order (each step's inverse runs in reverse on decode):
//! 1. UTF-8 encode the JSON text to bytes.
//! 2. zlib **deflate** (zlib header + Adler-32, *not* raw deflate or gzip).
//! 3. base64-encode the bytes (the original goes bytes → Latin-1 string → `btoa`;
//!    base64-on-bytes collapses those two steps with an identical result).
//! 4. Character-safe cleanup, in this exact order: strip trailing `=`, then
//!    `0`→`0a`, `+`→`0b`, `/`→`0c`. Order matters so we neither re-encode the
//!    `0`s we just introduced nor mis-decode `0c`→`/`.
//! 5. Append the `EndOfSavefile` marker (present for version `>= AAB`).
//! 6. Prepend `AntimatterDimensionsSavefileFormat` + the 3-char version `AAB`.
//!
//! This layer is JSON-string ⇄ save-string only. The `Infinity`/`Set` JSON
//! conventions and the `Decimal`-as-string handling live one layer up, in the
//! DTO/serialization code, not here.

use std::io::{Read, Write};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::SaveError;

/// Magic prefix every AD savefile string starts with.
const PREFIX: &str = "AntimatterDimensionsSavefileFormat";
/// The savefile format version we emit and target. Three characters, ordered so
/// that string inequality is a version comparison (`AAA` < `AAB` < `AAC`).
const VERSION: &str = "AAB";
/// Trailing marker, appended for version `>= AAB`.
const SUFFIX: &str = "EndOfSavefile";

/// Encodes raw JSON text into an AD save string (current `AAB` format).
///
/// Infallible: every step is an in-memory transform over a valid `&str`.
pub fn encode_pipeline(json: &str) -> String {
    encode_text_with_markers(json, PREFIX, SUFFIX)
}

/// The shared encode pipeline with caller-supplied magic markers
/// (`GameSaveSerializer.encodeText` — the same steps serve savefiles and the
/// Automator's script/data exports, which differ only in their markers).
pub(crate) fn encode_text_with_markers(
    text: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    // 1–2: UTF-8 bytes → zlib deflate.
    let deflated = deflate(text.as_bytes());

    // 3: base64 (padded, matching `btoa`).
    let b64 = BASE64.encode(deflated);

    // 4: character-safe cleanup. Strip the always-trailing `=` first, then the
    // `0`/`+`/`/` substitutions — `0` before the others so the `0`s introduced
    // by `0b`/`0c` are not themselves re-encoded.
    let cleaned = b64
        .trim_end_matches('=')
        .replace('0', "0a")
        .replace('+', "0b")
        .replace('/', "0c");

    // 5–6: ending marker, then the prefix + version.
    format!("{prefix}{VERSION}{cleaned}{suffix}")
}

/// Decodes an AD save string back into the raw JSON text.
///
/// Rejects anything lacking the [`PREFIX`] magic (legacy pre-Reality saves were
/// `atob`-only and are out of scope) with [`SaveError::UnrecognizedFormat`], and
/// anything whose version marker is not the supported [`VERSION`] (`AAB`) with
/// [`SaveError::UnsupportedVersion`].
pub fn decode_pipeline(save: &str) -> Result<String, SaveError> {
    decode_text_with_markers(save, PREFIX, SUFFIX)
}

/// The shared decode pipeline with caller-supplied magic markers
/// (`GameSaveSerializer.decodeText`).
pub(crate) fn decode_text_with_markers(
    save: &str,
    prefix: &str,
    suffix: &str,
) -> Result<String, SaveError> {
    // 6: strip the magic prefix. Its absence means this isn't an AAB-era save.
    let body = save
        .strip_prefix(prefix)
        .ok_or(SaveError::UnrecognizedFormat)?;

    // The 3-char version marker follows the prefix. We only support the current
    // `AAB` format, so require it exactly — an older pre-`AAB` save or a
    // hypothetical newer one is rejected rather than mis-decoded against the
    // wrong cleanup/step rules.
    let payload = body
        .strip_prefix(VERSION)
        .ok_or(SaveError::UnsupportedVersion)?;

    // 5: strip the `EndOfSavefile` marker. It is always present for a valid
    // `AAB` save, so its absence means the string is truncated or corrupt.
    let payload = payload
        .strip_suffix(suffix)
        .ok_or(SaveError::MissingEndMarker)?;

    // 4 (reversed): undo the cleanup. `0b`/`0c` before `0a` so e.g. `0c` becomes
    // `/` rather than being seen as a `0` followed by `c`.
    let cleaned = payload
        .replace("0b", "+")
        .replace("0c", "/")
        .replace("0a", "0");

    // 3 (reversed): re-pad to a multiple of 4 and base64-decode.
    let bytes = BASE64.decode(pad_base64(cleaned).as_bytes())?;

    // 2–1 (reversed): zlib inflate → UTF-8 JSON text.
    let json_bytes = inflate(&bytes)?;
    Ok(String::from_utf8(json_bytes)?)
}

/// Re-pads a base64 string (whose trailing `=` were stripped) back to a multiple
/// of 4 so the standard decoder accepts it.
fn pad_base64(mut s: String) -> String {
    let rem = s.len() % 4;
    if rem != 0 {
        s.push_str(&"=".repeat(4 - rem));
    }
    s
}

/// zlib-deflates the given bytes (zlib container, as `pako.deflate` produces).
fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    // Writing to / finishing an in-memory `Vec` writer cannot fail.
    encoder.write_all(data).expect("in-memory deflate write");
    encoder.finish().expect("in-memory deflate finish")
}

/// zlib-inflates the given bytes.
fn inflate(data: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(SaveError::Inflate)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_initial_save.txt"
    ));
    const SAMPLE_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_sample_save.txt"
    ));

    #[test]
    fn round_trips_json() {
        let json =
            r#"{"version":25,"antimatter":"10","records":{"totalAntimatter":"10"}}"#;
        let encoded = encode_pipeline(json);
        let decoded = decode_pipeline(&encoded).unwrap();
        assert_eq!(decoded, json);
    }

    #[test]
    fn encoded_string_is_well_formed() {
        let encoded = encode_pipeline(r#"{"a":1,"b":"x/y+z=0"}"#);
        assert!(encoded.starts_with(PREFIX));
        assert!(encoded[PREFIX.len()..].starts_with(VERSION));
        assert!(encoded.ends_with(SUFFIX));
        // The cleanup must remove every raw `+`, `/`, and padding `=` from the
        // base64 body (the markers themselves contain none of these).
        let body = &encoded[PREFIX.len() + VERSION.len()..encoded.len() - SUFFIX.len()];
        assert!(!body.contains('+'));
        assert!(!body.contains('/'));
        assert!(!body.contains('='));
    }

    #[test]
    fn decodes_initial_fixture() {
        let json = decode_pipeline(INITIAL_SAVE.trim()).unwrap();
        let player: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(player["version"], 25);
        assert_eq!(player["antimatter"], "10");
        assert_eq!(player["galaxies"], 0);
        assert_eq!(player["dimensions"]["antimatter"][0]["bought"], 0);
    }

    #[test]
    fn decodes_sample_fixture() {
        let json = decode_pipeline(SAMPLE_SAVE.trim()).unwrap();
        let player: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(player["version"], 25);
        assert_eq!(player["antimatter"], "16613773273375400000");
        assert_eq!(player["totalTickBought"], 12);
        assert_eq!(player["galaxies"], 1);
        assert_eq!(player["dimensions"]["antimatter"][0]["bought"], 50);
    }

    #[test]
    fn real_fixtures_survive_reencode() {
        // We can't byte-compare against the original (flate2 and pako emit
        // different compressed bytes), but decoding our own re-encoding of a real
        // save must reproduce the exact same JSON.
        for fixture in [INITIAL_SAVE, SAMPLE_SAVE] {
            let json = decode_pipeline(fixture.trim()).unwrap();
            let reencoded = encode_pipeline(&json);
            assert_eq!(decode_pipeline(&reencoded).unwrap(), json);
        }
    }

    #[test]
    fn rejects_non_ad_string() {
        assert!(matches!(
            decode_pipeline("not a save"),
            Err(SaveError::UnrecognizedFormat)
        ));
        // A plausible legacy (pre-Reality) save started with `eYJ` and lacks our
        // prefix — it must be rejected, not mis-decoded.
        assert!(matches!(
            decode_pipeline("eYJ0aGlzIjoidGhhdCJ9"),
            Err(SaveError::UnrecognizedFormat)
        ));
    }

    #[test]
    fn rejects_unsupported_version() {
        // Tamper with a real save's version marker (the first `AAB`, right after
        // the prefix). Both an older and a newer marker must be rejected rather
        // than mis-decoded as `AAB`.
        for marker in ["AAA", "AAC"] {
            let tampered = INITIAL_SAVE.trim().replacen(VERSION, marker, 1);
            assert!(matches!(
                decode_pipeline(&tampered),
                Err(SaveError::UnsupportedVersion)
            ));
        }
    }

    #[test]
    fn rejects_non_ascii_version_without_panicking() {
        // Multi-byte chars right after the prefix must not panic the version
        // check; they're simply not the `AAB` marker.
        let garbage = format!("{PREFIX}éé{SUFFIX}");
        assert!(matches!(
            decode_pipeline(&garbage),
            Err(SaveError::UnsupportedVersion)
        ));
    }

    #[test]
    fn rejects_missing_end_marker() {
        // A correctly-prefixed `AAB` save with its trailing `EndOfSavefile`
        // stripped is truncated/corrupt and must be rejected.
        let truncated = INITIAL_SAVE.trim().strip_suffix(SUFFIX).unwrap();
        assert!(matches!(
            decode_pipeline(truncated),
            Err(SaveError::MissingEndMarker)
        ));
    }
}
