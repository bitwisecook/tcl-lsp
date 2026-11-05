# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-7

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **global-declaration-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **variable-alternating-arguments:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **variable-declaration-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

## Assumptions

- **global-declaration-link:** Unique global declaration/link; clean global variable ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link_x.
- **variable-alternating-arguments:** Single variable declaration versus name/value alternation.
- **variable-declaration-link:** Namespace variable declaration in proc.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-3](fixtures/bigip-tmm-traffic-event-3.md)

- run: `nh20261012broad2`
- case: `variable-alternating-arguments`
- case.2: `variable-declaration-link`
- case.3: `global-declaration-link`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link {}
set ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link_x GLOBAL
namespace eval ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link {proc run {} {global __tcl_lsp_probe_nh20261012broad27_global_declaration_link_x; set __tcl_lsp_probe_nh20261012broad27_global_declaration_link_x CHANGED}}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012broad27_global_declaration_link::run; set ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link_x
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_global_declaration_link* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*] [info commands ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*] [info vars ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link {variable x NAMESPACE; proc run {} {variable x; set x CHANGED}}
```

### nh_operation.2

```tcl-visible
::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link::run; set ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link::x
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_variable_declaration_link* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*] [info commands ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*] [info vars ::__tcl_lsp_probe_nh20261012broad27_variable_declaration_link*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments {}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments {variable a b; list $a [info exists b]}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*] [info commands ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*] [info vars ::__tcl_lsp_probe_nh20261012broad27_variable_alternating_arguments*]⟦EOF⟧
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

- global-declaration-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.
