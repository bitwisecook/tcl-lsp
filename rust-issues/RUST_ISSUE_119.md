# RUST_ISSUE_119: `strftime` formats a user-supplied format string via chrono `format(...).to_string()`, which panics on any invalid specifier instead of returning an error

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/builtins/time_dt.rs:206` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip-query/src/builtins/time_dt.rs:206 — `strftime` formats a user-supplied format string via chrono `format(...).to_string()`, which panics on any invalid specifier instead of returning an error.
`strftime(0; "100%")` or `strftime(0; "%E")` lexes to `Item::Error` → `DelayedFormat::fmt` returns `fmt::Error` → `ToString` panics ("a Display implementation returned an error unexpectedly"); verified against vendored chrono-0.4.45 (`Item::Error => Err(fmt::Error)`). This violates the crate's own hardening contract (tests/hardening.rs requires clean errors, never panics) and aborts the in-report wasm console. Quote: `Ok(Value::Str(dt.naive_utc().format(&fmt).to_string()))`.
Confidence: high
