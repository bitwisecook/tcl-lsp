# naming.variable.scope-root-qualified-declaration

Kind: `native-observation`

## Problem statement

At root, variable ::a::x can target the qualified namespace cell or a local tail x depending on the handler. Treating a rooted declaration as an ordinary set receiver would mispublish the path constant.

## Question

Which of root x and ::a::x exists after variable ::a::x /TOP at root?

## Conclusion

C8.4–9.1 print flat:0 and target:1; Jim prints flat:1 and target:0. The exact root-qualified variable declaration targets the qualified cell in these C captures and the local tail in Jim. No argument/formal installation, linked array or physical compiler-local claim follows.

## Scope

One exact ASCII root-qualified-variable.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl8.4 tcl8.4; tcl8.5 tcl8.5; tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Actual selected output:

```text
flat:1
target:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_jim_qualified_variable_at_root_writes_only_local_tail` (linked): Checks /TOP lookup at x versus ::a::x under the independently selected constant-path source projection.

## Evidence

- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.1.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.jim.txt)
