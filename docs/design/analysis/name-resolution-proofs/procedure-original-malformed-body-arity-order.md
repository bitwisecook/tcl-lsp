# naming.procedure.original-malformed-body-arity-order

Kind: `native-observation`

## Problem statement

A proc definition can store malformed script. On invocation, argument validation and body parse failure may occur in a release-specific order; a well-formed error body adds an independent entered-error control.

## Question

Do missing argv or malformed-source diagnostics occur first for the retained malformed body, and what happens with a complete error body?

## Conclusion

All six definitions complete normally; C returns empty definition result and Jim returns p. C8.4 reports missing-quote even for the malformed body called without x; later C and Jim report wrong arity first. With x supplied all report malformed-source failure, using their exact quoted presenter. The complete error BODY procedure reports wrong arity without x and BODY with x on all providers. These are returned script diagnostics, not physical body cache or entered native callback proof.

## Scope

Exact ASCII shell program with catch/list/puts observer, two defined bodies and a separate side-effect sentinel. Original manifest records selected shell-path artifact SHA, coarse engine8.4/8.5/8.6/9.0/9.1/Jim labels, process status and inline output, but no launched full patchlevel/library/header/configuration identity. Original shell invocation argv is absent, so stdin versus file CLI entry cannot be resolved from the source filename. Seven output lines/provider are byte-identical to retained inline streams; no BIG-IP attempt. Original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 {}
p 1 {missing "}
{p OK} 1 {missing "}
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl8.5

Status: `observed`. Version: 8.5 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 {missing "}
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl8.6

Status: `observed`. Version: 8.6 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 {missing "}
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl9.0

Status: `observed`. Version: 9.0 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 {missing "}
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### tcl9.1

Status: `observed`. Version: 9.1 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 {missing "}
define 0 {}
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
```
The shell process exits0 while catch retains each guest Error separately.. Dialect: C Tcl.

Native shell source execution; original invocation argv/input-channel (stdin or file) is unrecorded.

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

```text
define 0 p
p 1 {wrong # args: should be "p x"}
{p OK} 1 {missing quote}
define 0 p
p 1 {wrong # args: should be "p x"}
{p OK} 1 BODY
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
