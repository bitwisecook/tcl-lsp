# naming.procedure.recompile-retained-procbody

Kind: `native-observation`

## Problem statement

A real procbody object owns a procedure independently of the registered declaration. Recompilation of that shared ownership can replace an older-release Proc while retaining the original default object.

## Question

Does holding an actual native procbody cause the next registered procedure invocation to replace Proc/body identity?

## Conclusion

C8.4/8.5 hold Proc refs2 and replace original Proc/body on the next invocation; the original default object remains shared, with new default refs3 then2 after procbody release. C8.6–9.1 retain Proc/body/default while held refs2 decline to1 at release and default staysrefs2 after recompilation. All warm/new invocations return DEFAULT normally. This measures native procbody-held ownership only, not a generic source representation or object allocation grant.

## Scope

Five C release runs for this exact independent mode. The source retains declaration/body/default addresses and samples live Proc/CallFrame fields, original key headers, compile epoch and compiled-local count before any result/string observer. ASCII source uses Tcl_EvalEx flags0; original p enters Tcl_EvalObjv flags0. Invalidation actually renames set and registers a forwarding compatibility handler, preserving its original native handler/clientdata lifetime. Original stdout/source/archive/executable digests, native-source metadata, process0/no-timeout/empty stderr are retained; compiler status/header/configuration/full runtime version query is unrecorded here. Native source SHA metadata has no attached source excerpt, so supplies no implementation explanation. No Jim/BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=dd7022e962d3dee3fa9ec1119f69ba96bdfcac8d608ab0dc2d6a0b1605ce5262; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|1|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|0
S|1|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|0
R|1|warm-return|0|44454641554c54
S|1|procbody-held|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|0
S|1|invalidated|1|1|1|2|1|bytecode|1|none|1|1|none|-1|0|-1|1|1
S|1|recompiled-return|0|0|1|1|1|bytecode|1|none|3|1|none|-1|0|-1|1|1
R|1|recompiled-return|0|44454641554c54
S|1|procbody-released|0|0|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|1
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=4a6b86dd8246f045785205a865e874fcc9d0e6a5bd0b040e78fde2d1da2e10a5; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|1|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|3
S|1|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|3
R|1|warm-return|0|44454641554c54
S|1|procbody-held|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|3
S|1|invalidated|1|1|1|2|1|bytecode|1|none|1|1|none|-1|0|-1|1|4
S|1|recompiled-return|0|0|1|1|1|bytecode|1|none|3|1|none|-1|0|-1|1|4
R|1|recompiled-return|0|44454641554c54
S|1|procbody-released|0|0|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|4
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=fb9af2daceb502152cb40ce753df19c3a397a058823b30b31f220e4d2b72524a; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|1|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|17
S|1|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|17
R|1|warm-return|0|44454641554c54
S|1|procbody-held|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|17
S|1|invalidated|1|1|1|2|1|bytecode|1|none|1|1|none|-1|0|-1|1|18
S|1|recompiled-return|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|18
R|1|recompiled-return|0|44454641554c54
S|1|procbody-released|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|18
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=48fc46e306458a52be3b4c754922292397bfb2b4a9d413f7d12f2e67400086f6; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|1|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|20
S|1|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|20
R|1|warm-return|0|44454641554c54
S|1|procbody-held|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|20
S|1|invalidated|1|1|1|2|1|bytecode|1|none|1|1|none|-1|0|-1|1|21
S|1|recompiled-return|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|21
R|1|recompiled-return|0|44454641554c54
S|1|procbody-released|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|21
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=6ab8269ea42c18c73b8e888891890ac93a5d3e8693524712a25c23a6687593ba; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|1|defined|1|1|1|1|1|none|1|none|1|1|none|-1|0|-1|1|24
S|1|warm-return|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|24
R|1|warm-return|0|44454641554c54
S|1|procbody-held|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|24
S|1|invalidated|1|1|1|2|1|bytecode|1|none|1|1|none|-1|0|-1|1|25
S|1|recompiled-return|1|1|1|2|1|bytecode|1|none|2|1|none|-1|0|-1|1|25
R|1|recompiled-return|0|44454641554c54
S|1|procbody-released|1|1|1|1|1|bytecode|1|none|2|1|none|-1|0|-1|1|25
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
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-1.txt). SHA-256 `0a0e765a2e2a2092d3f9038bf35e334eee172d54b14b9a2cab8a8cc5d2af2186`. Full original S/R stream for exact mode1.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-1.txt). SHA-256 `d740b21ac8e32a438e15f2cbbef176852cf43f5fa224cf1d468336bd5d85ce4e`. Full original S/R stream for exact mode1.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-1.txt). SHA-256 `f6615fe9ca2018a3d9b8866c617cf6001ef03c871d9c87106d8a3213b572196e`. Full original S/R stream for exact mode1.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-1.txt). SHA-256 `ac7681c8ac670762c5b68025b202d5c2d24f93c56a122ac4b4e5374e512e886c`. Full original S/R stream for exact mode1.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-1.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-1.txt). SHA-256 `04844d11de53f99559d2b7eac94761cd3ac58910c2c1841ed4c3849b042d6af4`. Full original S/R stream for exact mode1.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::recompilation_action`: Selects declaration retention versus replacement only from actual native compilation purpose and retained procedure role count.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::recompilation_ownership_tests::recompilation_matches_all_20_native_declaration_identity_controls` (linked): Checks declaration retention/replacement against the20 original before/after role cases; other native body/default/frame/header counts are retained evidence, not assertions this pure recipe test executes.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.
