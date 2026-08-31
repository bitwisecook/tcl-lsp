# TclVM bootstrap centralisation lane

## Goal

Resolve #1452 at the deepest owner: both interpreters must install one
canonical `tcl_platform` schema, and safe interpreters must derive their scrub
set from that same schema. Verify the adjacent #1450 and #1464 findings against
the current ancestor rather than duplicating fixes that have already landed.

## Design decisions

- `tcl-platform::bootstrap` owns the element names, portable constant values,
  and `scrub_in_safe` policy. `bootstrap::Values` is the narrow adapter for
  values that genuinely differ by engine or host.
- Both engines currently use the empty Tcl failure value for `osVersion`
  because the common `Host` seam has no uname/version operation. This removes
  runtime/rust's unsupported `"0"` spelling without adding a syscall to the
  leaf platform crate.
- `machine` is the compiled Rust target architecture. `user` comes from
  `USER`, then `USERNAME`, through the engine's environment seam (and is empty
  on a host that exposes neither).
- Safe scrubbing follows Tcl 9.0.4's `Tcl_MakeSafe`: remove `os`, `osVersion`,
  `machine`, and `user`; preserve portable keys. The project-specific runtime,
  WASM, WASI, and eBPF introspection facts are identity-bearing and are also
  removed. `threaded` remains, matching Tcl source and a threaded 8.6 oracle.
- `runtime/rust::Interp::new` performs the predefined-global bootstrap before
  `Tcl_Init`, as `Tcl_CreateInterp` does. Children get the same surface from
  construction and then inherit the parent's dialect profile; they do not run
  a second bootstrap pass.

## Site inventory

| Site | Status | Result |
| --- | --- | --- |
| `rust/tcl-platform/src/lib.rs` | done | canonical entries, providers, derived safe iterator, owner tests |
| `rust/tcl-vm/src/interp.rs::bootstrap_globals` | done | consumes all shared entries |
| `rust/tcl-vm/src/interp.rs::make_safe` | done | consumes shared safe iterator; local list removed |
| `runtime/rust/src/interp.rs::set_startup_globals` | done | consumes all shared entries, including `machine`/`user`; `osVersion` is empty |
| `runtime/rust/src/interp.rs::Interp::new` / child creation | done | fresh top-level and child predefined globals installed once |
| `runtime/rust/src/interp.rs::make_safe` | done | consumes shared safe iterator; local list removed |
| shared-owner contract and agent map | done | platform bootstrap owner recorded |
| deepest-owner and two-engine regressions | done | exact normal/child/safe schemas derived from owner |

## Behavioural deltas

- A fresh tree-walk interpreter now has `tcl_platform`, `env`, `argv`, `argv0`,
  `argc`, `auto_path`, and `tcl_library` before `init_library` is called.
- Tree-walk top-level and child interpreters now expose `machine` and `user`.
- Tree-walk `osVersion` changed from `"0"` to the shared empty fallback.
- Both safe engines retain `tcl_platform(threaded)` and remove every
  host/backend identity key from the shared schema.

## Adjacent issue audit

- #1450 was fixed by ancestor `f0f045517`: both engines consume
  `tcl_registry::safe_interp_hidden_commands()`, with release narrowing, and
  engine tests pin the Tcl-measured sets. This lane changes only the platform
  half that #1450 explicitly delegated to #1452.
- #1464 was fixed by the same ancestor: `TclVersion::core_provided_packages()`
  and shared build-info own the facts; both engines re-seed core packages when
  their profile changes. Existing release-matrix tests cover both engines.

## Validation and resumption state

- Checkpoint `9e0079913` compiles `tcl-platform`, `tcl-vm`, and the standalone
  `runtime/rust` crate on Rust 1.98.0.
- The `tcl-platform` owner tests, bytecode-VM top-level/child/safe tests, and
  tree-walk top-level/child/safe tests pass. `cargo xtask owner-resolution`
  resolves all 22 owner rows.
- The pre-existing #1450 suite passes all 12 bytecode-VM safe-interpreter
  tests. With `TCL_TOMMATH_DIR` pointing at the Tcl 9.0.4 source, the tree-walk
  hidden-set and callable-clock tests pass too.
- The pre-existing #1464 release-matrix tests pass in `tcl-dialect`, the
  bytecode VM, and the tree-walk runtime, including lowercase-package and
  build-info assertions.
- Mutation verification is complete and reverted: renaming the one shared
  `machine` row made both engines' fresh-platform tests fail on the missing
  canonical key; marking the one shared `threaded` row unsafe made both
  engines' safe-child tests fail on the required retained key.
- `cargo clippy -p tcl-platform -p tcl-vm --lib --tests -- -D warnings` is
  clean. Standalone runtime clippy is clean with its existing `cmd_fs.rs`
  `needless_borrow`/`useless_vec` lints allowed; the unmodified full strict
  invocation stops on those 16 pre-existing lints before reaching this lane's
  code.
- Tcl 9.0.4 oracle: core provides are `Tcl`/`tcl` `9.0.4` and
  `TclOO`/`tcl::oo` `1.3.1`; build-info says `9.0`/`9.0.4`; top-level safe
  hidden commands are `cd encoding exec exit fconfigure file glob load open
  pwd socket source unload zipfs`; safe `tcl_platform` retains
  `byteOrder engine pathSeparator platform pointerSize wordSize`.
- Tcl 8.6.17 oracle: only `Tcl` `8.6.17` and `TclOO` `1.1.0` are provided;
  build-info is absent; a safe child retains `threaded` along with the portable
  keys. Both oracles scrub `os`, `osVersion`, `machine`, and `user`.
- The final diff audit found no residual engine-local platform/safe table and
  no unreverted mutation. Implementation and validation are complete; the
  remaining action is orchestrator handoff. No push is permitted from this
  lane.

## Open uncertainty

Publishing a real kernel `osVersion` would require a new host capability and
is outside #1452. The shared schema makes that a single future provider change;
it must not be implemented by restoring an engine-local uname call.
