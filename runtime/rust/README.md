# `tcl-runtime` — Rust port of the Tcl WASM runtime

The Rust runtime (Track 1 of the runtime port):
every allocation is balanced by a refcount-driven free, and the alloc/free
counters prove it.

## What's landed (T1.1)

- **`obj`** — the `#[repr(C)]` `TclObj` value model, ABI-faithful to
  [`c-extension-abi.md`](../../docs/design/runtime/c-extension-abi.md) §4.2
  (extensions dereference `objPtr->refCount`/`->bytes` directly). `fresh_zero`
  constructors (refCount 0, per
  [`c-api-ownership-contract.md`](../../docs/design/runtime/c-api-ownership-contract.md)),
  immediate refcount-driven free (`TclFreeObj`), on-demand int→string shimmer.
- **`interp`** — a minimal result-only `Interp` exercising the
  `Tcl_SetObjResult`/`Tcl_GetObjResult` ownership handshake.
- **`counters`** — leak-check instrumentation (`tcl_test_*`); see
  `memory-management.md` MM-C.
- **`capi`** — the `#[no_mangle] extern "C"` C-API exports for the above.

## Not yet (later Track-1 chunks)

Parse/subst (T1.2), eval loop + frames (T1.3), namespaces + command table
(T1.4), builtins (T1.5), and the `tcl_*`/`obj_*` codegen-import re-exports + the
wasm `memory`/table exports (T1.6). The codegen handle/tagged-immediate and
inline-string optimisations (the 32-byte layout) layer on at T1.5/S6 over
this ABI-faithful struct.

## Why it's outside the workspace

This crate requires raw-pointer `unsafe` over one shared linear memory
(`c-extension-abi.md` §9); the root workspace sets `unsafe_code = "forbid"`. It
is therefore **excluded** from the workspace (`/Cargo.toml`), like
`editors/zed`, and keeps its own lockfile and `target/`. This also keeps the
LSP/compiler `cargo test --workspace` (run in CI) from regressing on
runtime-port churn.

## Build / test

```
make runtime-rust-test     # cargo test  (the T1.1 leak round-trip gate)
make runtime-rust-lint     # direct cargo fmt + locked clippy gate (also in check-rust / CI)
```

The acceptance gate is `round_trip_zero_residual`: `Tcl_NewObj` →
`Tcl_IncrRefCount` → `Tcl_SetObjResult` → `Tcl_DecrRefCount` → interp teardown
leaves **zero residual** under the counters.
