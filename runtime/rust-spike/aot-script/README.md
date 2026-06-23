# AOT-script spike — run an AOT-compiled Tcl script against the real Rust runtime

> **Spike, not the final design.** Proves the AOT-script path end to end at the
> structural level and pins the remaining runtime bug. The durable target is a
> single self-contained `.wasm` (runtime statically linked in); this spike uses
> dynamic linking (host wires the emitted module's imports to the runtime's
> exports) as the cheaper iteration loop.

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

## Next step

Broaden the script set / framing checks, then move from this dynamic-link host
to the single self-contained binary (static link via `wasm-ld`, see
`../static-link/`).
