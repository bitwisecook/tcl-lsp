# naming.namespace-compiler.declined-prefix-residual

Kind: `native-observation`

## Problem statement

A later non-compilable name can abandon an opcode attempt after an earlier local was already recorded. All-or-nothing local-table reconstruction would discard that residual.

## Question

What opcode and local inventory remains after variable v 1 a) 2 or global v a) declines at a later operand?

## Conclusion

C8.4 retains only formals. C8.5 and later retain earlier local v but emit zero binding opcodes for both declined controls. The residual local inventory does not supply successful execution or a completed binding.

## Scope

Twenty fixed ASCII procedure bodies are constructed by original Tcl_EvalObjEx in a fresh native interpreter, called under catch and then inspected through private Proc/ByteCode/CompiledLocal fields. The observer counts actual VARIABLE/NSUPVAR instructions and prints counted local-name bytes. It does not retain caught guest values, errors, cell storage or reference counts. Compile-hook presence and preparation residuals remain separate from successful invocation. Five C release associations; Jim/BIG-IP not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection. Version: tcl8.4 8.4.20 (capture association).

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	0	0	0	6e73,76616c
18	0	0	0	6e73,76616c
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection. Version: tcl8.5 8.5.19 (capture association); tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	1	0	0	6e73,76616c,76
18	1	0	0	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### jim, bigip

Status: `not-tested`.

No native namespace binding compiler-table capture for this provider is attached.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_binding_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_binding_compilation.rs), `native_namespace_binding_compilation::tests::namespace_binding_recipes_match_all_100_original_native_windows` (linked): Compares the measured hook/outcome/opcode/local inventory for the selected fixed bodies; actual test execution is independent of the native receipt.
- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_artifacts_match_native_instruction_and_local_matrix` (linked): Compares original body preparation and counted table layout to the native matrix; no native storage/cell ownership is granted by that comparison.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_bindings/probe.c](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/probe.c)
- [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json)
- [rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv)
