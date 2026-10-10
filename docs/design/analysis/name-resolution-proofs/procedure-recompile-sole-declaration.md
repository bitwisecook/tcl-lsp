# naming.procedure.recompile-sole-declaration

Kind: `native-observation`

## Problem statement

Compiler epoch invalidation may retire a bytecode cache while retaining or replacing the registered declaration. A sole live declaration is a separate ownership case from active/retained procedures.

## Question

After replacing core set with a forwarding handler, does the sole registered procedure retain original Proc/body/default across its next invocation?

## Conclusion

Every C capture retains original Proc/body/default identity in this sole-declaration mode. Body becomes bytecode after warm invocation and remains a resident bytecode object; default refs move1 at definition to2 warm,1 after invalidation,2 after recompilation. Both completions return DEFAULT normally. The actual compile epoch increases by1 for this replacement; absolute initial epochs differ and are not a clock inference.

## Scope

Five C release runs for this exact independent mode. The source retains declaration/body/default addresses and samples live Proc/CallFrame fields, original key headers, compile epoch and compiled-local count before any result/string observer. ASCII source uses Tcl_EvalEx flags0; original p enters Tcl_EvalObjv flags0. Invalidation actually renames set and registers a forwarding compatibility handler, preserving its original native handler/clientdata lifetime. Original stdout/source/archive/executable digests, native-source metadata, process0/no-timeout/empty stderr are retained; compiler status/header/configuration/full runtime version query is unrecorded here. Native source SHA metadata has no attached source excerpt, so supplies no implementation explanation. No Jim/BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=dd7022e962d3dee3fa9ec1119f69ba96bdfcac8d608ab0dc2d6a0b1605ce5262; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|0|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|0
S|0|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|0
R|0|warm-return|0|44454641554c54
S|0|invalidated|1|1|1|1|1|bytecode|1|none|1|1|none|-1|0|-1|1|1
S|0|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|1
R|0|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=4a6b86dd8246f045785205a865e874fcc9d0e6a5bd0b040e78fde2d1da2e10a5; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|0|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|3
S|0|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|3
R|0|warm-return|0|44454641554c54
S|0|invalidated|1|1|1|1|1|bytecode|1|none|1|1|none|-1|0|-1|1|4
S|0|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|4
R|0|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=fb9af2daceb502152cb40ce753df19c3a397a058823b30b31f220e4d2b72524a; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|0|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|17
S|0|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|17
R|0|warm-return|0|44454641554c54
S|0|invalidated|1|1|1|1|1|bytecode|1|none|1|1|none|-1|0|-1|1|18
S|0|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|18
R|0|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=48fc46e306458a52be3b4c754922292397bfb2b4a9d413f7d12f2e67400086f6; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|0|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|20
S|0|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|20
R|0|warm-return|0|44454641554c54
S|0|invalidated|1|1|1|1|1|bytecode|1|none|1|1|none|-1|0|-1|1|21
S|0|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|21
R|0|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=6ab8269ea42c18c73b8e888891890ac93a5d3e8693524712a25c23a6687593ba; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|0|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|24
S|0|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|24
R|0|warm-return|0|44454641554c54
S|0|invalidated|1|1|1|1|1|bytecode|1|none|1|1|none|-1|0|-1|1|25
S|0|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|25
R|0|recompiled-return|0|44454641554c54
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
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-0.txt). SHA-256 `4d83b5a470c32c5567aa48dfabe8eb3c9e475205c40c15475bc71c50c95feb08`. Full original S/R stream for exact mode0.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-0.txt). SHA-256 `99a9ce6530cbe29367174c3c2637ba9c8c8c6df4ceec149992f016b71092eff1`. Full original S/R stream for exact mode0.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-0.txt). SHA-256 `117b9da82ae11d5c83db49d7bb8cea44ba7def34be65e2b3ca9c530ec0a631d8`. Full original S/R stream for exact mode0.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-0.txt). SHA-256 `500ea8a7e146233d17b8fe9a86503f422b10d8fec823b48f293b4526b2ad0eaf`. Full original S/R stream for exact mode0.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-0.txt). SHA-256 `34902891df65eb4417d54364dfd6f0e1b8af0034806884a9d29d80d26bf89fdd`. Full original S/R stream for exact mode0.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::recompilation_action`: Selects declaration retention versus replacement only from actual native compilation purpose and retained procedure role count.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::recompilation_ownership_tests::recompilation_matches_all_20_native_declaration_identity_controls` (linked): Checks declaration retention/replacement against the20 original before/after role cases; other native body/default/frame/header counts are retained evidence, not assertions this pure recipe test executes.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
