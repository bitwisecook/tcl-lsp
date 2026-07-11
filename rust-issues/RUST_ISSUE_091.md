# RUST_ISSUE_091: `%u` is grouped with `%d`/`%i` and rendered SIGNED (`int_field`'s `negative = value < 0 && radix == 10` fires for `%u`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_format.rs:204,275` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

runtime/rust/src/cmd_format.rs:204,275 — `%u` is grouped with `%d`/`%i` and rendered SIGNED (`int_field`'s `negative = value < 0 && radix == 10` fires for `%u`).
`format %u -1` → `-1`; real Tcl → unsigned wrap `18446744073709551615`. The unsigned path (`value as u64 as u128`) exists but is only taken for radix≠10. Not a documented limitation. Confidence: medium
