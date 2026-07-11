# RUST_ISSUE_010: structured `if`/`while`/`for` silently swallow error/return/break completion codes from eval'd leaf commands

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

WASM backend — structured `if`/`while`/`for` silently swallow error/return/break completion codes from eval'd leaf commands.
`tcl_eval` discards completion codes (codegen_abi.rs:148 "Completion codes are discarded in this tier"); the structured walk emits body/leaf commands as emit_command→tcl_eval (structured.rs:128,136) and `return` as eval-then-`WasmOp::Return` dropping the code (:112-115). So `error` raised inside a compiled while/for/if body does not propagate (loop keeps going), and `return -code error`/`-level N` degrade to a plain return — diverging from VM/runtime/tclsh. Confidence: high

## Resolution

The eval-fallback tier now **honours** each leaf command's completion code instead of discarding it.

- The runtime codegen ABI gains `tcl_eval_code(script) -> i32` (`runtime/rust/src/codegen_abi.rs`): it evaluates the boxed command and returns its raw completion code (`0` ok … `4` continue, or a `return -code N`), leaving the result as the interp's own (no owned reference to release). `tcl_eval` (result-returning) is kept for the whole-program bootstrap's query read.
- The WASM emitter (`rust/tcl-compiler/src/codegen/wasm/backend.rs`) replaces `emit_command`'s `tcl_eval` + `tcl_obj_release` with `tcl_eval_code` followed by a **completion dispatch**: inside a loop, `break` (3) / `continue` (4) re-enter that loop's structural scopes (so a *dynamic* break/continue from a called command behaves like a literal one); any other non-`OK` code (error, return, `return -code N`, or a break/continue with no enclosing loop) unwinds the function with `return`; `OK` (0) falls through. Each emitted function declares one `i32` scratch local for the code. This mirrors the tree-walker's "stop the script on the first non-`OK` command" loop.
- Note on `return -code error`/`-level N`: a deferred `return` (the default `-level 1`) completes with code `return` at eval time — the `-code`/`-level` settle only at a *proc boundary* (`Interp::settle_return`), so in this whole-program tier it unwinds the compiled function (the faithful behaviour for a void, non-code-returning function); an immediate `-level 0 -code X` surfaces `X` directly and unwinds the same way.

Verified: new runtime ABI unit tests (`tcl_eval_code` reports each code, leak-balanced); a byte-exact `wasmtime` execution test (`emitted_completion_codes_propagate`, `tcl-compiler/tests/wasm_execute.rs`) driving the abrupt paths through a recording host; and in-process `wasmtime`-embedded value-differential tests (`tcl-fuzz/src/wasm_diff.rs`) proving error/return/`eval break`/`eval continue` inside compiled loops match direct `tcl-vm` execution (they would `WasmHang`/diverge under the swallow). Cross-checked against a locally-built **tclsh 9.0.4** oracle.
