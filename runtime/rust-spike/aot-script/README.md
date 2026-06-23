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

## Status — WORKING for simple computational scripts

A simple AOT-compiled script runs **correctly** end to end:

```
set x 41 ; incr x                         ->  x = 42
set greeting "hello world"
set n [string length $greeting]           ->  n = 11   (command subst works)
set doubled [string repeat ab 3]          ->  doubled = ababab
```

The emitted `::top` links against the real runtime in one wasmtime instance,
shares the runtime's linear memory, and its eval-fallback `tcl_eval` calls
execute genuine Tcl with persistent side effects.

Two fixes got it there:
1. `codegen_abi.rs` — `CURRENT_INTERP` is a single-threaded `AtomicPtr` global on
   wasm32 (the `thread_local!` reads an uninitialised `__tls_base` in the bare
   wasip1 cdylib and never observes `set_current_interp`).
2. `emit_wasm` relocates the constant pool to `RESERVED_DATA_BASE` — at base 0
   the first boxed command sits at offset 0, and `tcl_obj_new_string(ptr=0, …)`
   is read as a null/empty pointer, silently dropping that command.

## Open (runtime command/IO gaps, not AOT-pipeline gaps)

- **`expr` command unregistered** — `expr 1+1` → `invalid command name "expr"`,
  so arithmetic via `expr` (and `tcl_expr_bool`) fails. `incr`/`string`/`set`/
  `list` work.
- **`puts` → WASI stdout not wired** — `puts` runs but emits no output.

## Next step

Close the two runtime gaps (register `expr`; wire `puts`/channels to WASI
`fd_write`), broaden the script set, add `errorInfo`/framing checks, then move
from this dynamic-link host to the single self-contained binary (static link
via `wasm-ld`, see `../static-link/`).
