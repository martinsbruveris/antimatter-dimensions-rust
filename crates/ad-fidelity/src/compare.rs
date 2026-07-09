//! The tolerant diff walker at the heart of the save-replay comparison.
//!
//! Both engines round-trip the same save schema (design §6), so the comparator
//! works on two `player`-tree [`Value`]s — the JS oracle's expected save on one
//! side, the Rust-replayed save on the other — and walks a per-field
//! [`FieldRule`] table (the allowlist, [`crate::allowlist`]). Each rule names a
//! JS/save-key path and a [`Compare`] mode; the walker resolves that path in both
//! trees and compares the leaves under the mode, collecting a [`FieldDiff`] per
//! divergence.
//!
//! The allowlist is include-only: fields absent from it (options, UI, unported
//! systems, `Date.now`/real-time bookkeeping — design §5) are simply never
//! visited, so the template defaults the Rust write path fills in for unmodelled
//! keys don't produce noise.

use break_infinity::Decimal;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use crate::tolerance::{approx_eq_f64, approx_eq_log};

/// How a single field (or a whole container, for [`Compare::IdSet`] /
/// [`Compare::Glyphs`]) is compared. See design §8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compare {
    /// Structural JSON equality — ints, bools, bitmasks, id-keyed maps.
    Exact,
    /// A Decimal (serialized as a string or number): log-space relative
    /// tolerance. Also the `expectation` mode of design §8 — replicanti amounts
    /// are compared this way, the JS sampler having been mocked to its mean.
    Decimal,
    /// A plain `f64`/number: relative tolerance. Used where one engine may write
    /// an integral value as a float (`interval`, `time`) or where the value
    /// accumulates FP error per tick (`partInfinityPoint`, `chall2Pow`).
    Number,
    /// An order-insensitive array of ids (time studies, upgrades, perks).
    /// The path points at the array itself.
    IdSet,
    /// A glyph object-array (`reality.glyphs.{active,inventory}`): matched by
    /// slot `idx`, with `type`/`effects` exact and `level`/`strength`/`rawLevel`
    /// numerically tolerant (design §5 "Reality"). The path points at the array.
    Glyphs,
    /// A bitmask array whose trailing zero rows are insignificant
    /// (`achievementBits`): Rust always writes the full row count, while JS grows
    /// the array on demand, so the JS side can be a row shorter — an omitted
    /// trailing zero row. The shorter side is zero-padded before an element-wise
    /// exact compare. The path points at the array.
    PaddedBits,
    /// The Automator run mode (`reality.automator.state.mode`): 1 pause / 2 run /
    /// 3 step. JS omits the key entirely on a save whose Automator never ran (an
    /// undefined enum member that reads as *paused*), while Rust always writes a
    /// value; so an absent side is treated as `1` before an exact compare.
    AutomatorMode,
    /// The Automator execution stack (`reality.automator.state.stack`): an ordered
    /// array of `{lineNumber, commandState}` frames. Compared depth- and
    /// order-sensitively; `lineNumber` and the `commandState` shape are exact, but
    /// a WAIT frame's `commandState.timeMs` is a real-time accumulator and is
    /// compared with numeric tolerance.
    AutomatorStack,
}

/// One row of the comparison allowlist: a JS/save-key path and its mode.
///
/// Paths are dot-separated; a `[]` suffix on a segment iterates that array
/// element-wise (e.g. `dimensions.antimatter[].amount`, `blackHole[].active`).
/// For [`Compare::IdSet`] / [`Compare::Glyphs`] the path names the container and
/// carries no `[]`.
#[derive(Clone, Copy, Debug)]
pub struct FieldRule {
    pub path: &'static str,
    pub mode: Compare,
}

impl FieldRule {
    pub const fn new(path: &'static str, mode: Compare) -> Self {
        Self { path, mode }
    }
}

/// Comparison tolerance, expressible as a function of the replay horizon (design
/// §10: "tolerance can be a function of horizon — constant or linear in tick
/// count"). The shape is kept general; the constants are provisional until the
/// oracle produces enough data to fix them empirically.
#[derive(Clone, Copy, Debug)]
pub struct Tolerance {
    /// log-space epsilon at horizon 0.
    pub log_base: f64,
    /// added to the log epsilon per tick of horizon.
    pub log_per_tick: f64,
    /// relative epsilon for [`Compare::Number`] at horizon 0.
    pub num_base: f64,
    /// added to the number epsilon per tick of horizon.
    pub num_per_tick: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        // Default epsilon: 1e-4 for both the log-space (Decimal) and the relative
        // number-field comparison, which absorbs the accumulated-rounding /
        // cancellation drift between Rust f64 and V8 (see the 2026-07-09 worklog).
        // Structural count/flag mismatches diverge by far more than 1e-4, and
        // `Compare::Exact` bitmask/bool fields are unaffected by this epsilon.
        Self {
            log_base: 1e-4,
            log_per_tick: 0.0,
            num_base: 1e-4,
            num_per_tick: 0.0,
        }
    }
}

impl Tolerance {
    /// The log-space epsilon at `horizon` ticks.
    pub fn log_eps(&self, horizon: u32) -> f64 {
        self.log_base + self.log_per_tick * horizon as f64
    }

    /// The relative number epsilon at `horizon` ticks.
    pub fn num_eps(&self, horizon: u32) -> f64 {
        self.num_base + self.num_per_tick * horizon as f64
    }

    /// A tolerance with the log epsilon set to `eps` (per-tick growth zeroed).
    pub fn with_log_eps(eps: f64) -> Self {
        Self {
            log_base: eps,
            ..Self::default()
        }
    }
}

/// A single field that diverged between the two engines.
#[derive(Clone, Debug)]
pub struct FieldDiff {
    /// The resolved JS/save-key path, with concrete array indices, e.g.
    /// `dimensions.antimatter[3].amount`.
    pub path: String,
    /// The JS (oracle) value, display-formatted.
    pub expected: String,
    /// The Rust (replay) value, display-formatted.
    pub actual: String,
    /// How they differ (Δlog10, relative error, set difference, …).
    pub detail: String,
}

/// Compare the `expected` (JS oracle) and `actual` (Rust replay) `player` trees
/// over `rules` at `horizon` ticks, returning one [`FieldDiff`] per divergence.
/// An empty result means the two engines agree over the allowlist.
pub fn compare_trees(
    expected: &Value,
    actual: &Value,
    rules: &[FieldRule],
    tol: &Tolerance,
    horizon: u32,
) -> Vec<FieldDiff> {
    let mut out = Vec::new();
    let mut path = String::new();
    for rule in rules {
        let segs = parse_path(rule.path);
        walk(
            Some(expected),
            Some(actual),
            &segs,
            rule.mode,
            tol,
            horizon,
            &mut path,
            &mut out,
        );
        debug_assert!(path.is_empty());
    }
    out
}

/// A parsed path segment.
enum Seg {
    /// Descend into an object key.
    Key(String),
    /// Iterate every element of the current array.
    Wild,
}

fn parse_path(path: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    for tok in path.split('.') {
        if let Some(base) = tok.strip_suffix("[]") {
            segs.push(Seg::Key(base.to_string()));
            segs.push(Seg::Wild);
        } else {
            segs.push(Seg::Key(tok.to_string()));
        }
    }
    segs
}

#[allow(clippy::too_many_arguments)]
fn walk(
    exp: Option<&Value>,
    act: Option<&Value>,
    segs: &[Seg],
    mode: Compare,
    tol: &Tolerance,
    horizon: u32,
    path: &mut String,
    out: &mut Vec<FieldDiff>,
) {
    match segs.first() {
        None => compare_leaf(exp, act, mode, tol, horizon, path, out),
        Some(Seg::Key(k)) => {
            let e = exp.and_then(|v| v.get(k));
            let a = act.and_then(|v| v.get(k));
            let saved = path.len();
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(k);
            walk(e, a, &segs[1..], mode, tol, horizon, path, out);
            path.truncate(saved);
        }
        Some(Seg::Wild) => {
            let el = exp.and_then(Value::as_array);
            let al = act.and_then(Value::as_array);
            let len = el.map_or(0, Vec::len).max(al.map_or(0, Vec::len));
            for i in 0..len {
                let e = el.and_then(|v| v.get(i));
                let a = al.and_then(|v| v.get(i));
                let saved = path.len();
                path.push_str(&format!("[{i}]"));
                walk(e, a, &segs[1..], mode, tol, horizon, path, out);
                path.truncate(saved);
            }
        }
    }
}

fn compare_leaf(
    exp: Option<&Value>,
    act: Option<&Value>,
    mode: Compare,
    tol: &Tolerance,
    horizon: u32,
    path: &str,
    out: &mut Vec<FieldDiff>,
) {
    if mode == Compare::Exact {
        // Exact tolerates absence symmetrically (both-missing is not a
        // divergence) and compares numbers by value, not representation — the
        // two engines legitimately serialize an integral value as `0` vs `0.0`,
        // which is not a fidelity difference.
        let equal = match (exp, act) {
            (None, None) => true,
            (Some(e), Some(a)) => values_equal(e, a),
            _ => false,
        };
        if !equal {
            out.push(diff(path, exp, act, "values differ".to_string()));
        }
        return;
    }

    if mode == Compare::AutomatorMode {
        // JS omits `mode` on a never-run Automator (an undefined enum member that
        // reads as paused); Rust always writes 1/2/3. Treat an absent side as `1`
        // before comparing by value, so a fresh save (JS absent vs Rust 1) agrees.
        let val = |v: Option<&Value>| v.and_then(Value::as_i64).unwrap_or(1);
        if val(exp) != val(act) {
            out.push(diff(path, exp, act, "values differ".to_string()));
        }
        return;
    }

    let (e, a) = match (exp, act) {
        (None, None) => return,
        (Some(e), Some(a)) => (e, a),
        _ => {
            out.push(diff(path, exp, act, "present on one side only".to_string()));
            return;
        }
    };

    match mode {
        Compare::Exact | Compare::AutomatorMode => unreachable!("handled above"),
        Compare::Decimal => {
            if e == a {
                return;
            }
            match (to_decimal(e), to_decimal(a)) {
                (Some(de), Some(da)) => {
                    let eps = tol.log_eps(horizon);
                    if !approx_eq_log(da, de, eps) {
                        out.push(diff(path, exp, act, decimal_detail(de, da, eps)));
                    }
                }
                _ => out.push(diff(path, exp, act, "unparseable Decimal".to_string())),
            }
        }
        Compare::Number => match (e.as_f64(), a.as_f64()) {
            (Some(ef), Some(af)) => {
                let eps = tol.num_eps(horizon);
                if !approx_eq_f64(af, ef, eps) {
                    let rel = if ef != 0.0 {
                        ((af - ef) / ef).abs()
                    } else {
                        af.abs()
                    };
                    out.push(diff(
                        path,
                        exp,
                        act,
                        format!("rel={rel:.3e} > eps={eps:.1e}"),
                    ));
                }
            }
            _ => out.push(diff(path, exp, act, "not a number".to_string())),
        },
        Compare::IdSet => match (e.as_array(), a.as_array()) {
            (Some(ea), Some(aa)) => {
                if let Some(detail) = idset_diff(ea, aa) {
                    out.push(diff(path, exp, act, detail));
                }
            }
            _ => out.push(diff(path, exp, act, "not an array".to_string())),
        },
        Compare::Glyphs => compare_glyphs(e, a, tol, horizon, path, out),
        Compare::PaddedBits => match (e.as_array(), a.as_array()) {
            (Some(ea), Some(aa)) => {
                if let Some(detail) = padded_bits_diff(ea, aa) {
                    out.push(diff(path, exp, act, detail));
                }
            }
            _ => out.push(diff(path, exp, act, "not an array".to_string())),
        },
        Compare::AutomatorStack => {
            compare_automator_stack(e, a, tol, horizon, path, out)
        }
    }
}

/// Compare two Automator execution stacks (`reality.automator.state.stack`):
/// ordered `{lineNumber, commandState}` frames. Depth- and order-sensitive;
/// `lineNumber` and the `commandState` shape are exact, but a WAIT frame's
/// `commandState.timeMs` (a real-time accumulator) is compared with numeric
/// tolerance.
fn compare_automator_stack(
    exp: &Value,
    act: &Value,
    tol: &Tolerance,
    horizon: u32,
    path: &str,
    out: &mut Vec<FieldDiff>,
) {
    let (ea, aa) = match (exp.as_array(), act.as_array()) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            out.push(diff(
                path,
                Some(exp),
                Some(act),
                "not a stack array".to_string(),
            ));
            return;
        }
    };
    if ea.len() != aa.len() {
        out.push(diff(
            path,
            Some(exp),
            Some(act),
            format!("stack depth differs: JS={}, Rust={}", ea.len(), aa.len()),
        ));
        return;
    }
    for (i, (ef, af)) in ea.iter().zip(aa).enumerate() {
        let fp = format!("{path}[{i}]");
        compare_leaf(
            ef.get("lineNumber"),
            af.get("lineNumber"),
            Compare::Exact,
            tol,
            horizon,
            &format!("{fp}.lineNumber"),
            out,
        );
        compare_command_state(
            ef.get("commandState"),
            af.get("commandState"),
            tol,
            horizon,
            &format!("{fp}.commandState"),
            out,
        );
    }
}

/// Compare a single Automator frame's `commandState`: `null`/absent on both sides
/// agree; otherwise the objects are compared key-by-key with `timeMs` (a WAIT
/// accumulator) numerically tolerant and every other key exact.
fn compare_command_state(
    exp: Option<&Value>,
    act: Option<&Value>,
    tol: &Tolerance,
    horizon: u32,
    path: &str,
    out: &mut Vec<FieldDiff>,
) {
    let is_empty = |v: Option<&Value>| matches!(v, None | Some(Value::Null));
    if is_empty(exp) && is_empty(act) {
        return;
    }
    match (exp, act) {
        (Some(Value::Object(e)), Some(Value::Object(a))) => {
            let keys: BTreeSet<&String> = e.keys().chain(a.keys()).collect();
            for k in keys {
                let mode = if k == "timeMs" {
                    Compare::Number
                } else {
                    Compare::Exact
                };
                compare_leaf(
                    e.get(k),
                    a.get(k),
                    mode,
                    tol,
                    horizon,
                    &format!("{path}.{k}"),
                    out,
                );
            }
        }
        _ => out.push(diff(
            path,
            exp,
            act,
            "commandState shape differs".to_string(),
        )),
    }
}

/// Compare two bitmask arrays (`achievementBits`) element-wise, zero-padding the
/// shorter side. Rust always writes the full row count; JS grows the array on
/// demand, so its array can be a row shorter — a trailing zero row it simply
/// omits, which is not a divergence. Returns the first differing row, or `None`
/// if the two agree once the shorter side is zero-padded.
fn padded_bits_diff(exp: &[Value], act: &[Value]) -> Option<String> {
    let zero = Value::from(0);
    let len = exp.len().max(act.len());
    for i in 0..len {
        let e = exp.get(i).unwrap_or(&zero);
        let a = act.get(i).unwrap_or(&zero);
        if !numbers_equal(e, a) {
            return Some(format!("row {i}: JS={e}, Rust={a}"));
        }
    }
    None
}

fn compare_glyphs(
    exp: &Value,
    act: &Value,
    tol: &Tolerance,
    horizon: u32,
    path: &str,
    out: &mut Vec<FieldDiff>,
) {
    let (ea, aa) = match (exp.as_array(), act.as_array()) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            out.push(diff(
                path,
                Some(exp),
                Some(act),
                "not a glyph array".to_string(),
            ));
            return;
        }
    };
    // Index each side by the glyph's slot `idx` (glyph ordering may differ, and
    // the `id` counter is fragile bookkeeping — design §5).
    let by_idx = |arr: &[Value]| -> BTreeMap<i64, Value> {
        arr.iter()
            .filter_map(|g| g.get("idx").and_then(Value::as_i64).map(|i| (i, g.clone())))
            .collect()
    };
    let em = by_idx(ea);
    let am = by_idx(aa);
    let idxs: BTreeSet<i64> = em.keys().chain(am.keys()).copied().collect();
    for idx in idxs {
        match (em.get(&idx), am.get(&idx)) {
            (Some(g), Some(h)) => {
                let field = |name: &str, mode: Compare, out: &mut Vec<FieldDiff>| {
                    compare_leaf(
                        g.get(name),
                        h.get(name),
                        mode,
                        tol,
                        horizon,
                        &format!("{path}[idx={idx}].{name}"),
                        out,
                    );
                };
                field("type", Compare::Exact, out);
                field("effects", Compare::Exact, out);
                field("level", Compare::Number, out);
                field("strength", Compare::Number, out);
                field("rawLevel", Compare::Number, out);
            }
            (js, rust) => out.push(diff(
                &format!("{path}[idx={idx}]"),
                js,
                rust,
                "glyph present on one side only".to_string(),
            )),
        }
    }
}

/// The symmetric multiset difference of two id-arrays, or `None` if they match
/// as multisets (order-insensitive).
fn idset_diff(exp: &[Value], act: &[Value]) -> Option<String> {
    let key = |v: &Value| serde_json::to_string(v).unwrap_or_default();
    let mut em: BTreeMap<String, i64> = BTreeMap::new();
    let mut am: BTreeMap<String, i64> = BTreeMap::new();
    for v in exp {
        *em.entry(key(v)).or_default() += 1;
    }
    for v in act {
        *am.entry(key(v)).or_default() += 1;
    }
    if em == am {
        return None;
    }
    let only = |a: &BTreeMap<String, i64>, b: &BTreeMap<String, i64>| -> Vec<String> {
        a.iter()
            .filter(|(k, c)| b.get(*k).copied().unwrap_or(0) < **c)
            .map(|(k, _)| k.clone())
            .collect()
    };
    Some(format!(
        "only in JS: {:?}; only in Rust: {:?}",
        only(&em, &am),
        only(&am, &em)
    ))
}

/// Structural equality that compares JSON numbers by value (so `0` == `0.0`)
/// and recurses through arrays and objects. Integer numbers are compared as
/// integers to preserve full precision above 2^53 (bitmasks, seeds).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => numbers_equal(a, b),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| values_equal(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| values_equal(v, w)))
        }
        _ => a == b,
    }
}

fn numbers_equal(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (a.as_i64(), b.as_i64()) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (a.as_u64(), b.as_u64()) {
        return x == y;
    }
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn to_decimal(v: &Value) -> Option<Decimal> {
    match v {
        // The AD serializer stores Decimals as strings ("1.5e10", "Infinity",
        // "NaN"). `Decimal::from_str` handles the e-notation and NaN, and falls
        // back to an `f64` parse (which accepts "Infinity").
        Value::String(s) => Decimal::from_str(s).ok(),
        Value::Number(n) => n.as_f64().map(Decimal::from_float),
        _ => None,
    }
}

fn decimal_detail(exp: Decimal, act: Decimal, eps: f64) -> String {
    if exp == Decimal::ZERO || act == Decimal::ZERO {
        return format!("one side zero (eps={eps:.1e})");
    }
    let dlog = (exp.abs().log10() - act.abs().log10()).abs();
    format!("Δlog10={dlog:.3e} > eps={eps:.1e}")
}

fn diff(
    path: &str,
    exp: Option<&Value>,
    act: Option<&Value>,
    detail: String,
) -> FieldDiff {
    FieldDiff {
        path: path.to_string(),
        expected: show(exp),
        actual: show(act),
        detail,
    }
}

fn show(v: Option<&Value>) -> String {
    match v {
        None => "<missing>".to_string(),
        Some(Value::String(s)) => truncate(s, 44),
        Some(other) => truncate(&other.to_string(), 44),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max - 1).collect();
        t.push('…');
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules(path: &'static str, mode: Compare) -> Vec<FieldRule> {
        vec![FieldRule::new(path, mode)]
    }

    #[test]
    fn exact_match_and_mismatch() {
        let tol = Tolerance::default();
        let e = json!({ "galaxies": 3 });
        let a = json!({ "galaxies": 3 });
        assert!(
            compare_trees(&e, &a, &rules("galaxies", Compare::Exact), &tol, 1)
                .is_empty()
        );

        let a2 = json!({ "galaxies": 4 });
        let d = compare_trees(&e, &a2, &rules("galaxies", Compare::Exact), &tol, 1);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path, "galaxies");
    }

    #[test]
    fn decimal_within_and_outside_tolerance() {
        let tol = Tolerance::with_log_eps(1e-6);
        // Different string spelling of the same magnitude — log compare passes.
        let e = json!({ "antimatter": "1e+100" });
        let a = json!({ "antimatter": "1e100" });
        assert!(
            compare_trees(&e, &a, &rules("antimatter", Compare::Decimal), &tol, 1)
                .is_empty()
        );

        // An order of magnitude off — fails.
        let a2 = json!({ "antimatter": "1e101" });
        let d = compare_trees(&e, &a2, &rules("antimatter", Compare::Decimal), &tol, 1);
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.starts_with("Δlog10="), "{}", d[0].detail);
    }

    #[test]
    fn number_mode_ignores_int_vs_float_spelling() {
        let tol = Tolerance::default();
        // JS writes an integer, Rust an integral float — Number mode agrees.
        let e = json!({ "auto": { "interval": 500 } });
        let a = json!({ "auto": { "interval": 500.0 } });
        assert!(compare_trees(
            &e,
            &a,
            &rules("auto.interval", Compare::Number),
            &tol,
            1
        )
        .is_empty());
    }

    #[test]
    fn exact_compares_numbers_by_value_not_representation() {
        let tol = Tolerance::default();
        // Rust writes an integral float, JS an int — same value, not a diff.
        let e = json!({ "reality": { "seed": 1 } });
        let a = json!({ "reality": { "seed": 1.0 } });
        assert!(
            compare_trees(&e, &a, &rules("reality.seed", Compare::Exact), &tol, 1)
                .is_empty()
        );

        // Nested objects (e.g. reality.glyphs.sac) too.
        let e = json!({ "sac": { "power": 0, "time": 2 } });
        let a = json!({ "sac": { "power": 0.0, "time": 2.0 } });
        assert!(
            compare_trees(&e, &a, &rules("sac", Compare::Exact), &tol, 1).is_empty()
        );

        // But a genuine value difference is still caught.
        let a = json!({ "sac": { "power": 0.0, "time": 3.0 } });
        assert_eq!(
            compare_trees(&e, &a, &rules("sac", Compare::Exact), &tol, 1).len(),
            1
        );
    }

    #[test]
    fn idset_is_order_insensitive() {
        let tol = Tolerance::default();
        let e = json!({ "studies": [11, 21, 22] });
        let a = json!({ "studies": [22, 11, 21] });
        assert!(
            compare_trees(&e, &a, &rules("studies", Compare::IdSet), &tol, 1).is_empty()
        );

        let a2 = json!({ "studies": [11, 21] });
        let d = compare_trees(&e, &a2, &rules("studies", Compare::IdSet), &tol, 1);
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.contains("only in JS"));
    }

    #[test]
    fn wildcard_iterates_array_elements() {
        let tol = Tolerance::default();
        let e = json!({ "dims": [ { "bought": 1 }, { "bought": 2 } ] });
        let a = json!({ "dims": [ { "bought": 1 }, { "bought": 9 } ] });
        let d = compare_trees(&e, &a, &rules("dims[].bought", Compare::Exact), &tol, 1);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path, "dims[1].bought");
    }

    #[test]
    fn padded_bits_zero_pads_shorter_js_array() {
        let tol = Tolerance::default();
        // JS grew the array on demand (17 rows); Rust always writes 18, the last
        // an implicit zero — not a divergence.
        let e = json!({ "achievementBits": [1, 2, 3] });
        let a = json!({ "achievementBits": [1, 2, 3, 0] });
        assert!(compare_trees(
            &e,
            &a,
            &rules("achievementBits", Compare::PaddedBits),
            &tol,
            1
        )
        .is_empty());

        // A trailing non-zero row that JS omits is still a genuine divergence.
        let a2 = json!({ "achievementBits": [1, 2, 3, 5] });
        let d = compare_trees(
            &e,
            &a2,
            &rules("achievementBits", Compare::PaddedBits),
            &tol,
            1,
        );
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.contains("row 3"), "{}", d[0].detail);

        // An interior mismatch is caught regardless of padding.
        let a3 = json!({ "achievementBits": [1, 9, 3, 0] });
        let d = compare_trees(
            &e,
            &a3,
            &rules("achievementBits", Compare::PaddedBits),
            &tol,
            1,
        );
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.contains("row 1"), "{}", d[0].detail);
    }

    #[test]
    fn present_on_one_side_only() {
        let tol = Tolerance::default();
        let e = json!({ "replicanti": { "amount": "10" } });
        let a = json!({ "replicanti": {} });
        let d = compare_trees(
            &e,
            &a,
            &rules("replicanti.amount", Compare::Decimal),
            &tol,
            1,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "<missing>");
    }
}
