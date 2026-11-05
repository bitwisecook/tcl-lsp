# naming.procedure.original-wrong-arity-body-withdrawal

Kind: `native-observation`

## Problem statement

A valid body can have an observable store that must not run when argv validation fails. An error message alone cannot establish whether that earlier effect occurred.

## Question

Does the exact q body write side before a missing required argument error?

## Conclusion

Every recorded provider reports guest Error with exact usage q x and leaves side at BEFORE. The attempted body store to BODY was not reached in this control; this does not prove arbitrary body effects or an untested native frame recipe.

## Scope

Exact ASCII shell program with catch/list/puts observer, two defined bodies and a separate side-effect sentinel. Original manifest records selected shell-path artifact SHA, coarse engine8.4/8.5/8.6/9.0/9.1/Jim labels, process status and inline output, but no launched full patchlevel/library/header/configuration identity. Original shell invocation argv is absent, so stdin versus file CLI entry cannot be resolved from the source filename. Seven output lines/provider are byte-identical to retained inline streams; no BIG-IP attempt. Original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl8.5

Status: `observed`. Version: 8.5 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl8.6

Status: `observed`. Version: 8.6 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl9.0

Status: `observed`. Version: 9.0 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl9.1

Status: `observed`. Version: 9.1 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
valid_wrongarity 1 {wrong # args: should be "q x"} BEFORE
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: Jim Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::validates_arguments_before_body`: Owns the release-selected activation boundary separately from definition capture.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::procedure_definition_and_activation_match_42_native_observations` (linked): Compares exact42 original shell definition/activation/side-effect lines; full runtime build and shell input-channel metadata remain separate absent fields.

## Evidence

- [rust/tcl-vm/tests/data/native_procedure_activation/probe.tcl](../../../../rust/tcl-vm/tests/data/native_procedure_activation/probe.tcl)
- [rust/tcl-vm/tests/data/native_procedure_activation/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_activation/manifest.json)
- [rust/tcl-vm/tests/data/native_procedure_activation/8.4.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.4.txt)
- [rust/tcl-vm/tests/data/native_procedure_activation/8.5.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.5.txt)
- [rust/tcl-vm/tests/data/native_procedure_activation/8.6.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.6.txt)
- [rust/tcl-vm/tests/data/native_procedure_activation/9.0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/9.0.txt)
- [rust/tcl-vm/tests/data/native_procedure_activation/9.1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/9.1.txt)
- [rust/tcl-vm/tests/data/native_procedure_activation/Jim.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/Jim.txt)
