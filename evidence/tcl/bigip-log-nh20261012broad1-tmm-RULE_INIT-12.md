# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-12

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-deleted-lookup:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **command-trace-rename:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

- **rename-procedure-namespace:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

## Assumptions

- **alias-deleted-lookup:** Alias deletion followed by fresh lookup.
- **command-trace-rename:** Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.
- **rename-procedure-namespace:** Procedure namespace after moving owned command.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-2](fixtures/bigip-tmm-rule-init-2.md)

- run: `nh20261012broad1`
- case: `rename-procedure-namespace`
- case.2: `alias-deleted-lookup`
- case.3: `command-trace-rename`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup {proc target {} {return TARGET}}
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::target
interp alias {} ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias {}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*] [info commands ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*] [info vars ::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace::one {variable x ONE; proc target {} {variable x; return $x}}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace::two {variable x TWO}
rename ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace::one::target ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace::two::target
```

### nh_operation.2

```tcl-visible
::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace::two::target
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*] [info commands ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*] [info vars ::__tcl_lsp_probe_nh20261012broad15_rename_procedure_namespace*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename {
    set events {}
    proc target {} {return TARGET}
    proc changed {old new op} {variable events; lappend events [list $old $new $op]}
}
trace add command ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::target rename ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::changed
```

### nh_operation.3

```tcl-visible
rename ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::renamed; set ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::events
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_command_trace_rename* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*] [info commands ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*] [info vars ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename*]⟦EOF⟧
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

- alias-deleted-lookup, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad15_alias_deleted_lookup::alias"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad15_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.
