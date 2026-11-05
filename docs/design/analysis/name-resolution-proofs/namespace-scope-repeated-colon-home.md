# naming.namespace.scope-repeated-colon-home

Kind: `native-observation`

## Problem statement

A namespace is created as a:::b and its variable read through the same spelling. A consumer could collapse every separator identically across dialects and then incorrectly find the shorter a::b cell.

## Question

Does namespace a:::b publish dir at the same home as ::a::b::dir?

## Conclusion

All five C captures print home:/LIB and normal:1; Jim prints home:/LIB and normal:0. The exact repeated-colon namespace producer and rooted variable read therefore converge with the shorter spelling in these C controls but remain distinct in this Jim control. This does not prove arbitrary structured namespace context equality or native cache ownership.

## Scope

One exact ASCII colon-home.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl8.4 tcl8.4; tcl8.5 tcl8.5; tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Actual selected output:

```text
home:/LIB
normal:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Actual selected output:

```text
home:/LIB
normal:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_scope_names_keep_six_engine_colon_and_home_rules` (linked): Checks the two written receiver lookups for the exact native source using the dialect-selected path constant projection.

## Evidence

- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.tcl9.1.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/colon-home.jim.txt)
