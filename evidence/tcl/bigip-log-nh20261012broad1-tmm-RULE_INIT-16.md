# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-16

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-missing-command:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **expr-mathfunc-replaced:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

- **oo-forward-owned-target:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

## Assumptions

- **error-missing-command:** Direct missing-command error, no inner catch.
- **expr-mathfunc-replaced:** Fresh expression after custom mathfunc replacement, not a cache claim.
- **oo-forward-owned-target:** Optional TclOO capability; destroy owned object/class before namespace teardown.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-2](fixtures/bigip-tmm-rule-init-2.md)

- run: `nh20261012broad1`
- case: `oo-forward-owned-target`
- case.2: `expr-mathfunc-replaced`
- case.3: `error-missing-command`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced::tcl::mathfunc {proc pick {} {return 17}; proc pick {} {return 29}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*] [info commands ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*] [info vars ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target {proc target x {list OLD $x}}
oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::target fixed}
set ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::object [::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class new]
proc ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::target x {list NEW $x}
```

### nh_operation.2

```tcl-visible
[set ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::object] send
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*] [info commands ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*] [info vars ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_error_missing_command* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*] [info commands ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*] [info vars ::__tcl_lsp_probe_nh20261012broad15_error_missing_command*]⟦EOF⟧
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

- expr-mathfunc-replaced, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- expr-mathfunc-replaced, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.
