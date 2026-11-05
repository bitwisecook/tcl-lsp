# naming.namespace-compiler.variable-value-residual

Kind: `native-observation`

## Problem statement

A variable compiler prepares an early v/value before a later dynamic or unmatched-closing tail declines. Dropping the failed attempt can erase its literal order and recorded local while generic preparation revisits the same source.

## Question

What locals and original literal type/order remain for variable v [list VALUE] followed by dynamic ${ns}v or a) names?

## Conclusion

C8.4 retains only formals and generic literals. C8.5 retains local v and an early VALUE literal before the generic command. C8.6 and later retain local v plus two distinct table entries with list primary and VALUE bytes, followed by the separately prepared late operands. No pointer identity or allocation count is printed by this observer.

## Scope

Five fixed original ASCII procedure bodies, called before observing private compile hook, opcode counts, counted locals and actual literal table. Literal primary is sampled before the string getter. The complete type/byte/order rows are retained; no pointer/refcount/cache lifetime or guest completion is printed. The bodies differ from the separate20-body binding matrix and are not labelled duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.4 8.4.20 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
0	0	0	0	6e73,76616c	4	cmdName:7661726961626c65,parsedVarName:76,none:56414c5545,none:4c415445
1	0	0	0	6e73,76616c	5	cmdName:7661726961626c65,parsedVarName:76,none:56414c5545,parsedVarName:6129,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.5

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.5 8.5.19 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
0	1	0	0	6e73,76616c,76	4	parsedVarName:76,none:56414c5545,cmdName:7661726961626c65,none:4c415445
1	1	0	0	6e73,76616c,76	5	parsedVarName:76,none:56414c5545,cmdName:7661726961626c65,parsedVarName:6129,none:4c415445
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
0	1	0	0	6e73,76616c,76	5	parsedVarName:76,list:56414c5545,cmdName:7661726961626c65,list:56414c5545,list:4c415445
1	1	0	0	6e73,76616c,76	6	parsedVarName:76,list:56414c5545,cmdName:7661726961626c65,list:56414c5545,parsedVarName:6129,list:4c415445
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
