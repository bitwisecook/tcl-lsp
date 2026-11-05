# naming.command-head.dynamic-expansion-after-replacement

Kind: `native-observation`

## Problem statement

A dynamic list prefix is evaluated before an operand that replaces set. A consumer that donates literal-head compiler identity to that produced prefix could execute a store rather than reaching the changed command lookup.

## Question

What does a dynamic {set x} command prefix do after its operand renames set, and what original compiled instruction is printed?

## Conclusion

C8.5–9.1 catch too many nested evaluations (infinite loop?) for the dynamic mutation control, and print invokeExpanded after the operand calls in its compiled-body listing. C8.4 rejects expansion syntax before that dynamic route. Jim catches invalid command name set. Process0 only means the outer catches and reporter completed; it does not convert these guest errors into successful stores or prove instruction entry.

## Scope

Selected dynamic_mutation rows from one ASCII five-control script per provider. Each control uses catch and reports guest code/result; the original six shell processes exit0 with empty stderr. C disassembly, where available, includes process-local addresses and actual stored body instructions, not an executed instruction trace. Source/executable checksums and original60-second budget are retained; shell channel and full patchlevel/configure/library closure are absent. No opaque source, physical native name cache, TclOO or appliance capture.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
dynamic_mutation 1 {extra characters after close-brace}
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
dynamic_mutation 1 {too many nested evaluations (infinite loop?)}
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes. Its stored dynamic-body listing includes expandStart, expandStkTop and invokeExpanded after operand calls.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
dynamic_mutation 1 {invalid command name "set"}
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
