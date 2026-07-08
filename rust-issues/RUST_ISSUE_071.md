# RUST_ISSUE_071: the multi-arg `switch` form lowers arm *bodies* with no static-literal/single-token guard, so a substituted body is compiled from its unsubstituted spelling

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler lowering / variable scoping |
| **Location** | `rust/tcl-compiler/src/lowering/structured.rs:797-820 (+ build_switch_arms :681-683)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/lowering/structured.rs:797-820 (+ build_switch_arms :681-683) — the multi-arg `switch` form lowers arm *bodies* with no static-literal/single-token guard, so a substituted body is compiled from its unsubstituted spelling.
Unlike if/catch/try/while/for (which bail on a non-`Str` body word), the multi-word branch pushes `SwitchPair { body_text, body_arg_idx: Some(i+1) }` unconditionally and calls `lower_body_from_tok`. `set handler {puts hi}; switch $x a $handler` — real Tcl runs the *value* of `$handler`; the compiler lowers literal `${handler}` as the body, producing a phantom Call to a command `${handler}`. Downstream dead-code/taint/def-use/call-graph reason about nonexistent code, no fallback. Confidence: high
