# naming.variable.scope-namespace-local-and-linked-publication

Kind: `native-observation`

## Problem statement

An unqualified namespace-body set and an explicit variable link can affect different homes. A static single-assignment path consumer must distinguish a second global write from a separate local write and an explicitly published linked cell.

## Question

Where do the unlinked dir assignment and explicit linked variable publish during namespace eval N?

## Conclusion

Every capture prints local:/LOCAL, link:/NEW and published:/NEW. C8.4–8.6 subsequently print global:/LOCAL and unpublished:0; C9 prints global:/GLOBAL and unpublished:1; Jim prints global:/GLOBAL and unpublished:0. Thus the unlinked write reaches root in C8, a published N cell in C9 and an unexported activation local in Jim, while the explicit link publishes N::linked. The Rust constant-path consumer deliberately withdraws the two C8 reads under repeated-assignment abstraction instead of choosing one runtime value.

## Scope

One exact ASCII local-and-linked.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl8.4 tcl8.4; tcl8.5 tcl8.5; tcl8.6 tcl8.6.

Actual selected output:

```text
local:/LOCAL
link:/NEW
global:/LOCAL
unpublished:0
published:/NEW
```

Process status0 and empty stderr are recorded for this exact program.

### tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Actual selected output:

```text
local:/LOCAL
link:/NEW
global:/GLOBAL
unpublished:1
published:/NEW
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded.

Actual selected output:

```text
local:/LOCAL
link:/NEW
global:/GLOBAL
unpublished:0
published:/NEW
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::namespace_read_sites_retain_c8_c9_and_jim_receiver_channels` (linked): Checks C8 repeated-write withdrawal versus separately retained C9/Jim inside/outside source constants. This conservative projection is not a Rust reproduction of every runtime value.

## Evidence

- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.tcl9.1.txt)
- [rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/local-and-linked.jim.txt)
