# naming.interpreter.original-child-path-source-binding

Kind: `implementation-contract`

## Problem statement

A public parent command can move while its child path stays independent. Reporting-key lookup can lose original visibility after a rename or attach source operations to the wrong child.

## Question

How can typed original interpreter path selectors retain Create, call and move lineage for the same source child without claiming a successful operation or entered body?

## Conclusion

OriginalSourceInterpreterPathBinding retains the genuine original path producer, complete creation and current call vectors, original parent moves, complete ContextRegistry semantic key, full lexer configuration and source image. BodyInterpreter::Argument and typed interpreter, alias and delete transitions own the path ordinals. A unique exact creation-vector join selects the retained conditional source visibility snapshot independently of the reporting command spelling. Successful-operation, materialisation and body-entry obligations remain attached; this grants no runtime allocation, entered frame, selected current callable, Normal, effects closure or edit authority.

## Scope

Bounded root C source contexts with one genuine named child cell. Missing path producers, unknown intervening operations and ambiguous creation matches decline. Jim and unmeasured hosted inputs receive no counterpart capability. The separate native parent-command-move question retains its own evidence and does not execute this source contract.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This conditional source binding contract supplies no native interpreter allocation, successful operation, body entry or normal completion observation.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_transition_advice/interpreter_path.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/interpreter_path.rs), `OriginalSourceInterpreterPathBinding`: Retain independent original path input and complete creation/call/move lineage under full source/config/context checks.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/interpreter_path.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/interpreter_path.rs), `AdviceInvocationContext::retain_interpreter_paths`: Select path ordinals from the actual Registry body/transition descriptors and retain only a unique matching created source child without unknown intervening operations.
- [rust/tcl-compiler/src/realm/source_transition_advice.rs](../../../../rust/tcl-compiler/src/realm/source_transition_advice.rs), `CommandBindingRealm::original_source_interpreter_path_binding`: Expose the same sealed path binding only for the full original input and current operation vector.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `Analyser::original_interp_state_key`: Join conditional source visibility by exact original creation rather than parent-command reporting spelling; tainted or ambiguous states refuse.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_path_visibility_survives_parent_command_moves` (linked): C8.4–9.1 source controls retain hidden source targets through one move, two moves and a namespace-qualified parent move. Expose, delete and unknown-operation controls do not borrow hidden-source advice. These are conditional source edges, separate from the native move captures.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked source assertions require their own exact Rust source/executable receipts. Native parent movement remains the separate original-created-parent-command-move question; no observed allocation, successful transition, runtime entry or current-source parity is inferred.
