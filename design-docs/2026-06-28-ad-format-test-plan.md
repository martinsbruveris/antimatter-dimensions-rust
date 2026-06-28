# Test Plan: `ad-format`

A comprehensive test plan for the `ad-format` crate (`format()` plus the Scientific,
Engineering, Standard, and Letters notations). The goal is to cover not just the happy
paths the existing tests already hit, but the **edge cases, boundaries, and
less-exercised code paths**, and to document the handful of paths that are effectively
unreachable through the public API or that knowingly diverge from the JS reference.

## How to read this plan

Each section maps to a source module and lists:

- **Covered** — what the existing tests (`tests/notations.rs`, `src/router/tests.rs`,
  `src/exponent/tests.rs`) already exercise.
- **Gaps / edge cases** — concrete cases to add, with the input and the path they hit.
- **Notes** — dead paths, divergences, and infrastructure recommendations.

Expected strings for *fidelity* cases must be regenerated from the real
`@antimatter-dimensions/notations` package (see the header of `tests/notations.rs`).
Cases marked **[pin]** assert *current Rust behaviour* that is known to differ from JS;
they exist to catch regressions, not to certify fidelity, and should carry a comment
pointing at the relevant `TODO(fidelity)`.

---

## 1. Router (`src/router.rs`) — `format`

The router dispatches on the base-10 exponent into four regions and applies the sign
uniformly. The branch order is: infinite threshold → very-small (`exp < -300`) →
under-1000 (`exp < 3`) → big number.

### Covered
- Under-1000 fixed point: `0`, `12.5`, `999`, negative `-42` (Scientific default).
- Very-small collapse to `"0"` at `1e-310`.
- Infinite threshold capping `±1e309` against `1e308`.

### Gaps / edge cases

| # | Input | `opts` | Path / expectation |
|---|-------|--------|--------------------|
| 1.1 | `value == threshold` exactly | `inf_threshold = Some(1e308)`, value `1e308` | `>=` is inclusive → `"Infinite"` (boundary). |
| 1.2 | `value` just below threshold | `Some(1e308)`, value `9.99e307` | Falls through to big-number, **not** Infinite. |
| 1.3 | `-value == threshold` exactly | `Some(1e308)`, value `-1e308` | `"-Infinite"` (negative boundary). |
| 1.4 | `inf_threshold = None`, very large value | `1e309` | Never Infinite; formats normally. |
| 1.5 | Exponent boundary `-300` vs `-301` | `1e-300` and `1e-301` | `-300` is **not** `< -300` → under-1000 path; `-301` → very-small. Both render `"0"` (verify which branch via a notation that overrides `format_very_small` once one exists). |
| 1.6 | Negative very-small | `-1e-310` | Sign is applied to the collapsed magnitude → **[pin]** likely `"-0"`. Verify and pin; flag if undesirable. |
| 1.7 | Exponent boundary `2` vs `3` | `999` (`exp 2`) and `1000` (`exp 3`) | `999` under-1000, `1000` big-number. Pairs with 1.x boundary discipline. |
| 1.8 | Under-1000 / very-small for **every** notation | `42`, `-42` for Standard and Letters | Existing test only loops Scientific + Engineering; Standard and Letters use the same default fallbacks but are unverified end-to-end. |
| 1.9 | Negative big numbers for Standard and Letters | `-1e6` Standard → `"-1.00 M"`, `-1e10` Letters → `"-15.00c"` | Sign path with notations other than Scientific/Engineering. |
| 1.10 | Infinite threshold with non-Scientific notation | `Standard`, `Letters` | Threshold logic is notation-independent; confirm it short-circuits before `format_decimal`. |

### Notes
- The sign is taken from `value.sign()`; confirm `Decimal::ZERO` yields `""` (no `"-0"`
  for plain zero) — case in 1.x, distinct from the very-small `-0` quirk in 1.6.

---

## 2. Mantissa engine (`src/mantissa.rs`) — `format_mantissa_with_exponent`

The shared split/round/join engine behind Scientific, Engineering, and Standard.

### Covered (indirectly, via notation tests)
- Mantissa roll-over (`9.999e9 → 1.00e10`, and at low exp `9.9999e3 → 1.00e4`).
- Engineering 3-step rebasing (`1e10 → 15.00e9`, `1e100 → 10.00e99`).

### Gaps / edge cases

| # | Input | Notation | Path / expectation |
|---|-------|----------|--------------------|
| 2.1 | Roll-over for Engineering at a step boundary | `9.9999e11` | mantissa `999.99…` rounds to `1000.00` → rolls into `1.00e12`. |
| 2.2 | Roll-over for Standard across a letter | `9.999e8` (no roll, `"999.90 M"`) vs a value that rounds `999.xx K → 1.00 M` | Exercises the roll-over `+= steps_f` combined with `abbreviate_standard`. |
| 2.3 | Precision-correction branch (`!(1.0..real_base).contains(mantissa)`) | hard to hit deterministically | See Notes — cover with a **property test**, not a literal. |

### Notes — effectively unreachable paths
- **`force_positive_exponent` clamp**: the router only calls `format_decimal` for
  exponent ≥ 3, so Standard's `log_1000(n) ≥ 1` and the exponent is never negative. The
  `exponent.max(0.0)` clamp is dead through `format`. Document, or add a `#[cfg(test)]`
  unit test that calls the engine directly to cover it.
- **`exponent == 0` short-circuit** (returns the bare mantissa): also unreachable via
  `format` for the same reason (smallest big-number input is `1000`, giving a nonzero
  displayed exponent in every notation). Faithful to the JS port but currently dead.
- **Precision correction**: rare floating `log` mis-rounding. Best covered by a
  **proptest** asserting an invariant rather than a hand-picked value (Section 7).

---

## 3. Exponent helpers (`src/exponent.rs` + `notations/mod.rs::format_exponent`)

`format_exponent` chooses between *plain*, *comma-grouped*, and *recursive-in-notation*
rendering of a large exponent, based on `ExponentDisplay { show, min, max }`.

### Covered
- `format_with_commas`: thousands grouping, `999`, `1000`, negative `-12345`.
- Comma branch: `1e100000 → e100,000`, `1e1234567 → e1,234,567`.
- Recursive branch: `1e1e9 → e1.000e9`, deep `1e1.23e15`, roll-over crossing the
  recursion boundary, Engineering's step-3 boundary shift.

### Gaps / edge cases

| # | Input | `opts` | Path / expectation |
|---|-------|--------|--------------------|
| 3.1 | `exponent == min` exactly | `1e100000` (min `1e5`) | `< min` is false → commas, not plain (boundary; already implicitly hit — make explicit for Scientific). |
| 3.2 | `exponent == min - 1` | `1e99999` Scientific | Plain, no commas (`"1.00e99999"`). Existing only covers this for Engineering. |
| 3.3 | `exponent == max` exactly | `1e1000000000` (max `1e9`) | `< max` false → recursive. |
| 3.4 | `exponent == max - 1` | `1e999999999` Scientific | Commas (`"…e999,999,999"`), not recursive. |
| 3.5 | **`show = false`** with exponent in `[min, max)` | `1e100000`, `show=false` | Skips the comma branch → falls to **recursive** (`"1.00e1.000e5"`). Distinct, untested path. |
| 3.6 | `show = false` with exponent `< min` | `1e99999`, `show=false` | Still plain (show only gates commas). |
| 3.7 | Custom `places_exponent < 2` | recursive case, `places_exponent = 0` | Nested places use `places_exponent.max(2)` → `2` → `"…e1.00e9"` (vs default `1.000e9`). |
| 3.8 | Custom `places_exponent` large | recursive case, `places_exponent = 4` | Nested mantissa shows 4 places. |
| 3.9 | Custom `min`/`max` | e.g. `min=10`, `max=1000` | Move the plain/commas/recursive boundaries to small, easily-checked exponents. |
| 3.10 | Extreme exponent near `i64` cap | `Decimal::new(1.0, 9_000_000_000_000_000_000)` | Recurses **once**: `"1.00e9.000e18"` — verifies recursion depth is bounded (see Notes). |

### Notes
- **Recursion depth is at most 1.** The exponent is an `i64` (≤ ~9.2e18); the recursive
  call formats `from_float(exponent)`, whose own exponent is ≤ ~19 — always `< min`, so
  it renders plain. A double-nested `e…e…e…` is therefore unreachable. Worth one test
  (3.10) to lock this property in.
- `format_with_commas` negative path is only reachable via direct unit test (the
  exponent fed by `format_exponent` is always ≥ `min > 0`). Keep the existing unit test;
  add inputs of length exactly 3 (`"123"`), 6 (`"123456"`), and 1 (`"7"`).

---

## 4. Standard notation (`src/notations/standard.rs`)

Letter abbreviation plus four regex-port string cleanups. After the base-10 refactor,
`abbreviate_standard` receives a **power-of-ten exponent (multiple of 3)** and divides
by 3 to recover the thousands index.

### Covered
- Single letters `K … No`, `Dc`, multi-letter `UDc`, `Vg`, `Ce`, the `MI` regex-cleanup
  path (`1e3003`), and `1e100002 → "TTgMI-TTgTc"`.
- No-roll-over case `9.999e8 → "999.90 M"`.

### Gaps / edge cases

| # | Target | Rationale |
|---|--------|-----------|
| 4.1 | `abbreviate_standard` unit tests (private `#[cfg(test)] mod`) | Test the function directly at raw exponents `3, 6, …, 30` (`K…No`), `33` (`Dc`), `36` (`UDc`), and `0` (the `exp == -1` → empty branch, unreachable via `format` but trivially unit-testable). Asserts the `/3` rescale in isolation. |
| 4.2 | `collapse_inner_dashes` unit tests | `-XX-` → `-`; non-overlapping left-to-right; pattern at string start/end; `-X-` (single letter, no match); `-XXX-` (no match); back-to-back `-XX--YY-`. |
| 4.3 | `strip_leading_u` unit tests | `U` before `XX-` dropped; `UU` before `XX-`; `U` not before `XX-` kept; multiple occurrences. |
| 4.4 | `strip_trailing_dash` unit tests | trailing `-` removed; no trailing `-`; empty string; lone `"-"`. |
| 4.5 | Roll-over into a new letter | a value whose mantissa rounds `999.x K → 1.00 M` (pairs with 2.2). |
| 4.6 | `places_under_1000` vs `places` separation | Standard `places` applies to mantissa only; verify `1e3` with `places=2` is `"1.00 K"` while a sub-1000 value uses `places_under_1000`. |

### Notes
- The four cleanup helpers encode JS regexes (`/-[A-Z]{2}-/g`, `/U([A-Z]{2}-)/g`,
  `/-$/`). The non-overlapping, left-to-right semantics and the `i+3 < len` boundary are
  the riskiest part of the port and deserve the dedicated unit tests above rather than
  only end-to-end coverage.
- `STANDARD_PREFIXES_2.get(i).unwrap_or("")` beyond the table is unreachable within
  `Decimal`'s range — document, do not chase.

---

## 5. Letters notation (`src/notations/letters.rs`)

Engineering-style 3-digit mantissa plus a bijective base-26 transcription of
`exponent / 3`.

### Covered
- `a`, `b`, `z` (boundary `n == base`, `1e78`), carry `aa` (`1e81`), `ab`, the
  `remainder == 0` case `yz` (`1e2028`), `za`, and `999.90b`.

### Gaps / edge cases

| # | Input | Expectation / path |
|---|-------|--------------------|
| 5.1 | Three-letter transcription | `1e2109 → "1aaa"` (`n = 703 = 26²+26+1`) — exercises the loop past two iterations. |
| 5.2 | Three-letter `remainder == 0` carry | `1e2106 → "1zz"` (`n = 702`) — the `n -= 1` carry at depth 2. |
| 5.3 | `to_engineering` offset for each residue | exponents `≡ 0, 1, 2 (mod 3)`: `1e9`, `1e10`, `1e11` → mantissa `×1, ×10, ×100`. Confirms `rem_euclid(3)`. |
| 5.4 | Letters under-1000 / very-small | `42` → `"42"`, `-42` → `"-42"` (default fallbacks; see 1.8/1.9). |
| 5.5 | Very large exponent (no commas/recursion) | `1e1000000002` → Letters **ignores** `exponent_display`; transcribes the whole exponent into a long base-26 string. Contrast with Scientific's recursion at the same magnitude. |
| 5.6 | `places` applied, `places_exponent` ignored | set `places_exponent`/`exponent_display` to non-defaults and confirm the Letters output is unchanged. |

### Notes
- `transcribe` has a fast path for `n <= base` that must agree with the loop at the
  boundary `n == base` (`z`). 5.1/5.2 push past it; keep `1e78` (`z`) as the boundary
  guard.

---

## 6. Options & cross-cutting (`src/options.rs`)

| # | Target | Case |
|---|--------|------|
| 6.1 | `FormatOptions::default()` | Scientific, `places=2`, `places_under_1000=0`, `places_exponent=3`, default `ExponentDisplay`, `inf_threshold=None`. Lock the defaults (they are an API contract for `ad-gui`). |
| 6.2 | `FormatOptions::new(n)` | Overrides only `notation`, leaves the rest at default. |
| 6.3 | `ExponentDisplay::default()` | `show=true`, `min=100_000`, `max=1_000_000_000`. |
| 6.4 | `places` sweep | `places ∈ {0,1,2,5}` for a fixed value across notations — digit count and trailing zeros. |
| 6.5 | `places_under_1000` independent of `places` | a sub-1000 value with `places=2, places_under_1000=3` uses the latter. |

---

## 7. Rounding divergence — **[pin]** (`format_mantissa`)

`format_mantissa` uses Rust `{:.*}` (round-half-to-**even** on the exact binary value);
JS `toFixed` rounds half-**away-from-zero**. This is the open `TODO(fidelity)`. Pin the
current Rust behaviour so the eventual reconciliation is a visible, intentional change.

| # | Input | Rust (current) | JS (`toFixed`) |
|---|-------|----------------|----------------|
| 7.1 | `2.5e3`, `places=0`, Scientific | `"2e3"` | `"3e3"` |
| 7.2 | `1.25e3`, `places=1`, Scientific | `"1.2e3"` | `"1.3e3"` |
| 7.3 | `0.5` (under-1000), `places_under_1000=0` | `"0"` (or `"1"`?) — verify | `"1"` |

Use only **exactly-representable** half-way values (`0.5`, `1.25`, `2.5`); values like
`1.35`/`9.995` round according to their binary representation in *both* languages and are
not clean divergence cases. Each pinned assertion gets a comment referencing the
`TODO(fidelity)` in `mantissa.rs`.

---

## 8. Recommended test infrastructure

1. **Property tests (`proptest`).** Add a `tests/properties.rs` asserting invariants that
   are hard to hit with literals and guard the precision-correction branch (2.3):
   - For random `Decimal` with exponent in `[3, 1e8]`, the parsed mantissa of Scientific
     output lies in `[1, 10)`; Engineering/Standard in `[1, 1000)` (or the string is the
     roll-over `1.00…`).
   - `format(value)` and `"-" + format(-value)` agree for non-zero values (sign
     symmetry).
   - Output never contains `NaN`/`inf` substrings for any in-range input.
2. **Direct engine unit tests.** Move/add `#[cfg(test)]` modules inside `mantissa.rs` and
   `standard.rs` to reach the `pub(crate)` engine and the private regex helpers (covers
   the otherwise-dead clamp / `exponent == 0` / `exp == -1` paths intentionally).
3. **Fidelity regeneration script.** A small Node script that requires
   `@antimatter-dimensions/notations` and emits the new cases from Sections 1–6 as
   `(value, places, expected)` rows, so the fidelity assertions stay generated rather than
   hand-typed.
4. **Boundary table helper.** A parametrised helper that asserts a value and its
   `±1`-exponent neighbours land in the expected region, to make the router/exponent
   boundary cases (1.5, 1.7, 3.1–3.4) compact.

---

## 9. Coverage summary

| Area | Existing | Highest-value additions |
|------|----------|-------------------------|
| Router regions & sign | partial | inf-threshold boundary, negative very-small `-0`, all-notation under-1000/negatives |
| Mantissa engine | indirect | direct-call tests for clamp & `exp==0`; proptest for precision correction |
| `format_exponent` | good | `show=false`, exact min/max boundaries, `places_exponent` sweep, recursion-depth cap |
| Standard | end-to-end only | private unit tests for the 4 regex ports + `abbreviate_standard` |
| Letters | good | 3-letter transcription, `rem_euclid` residues, exponent-display ignored |
| Options | none | default/`new` contracts, places independence |
| Rounding | none | pinned half-to-even divergence cases |
