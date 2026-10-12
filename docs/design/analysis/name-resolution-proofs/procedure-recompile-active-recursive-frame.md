# naming.procedure.recompile-active-recursive-frame

Kind: `native-observation`

## Problem statement

An active frame can own the original Proc while a nested invocation recompiles the registered declaration after a real command mutation. The old active frame and new callable declaration must not be conflated.

## Question

Which registered Proc/body/default and actual active-frame identities remain after recursive recompilation triggered inside the original frame?

## Conclusion

All C captures begin active with original Proc/frame refs2 and matching current frame. C8.4/8.5 replace the registered Proc/body during the nested call while the outer actual frame still owns the old Proc; after recursion the frame no longer equals current registration. Later C retain original Proc/body and matching active frame. Default identity remains original, with active-after-recursive refs4 on older C versus3 later, then2 after outer completion. Both reported completions are normal DEFAULT. No unobserved next invocation or frame namespace effect is inferred.

## Scope

Five C release runs for this exact independent mode. The source retains declaration/body/default addresses and samples live Proc/CallFrame fields, original key headers, compile epoch and compiled-local count before any result/string observer. ASCII source uses Tcl_EvalEx flags0; original p enters Tcl_EvalObjv flags0. Invalidation actually renames set and registers a forwarding compatibility handler, preserving its original native handler/clientdata lifetime. Original stdout/source/archive/executable digests, native-source metadata, process0/no-timeout/empty stderr are retained; compiler status/header/configuration/full runtime version query is unrecorded here. Native source SHA metadata has no attached source excerpt, so supplies no implementation explanation. No Jim/BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=dd7022e962d3dee3fa9ec1119f69ba96bdfcac8d608ab0dc2d6a0b1605ce5262; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|2|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|0
S|2|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|0
R|2|warm-return|0|44454641554c54
S|2|active-before-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|0
S|2|active-after-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|1
S|2|active-after-recursive-call|0|0|1|1|1|bytecode|1|none|4|1|none|-1|0|1|1|1
S|2|recompiled-return|0|0|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|1
R|2|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=4a6b86dd8246f045785205a865e874fcc9d0e6a5bd0b040e78fde2d1da2e10a5; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|2|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|3
S|2|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|3
R|2|warm-return|0|44454641554c54
S|2|active-before-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|3
S|2|active-after-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|4
S|2|active-after-recursive-call|0|0|1|1|1|bytecode|1|none|4|1|none|-1|0|1|1|4
S|2|recompiled-return|0|0|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|4
R|2|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=fb9af2daceb502152cb40ce753df19c3a397a058823b30b31f220e4d2b72524a; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|2|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|17
S|2|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|17
R|2|warm-return|0|44454641554c54
S|2|active-before-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|17
S|2|active-after-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|18
S|2|active-after-recursive-call|1|1|1|2|1|bytecode|1|none|3|1|none|-1|1|2|1|18
S|2|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|18
R|2|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=48fc46e306458a52be3b4c754922292397bfb2b4a9d413f7d12f2e67400086f6; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|2|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|20
S|2|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|20
R|2|warm-return|0|44454641554c54
S|2|active-before-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|20
S|2|active-after-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|21
S|2|active-after-recursive-call|1|1|1|2|1|bytecode|1|none|3|1|none|-1|1|2|1|21
S|2|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|21
R|2|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=6ab8269ea42c18c73b8e888891890ac93a5d3e8693524712a25c23a6687593ba; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|2|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|24
S|2|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|24
R|2|warm-return|0|44454641554c54
S|2|active-before-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|24
S|2|active-after-invalidation|1|1|1|2|1|bytecode|1|none|2|1|none|-1|1|2|1|25
S|2|active-after-recursive-call|1|1|1|2|1|bytecode|1|none|3|1|none|-1|1|2|1|25
S|2|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|25
R|2|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No retained observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_procedure_recompile/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/probe.c). SHA-256 `55ec7acc5f09e0785d36108719545602ef12ef1f2928a6000ddc4c29d18161a5`. Exact original retained Proc/body/default/key and active-frame pre-observer protocol, real replacement and four modes.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json). SHA-256 `6c152d459dcba4af5cea730d88ceba3a3a0a19f1cc22d2253a5d1839036c3b31`. Original20 independent process/output associations and actual native source/archive/executable SHA metadata.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-2.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-2.txt). SHA-256 `ade498401b07ff9868e34a5fe6ae3b5586a3651d8eb343d1339d16d4d9d1a4fe`. Full original S/R stream for exact mode2.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-2.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-2.txt). SHA-256 `89dc619120f347e85df25112b25389ac6416b23a371add65eaef577a1d320beb`. Full original S/R stream for exact mode2.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-2.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-2.txt). SHA-256 `5622965afb5c1fde77b5f43a1f40e0f847ed877b868d3904eedc28a17394dc2d`. Full original S/R stream for exact mode2.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-2.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-2.txt). SHA-256 `4b92c38ce0c19178c282e46b6fa2cb035b9a752553c305f3cce3c7e52b8e5ac6`. Full original S/R stream for exact mode2.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-2.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-2.txt). SHA-256 `0b4d8502348041670945badd225d3547d667209a546d854acf6442cd8fededed`. Full original S/R stream for exact mode2.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::recompilation_action`: Selects declaration retention versus replacement only from actual native compilation purpose and retained procedure role count.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::recompilation_ownership_tests::recompilation_matches_all_20_native_declaration_identity_controls` (linked): Checks declaration retention/replacement against the20 original before/after role cases; other native body/default/frame/header counts are retained evidence, not assertions this pure recipe test executes.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
