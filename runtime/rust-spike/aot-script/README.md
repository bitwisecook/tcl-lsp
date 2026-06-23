# AOT-script spike — run an AOT-compiled Tcl script against the real Rust runtime

> **Spike, not the final design.** Proves the AOT-script path end to end. Two
> link models are demonstrated: a **dynamic-link** Python host (`host.py` — the
> cheap iteration loop) and a **single self-contained `.wasm`**
> (`build_standalone.sh` — the durable target: the runtime fused into one core
> module via `wasm-merge`, runnable with bare `wasmtime merged.wasm`).

## What it does

1. **Emit** a Tcl script's `::top` module:
   `cargo run -p tcl-compiler --example emit_wasm -- simple.tcl simple_top.wasm`
   (eval-fallback tier — each leaf command is boxed and handed to `tcl_eval`;
   the module imports the codegen ABI + `memory` from module `"tcl"`).
2. **Build** the real runtime to wasm:
   `cd runtime/rust && cargo build --target wasm32-wasip1 --lib --release`
   → `tcl_runtime.wasm` (exports `memory` + `tcl_eval`/`tcl_obj_new_string`/
   `tcl_expr_bool`/`tcl_obj_release` + `tcl_runtime_{create,set_current,delete}_interp`).
3. **Run** via the Python wasmtime host (`host.py`): instantiate the runtime
   (with WASI), `tcl_runtime_create_interp` + `set_current_interp`, instantiate
   the emitted module sharing the runtime's linear memory, call `::top`, then
   verify the side effect with `tcl_expr_bool`.

```
uv run --with wasmtime python host.py \
    ../../rust/.../tcl_runtime.wasm simple_top.wasm
```

## Status — WORKING incl. arithmetic, bignums, and conditions

An AOT-compiled script runs **correctly** end to end, including the numeric
tower and structured control flow:

```
set x 41 ; incr x                         ->  x = 42
set n [string length "hello world"]       ->  n = 11   (command subst works)
set doubled [string repeat ab 3]          ->  doubled = ababab
set product [expr {6 * 7}]                ->  product = 42      (expr tower)
set big [expr {2 ** 70}]                  ->  big = 1180591620717411303424 (bignum)
if {$product == 42} { ... }               ->  taken              (condition)
while {$i <= 5} { ... }                   ->  total = 15, i = 6  (loop condition)
puts "the answer is $n"                   ->  visible on stdout  (WASI fd_write)
```

The emitted `::top` links against the real runtime in one wasmtime instance,
shares the runtime's linear memory, and its eval-fallback `tcl_eval` calls (and
`tcl_expr_bool` condition checks) execute genuine Tcl with persistent side
effects.

Four fixes got it there:
1. `codegen_abi.rs` — `CURRENT_INTERP` is a single-threaded `AtomicPtr` global on
   wasm32 (the `thread_local!` reads an uninitialised `__tls_base` in the bare
   wasip1 cdylib and never observes `set_current_interp`).
2. `emit_wasm` relocates the constant pool to `RESERVED_DATA_BASE` — at base 0
   the first boxed command sits at offset 0, and `tcl_obj_new_string(ptr=0, …)`
   is read as a null/empty pointer, silently dropping that command.
3. `runtime/rust/build.rs` cross-compiles libtommath to `wasm32-wasi` with
   `zig cc`/`zig ar` and sets `have_tommath` on wasm — so the numeric tower
   (`expr`, `::tcl::math*`, `lseq`, the bignum obj rep) is present on wasm, and
   `tcl_expr_bool` uses the real evaluator (AOT `if`/`while` conditions, which
   previously always read false on the tower-less wasm build, now work).
4. `host_wasm.rs` adds a `WasiHost` (selected on `wasm32-wasip1`) whose
   `stdout`/`stderr` reach the real WASI `fd_write` via `std::io`, so `puts` is
   visible under `wasmtime`/any WASI host (the `wasm32-unknown-unknown`
   `BrowserHost` still discards — a browser console import lands later).

## Single self-contained binary (`build_standalone.sh`)

The durable target — one core `.wasm` with the runtime inside, no host
orchestration:

```
./build_standalone.sh demo.tcl merged.wasm   # build runtime + emit + wasm-merge
wasmtime merged.wasm                          # runs ::top via an emitted _start
```

How it works:
1. `emit_wasm --standalone` emits `::top` + procs **plus** a WASI `_start` that
   calls `tcl_runtime_create_interp` / `set_current_interp` (imported from
   `"tcl"`) then `::top`.
2. `wasm-merge runtime.wasm tcl user.wasm user -all` fuses the two modules into
   one: the emitted module's `"tcl"` imports (codegen ABI + memory + bootstrap)
   resolve to the runtime's exports, and the two linear memories collapse into
   the runtime's single memory.
3. The result is a plain core module (one memory, only WASI imports) — `wasmtime
   merged.wasm` runs it, and a browser can instantiate it with a WASI shim.

The runtime is built with `wasm-ld --global-base=0x20_0000` (`build.rs`) so its
data starts above the `RESERVED_DATA_BASE` (`0x10_0000`) window the emitted
constant pool relocates into — the two never overlap in the fused memory.

Verified end to end (`demo.tcl`, `broad.tcl`): `expr`/bignums, `if`/`while`/`for`,
recursive procs (`fib 10 = 55`), `string`/`list`/`dict`, and `puts` all run
correctly from a single `wasmtime merged.wasm` invocation.

## Running the real stdlib + tcltest (`build_test.sh`)

`build_standalone.sh` runs a bare script. `build_test.sh` goes further: it runs a
script (or a tcltest `.test` file) against the **real C-Tcl-9 standard library**
— `init.tcl`, the `package`/Tcl-module machinery, and the `tcltest` package —
all **embedded in the binary** and seeded into an in-memory VFS. No host
filesystem, no `--dir` preopen, no source files shipped alongside.

```
./build_test.sh ../../../tmp/tcl9.0.3/tests/set.test set.wasm
wasmtime set.wasm        # sources init.tcl, package require tcltest, runs the tests
```

Two pieces make this work, on top of the standalone path:

1. **Embedded stdlib + VFS.** The runtime, built with `--features wasm_stdlib`,
   embeds the read-closure of bootstrapping `init.tcl` and loading `tcltest`
   (the 14 files vendored under `runtime/rust/vendor/tcl_library/`) and seeds
   them into a [`MemFs`](../../rust/src/mem_fs.rs) the `WasiHost` mounts, reporting
   its mount point as `$TCL_LIBRARY`. The channel layer's `open` reads through
   the host filesystem when there is no native file (so `auto_load_index` can
   `open`/`gets` the `tclIndex`; `source`/`glob`/`file` already used the host).
2. **`init_library` in `_start`.** `emit_wasm --init` emits a `_start` that calls
   `tcl_runtime_init_library` (C's `Tcl_Init` — `source $TCL_LIBRARY/init.tcl`)
   between `set_current_interp` and `::top`, so the compiled script runs against
   a fully initialised interpreter where `package require` works.

### Status — `set.test` passes with full native parity

`set.test` from C Tcl 9 runs end to end from a single `wasmtime set.wasm`:

```
set.test:  Total 64  Passed 59  Skipped 1  Failed 4
```

— byte-for-byte the same result as running it through the native runtime
(`run_script --init`). The 4 failures and 1 skip are pre-existing runtime gaps
(a quoted-word parse-error case, variable-trace `errorInfo` framing, and the
`testset2` C-only constraint), **not** WASM/AOT/stdlib regressions — the WASM
path adds none.

## Next step

Lift more leaf commands out of the `tcl_eval` fallback into true inline AOT
(variable slots, arithmetic, per-command hooks); shrink the merged binary
(tree-shake unused runtime with `wasm-opt`); pre-compile the stdlib files
themselves (today they are embedded as source and interpreted — the eval tier
makes per-file AOT ~equivalent, but an inline-AOT tier would change that); fix
the pre-existing runtime parser/trace gaps so `set.test` reaches 63/64.
