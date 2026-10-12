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

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h', 'sha256': '824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h', 'sha256': 'f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a'}]; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=1427d353cc2172c8dcbd490481a779058c9566167fbf2bdd3e6558f013ae84c4; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h', 'sha256': 'c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h', 'sha256': '72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa'}]; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=05ae6a9be592d979db80a1c0da7149644fd821cf2ece5d45e51e963f099a0b73; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h', 'sha256': 'aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245'}, {'path': '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h', 'sha256': 'e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e'}]; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=5b9e59cbd05669e5249f559b79306e910f75f6beec8b638f1db3cc3d2a4fab97; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h', 'sha256': 'eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h', 'sha256': 'f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053'}]; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=9db50cd7438fa42f17e445d0c8a71cddc4934f8c2d109fef6b49740bebb1a8a9; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: headers=[{'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h', 'sha256': '30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950'}, {'path': '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h', 'sha256': 'fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc'}]; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=3d6b4059daa02daece1687aed6f19ea4c9cb6e9cacae5be4fc43609ab5169f7a; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: headers=[{'path': '/tmp/2286-oracles/jimtcl/jim.h', 'sha256': 'd9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d'}]; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; binary_sha256=8aee83883d852d01b2cddadc9963cb473cd25c4b0ec882f397d5c55ad3d540ac; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact selected rows in the manifest12-column order:

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

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/probe.c). SHA-256 `9e3ac596288aa6ca93319ea7a421a960e99c0100282fa076d2ebf1d59cb7825c`. Exact original body constructors and owner-aware before/stored-body observer.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/manifest.json). SHA-256 `198367bcff3aab8515a41ce005318e439e433ca616cea540f5f1e6cafd62b1ff`. Six original compile/run/build/output associations; Jim Bytearray skip remains explicit.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.4.20.tsv). SHA-256 `e18091015fcd43d95d0f71e829120fce2c423f7489a985c2214ef5e9455a4ddb`. Full original12-column header stream; selected kind IDs [0, 3, 4].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.5.19.tsv). SHA-256 `e18091015fcd43d95d0f71e829120fce2c423f7489a985c2214ef5e9455a4ddb`. Full original12-column header stream; selected kind IDs [0, 3, 4].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/8.6.18.tsv). SHA-256 `e18091015fcd43d95d0f71e829120fce2c423f7489a985c2214ef5e9455a4ddb`. Full original12-column header stream; selected kind IDs [0, 3, 4].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.0.4.tsv). SHA-256 `e18091015fcd43d95d0f71e829120fce2c423f7489a985c2214ef5e9455a4ddb`. Full original12-column header stream; selected kind IDs [0, 3, 4].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/9.1.0.tsv). SHA-256 `e18091015fcd43d95d0f71e829120fce2c423f7489a985c2214ef5e9455a4ddb`. Full original12-column header stream; selected kind IDs [0, 3, 4].
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_body_capture/Jim.tsv). SHA-256 `49ea0f02deb9fb79c85010e718bad6badb9ee787db300b625972fbdcca83c0d1`. Full original12-column header stream; selected kind IDs [0, 3, 4].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::copies_shared_definition_body`: Selects original shared-body capture independently from entered body compilation/activation.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::procedure_body_capture_matches_58_native_definition_windows` (linked): Compares all58 original definition header windows, keeping declared external refs and stored-body identity separate from body execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
