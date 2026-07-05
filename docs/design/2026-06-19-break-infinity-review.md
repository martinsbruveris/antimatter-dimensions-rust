---
status: Reference
---

# Break Infinity Code Review

**Date:** 2026-06-19 **Focus:** Function naming, code style, visible API

## Function Naming

| Issue | Location | Suggestion |
|-------|----------|------------|
| `p_log10` is cryptic | L884 | Rename to `positive_log10` or `log10_clamped` |
| `dp()` is opaque | L1055 | Rename to `decimal_places()` |
| `sqr()` vs `sqrt()` easy to confuse | L984 | Consider `squared()` |
| `to_number()` is a JS-ism | L616 | Prefer `to_f64()` (Rust convention) |
| `sign()` returns `f64` | L761 | Return `i8`/`i32` or align naming with `signum()` |
| `logarithm()` is just an alias for `log()` | L897 | Remove—two names for one thing is confusing |
| `pow_base()` ("raise base to self") | L963 | Confusing; consider removing or `as_exponent_of()` |
| `egg()` / `ascension_penalty()` | L1074–1086 | Game-specific joke fns don't belong in a numeric lib |

## Code Style

1. **`Neg for Decimal` (L358–365):** Unnecessary `.clone()` on a `Copy` type. Simplify to
   `Decimal::new_no_normalize(-self.mantissa, self.exponent)`.

2. **`partial_cmp` bug (L402–408):** Second infinity branch is identical to the first
   (same condition, wrong result). `Greater` is never returned for the infinity case.

3. **`pad_end` (L46):** Over-engineered for internal use—`Cow`, repeated `format!`,
   `&'static str`. Standard `format!` width specifiers would suffice.

4. **`to_fixed_num` (L81):** Format-to-string then parse-back-to-f64 is wasteful. Use
   arithmetic rounding instead.

5. **Dead JS guard (L47):** `f32::is_nan(max_length as u32)` can never be true—`u32` is
   never NaN.

6. **`exponent: f64` (L108):** An inherently integer value stored as `f64` forces
   floating-point comparisons everywhere. Should be `i64`.

7. **Magic number `3.16227766016838` (L994):** Replace with `10.0_f64.sqrt()` or a named
   constant.

## Visible API

1. **Public implementation details:** `LENGTH`, `CACHED_POWERS`, `NUMBER_EXP_MIN/MAX`,
   `pad_end`, `to_fixed`, `to_fixed_num` are internal helpers exposed as `pub`. Should be
   `pub(crate)`.

2. **Game economics functions** (`afford_geometric_series`, `sum_geometric_series`,
   etc.): Belong in a separate module or behind a feature flag.

3. **`Eq` without `Ord`:** `Eq` is implemented but `Ord` is not. Since NaN values exist,
   `Eq` violates reflexivity (NaN ≠ NaN). Either remove `Eq` or handle NaN canonically
   and implement `Ord`.

4. **`From<u128>`/`From<i128>`:** Silently loses precision via `as f64`. Violates the
   `From` contract (lossless). Use `TryFrom` or document precision loss.

5. **No `Ord` implementation:** Users can't sort `Vec<Decimal>` or use it in `BTreeMap`.

6. **Deprecated free constructors (L91–99):** Already deprecated—remove in next breaking
   release.

## Exponent Type: `f64` → `i64` Impact Analysis

The `f64` exponent is a JS-port artifact. The value is conceptually always an integer.
Converting to `i64` would eliminate NaN/infinity edge cases, enable sound `Eq`/`Ord`, and
make comparisons faster—at the cost of a breaking API change and some mechanical
refactoring.

### Trivial conversions (direct integer arithmetic)

| Operation | Current | With i64 |
|-----------|---------|----------|
| `exponent + decimal.exponent` (mul, L265) | f64 add | i64 add |
| `exponent - 14.0` (add, L175) | f64 sub | `exponent - 14` |
| `-self.exponent` (recip, L819) | f64 neg | i64 neg |
| `exponent * 2.0` (sqr, L985) | f64 mul | `exponent * 2` |
| `exponent * 3.0` (cube, L1002) | f64 mul | `exponent * 3` |
| `exponent % 2.0` (sqrt, L991) | f64 rem | `exponent % 2` |
| `exponent % 3.0` (cbrt, L1015) | f64 rem | `exponent % 3` |
| All comparisons (L114, 120, 122, 158…) | f64 cmp | i64 cmp (no NaN edge cases) |

### Require `as f64` cast (semantically unchanged)

| Location | Pattern | With i64 |
|----------|---------|----------|
| `log10()` L877 | `self.exponent + mantissa.log10()` | `self.exponent as f64 + …` |
| `abs_log10()` L881 | same | same |
| `to_precision()` L732–733 | `(places as f64) > self.exponent` | `places > self.exponent as u32` |
| `to_exponential()` L696–697 | `format!(..., self.exponent)` | works directly (i64 formats cleanly) |
| `to_fixed()` L716 | `(self.exponent + 1.0) as u32` | `(self.exponent + 1) as u32` |

### Floor-division vs truncation (needs care)

Rust's integer `/` truncates toward zero, but the current code uses `.floor()`:

```rust
// sqrt (L995, L998): (self.exponent / 2.0).floor()
// cbrt (L1018–1026): (self.exponent / 3.0).floor()
```

For negative exponents: `floor(-5 / 2)` = -3, but `-5i64 / 2` = -2. Fix with:
```rust
self.exponent.div_euclid(2)  // correct floor division for i64
```

### The `pow()` function (L928) — hardest case

```rust
let temp = self.exponent * number;       // exponent × arbitrary f64 power
let new_exponent = temp.trunc();         // integer part → new exponent
let residue = temp - new_exponent;       // fractional → mantissa adjustment
```

This **already splits** into integer and fractional parts. With i64:
```rust
let temp = (self.exponent as f64) * number;
let new_exponent = temp.trunc() as i64;
let residue = temp - temp.trunc();
```

No loss of correctness—the result is always truncated to integer anyway.

### `normalize()` (L604–611)

```rust
let temp_exponent = self.mantissa.abs().log10().floor(); // f64, integer-valued
exponent: self.exponent + temp_exponent,
```

Becomes: `self.exponent + temp_exponent as i64`

### Things that get simpler

- **`partial_cmp`**: Remove all `f64::is_nan(self.exponent)` checks
- **`Eq` soundness**: Only mantissa NaN remains (much easier to handle)
- **`Ord` implementable**: i64 comparison is total
- **`new()` validation**: Remove `!f64::is_finite(exponent)` check
- **`EXP_LIMIT`**: `9_000_000_000_000_000_i64` fits (i64 max ≈ 9.2e18)
- **Struct size unchanged**: f64 and i64 are both 8 bytes

### Breaking changes

1. Public API signatures: `new(f64, f64)` → `new(f64, i64)`
2. `EXP_LIMIT` type: `f64` → `i64`
3. `FromStr`: exponent parse changes from `f64` to `i64` (different error type)
4. Serde serialization format (exponent as integer)
5. `pow10(power: f64)`: needs `power.trunc() as i64` internally (API unchanged)

### Verdict

The refactor is safe and mechanical. The only "real" work is in `pow()` and
`normalize()`, both of which already treat the result as integer. The main cost is the
breaking API change.

## Priority

1. **Fix the `partial_cmp` bug** — produces incorrect ordering for infinities.
2. **Fix `Eq` + NaN inconsistency** — unsound, can cause UB in unsafe code relying on
   `Eq`.
3. **Hide internal helpers** — reduce public API surface.
4. **Rename opaque functions** — `dp`, `p_log10`, `to_number`.
5. **Move game-specific code** out of the core numeric lib.
