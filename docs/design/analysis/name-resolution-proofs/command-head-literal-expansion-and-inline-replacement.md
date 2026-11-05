# naming.command-head.literal-expansion-and-inline-replacement

Kind: `native-observation`

## Problem statement

A literal list {set x} expanded into the command head and a written set x can select a compiled store, while a changed command table could otherwise force a later command lookup. Reusing the spelling alone would hide release-specific timing.

## Question

How do literal head expansion and written set behave when an operand renames set during evaluation?

## Conclusion

C8.5–9.1 return A for both the literal expanded store and written inline store despite the operand rename, and the disassembly prints storeScalar instructions for the literal projected procedure. C8.4 rejects the two expansion controls with extra characters after close-brace while the written inline control returns A. Jim accepts the baseline literal expansion but both mutation controls fail with invalid command name set. The stored disassembly is compilation output, not an instruction-entry trace or arbitrary command-table guarantee.

## Scope

Selected literal, expanded_mutation, inline_mutation rows from one ASCII five-control script per provider. Each control uses catch and reports guest code/result; the original six shell processes exit0 with empty stderr. C disassembly, where available, includes process-local addresses and actual stored body instructions, not an executed instruction trace. Source/executable checksums and original60-second budget are retained; shell channel and full patchlevel/configure/library closure are absent. No opaque source, physical native name cache, TclOO or appliance capture.

## Provider answers

### tcl8.4

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
literal 1 {extra characters after close-brace}
expanded_mutation 1 {extra characters after close-brace}
inline_mutation 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
literal 0 A
expanded_mutation 0 A
inline_mutation 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes. Its literal procedure listing includes a storeScalar operation.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
literal 0 A
expanded_mutation 1 {invalid command name "set"}
inline_mutation 1 {invalid command name "set"}
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/native_byte_compilation.rs](../../../../rust/tcl-compiler/src/native_byte_compilation.rs), `native_byte_command_plan`: Keeps authentic parser-projected literal heads separate from produced dynamic prefixes and their generic invocation timing.
- [rust/tcl-compiler/src/native_byte_compilation.rs](../../../../rust/tcl-compiler/src/native_byte_compilation.rs), `native_byte_compilation::tests::original_dynamic_expanded_head_keeps_generic_timing_and_literal_cpp_obligations` (linked): Checks retained marker projections and original dynamic/empty-head Generic compilation selection with independent entry/point guards; those source-owner assertions do not establish a native entered store or execution result.

## Evidence

- [rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl](../../../../rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl)
- [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json)
- [rust/tcl-compiler/tests/data/native_expanded_head/observations.tsv](../../../../rust/tcl-compiler/tests/data/native_expanded_head/observations.tsv)
