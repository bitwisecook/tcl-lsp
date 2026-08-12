# WASM runtime boundary

The canonical Tcl-to-WASM compiler is `rust/tcl-compiler/src/codegen/wasm/`.
Its output uses the ABI implemented by `runtime/rust/src/codegen_abi.rs` and
the shared declarations in `rust/tcl-runtime-api/`. This document records the
current boundary; it does not describe a separate emitter or linker.

## Invocation and completion

`tcl_invoke_argv` receives an already evaluated argv, dispatches it through the
runtime interpreter, and writes the completion triple (code, result object,
return-options object) to the caller-owned output structure. The ABI validates
the interpreter, argc, argv, and word pointers before dispatch. It preserves
normal namespace lookup, aliases, ensembles, traces, safe-interpreter policy,
and Tcl command completion because the runtime performs the dispatch.

`tcl_eval` and `tcl_eval_code` remain explicit source-evaluation entry points
for runtime fallback and embedding. They have different ownership and return
contracts documented in `codegen_abi.rs`; generated code must not infer those
contracts from a rendered WAT module.

## Object ownership

The ABI exposes object construction, retain, release, and string access through
`runtime/rust/src/codegen_abi.rs`. Generated code uses `i32` handles into the
shared linear memory. Every borrowed argv word, returned result, and options
object follows the retain/release rules in that module. The compiler and
runtime do not maintain duplicate ABI constants.

## Runtime services

Command implementations are installed by `runtime/rust/src/builtins.rs` and
the `cmd_*.rs` modules. Parsing and substitutions are implemented by
`runtime/rust/src/parse.rs` and `subst.rs`; expression evaluation is in
`expr.rs`. Frame, namespace, trace, package, filesystem, coroutine, and TclOO
state remain runtime state, not compiler-side approximations.

The optional `wasm_stdlib` feature embeds Tcl scripts and package indices in
the runtime VFS (`embedded_stdlib.rs`). It does not currently provide a
package-driven compiler extension selector or a separate tcltest C-command
runtime. See [WASM extensions](wasm-extensions.md) for the dated future design.

## Capability and host boundaries

Host-facing operations are capability-gated in `runtime/rust/src/host_wasm.rs`.
The default sandbox rejects operations that require host process or filesystem
authority. Embedders provide the host hooks and decide which capabilities are
available; the Tcl completion path remains catchable by the interpreter.

## Related

- [WASM code generation](wasm-codegen.md)
- [WASM Explorer view contract](../contracts/wasm-explorer-view.md)
- [Runtime variable and frame model](../contracts/runtime-variable-frame-model.md)
