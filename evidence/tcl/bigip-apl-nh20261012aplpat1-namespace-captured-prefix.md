# BIG-IP APL callback: namespace-captured-prefix

## Question

Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

This callback addresses only the `namespace-captured-prefix` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `namespace-captured-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix {
    proc __tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix_emit {value args} {list local $value $args}
    set prefix [namespace code [list __tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix_emit {a b}]]
}
proc ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix_emit {args} {return global}
```

### nh_operation

```tcl-visible
eval [linsert $::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix::prefix end {c d}]
```

### nh_cleanup

```tcl-visible
if {[llength [info commands ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix_emit]]} {rename ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix_emit {}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_namespace_captured_prefix}
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
global⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
