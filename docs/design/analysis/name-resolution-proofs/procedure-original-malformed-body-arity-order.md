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

Status: `observed`. Version: 8.4 (original capture association; full launched patchlevel unqueried). Build: sha256=551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `observed`. Version: 8.5 (original capture association; full launched patchlevel unqueried). Build: sha256=7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `observed`. Version: 8.6 (original capture association; full launched patchlevel unqueried). Build: sha256=0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `observed`. Version: 9.0 (original capture association; full launched patchlevel unqueried). Build: sha256=cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `observed`. Version: 9.1 (original capture association; full launched patchlevel unqueried). Build: sha256=d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: sha256=d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected reached presenter lines:

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

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Exact evidence

- `program` (input): [rust/tcl-vm/tests/data/native_procedure_activation/probe.tcl](../../../../rust/tcl-vm/tests/data/native_procedure_activation/probe.tcl). SHA-256 `d8cb2de243beeabc1ed6b51a81fbfe1e899ae334486bfcd1e189e1c79bad596a`. Exact ASCII catch/presenter and malformed/valid side-effect bodies.
- `receipt` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_activation/manifest.json). SHA-256 `b1407bdb66ea1cd80b0df97ebd965e99dc542a7849c8e2d3477788488f14003b`. All six original selected-path SHA/process/stdout/stderr captures, with limited version and unknown stdin/file attribution.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/8.4.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.4.txt). SHA-256 `44ca52887bd738932993bd0cd3eee7eb6dbdd942ab3f334d7ee8d7ba9f261a14`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/8.5.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.5.txt). SHA-256 `6a818291e701d0df1d25812a58260e52e14799c9ebd4c8653dd0bef72355d715`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/8.6.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/8.6.txt). SHA-256 `6a818291e701d0df1d25812a58260e52e14799c9ebd4c8653dd0bef72355d715`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/9.0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/9.0.txt). SHA-256 `6a818291e701d0df1d25812a58260e52e14799c9ebd4c8653dd0bef72355d715`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/9.1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/9.1.txt). SHA-256 `6a818291e701d0df1d25812a58260e52e14799c9ebd4c8653dd0bef72355d715`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_procedure_activation/Jim.txt](../../../../rust/tcl-vm/tests/data/native_procedure_activation/Jim.txt). SHA-256 `787095c08f7fe66429b8725a15123b00b503fb3c894d5fec57b37a13990ace0d`. Exact seven-line original full output; selected one-based lines [1, 2, 3, 4, 5, 6].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::validates_arguments_before_body`: Owns the release-selected activation boundary separately from definition capture.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::procedure_definition_and_activation_match_42_native_observations` (linked): Compares exact42 original shell definition/activation/side-effect lines; full runtime build and shell input-channel metadata remain separate absent fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
