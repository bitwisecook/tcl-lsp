# naming.command-head.empty-dynamic-expansion-exposes-written-head

Kind: `native-observation`

## Problem statement

An empty produced prefix can remove the entire expanded head word and expose the next written word as the command. Rejecting it as an absent command or assuming a nonempty prefix would change the actual invocation.

## Question

Does an empty dynamic head expansion expose the following set x A words as the command?

## Conclusion

C8.5–9.1 and Jim return A when the dynamic expanded prefix is empty and the following words are set x A. C8.4 catches extra characters after close-brace. This proves only the selected finite empty-prefix source behaviour; source-only prefix projection and independent native compilation/frame ownership remain separate.

## Scope

Selected dynamic_empty rows from one ASCII five-control script per provider. Each control uses catch and reports guest code/result; the original six shell processes exit0 with empty stderr. C disassembly, where available, includes process-local addresses and actual stored body instructions, not an executed instruction trace. Source/executable checksums and original60-second budget are retained; shell channel and full patchlevel/configure/library closure are absent. No opaque source, physical native name cache, TclOO or appliance capture.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
dynamic_empty 1 {extra characters after close-brace}
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Exact selected reported lines:

```text
dynamic_empty 0 A
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
