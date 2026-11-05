# naming.namespace-compiler.global-prefix-residual

Kind: `native-observation`

## Problem statement

A global compiler can prepare an early :: prefix/local tail before declining at a later operand. A single generic reconstructed word would conceal that preparation and its literal order.

## Question

What literal and local tables remain for global [list ::N]::v followed by ${ns}v or a)?

## Conclusion

All captures emit zero binding opcodes. C8.5 retains an early :: literal without local v. C8.6 and later retain local v, an early :: literal and two list entries with ::N bytes separated by command/literal preparation; C8.4 retains only generic literals and formal locals.

## Scope

Five fixed original ASCII procedure bodies, called before observing private compile hook, opcode counts, counted locals and actual literal table. Literal primary is sampled before the string getter. The complete type/byte/order rows are retained; no pointer/refcount/cache lifetime or guest completion is printed. The bodies differ from the separate20-body binding matrix and are not labelled duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.4 8.4.20 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
2	0	0	0	6e73,76616c	4	cmdName:676c6f62616c,none:3a3a4e,none:3a3a76,none:76
3	0	0	0	6e73,76616c	4	cmdName:676c6f62616c,none:3a3a4e,none:3a3a76,parsedVarName:6129
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.5

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.5 8.5.19 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
2	1	0	0	6e73,76616c	5	none:3a3a,cmdName:676c6f62616c,none:3a3a4e,none:3a3a76,none:76
3	1	0	0	6e73,76616c	5	none:3a3a,cmdName:676c6f62616c,none:3a3a4e,none:3a3a76,parsedVarName:6129
```
Repeated equal list bytes identify distinct literal-table positions, not observed pointer equality.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_EvalObjEx original procedure/caught call; private original literal primary/count/bytes and local table. Version: tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

Selected complete original rows:

```text
case	hook	variables	nsupvars	locals	literal_count	literals
2	1	0	0	6e73,76616c,76	6	none:3a3a,list:3a3a4e,none:3a3a76,cmdName:676c6f62616c,list:3a3a4e,none:76
3	1	0	0	6e73,76616c,76	6	none:3a3a,list:3a3a4e,none:3a3a76,cmdName:676c6f62616c,list:3a3a4e,parsedVarName:6129
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
