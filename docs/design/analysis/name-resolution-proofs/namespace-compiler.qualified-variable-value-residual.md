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

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.4 8.4.20 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	0	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.5

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.5 8.5.19 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c	6	cmdName:7661726961626c65,none:3a3a4e,none:3a3a76,none:56414c5545,none:76,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
4	1	0	0	6e73,76616c,76	8	list:3a3a4e,none:3a3a76,list:56414c5545,cmdName:7661726961626c65,list:3a3a4e,list:56414c5545,none:76,list:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### jim, bigip

Status: `not-tested`.

No original namespace compiler literal-table capture for this provider is attached.

## Consumer bindings

- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_declined_attempt_retains_native_pool_order_and_private_headers` (linked): Compares retained literal table/order and locals; any extra object-identity assertion remains a Rust contract beyond printed native fields.

## Evidence

- [runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/probe.c)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/manifest.json)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.4.20.tsv)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.5.19.tsv)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/8.6.18.tsv)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.0.4.tsv)
- [runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv](../../../../runtime/rust/tests/data/native_namespace_binding_artifacts/9.1.0.tsv)
