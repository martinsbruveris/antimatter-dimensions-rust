//! End-to-end plumbing tests for the Rust replay/comparison side, exercised
//! against the real `saves/01_pre_big_crunch.txt` without needing the JS oracle.
//!
//! These do not assert JS-vs-Rust *fidelity* (that needs generated fixtures);
//! they validate that the comparison machinery — allowlist path resolution, the
//! diff walker, fixture loading, and rendering — works end-to-end on a real save
//! tree, and that the Rust save round-trip preserves every allowlisted field.

use std::fs;
use std::path::PathBuf;

use ad_core::save::encode_save;
use ad_fidelity::allowlist::allowlist;
use ad_fidelity::compare::{compare_trees, Tolerance};
use ad_fidelity::fixture::{decode_expected, load_dir, replay_rust};
use ad_fidelity::report::{table, verbose};
use ad_fidelity::run::{run, RunConfig};

fn real_save() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("saves")
        .join("01_pre_big_crunch.txt");
    fs::read_to_string(path)
        .expect("read saves/01_pre_big_crunch.txt")
        .trim()
        .to_string()
}

/// Rust's decode→re-encode of a real save must agree with the save's own tree
/// over the entire allowlist. This is the round-trip identity guard (design §6)
/// run over real data: a fail means the write path dropped or mangled a modelled
/// field, not a tick bug.
#[test]
fn roundtrip_identity_holds_over_allowlist() {
    let save = real_save();
    let expected = decode_expected(&save).expect("decode JS-side tree");
    let actual = replay_rust(&save, 0, 50.0).expect("rust decode->encode");

    let rules = allowlist();
    let diffs = compare_trees(&expected, &actual, &rules, &Tolerance::default(), 0);
    assert!(
        diffs.is_empty(),
        "round-trip diverged on {} field(s):\n{}",
        diffs.len(),
        diffs
            .iter()
            .map(|d| format!(
                "  {} — JS={} Rust={} ({})",
                d.path, d.expected, d.actual, d.detail
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A deliberately mutated expected value must be reported — proof that the field
/// is actually compared, not silently skipped (guards against allowlist typos
/// resolving to missing-on-both-sides).
#[test]
fn a_mutated_field_is_flagged() {
    let save = real_save();
    let mut expected = decode_expected(&save).expect("decode");
    let actual = replay_rust(&save, 0, 50.0).expect("replay");

    // Multiply antimatter by 10 in the JS-side tree: guaranteed a >1-OOM diff.
    let am: String = expected["antimatter"].as_str().unwrap().to_string();
    let bumped = format!("{}0", am.replace('.', "")); // crude ×~10, always different
    expected["antimatter"] = serde_json::Value::String(bumped);

    let rules = allowlist();
    let diffs = compare_trees(&expected, &actual, &rules, &Tolerance::default(), 1);
    assert!(
        diffs.iter().any(|d| d.path == "antimatter"),
        "expected an `antimatter` diff, got: {diffs:?}"
    );
}

/// The full pipeline over an on-disk fixture: load → replay → diff → render.
/// The "expected" saves are Rust's own re-encodes, so every cell passes by
/// construction; this checks fixture parsing, horizon columns, the round-trip
/// column, and that the table/verbose renderers run.
#[test]
fn full_run_over_a_fabricated_fixture() {
    use ad_core::save::decode_save;

    let save = real_save();

    // Build expected saves at a few horizons from Rust's own replay.
    let horizons = [1u32, 10];
    let mut expected = serde_json::Map::new();
    for &h in &horizons {
        let mut state = decode_save(&save).expect("decode");
        state.ticks(50.0, h);
        expected.insert(
            h.to_string(),
            serde_json::Value::String(encode_save(&state, 0)),
        );
    }
    let fixture = serde_json::json!({
        "meta": { "sourceSave": "01_pre_big_crunch.txt", "tickMs": 50, "horizons": horizons },
        "input": save,
        "expected": expected,
    });

    let dir =
        std::env::temp_dir().join(format!("ad-fidelity-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("01_pre_big_crunch.json"), fixture.to_string()).unwrap();

    let fixtures = load_dir(&dir).expect("load fixtures");
    assert_eq!(fixtures.len(), 1);
    assert_eq!(fixtures[0].tick_ms, 50.0);

    // Columns: the round-trip baseline plus the two fixture horizons.
    let cols = vec![0u32, 1, 10];
    let result = run(&fixtures, &cols, &RunConfig::default(), &allowlist());

    let (passed, total) = result.tally();
    assert_eq!(passed, total, "all cells should pass by construction");
    assert_eq!(total, 3, "3 columns × 1 fixture");
    assert!(!result.any_failure());

    // Renderers must produce output and not panic.
    let t = table(&result);
    assert!(t.contains("01_pre_big_crunch"));
    assert!(t.contains("cells passed"));
    let v = verbose(&result);
    assert!(v.is_empty(), "no failures → empty verbose output");

    let _ = fs::remove_dir_all(&dir);
}
