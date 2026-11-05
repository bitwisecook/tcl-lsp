# BIG-IP APL callback: no-ancestor-walk

## Question

Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

This callback addresses only the `no-ancestor-walk` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: NH04 adapted names; unique tail avoids accidental global fallback.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `no-ancestor-walk`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk {proc __tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk_parent {} {return ANCESTOR}; namespace eval child {}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk_parent}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_no_ancestor_walk*]⟦EOF⟧
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
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
ANCESTOR⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
