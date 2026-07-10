# RUST_ISSUE_009: TclOO works in the runtime + WASM-eval path but is entirely absent from the bytecode VM

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `VM` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

VM — TclOO works in the runtime + WASM-eval path but is entirely absent from the bytecode VM.
runtime/rust/src/cmd_oo.rs registers `oo::define/objdefine/copy/method/…` unconditionally (runs under WASM via eval→runtime). rust/tcl-vm has no OO whatsoever (grep finds no oo::class/method/self/MethodDef). `oo::class create C {...}` runs under WASM and native but errors/traps in the VM. Confidence: high

## Resolution

TclOO now runs in the bytecode VM. A new `rust/tcl-vm/src/cmd_oo.rs` ports the
object system from the WASM runtime to the VM's `Rc`-value / bytecode model,
with the design mirroring the runtime where the VM's common-case parity target
allows:

- Objects and classes are a new `Command::Object(name)` variant (analogous to
  `Command::ChildInterp`) keyed into a per-interp `OoState` (`classes` /
  `objects` registries, the active method-call stack, the current definition
  target). **Every class is also an object** (an instance of `::oo::class`).
- A method runs as a normal proc activation via a new `Vm::oo_run_method` seam:
  it enters the proc, links the declared instance variables into the fresh frame
  (they live in the object's private `::oo::ObjN` namespace), pushes an `OoFrame`
  so `self`/`my`/`next` can consult the resolved chain, runs to completion, and
  pops. Method bodies are compiled once, at definition, through the same
  `compile_dynamic_body` path `proc` uses.
- Method resolution walks a keep-last linearisation (object mixins → the object
  → the class MRO), so a diamond defers a shared base until after everything
  deriving from it — the order `next`/`nextto` follow. Constructor and destructor
  chains use the same linearisation, most-derived first.

Implemented (all verified against tclsh 9.0.4): `oo::class create` (with the
`method`/`constructor`/`destructor`/`superclass`/`variable`/`export`/`unexport`/
`mixin`/`forward` body directives), `oo::object`, `create`/`new` with the
constructor chain, public dispatch with export enforcement (a method is exported
by default iff its name begins with a lowercase letter), `my`, `self` (+
`object`/`class`/`method`/`namespace`/`call`/`target`), `next`, `nextto`,
instance variables (class-declared and `my variable`), `destroy` with the
destructor chain, `oo::define`/`oo::objdefine` (script and single-command forms),
anonymous classes/objects, and an `info object`/`info class` introspection subset
(`class`/`isa`/`methods`/`mixins`/`namespace`/`creationid`/`variables`/`vars` and
`superclasses`/`subclasses`/`instances`/`methods`/`constructor`/`destructor`/
`variables`). Error messages match C for the common cases (`unknown method "X":
must be …`, `no next method implementation`, `invalid command name "self"` at top
level, the empty-constructor-body optimisation, …).

Covered by `rust/tcl-vm/tests/cmd_oo_e2e.rs` (31 oracle-checked end-to-end
tests). Deferred to follow-ups (documented, not silently missing): the TIP 500
`private` scope, TIP 558 `oo::configurable`/`property`, the `oo::Slot` / TIP 380
slot machinery, `filter`s, `oo::copy`, the `oo::abstract`/`oo::singleton`
metaclasses, and full C3-vs-keep-last divergence on pathological multiple
inheritance (the VM uses one keep-last linearisation for both dispatch and
constructor/destructor chains).
