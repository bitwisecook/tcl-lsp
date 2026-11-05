# BIG-IP APL callback: frame-wrapper

## Question

Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?

This callback addresses only the `frame-wrapper` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `frame-wrapper`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper {}
proc ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_forward {prefix} {uplevel 1 $prefix}
proc ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_change {name} {upvar 1 $name cell; set cell changed}
proc ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_outer {} {
    set {a b} original
    ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_forward [list ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_change {a b}]
    set {a b}
}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_outer
```

### nh_cleanup

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_forward]]} {rename ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_forward {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_change]]} {rename ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_change {}}
if {[llength [info commands ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_outer]]} {rename ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper_outer {}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_frame_wrapper}
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
changed⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
