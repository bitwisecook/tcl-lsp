# RUST_ISSUE_022: `check_simple_arity`'s leading-option skip is name-only; a value-taking option's value word is counted as a positional argument, producing a false E003 on valid Tcl. The sibling W004 loop correctly uses `i += 1 + opt.value_word_count(...)` (validity.rs:1425) but this path does not

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `rust/tcl-compiler/src/analyser/diagnostics/validity.rs:439-452 (+ E003 at :518)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/analyser/diagnostics/validity.rs:439-452 (+ E003 at :518) — `check_simple_arity`'s leading-option skip is name-only; a value-taking option's value word is counted as a positional argument, producing a false E003 on valid Tcl. The sibling W004 loop correctly uses `i += 1 + opt.value_word_count(...)` (validity.rs:1425) but this path does not.
`regsub -start 0 $exp $str $sub out` (valid; `-start` value + exp/string/subSpec/varName = 4 = max) → `positional_start=1` skips only the name, leaving 5 counted → "Too many arguments for 'regsub': expected at most 4, got 5". (`regsub` arity `Arity::new(3,4)`; `-start` is `OptionValue::value`.) Confidence: high
