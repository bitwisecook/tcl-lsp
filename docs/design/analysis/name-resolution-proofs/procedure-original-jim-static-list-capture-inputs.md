# naming.procedure.original-jim-static-list-capture-inputs

Kind: `implementation-contract`

## Problem statement

Jim callable statics distinguish copied values, captured raw cells and literal initialisers. Opaque callable or static names must retain counted identity without making an optional display label a prerequisite for independently installed callable storage.

## Question

Do original static-list bytes retain Jim capture roles and counted names through the shared native specifier owner, while callable capture joins the exact installed allocation independently of its reporting name?

## Conclusion

The byte adapter uses the selected Jim native list owner and the same static specifier kernel as the original-object facade and compatibility parser. It retains distinct literal, current-value-copy and raw-cell-capture roles with counted names and independent CString diagnostics. Callable storage requires the original publication key and unique same-site installed implementation allocation; an optional reporting procedure name supplies no additional authority. The byte syntax adapter itself grants no capture or source lineage.

## Scope

Rust implementation for the selected Jim084 static-list purpose, including opaque bytes, raw zero in counted list names, reference markers, duplicate slots and invalid single-field array elements. Other engines and unresolved execution points decline. The existing source capture path retains supported ASCII static declaration storage; complete byte static-cell transport remains an independent producer/cell-owner requirement. No physical callable, native object, static cell, captured value lifetime or successful opaque-static body execution is inferred from parsing.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No guest execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No guest execution of this Rust implementation question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `parse_static_variables_bytes`: Selected Jim native list values and shared static specifier roles, without source or cell authority.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `static_variable_value_specifier`: Sole one/two-field, marker and array-element static-name grammar.
- [rust/tcl-syntax/src/list.rs](../../../../rust/tcl-syntax/src/list.rs), `split_native_list_bytes`: Original counted list decoding under the selected native string protocol.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `prepare_source_callable_statics`: Unique same-site installed callable allocation join before independent cell capture.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::tests::original_static_list_bytes_keep_counted_names_and_capture_roles_separate` (linked): Opaque and counted-zero names retain exact identity, one-field references and two-field literals retain distinct roles, duplicate and array-element failures use the shared specifier owner, and unsupported engines decline.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_opaque_jim_callable_name_does_not_gate_ascii_static_capture` (linked): Two opaque callable names with valid ASCII literal statics retain distinct installed source declarations in a complete modeled world; duplicate static slots cannot close that world.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors are validation obligations without attached execution receipts. Native object/list provenance and source static-cell captures remain independent from the syntax adapter.
