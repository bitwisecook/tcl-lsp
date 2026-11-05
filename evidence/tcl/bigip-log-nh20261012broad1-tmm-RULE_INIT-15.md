# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-15

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **expr-local-mathfunc:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

- **protocol-absolute-native:** Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

- **protocol-computed-native:** Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

## Assumptions

- **expr-local-mathfunc:** Custom mathfunc lookup is gated; stock Tcl8.4 need not support it.
- **protocol-absolute-native:** Dynamic absolute protocol call; record HTTP profile and actual invoking event.
- **protocol-computed-native:** Computed protocol head; requires actual HTTP flow for runtime claim.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-6](fixtures/bigip-tmm-rule-init-6.md)

- case: `protocol-absolute-native`
- case.2: `protocol-computed-native`
- case.3: `expr-local-mathfunc`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native::HTTP {proc method {} {return USER_SHADOW}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_protocol_absolute_native* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*] [info commands ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*] [info vars ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {set head ::HTTP::method}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_protocol_computed_native* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*] [info commands ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*] [info vars ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc::tcl::mathfunc {proc pick {} {return 17}}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*] [info commands ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*] [info vars ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc*]⟦EOF⟧
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once; tmm-RULE_INIT, unit 0:1, invocation once; tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.
