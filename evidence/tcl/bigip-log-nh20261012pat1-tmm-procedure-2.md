# BIG-IP Tcl log: nh20261012pat1, tmm-procedure-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **frame-wrapper:** Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?

- **frame-wrapper-near-miss:** When the same forwarding operation uses eval rather than uplevel 1, which caller frame does upvar 1 reach, and is the original caller's spaced variable changed?

- **keyed-variable-link:** Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

## Assumptions

- **frame-wrapper:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **frame-wrapper-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **keyed-variable-link:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-4](fixtures/bigip-tmm-procedure-4.md)

- run: `nh20261012pat1`
- case: `frame-wrapper-near-miss`
- case.2: `keyed-variable-link`
- case.3: `frame-wrapper`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper {}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_forward {prefix} {uplevel 1 $prefix}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_change {name} {upvar 1 $name cell; set cell changed}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_outer {} {
    set {a b} original
    ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_forward [list ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_change {a b}]
    set {a b}
}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_outer
```

### nh_cleanup

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_forward]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_forward {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_change]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_change {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_outer]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_outer {}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper}
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss {}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_forward {prefix} {eval $prefix}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_change {name} {upvar 1 $name cell; set cell changed}
proc ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_outer {} {
    set {a b} original
    ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_forward [list ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_change {a b}]
    set {a b}
}
```

### nh_operation.2

```tcl-visible
::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_outer
```

### nh_cleanup.2

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_forward]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_forward {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_change]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_change {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_outer]]} {rename ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss_outer {}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_frame_wrapper_near_miss}
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link {}
namespace eval ::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link {
    set {state.a b(value)} original
    proc change {name} {
        upvar ::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link::$name state
        set state(value) changed
    }
}
::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link::change {state.a b}
```

### nh_operation.3

```tcl-visible
set {::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link::state.a b(value)}
```

### nh_cleanup.3

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link]} {namespace delete ::__tcl_lsp_probe_nh20261012pat16_keyed_variable_link}
```

## Results

### tmm-procedure, unit 0:0, invocation once; tmm-procedure, unit 0:1, invocation once; tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.
