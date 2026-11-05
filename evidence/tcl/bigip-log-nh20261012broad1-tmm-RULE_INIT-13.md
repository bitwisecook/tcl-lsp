# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-13

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-code-deleted:** Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

- **namespace-unknown-handler:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

- **unknown-isolated-child:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

## Assumptions

- **namespace-code-deleted:** Captured namespace spelling after deletion.
- **namespace-unknown-handler:** Namespace-local handler; no global unknown redefinition.
- **unknown-isolated-child:** Only child unknown is redefined; delete __tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_child; parent stock commands remain intact.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-2](fixtures/bigip-tmm-rule-init-2.md)

- run: `nh20261012broad1`
- case: `namespace-unknown-handler`
- case.2: `unknown-isolated-child`
- case.3: `namespace-code-deleted`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child {}
interp create __tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_child
interp eval __tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_child {proc unknown {head args} {return [linsert $args 0 $head]}}
```

### nh_operation

```tcl-visible
interp eval __tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_child {__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_absent fixed}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_unknown_isolated_child* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*] [info commands ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*] [info vars ::__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unknown ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler::handler}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler {__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler_absent fixed}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*] [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*] [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner {set x OLD; set ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::prefix [namespace code {set x}]}
namespace delete ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner
```

### nh_operation.3

```tcl-visible
eval [set ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::prefix]
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_namespace_code_deleted* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*] [info commands ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*] [info vars ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted*]⟦EOF⟧
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

- unknown-isolated-child, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result:

```text-visible
__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result:

```text-visible
__tcl_lsp_probe_nh20261012broad15_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad15_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.
