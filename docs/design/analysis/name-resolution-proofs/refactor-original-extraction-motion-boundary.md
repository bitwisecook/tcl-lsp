# naming.refactor.original-extraction-motion-boundary

Kind: `implementation-contract`

## Problem statement

An extraction turns local reads into formal arguments and local writes into `upvar` links, relocates the selected commands into a new procedure frame and publishes that procedure. Most scripts cannot tell; a script that introspects its frame, traces a moved name or evaluates a computed script in the moved region can.

## Question

Where does procedure extraction on a release profile refuse because the interpreter can observe the move?

## Conclusion

A release profile plans an extraction with the same algorithm as the unversioned profile, and that algorithm's own preconditions apply: a selection that acts on the call frame, sits inside a definition body, captures an array element or a computed name, or needs a procedure name the current command table cannot prove free is refused. In addition a release refuses where the interpreter observes the move: the document establishes a trace (`Traits::ESTABLISHES_VARIABLE_TRACE`) or its commands cannot all be visited, so a trace cannot be ruled out; or a selected command introspects the current frame (`Traits::CURRENT_FRAME_INTROSPECTION`) or evaluates a computed script there (`Traits::DYNAMIC_EVAL_BODY`). A selected command whose descriptor is unavailable is refused, since its frame effects are unknown.

## Scope

`tcl-lsp-core` procedure extraction for C Tcl releases. Jim and BIG-IP are not covered. The boundary is stated for the capture forms the refactor produces: a value parameter for a variable only read, `upvar 1` for a variable written.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: script on standard input of the pinned reference `tclsh`. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

[lsp-extract-proc-preserved-captures.md](../../../../evidence/tcl/lsp-extract-proc-preserved-captures.md): a read-only capture passed as a parameter, a write carried by `upvar 1` and a loop accumulating into an `upvar`-linked variable give the values computed in place. [lsp-extract-proc-observable-motion.md](../../../../evidence/tcl/lsp-extract-proc-observable-motion.md): `info level`, a write-trace handler and `eval` of a computed script each observe the move.

### jim, bigip

Status: `not-tested`.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `reject_observable_motion`: Refuse a release extraction the interpreter can observe.
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `plan_extraction`: Apply the observable-motion refusal on a release profile before the shared plan.
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `refactor::extract_proc::tests::a_release_refuses_an_extraction_the_interpreter_can_observe` (linked): Frame introspection, a trace and a computed `eval` are refused on every release.

## Evidence

- [evidence/tcl/lsp-extract-proc-preserved-captures.md](../../../../evidence/tcl/lsp-extract-proc-preserved-captures.md)
- [evidence/tcl/lsp-extract-proc-observable-motion.md](../../../../evidence/tcl/lsp-extract-proc-observable-motion.md)
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs)
