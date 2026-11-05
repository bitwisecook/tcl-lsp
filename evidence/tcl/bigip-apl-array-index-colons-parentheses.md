# BIG-IP APL callback: array-index-colons-parentheses

## Question

Literal `{}`, `{$n}`, `{{a}}`, Unicode and byte-valued names; array indices with colons/parentheses. Are byte values representable in each context?

This callback addresses only the `array-index-colons-parentheses` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Array index contains namespace separator and parentheses.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `array-index-colons-parentheses`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses {set {arr(a::b(c))} ARRAY_DATA}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses {list [set {arr(a::b(c))}] [array names arr]}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_array_index_colons_parentheses*]⟦EOF⟧
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
ARRAY_DATA⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ARRAY_DATA a::b(c)⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
