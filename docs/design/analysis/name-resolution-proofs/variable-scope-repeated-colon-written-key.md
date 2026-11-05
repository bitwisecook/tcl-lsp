# naming.variable.scope-repeated-colon-written-key

Kind: `native-observation`

## Problem statement

Writing a:::raw and reading it back succeeds, but testing a::raw can select a different simple key in a dialect with different separator parsing. Source path advice must preserve the selected home/name rule.

## Question

Does the actual a:::raw assignment also make a::raw exist?

## Conclusion

All C captures print written:/ONE and short:1; Jim prints written:/ONE and short:0. The written key and shorter receiver are equivalent for these C controls and distinct for Jim. This finite source program does not measure original binary name objects or a private variable-table allocation.

## Scope

One exact ASCII raw-qualified-key.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl8.4 tcl8.4; tcl8.5 tcl8.5; tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Actual selected output:

```text
written:/ONE
short:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_scope_names_keep_six_engine_colon_and_home_rules` (linked): Checks exact written a:::raw versus a::raw presence without manufacturing an original native receiver.

## Evidence

- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.1.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.jim.txt)
