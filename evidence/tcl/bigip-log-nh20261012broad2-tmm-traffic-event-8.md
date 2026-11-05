# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-8

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **import-source-redefined:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-source-renamed:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **namespace-upvar-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

## Assumptions

- **import-source-redefined:** NH12 adapted names.
- **import-source-renamed:** NH11 adapted names; import invocation and origin text retained separately.
- **namespace-upvar-link:** Namespace-to-namespace link with explicit owner.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-3](fixtures/bigip-tmm-traffic-event-3.md)

- run: `nh20261012broad2`
- case: `import-source-redefined`
- case.2: `import-source-renamed`
- case.3: `namespace-upvar-link`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::source {variable x ORIGINAL}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer {}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::source x y; set y CHANGED}; set ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::source::x
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_namespace_upvar_link* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*] [info commands ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*] [info vars ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source {proc p {} {return ORIGINAL}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source::p}
rename ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source::p ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source::pp
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::consumer {list [p] [namespace origin p]}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_import_source_renamed* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*] [info commands ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*] [info vars ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined::source {proc p {} {return OLD}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined::consumer {namespace import ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined::source::p}
proc ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined::source::p {} {return NEW}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined::consumer {p}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_import_source_redefined* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*] [info commands ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*] [info vars ::__tcl_lsp_probe_nh20261012broad27_import_source_redefined*]⟦EOF⟧
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0; tmm-traffic-event, unit 0:0, invocation 1; tmm-traffic-event, unit 0:0, invocation 10; tmm-traffic-event, unit 0:0, invocation 11; tmm-traffic-event, unit 0:0, invocation 12; tmm-traffic-event, unit 0:0, invocation 13; tmm-traffic-event, unit 0:0, invocation 14; tmm-traffic-event, unit 0:0, invocation 15; tmm-traffic-event, unit 0:0, invocation 2; tmm-traffic-event, unit 0:0, invocation 3; tmm-traffic-event, unit 0:0, invocation 4; tmm-traffic-event, unit 0:0, invocation 5; tmm-traffic-event, unit 0:0, invocation 6; tmm-traffic-event, unit 0:0, invocation 7; tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad27_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad27_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.
