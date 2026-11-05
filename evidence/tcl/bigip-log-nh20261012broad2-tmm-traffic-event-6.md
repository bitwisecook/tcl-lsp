# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-6

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **upvar-global-frame:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **upvar-one-namespace-caller:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **upvar-zero-current-frame:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

## Assumptions

- **upvar-global-frame:** Explicit #0; clean global variable ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame_x.
- **upvar-one-namespace-caller:** NH15 adapted names; caller is namespace eval.
- **upvar-zero-current-frame:** NH14 adapted names.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-2](fixtures/bigip-tmm-traffic-event-2.md)

- run: `nh20261012broad2`
- case: `upvar-one-namespace-caller`
- case.2: `upvar-zero-current-frame`
- case.3: `upvar-global-frame`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame {proc run {} {set x 1; upvar 0 x y; set y 2; return $x}}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame::run
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*] [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*] [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_zero_current_frame*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller {variable x NAMESPACE_VALUE; proc run {} {upvar 1 x y; return $y}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller {run}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*] [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*] [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_one_namespace_caller*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame {}
set ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame_x GLOBAL
namespace eval ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame {proc run {} {upvar #0 __tcl_lsp_probe_nh20261012broad27_upvar_global_frame_x y; set y CHANGED; return $y}}
```

### nh_operation.3

```tcl-visible
list [::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame::run] [set ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame_x]
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_upvar_global_frame* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*] [info commands ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*] [info vars ::__tcl_lsp_probe_nh20261012broad27_upvar_global_frame*]⟦EOF⟧
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

- upvar-zero-current-frame, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-zero-current-frame, lookup, completion 0, result:

```text-visible
2⟦EOF⟧
```

- upvar-zero-current-frame, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-one-namespace-caller, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-one-namespace-caller, lookup, completion 0, result:

```text-visible
NAMESPACE_VALUE⟦EOF⟧
```

- upvar-one-namespace-caller, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-global-frame, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-global-frame, lookup, completion 0, result:

```text-visible
CHANGED CHANGED⟦EOF⟧
```

- upvar-global-frame, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-global-frame: intended operation reached.

- upvar-one-namespace-caller: intended operation reached.

- upvar-zero-current-frame: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- upvar-zero-current-frame, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-zero-current-frame, lookup, completion 0, result:

```text-visible
2⟦EOF⟧
```

- upvar-zero-current-frame, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-one-namespace-caller, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-one-namespace-caller, lookup, completion 0, result:

```text-visible
NAMESPACE_VALUE⟦EOF⟧
```

- upvar-one-namespace-caller, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-global-frame, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- upvar-global-frame, lookup, completion 0, result:

```text-visible
CHANGED CHANGED⟦EOF⟧
```

- upvar-global-frame, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- upvar-global-frame: intended operation reached.

- upvar-one-namespace-caller: intended operation reached.

- upvar-zero-current-frame: intended operation reached.
