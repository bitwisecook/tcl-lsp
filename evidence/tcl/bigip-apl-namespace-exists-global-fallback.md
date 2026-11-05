# BIG-IP APL callback: namespace-exists-global-fallback

## Question

Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

This callback addresses only the `namespace-exists-global-fallback` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Existence contrast to missing read; clean ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback_x.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `namespace-exists-global-fallback`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback {}
set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback_x GLOBAL_VALUE
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback {}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback {info exists __tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback_x}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_exists_global_fallback*]⟦EOF⟧
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
1⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
