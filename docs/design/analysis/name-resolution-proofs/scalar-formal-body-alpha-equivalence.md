# naming.variable.scalar-formal-body-alpha-equivalence

Kind: `implementation-contract`

## Problem statement

Renaming a formal can change info args/body, frame introspection or a variable/execution callback even when source occurrences have complete edit geometry. A naming-only rename inventory cannot prove behaviour preservation.

## Question

When can a conditional source procedure formal be alpha-renamed while preserving the activation, incoming argument object and return behaviour?

## Conclusion

The sealed receipt requires a completed closed source containing exactly one surviving procedure declaration, one required direct scalar formal, and a sole independently selected builtin ReturnResult whose operand is the same whole-object formal read. Exact original frame, formal topology, full source/configuration, unchanged handler allocation and independent quiet variable/command effects must agree. The rename retains argv, activation and release order and performs no new store, conversion or frame elimination. Defaults, rest/link/array formals, concatenation, extra commands, observers and unavailable ownership withdraw the proof. Original native byte bodies retain a materialised parser image when unchanged document coordinates are unavailable; declared-source geometry remains independent. Source edit geometry remains independent.

## Scope

Rust source-conditional equivalence for the explicit C Tcl/Jim naming models. Arbitrary external procedure introspection and later external observer registration are outside this closed-source contract. This supplies neither a reached call, current variable value, Normal completion, physical native activation nor a general minification permission.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs](../../../../rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs), `OriginalScalarBodyAlphaRename`: Seals exact declaration/frame/topology and independently quiet same-object return semantics.
- [rust/tcl-compiler/src/command_binding/declaration_preview.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_preview.rs), `original_declared_procedure_at`: Selects a unanimous genuine conditional declaration recipe without entering a call.
- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs), `original_operands_preserve_lookup`: Closes actual whole-word read effects; only the incoming fresh-formal observer residual is excluded.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `original_scalar_body_alpha_rename`: Requires exact original declaration membership and complete source rename coverage before the separate semantic issuer.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `retained_script_operand`: Retain actual native byte-valued body operands without requiring analytical Unicode or inventing an unchanged document mapping.
- [rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs](../../../../rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs), `command_binding::scalar_body_alpha::tests::original_scalar_body_alpha_preserves_frame_argv_and_whole_object_return` (linked): Retains the actual original local frame/formal and same-object return shape across six model profiles; rejects stale source/channel/configuration.
- [rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs](../../../../rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs), `command_binding::scalar_body_alpha::tests::original_scalar_body_alpha_declines_observers_introspection_and_changed_topology` (linked): Declines introspection, extra source/body commands, non-whole-object reads, changed arity/default/rest/link roles and invalid proposed scalar names.
- [rust/tcl-compiler/src/command_binding/executed_script_source.rs](../../../../rust/tcl-compiler/src/command_binding/executed_script_source.rs), `command_binding::executed_script_source::tests::original_counted_body_operand_keeps_native_image_without_document_coordinates` (linked): An actual C8 native body value with surrogate units retains its exact NativeValue parser image and a distinct materialised origin; the original Document image is preserved independently and no contiguous source coordinates or Unicode projection are issued.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. This record contains no interpreter observation or executed Rust result.
