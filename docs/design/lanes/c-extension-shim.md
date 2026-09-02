# Lane: C extension shim (#1372)

Branch `claude/spectcl-issues-jfb20a-1372`. The third leg of the Tcl
extension interface: a shim that hosts a C Tcl extension behind
`tcl-engine-api`.

## Goal

A command written against the C Tcl API (`Tcl_CreateObjCommand`, `objv`,
`Tcl_SetObjResult`) runs on whichever engine the host picked, through the
same `HostCommand` door the hook host uses, with no string round-trips for
ints and lists and byte-for-byte C Tcl messages for the argument-handling
core. Trusted-native: never reachable from a pack or a hook body.

## Decisions taken

- **Crate `rust/tcl-cshim`**, not part of `tcl-engine-api` or the tclvm
  engine: it is a consumer of the interface, engine-blind by construction
  (`Interp<E: Engine>`).
- **Interface extension**: `Engine::remove_command` (default `Unsupported`;
  tclvm implements via `Vm::remove_command`). Engine-neutral: "forget a
  registered host command" is the other half of `define_command`.
- **tclvm fix**: `HostCommandShim` now passes a `Script { message, code }`
  error through verbatim with its `-errorcode`, instead of `error: <msg>`.
  Needed so `catch` sees exactly what the C code set.
- **`Tcl_Obj` is dual-rep** (`obj.rs`): optional string + `Rep`; a
  `canonical` flag decides whether the typed rep or the text crosses the
  boundary (text wins when the value was born as text).
- **Refcounts = Rust ownership**: `ObjRef` clone/drop are incr/decr; a
  fresh C object has count 0, as in C Tcl.
- **Variadics live in the header** as `static inline` C fanning out to
  fixed-arity exports (`TclShim_AppendResultString`, …) — stable Rust has
  no C variadics, and this is the shim absorbing a C idiom.
- **8.x sources**: `TCL_SHIM_TCL_MAJOR=8` gives `int` `Tcl_Size` plus inline
  wrappers for the three `Tcl_Size *` out-param functions, the way Tcl 9's
  own header does.
- **Panic safety**: every export runs under `guarded` (catch_unwind →
  fallback + thread-local parked message); `ShimCommand` reports a parked
  panic as `EngineError::Crashed`. C UB is outside any boundary — stated in
  the design doc.
- **Command creation while the engine is busy**: recorded in `InterpState`
  as pending changes and published by `Interp::sync`, called by `eval` and
  `load_static`.
- **C test extension built by `build.rs` with `cc`** on non-Windows only;
  tests gated on `cfg(cshim_c_tests)`. Rust-defined extensions through the
  same exports cover Windows (`src/lib.rs` tests, `tests/sandbox_isolation.rs`).
- **Expected strings captured from `tclsh9.0`** (Tcl 9.0.4) by building the
  same `pkga.c` against the real `tcl.h`.

## Site inventory

- [x] `include/tclshim.h` — full declared subset, everything implemented.
- [x] `src/obj.rs`, `src/state.rs`, `src/ffi.rs`, `src/lib.rs`.
- [x] `tests/c/pkga.c`, `tests/pkga_e2e.rs`, `tests/sandbox_isolation.rs`.
- [x] `tcl-engine-api::Engine::remove_command`; tclvm impl + error mapping.
- [x] Workspace member; `cc` build-dependency (MIT/Apache, on the allowlist).
- [ ] Docs: design doc, KCS note, glossary, README, AGENTS, spec-packs,
  indexes.
- [ ] `make rust-check`, `cargo build --workspace`, smoke.

## Behavioural deltas accepted

- `Tcl_ListObjAppendElement` on a shared object returns `TCL_ERROR` where
  C Tcl panics the process.
- `TCL_BREAK` / `TCL_CONTINUE` from a C command surface as the
  "invoked outside of a loop" errors; `TCL_RETURN` is treated as `TCL_OK`.
- `Tcl_PkgProvide` records shim-side; the engine's `package` command does
  not see it (the interface has no package door).
- Arguments arrive unshared (refcount 1); C code that duplicates-if-shared
  simply mutates its private copy.

## Open uncertainties

- Windows: the C test extension is skipped there; the exports themselves
  are exercised by the Rust-defined extension tests.
