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

Status: `observed`. Version: 8.4.20 (capture association). Build: Original observer SHA256 4fdf2b31b02239bb9a04b216a74640d1ecc0dc7634849792714ac5dfd44c179c; archive SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	0	0	0	6e73,76616c
18	0	0	0	6e73,76616c
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association). Build: Original observer SHA256 4c710211d0f08e69e15bd96d290a5cd6f6c3bf33e1476006c62430eb39067a1f; archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	1	0	0	6e73,76616c,76
18	1	0	0	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Original observer SHA256 c628fd1fe1ba468fd27fdbf0a2576562dfbe2392539b68f34cf2e7c14e085258; archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	1	0	0	6e73,76616c,76
18	1	0	0	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Original observer SHA256 98a400711e6619b10eedda15761c0c5224feab78917d49e3ed4d12afa7a3db4d; archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	1	0	0	6e73,76616c,76
18	1	0	0	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Original observer SHA256 c9f85ea5dbe3b920e53518e78a3ab0cca8613e969fd236b11814c2922857bd11; archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; compiler/parser/private-header checksums retained in selected receipt; compile/process0. Full launched patchlevel/configure/compiler version unqueried.. Channel: Tcl_EvalObjEx source body, caught original call, private emitted opcode and counted local table inspection.. Dialect: C Tcl.

Selected original case/hook/VARIABLE/NSUPVAR/count-local table:

```text
case	hook	variables	nsupvars	locals
5	1	0	0	6e73,76616c,76
18	1	0	0	6e73,76616c,76
```
Each row remains a physical compiler observation; it does not establish a successful guest binding.

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

No native namespace binding compiler-table capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No native namespace binding compiler-table capture for this provider is attached.

## Exact evidence

- `input` (input): [rust/tcl-registry/tests/data/native_namespace_bindings/probe.c](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/probe.c). SHA-256 `06a85a6b5d820bfc8eb47534482b71747e96436011d428d25bdc0f01906cfe85`. Exact original procedure bodies, caught calls and private table observer.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. Original headers/compiler/parser/archive/observer/source/output associations.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv). SHA-256 `2ca72225354870bc9ffd8bf6967e2115cac4b2431b1ffb746bfdf8bba955e2df`. Exact complete table; selected cases [5, 18].
- `provider-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. JSON pointer `/runs/0`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv). SHA-256 `abc924968b5052da8fff315127ca1cd384f30a31093835afa6a020fc4aebac69`. Exact complete table; selected cases [5, 18].
- `provider-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. JSON pointer `/runs/1`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv). SHA-256 `04859357a2bc5f8ed1ca3e0590d091b640c60b7cead534052af01bdbf5ec59f7`. Exact complete table; selected cases [5, 18].
- `provider-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. JSON pointer `/runs/2`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv). SHA-256 `04859357a2bc5f8ed1ca3e0590d091b640c60b7cead534052af01bdbf5ec59f7`. Exact complete table; selected cases [5, 18].
- `provider-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. JSON pointer `/runs/3`. Exact original source/private-header/compiler/archive/observer association.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv). SHA-256 `04859357a2bc5f8ed1ca3e0590d091b640c60b7cead534052af01bdbf5ec59f7`. Exact complete table; selected cases [5, 18].
- `provider-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_bindings/provenance.json). SHA-256 `89aff8337440efa2f25a7421c3d7b1268433e0594c702d5ebe266327d49fd80d`. JSON pointer `/runs/4`. Exact original source/private-header/compiler/archive/observer association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_binding_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_binding_compilation.rs), `native_namespace_binding_compilation::tests::namespace_binding_recipes_match_all_100_original_native_windows` (linked): Compares the measured hook/outcome/opcode/local inventory for the selected fixed bodies; actual test execution is independent of the native receipt.
- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_namespace_artifacts_match_native_instruction_and_local_matrix` (linked): Compares original body preparation and counted table layout to the native matrix; no native storage/cell ownership is granted by that comparison.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact original probe and all complete stdout tables match their receipt digests. Reconfirmation must use unchanged source with selected configured private headers/compiler tables/static archives and preserve the procedure call before inspection. Compiler/configure details, full launched patchlevel and separate stderr hashes are not recorded. No native replay or Rust pass is claimed here.
