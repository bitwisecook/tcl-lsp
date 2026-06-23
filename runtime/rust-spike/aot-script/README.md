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

## Status (proven vs. open)

**Proven (structural path works):**
- The real Rust runtime builds clean to `wasm32-wasip1`.
- The emitted `::top` module links against the runtime in one wasmtime instance,
  **sharing the runtime's linear memory** (host write/read round-trip verified).
- `::top` runs to completion **without trapping**.

**Open (the remaining bug):** the wasm runtime's **eval/expr execution returns
the empty/false (no-result) outcome for everything** — `tcl_expr_bool("1 == 1")`
is `0`, `set`/`incr` side effects are not observable — even though the runtime's
**native** `codegen_abi` unit tests (`set x 42`→`42`, `1<2`→`1`) pass. This path
had never been exercised in the wasm build. The `thread_local! CURRENT_INTERP`
could not work in the bare wasip1 cdylib (no `_initialize`/TLS bootstrap) and was
changed to a single-threaded `AtomicPtr` global (`codegen_abi.rs`, `cfg(wasm32)`
only) — necessary but **not sufficient**: a deeper eval-path issue remains.

## Next step

Debug why `tcl_eval`/`tcl_expr_bool` are inert in the wasm build (the runtime
has no `_initialize` export — investigate whether required global/interp setup
is skipped, or whether `Interp::new()` / the expr evaluator hits a wasm-only
failure). Once a simple script runs correctly, move from this dynamic-link host
to the single-binary static link (see `../static-link/`).
