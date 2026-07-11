# RUST_ISSUE_064: whole command families are never generated, so never differentially tested even for the one live pair

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `fuzzer` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

fuzzer — whole command families are never generated, so never differentially tested even for the one live pair.
generator.rs emits no `array` ops (despite arrays claimed in-scope), no upvar/uplevel/global/variable, apply/lambda, eval/subst, format/scan, regexp/regsub, trace, coroutines, floats, or string-comparison operators (eq/ne/in/ni/<=/>=). Confidence: high

## Resolution

`generator.rs` now generates the previously-missing families, each as a
**deterministic** production so a differential mismatch is a real backend bug,
not a test artifact:

- **`array`** — `array set` a scratch array, then a deterministic read (`array
  size`/`exists`, an element read, or `lsort`-wrapped `array names`/`array get`
  since hash-iteration order is unspecified); the scratch array is `array unset`
  afterwards so a later re-seed can't accumulate stale keys.
- **`format` / `scan`** — integer/string conversions only (float conversions are
  omitted so shortest-`double` formatting can't inject spurious divergences).
- **`apply`** a pure lambda, **`eval`** of a `list`-built command, **`subst`** of
  a string with an embedded read + command substitution.
- **scoping** — a one-shot proc reaching a seeded module global via `global` or
  `upvar 1`.
- **`regexp` / `regsub`** over a fixed set of safe patterns.
- **`string`** ensemble breadth (`compare`/`equal`/`first`/`last`/`repeat`/`map`/
  `trim`/`totitle`) and **`list`** breadth (`concat`/`join`/`linsert`/`lreplace`/
  `lmap`).
- **expression operators** — `<=`/`>=` plus the string/list relationals
  `eq`/`ne`/`in`/`ni`, and **float** literals in leaves (shortest-`double`
  printing of the *result* is itself under test).

Deliberately still excluded: `trace` (observably order-/timing-sensitive) and
coroutines (their own backend feature, `RUST_ISSUE_008`) — both are hard to
generate deterministically. The `broadened_grammar_is_exercised` unit test
asserts each new production actually appears, and `balanced_delimiters` still
holds over the wider grammar. Validated by a 1.8 K-seed `tclvm`↔`tclsh 9.0.4`
campaign (0 divergences — the new coverage is clean and the VM already matches C
on these families) and the in-process WASM value-differential arm.
