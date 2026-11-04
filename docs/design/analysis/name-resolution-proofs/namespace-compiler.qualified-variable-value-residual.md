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

Status: `observed`. Version: 8.4.20 (capture association). Build: Observer; archive; native header; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	0	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association). Build: Observer; archive; native header; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Observer; archive; native header; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Observer; archive; native header; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Observer; archive; native header; compile/process0, stderr byte count0. Full launched patchlevel/compiler/configure unqueried.. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table.. Dialect: C Tcl.

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

## Historical references

- `input` (input): [runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c). Exact original bodies and reached literal primary-before-getter observer.
- `receipt` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). Complete independent original artifact capture identities.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv). Complete original literal/local table with selected cases [4].
- `provider-tcl8.4` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). JSON pointer `/runs/0`. Original configured header/archive/observer/status association.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv). Complete original literal/local table with selected cases [4].
- `provider-tcl8.5` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). JSON pointer `/runs/1`. Original configured header/archive/observer/status association.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv). Complete original literal/local table with selected cases [4].
- `provider-tcl8.6` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). JSON pointer `/runs/2`. Original configured header/archive/observer/status association.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv). Complete original literal/local table with selected cases [4].
- `provider-tcl9.0` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). JSON pointer `/runs/3`. Original configured header/archive/observer/status association.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv). Complete original literal/local table with selected cases [4].
- `provider-tcl9.1` (provider): [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json). JSON pointer `/runs/4`. Original configured header/archive/observer/status association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_declined_attempt_retains_native_pool_order_and_private_headers` (linked): Compares retained literal table/order and locals; any extra object-identity assertion remains a Rust contract beyond printed native fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact unchanged source and complete stdout/table hashes are retained with original header/archive/observer/status associations. Reconfirmation requires the same configured private headers and original preparation/call order. Full launched patchlevel/compiler/configure and separate stderr hash are unrecorded; no new run is asserted.
