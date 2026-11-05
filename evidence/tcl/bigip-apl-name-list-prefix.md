# BIG-IP APL callback: name-list-prefix

## Question

Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

This callback addresses only the `name-list-prefix` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: List prefix retains spaced head and data.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `name-list-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix {proc {two words} x {return $x}; set prefix [list {two words} {a b}]}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix {eval $prefix}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_name_list_prefix*]⟦EOF⟧
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
{two words} {a b}⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
a b⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
