# RUST_ISSUE_011: the VM integer tower is bounded to i128 and wraps beyond; the runtime (and WASM-eval path) use arbitrary-precision bignum → different results

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `expr /,%,**` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

expr `/`,`%`,`**` — the VM integer tower is bounded to i128 and wraps beyond; the runtime (and WASM-eval path) use arbitrary-precision bignum → different results.
VM `int_arith`/`ipow` use `wrapping_add/mul` on i128 (expr.rs:200-235; doc :215-216 admits it wraps). The runtime routes through libtommath bignum. `expr {2**200}` or any product > 2^127: VM → wrapped/garbage, runtime/tclsh → exact bignum. (Floor div/mod semantics agree.) Confidence: high

## Resolution

The VM's `expr` integer tower is now genuinely arbitrary-precision (`rust/tcl-vm/src/expr.rs`), backed by pure-Rust `num-bigint` (the VM links no libtommath). The `i128` tier is kept as the fast path: `+`/`-`/`*` use checked arithmetic and **promote to a bignum on overflow** (`int_arith`); `**`/`<<` and any operation with an operand already past `i128` go straight to `big_arith`/`big_pow`. Floor div/mod, two's-complement bit ops, exact bignum comparison (an `f64` fallback would collapse `2**100` vs `2**100+1`), unary negate/bit-not, and `INST_TRY_CVT_TO_NUMERIC` all stay exact and canonicalise to a decimal string when past `i128`. Edge cases match tclsh: `0**-1` → *exponentiation of zero by negative power*, `-1**-n` by parity, `2**-1` → 0.

Verified against tclsh 9.0.4 (`2**200`, `(2**64)*(2**64)`, `10**30/3`, `1<<100`, exact comparisons, …) end-to-end through the `tclvm` CLI and a `bignum_tower_stays_exact` unit test.
