# BIG-IP APL callback: alias-destination-missing-parent

## Question

`interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

This callback addresses only the `alias-destination-missing-parent` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Observe whether qualified alias destination creates missing parent.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `alias-destination-missing-parent`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent {proc target {} {return TARGET}}
```

### nh_operation

```tcl-visible
interp alias {} ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent::absent::alias {} ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent::target
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent*]⟦EOF⟧
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
::__tcl_lsp_probe_nh20261012aplbroad1_alias_destination_missing_parent::absent::alias⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
