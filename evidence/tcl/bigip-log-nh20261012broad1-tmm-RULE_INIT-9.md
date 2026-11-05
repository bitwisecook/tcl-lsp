# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-9

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **import-chain-origin:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-force-conflict:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-source-deleted:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

## Assumptions

- **import-chain-origin:** Two import edges with owned namespaces.
- **import-force-conflict:** Force replaces only owned consumer command.
- **import-source-deleted:** Delete owned source command, then invoke imported tail.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-6](fixtures/bigip-tmm-rule-init-6.md)

- case: `import-source-deleted`
- case.2: `import-force-conflict`
- case.3: `import-chain-origin`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::source {proc p {} {return OLD}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::source::p}
rename ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::source::p {}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer {p}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_import_source_deleted* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*] [info commands ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*] [info vars ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict::source {proc p {} {return SOURCE}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict::consumer {proc p {} {return LOCAL}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict::consumer {namespace import -force ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict::source::p; p}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_import_force_conflict* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*] [info commands ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*] [info vars ::__tcl_lsp_probe_nh20261012broad15_import_force_conflict*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin {}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::source {proc p {} {return SOURCE}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::middle {namespace import ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::source::p; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::middle::p}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::consumer {list [p] [namespace origin p]}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad15_import_chain_origin* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*] [info commands ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*] [info vars ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin*]⟦EOF⟧
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

- import-source-deleted, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad15_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.
