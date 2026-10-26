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

Native core construction selects the engine's math-function entry points.
Tcl 8.4 and Jim use their fixed expression-function tables. Tcl 8.5 and later
publish math functions as commands in `::tcl::mathfunc` when the numeric
backend is available. The constructor's command inventory is separate from
library initialisation and from authority to execute a later native operation.

## Features and build gates

- `wasm_stdlib` (off by default) embeds the Tcl 9 standard library and the
  Tcl-level `tcltest` package and seeds them into the in-memory VFS, so a
  self-contained `wasm32-wasip1` module bootstraps `source` / `package require`
  with no host filesystem. See
  [`vendor/tcl_library/`](vendor/tcl_library/README.md).
- `have_tommath` is a `build.rs` cfg, not a Cargo feature: the bignum rung of
  the numeric tower (and therefore `expr` and the math commands) compiles only
  when a libtommath C source tree is found (`$TCL_TOMMATH_DIR`, else
  `tmp/tcl9.0.4/libtommath`). `make runtime-rust-test` requires that source
  tree and runs the full semantic suite with the backend.
  `make runtime-rust-test-no-tommath-capability` checks the original scalar
  cache tests and missing-bignum refusal with the backend absent. Set `TCL_RUNTIME_EXPECT_NO_TOMMATH=1` only when source
  absence is intentional. This asserts the result of normal source discovery;
  it does not disable a backend that is present. A found source tree or any
  value other than `1` makes the asserted build fail. Leave the variable unset
  for normal builds; unexpected absence retains its build warning.
  Intentional absence emits Cargo capability metadata and
  `OUT_DIR/tommath-capability.json` with `detected: false` and
  `intentional_absence: true`. The build withdraws that file before every
  rerun, so a changed expectation or backend cannot retain stale status.
- On `wasm32-unknown-unknown` the native-only paths — coroutines (native stack
  swap plus threads) and the libtommath tower — are cfg-disabled.

## Why it's outside the workspace

This crate requires raw-pointer `unsafe` over one shared linear memory
(`c-extension-abi.md` §9); the root workspace sets `unsafe_code = "forbid"`. It
is therefore listed in the root `Cargo.toml`'s `exclude`, like `editors/zed`,
and keeps its own lockfile and `target/`. That also keeps the LSP/compiler
`cargo test --workspace` from regressing on runtime churn.

## Checked character views

`ValueOps::checked_character_view` returns owned `Rc<str>` text and reports
`ValueError`. Its default delegates to the existing `as_chars` method, preserving
adapter implementations. Existing `try_as_str` and `as_chars` signatures and raw
UTF-8 checks remain unchanged.

The NativeCore implementation uses a private original Unicode getter entry
issued only after selected C construction, bootstrap, final result cleanup and
its final currency check. A profile pin or generic constructor cannot issue it.
The entry retains weak interpreter/host currency and the actual backend getter;
policy or context mutation permanently stales it. A present stale entry refuses
rather than falling back to a generic ASCII or UTF-8 view.

The original object must have caller-owned live allocation storage. A temporary
allocation lease preserves that storage across updater/free callbacks without
adding a Native object reference; it does not make an arbitrary dangling pointer
valid. Header incarnation, original operation and constructor currency are
checked around the getter and callbacks. Returned text owns its bytes and can
outlive both the original object and interpreter.

C8 Unicode units are decoded as checked UTF-16, and C9 units as checked Unicode
scalars. Unsupported units refuse without inventing a UTF-8 error or replacing
original bytes. For example, a fresh NativeCore string containing `41 ff 42`
exposes `A`, U+00FF and `B` through the checked getter while the same raw bytes
still fail strict UTF-8 access. Generic and profile-only construction retain the
existing strict path.

The byte-length fallback uses the checked API. Compiled Native string replacement
uses its existing `NativeIndexOps` character count, preserving C8 UTF-16 indexing
rather than counting Rust scalar characters. These are separate purposes; an
owned character view grants no representation, compilation or frame authority.

## Build / test

```
make runtime-rust-test                         # full semantic suite; requires libtommath sources
make runtime-rust-test-no-tommath-capability    # 17 scalar-cache tests + missing-bignum refusal
make runtime-rust-lint                         # cargo fmt --check + locked clippy -D warnings
```

The capability target asserts actual source absence with
`TCL_RUNTIME_EXPECT_NO_TOMMATH=1`; leave that expectation unset for the full
semantic target, which retains every original semantic assertion. The optional
`make runtime-rust-test-no-tommath` command still runs the whole suite without
the backend, but does not replace either gate.

Run a script directly with the `run_script` dev-tool example:

```
cd runtime/rust && cargo build --release --example run_script
```

The leak gate is `round_trip_zero_residual`: `Tcl_NewObj` →
`Tcl_IncrRefCount` → `Tcl_SetObjResult` → `Tcl_DecrRefCount` → interp teardown
leaves **zero residual** under the counters.
