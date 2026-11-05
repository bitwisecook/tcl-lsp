# naming.procedure.recompile-original-local-name-owner

Kind: `native-observation`

## Problem statement

An original localVarName may own its actual procedure or only a selected names table. Equal key bytes cannot decide whether that cache makes the declaration shared at a later compilation boundary.

## Question

Does the original dynamic local-name object alter Proc/body ownership at the next invalidated invocation?

## Conclusion

C8.4 installs original localVarName with key refs2 and Proc refs2, then replaces Proc/body on recompilation while preserving original default identity and current Proc refs2. C8.5–9.1 retain Proc/body/default with Proc refs1 while original localVarName/key refs2 remain. All measured completions return DEFAULT. The observed roles belong to this original dynamic key and forwarding set mutation, not any reconstructed same-byte key or general local cache authority.

## Scope

Five C release runs for this exact independent mode. The source retains declaration/body/default addresses and samples live Proc/CallFrame fields, original key headers, compile epoch and compiled-local count before any result/string observer. ASCII source uses Tcl_EvalEx flags0; original p enters Tcl_EvalObjv flags0. Invalidation actually renames set and registers a forwarding compatibility handler, preserving its original native handler/clientdata lifetime. Original stdout/source/archive/executable digests, native-source metadata, process0/no-timeout/empty stderr are retained; compiler status/header/configuration/full runtime version query is unrecorded here. Native source SHA metadata has no attached source excerpt, so supplies no implementation explanation. No Jim/BIG-IP attempt; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|3|defined|1|1|1|1|1|none|1|none|1|1|none|2|0|-1|1|0
S|3|warm-return|1|1|1|2|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|0
R|3|warm-return|0|44454641554c54
S|3|invalidated|1|1|1|2|1|bytecode|1|none|1|1|localVarName|2|0|-1|1|1
S|3|recompiled-return|0|0|1|2|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|1
R|3|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|3|defined|1|1|1|1|1|none|1|none|1|1|none|2|0|-1|1|3
S|3|warm-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|3
R|3|warm-return|0|44454641554c54
S|3|invalidated|1|1|1|1|1|bytecode|1|none|1|1|localVarName|2|0|-1|1|4
S|3|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|4
R|3|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|3|defined|1|1|1|1|1|none|1|none|1|1|none|2|0|-1|1|17
S|3|warm-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|17
R|3|warm-return|0|44454641554c54
S|3|invalidated|1|1|1|1|1|bytecode|1|none|1|1|localVarName|2|0|-1|1|18
S|3|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|18
R|3|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|3|defined|1|1|1|1|1|none|1|none|1|1|none|2|0|-1|1|20
S|3|warm-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|20
R|3|warm-return|0|44454641554c54
S|3|invalidated|1|1|1|1|1|bytecode|1|none|1|1|localVarName|2|0|-1|1|21
S|3|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|21
R|3|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

```text
S|3|defined|1|1|1|1|1|none|1|none|1|1|none|2|0|-1|1|24
S|3|warm-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|24
R|3|warm-return|0|44454641554c54
S|3|invalidated|1|1|1|1|1|bytecode|1|none|1|1|localVarName|2|0|-1|1|25
S|3|recompiled-return|1|1|1|1|1|bytecode|1|none|2|1|localVarName|2|0|-1|1|25
R|3|recompiled-return|0|44454641554c54
```
S columns are described in README and sampled before the R result-byte observer. S Proc/body comparisons refer to retained original addresses within this process.. Dialect: C Tcl.

ASCII Tcl_EvalEx plus original Tcl_EvalObjv flags0; private live native Proc/CallFrame and original object-header observer.

### jim, bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::recompilation_action`: Selects declaration retention versus replacement only from actual native compilation purpose and retained procedure role count.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::recompilation_ownership_tests::recompilation_matches_all_20_native_declaration_identity_controls` (linked): Checks declaration retention/replacement against the20 original before/after role cases; other native body/default/frame/header counts are retained evidence, not assertions this pure recipe test executes.

## Evidence

- [rust/tcl-vm/tests/data/native_procedure_recompile/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/probe.c)
- [rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-3.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-3.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-3.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-3.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-3.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-3.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-3.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-3.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-3.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-3.txt)
