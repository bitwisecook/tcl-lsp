# RUST_ISSUE_011: the VM integer tower is bounded to i128 and wraps beyond; the runtime (and WASM-eval path) use arbitrary-precision bignum → different results

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `expr /,%,**` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

expr `/`,`%`,`**` — the VM integer tower is bounded to i128 and wraps beyond; the runtime (and WASM-eval path) use arbitrary-precision bignum → different results.
VM `int_arith`/`ipow` use `wrapping_add/mul` on i128 (expr.rs:200-235; doc :215-216 admits it wraps). The runtime routes through libtommath bignum. `expr {2**200}` or any product > 2^127: VM → wrapped/garbage, runtime/tclsh → exact bignum. (Floor div/mod semantics agree.) Confidence: high
