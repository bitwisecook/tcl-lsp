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

Status: `unsupported`. Version: Capture association 8.4; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Exact selected reported lines:

```text
dynamic_empty 1 {extra characters after close-brace}
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl8.5

Status: `observed`. Version: Capture association 8.5; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Exact selected reported lines:

```text
dynamic_empty 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl8.6

Status: `observed`. Version: Capture association 8.6; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Exact selected reported lines:

```text
dynamic_empty 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl9.0

Status: `observed`. Version: Capture association 9.0; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Exact selected reported lines:

```text
dynamic_empty 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### tcl9.1

Status: `observed`. Version: Capture association 9.1; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Exact selected reported lines:

```text
dynamic_empty 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### jim

Status: `observed`. Version: Capture association jim; launched full patchlevel was not queried in this receipt. Jim revision and UTF build configuration are unrecorded.. Build: Recorded selected shell digest d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: Jim Tcl.

Exact selected reported lines:

```text
dynamic_empty 0 A
```

Outer process0 and empty stderr do not erase the separate caught guest completion codes.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Exact evidence

- `source` (input): [rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl](../../../../rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl). SHA-256 `806e7a653368e9cdd8a515d4dcd9ed10151af0681c0ebaecd3e93060c82a7de4`. Exact five-control script with independent literal, inline, dynamic and empty-prefix forms.
- `receipt` (provider): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. Six original source/executable/status/full-output associations.
- `matrix` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/observations.tsv](../../../../rust/tcl-compiler/tests/data/native_expanded_head/observations.tsv). SHA-256 `a5c10aedd96f72810f86b372a280b30c24a8ceca3bfc5655f34cd341383285eb`. Derived six-row marker matrix checked byte-for-byte against the embedded complete outputs; dynamic instruction/literal store flags are projections, not new observations.
- `row-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/0`. Complete original stdout, including caught guest errors and actual disassembly when emitted.
- `row-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/1`. Complete original stdout, including caught guest errors and actual disassembly when emitted.
- `row-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/2`. Complete original stdout, including caught guest errors and actual disassembly when emitted.
- `row-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/3`. Complete original stdout, including caught guest errors and actual disassembly when emitted.
- `row-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/4`. Complete original stdout, including caught guest errors and actual disassembly when emitted.
- `row-jim` (observation): [rust/tcl-compiler/tests/data/native_expanded_head/manifest.json](../../../../rust/tcl-compiler/tests/data/native_expanded_head/manifest.json). SHA-256 `f0e02e608610b666d482cce11afad98f203a90985d3a42b22154ebbfed10ddf8`. JSON pointer `/engines/5`. Complete original stdout, including caught guest errors and actual disassembly when emitted.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/native_byte_compilation.rs](../../../../rust/tcl-compiler/src/native_byte_compilation.rs), `native_byte_command_plan`: Keeps authentic parser-projected literal heads separate from produced dynamic prefixes and their generic invocation timing.
- [rust/tcl-compiler/src/native_byte_compilation.rs](../../../../rust/tcl-compiler/src/native_byte_compilation.rs), `native_byte_compilation::tests::original_dynamic_expanded_head_keeps_generic_timing_and_literal_cpp_obligations` (linked): Checks retained marker projections and original dynamic/empty-head Generic compilation selection with independent entry/point guards; those source-owner assertions do not establish a native entered store or execution result.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact script and original captured streams remain attached. This receipt does not retain the original invocation argument vector or whether the shell consumed a file or stdin. No exact original-channel replayer is claimed. A new run must record its selected shell/version/build, channel, exact input checksum, process status and separate streams; native disassembly addresses must not be mistaken for stable semantic coordinates. Retained executable digests do not reconstruct absent executables or establish the current provider. No new native run or Rust execution is claimed.
