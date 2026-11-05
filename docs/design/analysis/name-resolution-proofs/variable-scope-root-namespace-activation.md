# naming.variable.scope-root-namespace-activation

Kind: `native-observation`

## Problem statement

A namespace eval :: body writes ephemeral and can either publish root storage or keep a transient activation-local value. An inside read alone cannot decide whether the cell survives the namespace body.

## Question

Does ephemeral written inside namespace eval :: remain visible as ::ephemeral after the body returns?

## Conclusion

Every capture prints in:/LOCAL; all five C releases print out:1 and Jim prints out:0. The C root namespace body publishes the queried root cell, while Jim retains only the inside activation value in this exact control. It does not establish arbitrary frame retirement, variable lifetime or a native body/storage receipt.

## Scope

One exact ASCII root-activation.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl8.4 tcl8.4; tcl8.5 tcl8.5; tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Actual selected output:

```text
in:/LOCAL
out:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Actual selected output:

```text
in:/LOCAL
out:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_root_namespace_activation_does_not_export_jim_locals` (linked): Checks inside/outside presence and exported path constants for the exact retained source; no entered native namespace activation is asserted.

## Evidence

- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.tcl9.1.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-activation.jim.txt)
