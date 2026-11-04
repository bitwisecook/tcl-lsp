# naming.namespace-compiler.plain-name

Kind: `native-observation`

## Problem statement

A simple written name can select a namespace-variable or global binding compiler while retaining different actual local tables on older releases. A source declaration alone cannot grant an opcode.

## Question

What compile-hook, VARIABLE/NSUPVAR counts and counted locals remain for plain variable v and global v?

## Conclusion

C8.4 has no selected hook or emitted binding and retains only formals ns,val. C8.5 and later emit VARIABLE1 for variable v or NSUPVAR1 for global v and append local v.

## Scope

Twenty fixed ASCII procedure bodies are constructed by original Tcl_EvalObjEx in a fresh native interpreter, called under catch and then inspected through private Proc/ByteCode/CompiledLocal fields. The observer counts actual VARIABLE/NSUPVAR instructions and prints counted local-name bytes. It does not retain caught guest values, errors, cell storage or reference counts. Compile-hook presence and preparation residuals remain separate from successful invocation. Five C release associations; Jim/BIG-IP not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association). Build: Original observer; archive; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
0	0	0	0	6e73,76616c
12	0	0	0	6e73,76616c
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association). Build: Original observer; archive; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
0	1	1	0	6e73,76616c,76
12	1	0	1	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Original observer; archive; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
0	1	1	0	6e73,76616c,76
12	1	0	1	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Original observer; archive; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
0	1	1	0	6e73,76616c,76
12	1	0	1	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Original observer; archive; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
0	1	1	0	6e73,76616c,76
12	1	0	1	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

No native namespace binding compiler-table capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No native namespace binding compiler-table capture for this provider is attached.

## Historical references

- `input` (input): [rust/tcl-registry/tests/data/native_namespace_bindings/probe.c](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/probe.c). Exact original procedure bodies, caught calls and private table observer.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). Original headers/compiler/parser/archive/observer/source/output associations.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv). Exact complete table; selected cases [0, 12].
- `provider-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). JSON pointer `/runs/0`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv). Exact complete table; selected cases [0, 12].
- `provider-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). JSON pointer `/runs/1`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv). Exact complete table; selected cases [0, 12].
- `provider-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). JSON pointer `/runs/2`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv). Exact complete table; selected cases [0, 12].
- `provider-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). JSON pointer `/runs/3`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv). Exact complete table; selected cases [0, 12].
- `provider-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). JSON pointer `/runs/4`. Exact original source/private-header/compiler/archive/observer association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_binding_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_binding_compilation.rs), `native_namespace_binding_compilation::tests::namespace_binding_recipes_match_all_100_original_native_windows` (linked): Compares the measured hook/outcome/opcode/local inventory for the selected fixed bodies; actual test execution is independent of the native receipt.
- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_artifacts_match_native_instruction_and_local_matrix` (linked): Compares original body preparation and counted table layout to the native matrix; no native storage/cell ownership is granted by that comparison.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact original probe and all complete stdout tables match their receipt digests. Reconfirmation must use unchanged source with selected configured private headers/compiler tables/static archives and preserve the procedure call before inspection. Compiler/configure details, full launched patchlevel and separate stderr hashes are not recorded. No native replay or Rust pass is claimed here.
