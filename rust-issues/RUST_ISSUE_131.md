# RUST_ISSUE_131: `binary scan` `a`/`A` field defeats its own bounds check via `usize` overflow → OOB slice / add-overflow panic

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Support crates & regex |
| **Location** | `rust/tcl-cmd-core/src/binary.rs:714` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cmd-core/src/binary.rs:714 — `binary scan` `a`/`A` field defeats its own bounds check via `usize` overflow → OOB slice / add-overflow panic.
`parse_count` saturates a numeric count to `usize::MAX`. The int/float paths were hardened with `checked_mul`/`checked_add`, but the `a`/`A` path uses raw `cur + n`: with `cur>=1` and `n=usize::MAX`, debug panics on the add; release wraps (`1+usize::MAX→0`), check `0 > len` is false, `data[1..0]` panics. Trigger: `binary scan "xy" {@1 a99999999999999999999999} v`. Mirror allocations `vec![pad; n]`/`vec![0u8; n]` in `binary format` (lines 483,542) likewise panic "capacity overflow" instead of a catchable Tcl error. Confidence: high
