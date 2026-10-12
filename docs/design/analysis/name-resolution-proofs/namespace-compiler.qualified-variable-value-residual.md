# naming.namespace-compiler.qualified-variable-value-residual

Kind: `native-observation`

## Problem statement

A dynamic qualified variable name and constant list value can be prepared before a later non-compilable variable tail. Its actual literal table cannot be derived from displayed command arguments alone.

## Question

What locals and list/value literal order remain for variable [list ::N]::v [list VALUE] ${ns}v [list LATE]?

## Conclusion

C8.4/8.5 retain only formal locals and six generic table entries. C8.6 and later retain local v and eight table entries: early ::N/VALUE list entries, command variable, independently repeated ::N/VALUE list entries, v text and LATE list. Equal literal bytes do not prove equal native object identity.

## Scope

Five fixed original ASCII procedure bodies, called before observing private compile hook, opcode counts, counted locals and actual literal table. Literal primary is sampled before the string getter. The complete type/byte/order rows are retained; no pointer/refcount/cache lifetime or guest completion is printed. The bodies differ from the separate20-body binding matrix and are not labelled duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association). Build: Observer SHA256 7acc2013cffc8ef9c711919c86289d0f7d185ed1c862acba45907053e04ef667; archive SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; native header SHA256 f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	0	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association). Build: Observer SHA256 72fa34cc2d04bbb899dab037d284d162bc97c7dc54e3a964c98ee12a2366697e; archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; native header SHA256 72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Observer SHA256 e30fe08ee5288755107a47ae6dc5396ceebbcc47a2e757bbc92b28246fc2ccc5; archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; native header SHA256 e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Observer SHA256 ec492835c26665c4b7e741906d8364ef134305e139c78460810907c22575eb98; archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; native header SHA256 f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Observer SHA256 4a1d2999d0343b413e835dae3a100d0f1b76d03208b8e9740af14945093900ae; archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; native header SHA256 fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

No original namespace compiler literal-table capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No original namespace compiler literal-table capture for this provider is attached.

## Exact evidence

- `input` (input): [runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c). SHA-256 `5fdf3a2e8474bfd6f10c4a36e31bdb316d1eb1d2b400be46d3c5f47e87b0faa9`. Exact original bodies and reached literal primary-before-getter observer.
- `receipt` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. Complete independent original artifact capture identities.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv). SHA-256 `0377d11d558afe35573be06e8dca257640055b3abfdab932f8acc2384ea88206`. Complete original literal/local table with selected cases [4].
- `provider-tcl8.4` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. JSON pointer `/runs/0`. Original configured header/archive/observer/status association.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv). SHA-256 `a903f185ca0200995e175ee25ef878e265f2f1d2fa4b39575c9597b49c704094`. Complete original literal/local table with selected cases [4].
- `provider-tcl8.5` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. JSON pointer `/runs/1`. Original configured header/archive/observer/status association.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv). SHA-256 `e4bc4adf5e4661dc38c2119ec956472b2b8b046d81a4a2e82efde50aeee398c7`. Complete original literal/local table with selected cases [4].
- `provider-tcl8.6` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. JSON pointer `/runs/2`. Original configured header/archive/observer/status association.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv). SHA-256 `e4bc4adf5e4661dc38c2119ec956472b2b8b046d81a4a2e82efde50aeee398c7`. Complete original literal/local table with selected cases [4].
- `provider-tcl9.0` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. JSON pointer `/runs/3`. Original configured header/archive/observer/status association.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv). SHA-256 `e4bc4adf5e4661dc38c2119ec956472b2b8b046d81a4a2e82efde50aeee398c7`. Complete original literal/local table with selected cases [4].
- `provider-tcl9.1` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). SHA-256 `57f1bd14ef228a7dca67f92158e83a49e4a78ee0dbe7d9a832f428cdefe878c5`. JSON pointer `/runs/4`. Original configured header/archive/observer/status association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_declined_attempt_retains_native_pool_order_and_private_headers` (linked): Compares retained literal table/order and locals; any extra object-identity assertion remains a Rust contract beyond printed native fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact unchanged source and complete stdout/table hashes are retained with original header/archive/observer/status associations. Reconfirmation requires the same configured private headers and original preparation/call order. Full launched patchlevel/compiler/configure and separate stderr hash are unrecorded; no new run is asserted.
