# TclVM variable and namespace conformance lane

## Goal

Close the coherent variable-store and namespace-link group tracked by #1729,
#1582, and the live residual of #1588 against Tcl 9.0.4. The lane covers both
Rust Tcl runtimes where they expose the same semantics: the bytecode VM in
`rust/tcl-vm` and the compiler-target runtime in `runtime/rust`.

## Oracle

The gold implementation is `/home/jimd/src/tcl9.0.4`, run through
`/home/jimd/src/tcl9.0.4/unix/tclsh` with
`LD_LIBRARY_PATH=/home/jimd/src/tcl9.0.4/unix`. Deterministic regression rows
record the oracle result and error code; release-invariant rows may also be
spot-checked against the other locally available Tcl releases.

## Design decisions

- Array element access remains a `(base, key)` operation all the way through
  `tcl_runtime_api::VarStore`, including its frame-addressed implementation.
  A runtime implementation must not reconstruct `base(key)` and feed it back
  through the scalar name parser. `tcl-cmd-core` then serves both runtime
  adapters without a command-specific workaround.
- Parent-namespace validation belongs at variable target resolution/link
  installation, before element-shape checks. The written name and operation
  remain adapter concerns because Tcl's messages differ (`define`, `access`,
  and `create`).
- An `upvar` link from a procedure-local target into a namespace variable is
  rejected before either namespace existence or local element-shape checks.
  Tcl 9.0.4 calls this the inverted link (`TCL UPVAR INVERTED`): allowing the
  shorter-lived procedure cell to escape would leave a dangling namespace
  link.
- Generic command dispatch and specialised bytecode operations must call the
  same VM helpers. No command spelling is used to select semantics in the
  compiler or execution loop.

## Site inventory

| Site | Responsibility | Status |
|---|---|---|
| `rust/tcl-runtime-api/src/lib.rs` | Pair-valued `VarStore` contract | Done: non-recomposition invariant explicit |
| `rust/tcl-cmd-core/src/array.rs` | Shared `array get`/`unset` consumer | Done: unchanged shared consumer, covered through both adapters |
| `rust/tcl-vm/src/interp.rs` | Frame-aware pair element storage and link target classification | Done |
| `rust/tcl-vm/src/command.rs` | Generic `global`/`variable`/`upvar` messages and ordering | Done |
| `rust/tcl-vm/src/exec.rs` | Specialised `UPVAR`/`NSUPVAR`/`VARIABLE` execution | Done: `UPVAR` routes through the same semantic linker |
| `runtime/rust/src/vars.rs` | Pair element access and link-home ownership | Done, including non-active `FrameId` access |
| `runtime/rust/src/cmd_var.rs` | Runtime adapter messages and ordering | Done |
| `rust/tcl-vm/tests/variable_name_resolution_e2e.rs` | TclVM oracle-derived end-to-end rows | Done: generic and compiled paths, messages and `errorCode` |
| `runtime/rust` variable/array tests | Compiler-target runtime regressions | Done: pair store, inverted link, and precedence |

## Behavioural deltas

- #1729: `array get` must retain elements when the array base contains unmatched
  or balanced parentheses, while `names`, `size`, `unset`, direct element read,
  and direct element write keep their existing behaviour. Implemented and
  verified through both runtime adapters.
- #1582: creating a namespace link whose target is a procedure variable fails
  with `bad variable name "NAME": can't create namespace variable that refers
  to procedure variable` and error code `TCL UPVAR INVERTED`.
- #1588 residual: qualified `upvar` targets with absent parent namespaces fail
  in Tcl's precedence order. #1728 already supplied `global`/`variable`
  parent checks in `tcl-vm`; this lane must not duplicate them.

## Validation

- `rust/tcl-vm/tests/variable_name_resolution_e2e.rs` runs the 8.6 and 9.0
  VM profiles, exercises compiled and generic `upvar` dispatch, and compares
  every deterministic vector to a real Tcl shell when available.
- The link vector was run directly against Tcl 9.0.4 at
  `/home/jimd/src/tcl9.0.4/unix/tclsh`, yielding the recorded messages and
  `TCL UPVAR INVERTED`, `TCL LOOKUP VARNAME`, and `TCL UPVAR LOCAL_ELEMENT`
  codes byte-for-byte.
- `runtime/rust` unit tests cover the compiler-target runtime's semantic link
  rejection and its active and non-active frame `(base, key)` VarStore paths.

Qualified locals deliberately fall back to generic command dispatch because
the bytecode `UPVAR` opcode only models a simple procedure-local slot. Both
paths call the same target-home validation before their adapter renders Tcl's
operation-specific failure.
