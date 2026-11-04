# naming.procedure.original-list-bytearray-body-capture

Kind: `native-observation`

## Problem statement

A List or Bytearray body can have no resident string. Copying a shared body may materialize the original and store different String state while an unshared body retains its original primary.

## Question

What original/stored header identity and string state follows List/Bytearray proc body capture at one/two external references?

## Conclusion

All C releases retain the original List/Bytearray primary with absent bytes when one external reference exists; stored body is that same object atrefs2. With two references, C materializes resident bytes on the original while retaining its List/Bytearray primary, and stores a different untyped resident String atrefs1. Jim retains the same List with absent bytes under both axes, atrefs2 or3. Its Bytearray constructor branch is compiled out and not tested; no corresponding guest rejection is claimed.

## Scope

Exact five original body constructors and external refs1/2 axes enter C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector proc. Initial/input post-definition fields are captured before private stored-body lookup; lookup itself performs no value conversion. Stored body fields are then sampled without a string observer. Ten full C TSV rows and eight Jim rows/source/build/status/stream hashes are retained. Counted body bytes are original native input, not Document UTF or entered body. No BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h', 'sha256': '824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h', 'sha256': 'f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	0	2	list	0	list	1	none	1	1
2	1	0	1	2	bytearray	0	bytearray	0	bytearray	0	2
2	2	0	0	2	bytearray	0	bytearray	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h', 'sha256': 'c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h', 'sha256': '72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	0	2	list	0	list	1	none	1	1
2	1	0	1	2	bytearray	0	bytearray	0	bytearray	0	2
2	2	0	0	2	bytearray	0	bytearray	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h', 'sha256': 'aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h', 'sha256': 'e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	0	2	list	0	list	1	none	1	1
2	1	0	1	2	bytearray	0	bytearray	0	bytearray	0	2
2	2	0	0	2	bytearray	0	bytearray	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h', 'sha256': 'eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h', 'sha256': 'f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	0	2	list	0	list	1	none	1	1
2	1	0	1	2	bytearray	0	bytearray	0	bytearray	0	2
2	2	0	0	2	bytearray	0	bytearray	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h', 'sha256': '30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h', 'sha256': 'fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	0	2	list	0	list	1	none	1	1
2	1	0	1	2	bytearray	0	bytearray	0	bytearray	0	2
2	2	0	0	2	bytearray	0	bytearray	1	none	1	1
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: C Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: headers=[{'path': '/tmp/2286-oracles/jimtcl/jim.h', 'sha256': 'd9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d'}]; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

```tsv
1	1	0	1	2	list	0	list	0	list	0	2
1	2	0	1	3	list	0	list	0	list	0	3
```
External original references are declared by the probe; missing Jim kind2 is a compile skip, not a reached error.. Dialect: Jim Tcl.

Public original object-vector proc definition, native body constructors and private actual stored-body owner lookup. No procedure invocation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c). Exact original body constructors and owner-aware before/stored-body observer.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json). Six original compile/run/build/output associations; Jim Bytearray skip remains explicit.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv). Full original12-column header stream; selected kind IDs [1, 2].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv). Full original12-column header stream; selected kind IDs [1, 2].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv). Full original12-column header stream; selected kind IDs [1, 2].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv). Full original12-column header stream; selected kind IDs [1, 2].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv). Full original12-column header stream; selected kind IDs [1, 2].
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv). Full original12-column header stream; selected kind IDs [1, 2].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::copies_shared_definition_body`: Selects original shared-body capture independently from entered body compilation/activation.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::procedure_body_capture_matches_58_native_definition_windows` (linked): Compares all58 original definition header windows, keeping declared external refs and stored-body identity separate from body execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
