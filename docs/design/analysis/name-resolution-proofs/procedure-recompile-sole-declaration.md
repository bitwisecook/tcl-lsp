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

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

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

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

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

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

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

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

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

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Exact original S/R rows:

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

### jim, bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `NativeProcedureActivationProtocol::recompilation_action`: Selects declaration retention versus replacement only from actual native compilation purpose and retained procedure role count.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::recompilation_ownership_tests::recompilation_matches_all_20_native_declaration_identity_controls` (linked): Checks declaration retention/replacement against the20 original before/after role cases; other native body/default/frame/header counts are retained evidence, not assertions this pure recipe test executes.

## Evidence

- [rust/tcl-vm/tests/data/native_procedure_recompile/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/probe.c)
- [rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/manifest.json)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.4.20-0.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.5.19-0.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/8.6.18-0.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.0.4-0.txt)
- [rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-0.txt](../../../../rust/tcl-vm/tests/data/native_procedure_recompile/9.1.0-0.txt)
