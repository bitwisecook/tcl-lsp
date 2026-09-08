# `tcl-runtime` — Rust Tcl runtime for the WASM target

The support library and interpreter fallback the compiler's WASM output links
against: an ABI-faithful `Tcl_Obj` value model, a parser and substitution
engine, an eval loop with frames and namespaces, and the builtin command set.
Every allocation is balanced by a refcount-driven free, and the alloc/free
counters prove it.

## Layout

| Area | Modules |
|---|---|
| Value model | `obj`, `typed_value`, `value_ops`, `list`, `dict`, `bytearray`, `bignum` |
| Front end | `parse`, `subst`, `expr` |
| Execution | `interp`, `frame`, `namespace`, `vars`, `ensemble`, `builtins` |
| Commands | `cmd_*` (one module per command family) |
| Embedding | `capi`, `codegen_abi`, `codegen_native`, `host_wasm`, `mem_fs`, `embedded_stdlib` |
| Instrumentation | `counters` (`tcl_test_*`, per `memory-management.md` MM-C) |

The value model is ABI-faithful to
[`c-extension-abi.md`](../../docs/design/runtime/c-extension-abi.md) §4.2 —
extensions dereference `objPtr->refCount` / `->bytes` directly — with
`fresh_zero` constructors (refCount 0, per
[`c-api-ownership-contract.md`](../../docs/design/runtime/c-api-ownership-contract.md))
and immediate refcount-driven free.

Every command in `tcl-registry` needs backing here — a handler, an
interpreter-fallback path, or an explicit not-required classification.
`cargo xtask command-backing --check` cross-checks the two and writes
[`wasm-command-backing.md`](../../docs/generated/wasm-command-backing.md).

## Features and build gates

- `wasm_stdlib` (off by default) embeds the Tcl 9 standard library and the
  Tcl-level `tcltest` package and seeds them into the in-memory VFS, so a
  self-contained `wasm32-wasip1` module bootstraps `source` / `package require`
  with no host filesystem. See
  [`vendor/tcl_library/`](vendor/tcl_library/README.md).
- `have_tommath` is a `build.rs` cfg, not a Cargo feature: the bignum rung of
  the numeric tower (and therefore `expr` and the math commands) compiles only
  when a libtommath C source tree is found (`$TCL_TOMMATH_DIR`, else
  `tmp/tcl9.0.4/libtommath`). `make runtime-rust-test-no-tommath` covers the
  build without it.
- On `wasm32-unknown-unknown` the native-only paths — coroutines (native stack
  swap plus threads) and the libtommath tower — are cfg-disabled.

## Why it's outside the workspace

This crate requires raw-pointer `unsafe` over one shared linear memory
(`c-extension-abi.md` §9); the root workspace sets `unsafe_code = "forbid"`. It
is therefore listed in the root `Cargo.toml`'s `exclude`, like `editors/zed`,
and keeps its own lockfile and `target/`. That also keeps the LSP/compiler
`cargo test --workspace` from regressing on runtime churn.

## Build / test

```
make runtime-rust-test              # cargo test (leak round-trip + unit/parse/eval suite)
make runtime-rust-test-no-tommath   # the same suite with libtommath absent
make runtime-rust-lint              # cargo fmt --check + locked clippy -D warnings
```

Run a script directly with the `run_script` dev-tool example:

```
cd runtime/rust && cargo build --release --example run_script
```

The leak gate is `round_trip_zero_residual`: `Tcl_NewObj` →
`Tcl_IncrRefCount` → `Tcl_SetObjResult` → `Tcl_DecrRefCount` → interp teardown
leaves **zero residual** under the counters.
