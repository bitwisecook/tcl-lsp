# BIG-IP APL callback: unknown-isolated-child

## Question

Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

This callback addresses only the `unknown-isolated-child` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Only child unknown is redefined; delete __tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_child; parent stock commands remain intact.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `unknown-isolated-child`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child {}
interp create __tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_child
interp eval __tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_child {proc unknown {head args} {return [linsert $args 0 $head]}}
```

### nh_operation

```tcl-visible
interp eval __tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_child {__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_absent fixed}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child*]⟦EOF⟧
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
__tcl_lsp_probe_nh20261012aplbroad1_unknown_isolated_child_absent fixed⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
