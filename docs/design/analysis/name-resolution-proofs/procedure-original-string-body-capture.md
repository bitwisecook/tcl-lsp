# naming.procedure.original-string-body-capture

Kind: `native-observation`

## Problem statement

A procedure can retain a malformed or counted-zero body without executing it. Shared body ownership may force a different stored object while preserving the original input header, so successful proc definition does not prove body entry.

## Question

Does definition retain ordinary, malformed and counted-zero String bodies under one versus two external original references?

## Conclusion

Every tested C release retains the same original body with one external reference and raises refs to2; with two references it stores a different untyped String body at refs1 while the original staysrefs2. Jim retains the same original object under both axes, raising refs to2 or3. All measured definitions succeed, including malformed and counted-zero bodies. No invocation occurs, so malformed-source error timing or counted-zero execution is untested here.

## Scope

Exact five original body constructors and external refs1/2 axes enter C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector proc. Initial/input post-definition fields are captured before private stored-body lookup; lookup itself performs no value conversion. Stored body fields are then sampled without a string observer. Ten full C TSV rows and eight Jim rows/source/build/status/stream hashes are retained. Counted body bytes are original native input, not Document UTF or entered body. No BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	0	2	none	1	none	1	none	1	1
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	0	2	none	1	none	1	none	1	1
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	0	2	none	1	none	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	0	2	none	1	none	1	none	1	1
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	0	2	none	1	none	1	none	1	1
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	0	2	none	1	none	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	0	2	none	1	none	1	none	1	1
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	0	2	none	1	none	1	none	1	1
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	0	2	none	1	none	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	0	2	none	1	none	1	none	1	1
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	0	2	none	1	none	1	none	1	1
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	0	2	none	1	none	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	0	2	none	1	none	1	none	1	1
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	0	2	none	1	none	1	none	1	1
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	0	2	none	1	none	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: headers=[{'path': '/tmp/2286-oracles/jimtcl/jim.h'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
0	1	0	1	2	none	1	none	1	none	1	2
0	2	0	1	3	none	1	none	1	none	1	3
3	1	0	1	2	none	1	none	1	none	1	2
3	2	0	1	3	none	1	none	1	none	1	3
4	1	0	1	2	none	1	none	1	none	1	2
4	2	0	1	3	none	1	none	1	none	1	3
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: Jim Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::copies_shared_definition_body`: Selects original shared-body capture independently from entered body compilation/activation.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::procedure_body_capture_matches_58_native_definition_windows` (linked): Compares all58 original definition header windows, keeping declared external refs and stored-body identity separate from body execution.

## Evidence

- [rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv)
- [rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv)
