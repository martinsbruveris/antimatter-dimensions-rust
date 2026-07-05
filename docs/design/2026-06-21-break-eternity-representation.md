---
status: Proposed
---

# Break Eternity Number Representation

**Date:** 2026-06-21 **Focus:** Rust type design for break_eternity numbers

## Background

The game uses two number libraries with different internal representations:

**break_infinity.js** — scientific notation:
- Stores: `mantissa` (f64, normalized to [1, 10)) and `exponent` (f64)
- Represents: `mantissa × 10^exponent`
- Range: up to ~10^(9×10^15)

**break_eternity.js** — iterated exponentials:
- Stores: `sign` (±1), `layer` (integer), `mag` (f64)
- Layer 0: `sign × mag` (direct value, small numbers)
- Layer 1: `sign × 10^mag` (mag = log10 of absolute value)
- Layer 2: `sign × 10^(10^mag)`
- Layer n: `sign × 10↑↑n(mag)`

## The Layer 1 Mismatch

Layer 1 in break_eternity is **not** numerically equivalent to break_infinity's
(mantissa, exponent):

| | break_infinity | break_eternity layer 1 |
|---|---|---|
| 5×10^100 | mantissa=5, exponent=100 | sign=1, layer=1, mag=100.69897... |
| Conversion | → mag = log10(\|mantissa\|) + exponent | → mantissa = 10^fract(mag), exp = floor(mag) |

The conversion requires `log10` / `10^x`, introducing floating-point rounding errors.

## Practical Layer Values in the Game

From `src/core/constants.js`:
- `BIMAX = "e9e15"` — Break Infinity MAX, layer 1 (10^(9×10^15))
- `BEMAX = "10^^9000000000000000"` — Break Eternity MAX, layer ~9×10^15
- Devtools sanity check: `layer > 8e15` (corruption detection)
- Format threshold: `decimal.layer >= 2` triggers "large notation" display

In practice, ~99% of gameplay values stay within layer 0–1 (the Scientific range).

## Recommended Design: Enum with Two Variants

```rust
#[derive(Clone, Copy, Debug)]
pub enum Decimal {
    /// Layer 0–1: scientific notation (break_infinity semantics)
    /// value = mantissa × 10^exponent, mantissa ∈ [1, 10)
    /// Covers values from 0 up to ~10^(1.8×10^308)
    Scientific { mantissa: f64, exponent: i64 },

    /// Layer 2+: iterated exponentials (break_eternity semantics)
    /// value = sign × 10↑↑(layer)(mag), layer ≥ 2
    /// Covers values beyond 10^(1.8×10^308) up to 10^^(9×10^15)
    Tower { sign: i8, layer: u32, mag: f64 },
}
```

### Why Enum Over Unified Struct

A unified struct (`sign: i8, layer: u32, mag: f64`) matching break_eternity.js exactly
was considered. The enum is preferred because:

1. **Hot-path precision.** The Scientific variant preserves exact mantissa/exponent
   arithmetic for the common case. Multiplication is `mantissa₁ × mantissa₂` + `exp₁ +
   exp₂` — no log/pow round-trip needed.

2. **Branch prediction.** Since `Tower` is rare in practice (only late endgame), the CPU
   will predict `Scientific` nearly 100% of the time — no pipeline stalls.

3. **Cheap comparison.** Comparing two `Scientific` values: compare exponents first
   (integer compare), then mantissas. No need to compute differences of logarithms.

4. **Formatting.** The mantissa is directly available for display without `10^fract(mag)`
   recovery.

5. **Compatibility.** The existing break_infinity implementation is preserved as-is for
   the Scientific variant. Only the Tower path is new code.

### Tradeoffs

| | Enum approach | Unified struct |
|---|---|---|
| Multiplication (layer 1) | f64 mul + i64 add | f64 add (faster!) |
| Addition (layer 1) | align exponents + add | log10(10^a + 10^b) (lossy) |
| Comparison | integer cmp (fast) | f64 cmp on mag |
| Code complexity | match arms on binary ops | single code path |
| Memory | 24 bytes (discriminant + padding) | 16 bytes |
| Mantissa precision | exact (15–16 digits) | recovered via 10^fract(mag) |

### Cross-Variant Operations

When a binary operation involves one `Scientific` and one `Tower` value:
- Promote the `Scientific` to `Tower`: `mag = log10(|mantissa|) + exponent as f64`, layer
  = 1
- Then use the general tower arithmetic (layer comparison, magnitude comparison)
- This conversion is rare and only happens at the boundary

### Size Considerations

- `Scientific`: 8 (f64 mantissa) + 8 (i64 exponent) = 16 bytes
- `Tower`: 1 (i8 sign) + 4 (u32 layer) + 8 (f64 mag) + padding = 16 bytes
- Enum total: 16 + 8 (discriminant + alignment) = 24 bytes

The 8-byte overhead per value is acceptable. For bulk storage (e.g., save files), values
can be serialized compactly without the discriminant.

## Compiler Optimization Analysis

### What Optimizes Well

**Branch prediction:** If 99%+ of runtime values are `Scientific`, the CPU branch
predictor will near-perfectly predict the match arm. Cost: ~1 cycle for the
correctly-predicted discriminant check. Essentially free.

**Inlining:** With `#[inline]` on operations, LLVM inlines the match + hot path directly
at call sites. When the variant is statically known (e.g., `Decimal::Scientific { ..
}.add(x)`), LLVM eliminates the dead arm entirely via constant propagation.

**Constant folding:** If either operand is a compile-time constant (like `DC.D2`), LLVM
can often resolve the match at compile time, removing the branch completely.

### What Doesn't Optimize Well

**1. Auto-vectorization is killed.**

```rust
// LLVM cannot SIMD-vectorize this — discriminant check per element
for (a, b) in values.iter().zip(multipliers.iter()) {
    results.push(a.mul(*b));
}
```

A uniform struct would allow SIMD on the `mag` field. In practice this matters less than
expected — incremental game ticks aren't doing homogeneous bulk math on arrays.

**2. Size = 24 bytes (awkward for cache).**

A 64-byte cache line fits 2.67 enums vs 4 uniform structs (16 bytes). Matters for tight
loops over `Vec<Decimal>`; irrelevant for individual game state fields.

**3. Match on pairs = 4 arms.**

```rust
match (self, other) {
    (Scientific{..}, Scientific{..}) => ..., // hot path
    (Scientific{..}, Tower{..})      => ..., // rare
    (Tower{..}, Scientific{..})      => ..., // rare
    (Tower{..}, Tower{..})           => ..., // very rare
}
```

LLVM emits a nested branch (check self, then check other). Two predicted branches ≈ 2
cycles. Negligible vs the f64 arithmetic inside each arm.

### Cost Comparison Per Multiply (Hot Path)

| Approach | Instructions | Bottleneck |
|---|---|---|
| Enum (Scientific arm) | branch + f64 mul + i64 add + normalize | f64 mul latency (~5 cycles) |
| Unified struct (layer 1) | f64 add | f64 add latency (~3 cycles) |
| Enum overhead vs unified | ~2 cycles (branch + discriminant load) | masked by f64 pipeline |

The enum's overhead is **dominated by the arithmetic itself**. The branch cost is in the
noise.

### Potential Bottleneck Scenario

The one case where the enum costs measurably:

```rust
// Simulating many dimensions per tick, each with a multiplier chain
for dim in dimensions.iter_mut() {
    dim.amount = dim.amount.mul(dim.multiplier); // branch on every iteration
}
```

Mitigations if profiling shows this is hot:
- Use `std::hint::unreachable_unchecked()` on Tower arms in inner loops where values are
  guaranteed to be Scientific
- Keep a separate `ScientificDecimal` type for hot game state, convert to full `Decimal`
  only at boundaries (optimization, not redesign)

### Verdict

The enum approach is efficient for this use case because:
1. The hot path is a single predictable branch
2. f64 arithmetic cost >> branch cost
3. Incremental games don't do bulk homogeneous SIMD math
4. If profiling later reveals issues, the Scientific path can be extracted into its own
   type for inner loops without architectural changes

## Open Questions

1. **Should layer 0 be a separate variant?** Layer 0 numbers (mag < 10, stored directly)
   are uncommon in practice. Keeping them as `Scientific { mantissa: value, exponent: 0
   }` is simpler.

2. **NaN/Infinity handling.** Should these be a third variant, or sentinel values within
   `Scientific`?

3. **The `exponent: i64` assumption.** The existing break_infinity review recommends
   converting exponent from f64 to i64. This design assumes that refactor has been done.
