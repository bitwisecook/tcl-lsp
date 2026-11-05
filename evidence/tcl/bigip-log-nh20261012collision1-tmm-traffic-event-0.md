# BIG-IP Tcl log: nh20261012collision1, tmm-traffic-event-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **frame-wrapper:** Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?

## Assumptions

- **frame-wrapper:** Complete separately adapted program in actual host. Intended original final lookup is not inferred from whole-program completion. Enumerated spelling substitutions avoid unowned native commands/frame variables.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-1](fixtures/bigip-tmm-traffic-event-1.md)

- run: `nh20261012collision1`
- case: `frame-wrapper`

### nh_setup

```tcl-visible
set nh_guard_scope [namespace current]
set nh_guard_procs {}
set nh_guard_absent {}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision namespace-or-command $nh_guard_name]}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}
foreach nh_guard_tail {__tcl_lsp_probe_nh20261012collision1_2_0_forward __tcl_lsp_probe_nh20261012collision1_2_0_change __tcl_lsp_probe_nh20261012collision1_2_0_outer} {
    set nh_guard_name [string trimright $nh_guard_scope :]::$nh_guard_tail
    if {[llength [info commands $nh_guard_name]] || [string length [namespace which -command $nh_guard_tail]]} {error [list ownership-collision procedure $nh_guard_name resolved [namespace which -command $nh_guard_tail]]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list procedure $nh_guard_name fallback $nh_guard_tail]
}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision command-or-namespace $nh_guard_name]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list command-and-namespace $nh_guard_name]
}
foreach nh_guard_name {} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
proc __tcl_lsp_probe_nh20261012collision1_2_0_forward {prefix} {uplevel 1 $prefix}
proc __tcl_lsp_probe_nh20261012collision1_2_0_change {name} {upvar 1 $name cell; set cell changed}
proc __tcl_lsp_probe_nh20261012collision1_2_0_outer {} {
    set {a b} original
    __tcl_lsp_probe_nh20261012collision1_2_0_forward [list __tcl_lsp_probe_nh20261012collision1_2_0_change {a b}]
    set {a b}
}
__tcl_lsp_probe_nh20261012collision1_2_0_outer

```

### nh_cleanup

```tcl-visible
if {$nh_setup_code == 0} {
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {rename $nh_guard_name {}}
    }
    foreach nh_guard_name {} {
        if {[namespace exists $nh_guard_name]} {namespace delete $nh_guard_name}
    }
    foreach nh_guard_name {} {unset -nocomplain $nh_guard_name}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {} {
        if {[info exists $nh_guard_name]} {lappend nh_guard_remaining [list frame-variable $nh_guard_name]}
    }
    list ownership-cleanup execution-namespace $nh_guard_scope remaining $nh_guard_remaining
} else {
    list ownership-not-acquired original-source-unreached
}
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

- frame-wrapper, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_forward fallback __tcl_lsp_probe_nh20261012collision1_2_0_forward} {procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_change fallback __tcl_lsp_probe_nh20261012collision1_2_0_change} {procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_outer fallback __tcl_lsp_probe_nh20261012collision1_2_0_outer}}⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- frame-wrapper: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_forward fallback __tcl_lsp_probe_nh20261012collision1_2_0_forward} {procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_change fallback __tcl_lsp_probe_nh20261012collision1_2_0_change} {procedure ::__tcl_lsp_probe_nh20261012collision1_2_0_outer fallback __tcl_lsp_probe_nh20261012collision1_2_0_outer}}⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- frame-wrapper: intended operation reached.
