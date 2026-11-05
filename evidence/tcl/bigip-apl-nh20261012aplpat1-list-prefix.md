# BIG-IP APL callback: list-prefix

## Question

Does a list command prefix preserve a command name containing spaces and its fixed multiword argument when eval invokes it with an additional argument?

This callback addresses only the `list-prefix` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `list-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_list_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_list_prefix {}
proc {::__tcl_lsp_probe_nh20261012aplpat1_list_prefix::two words} {fixed value} {list $fixed $value}
set target {::__tcl_lsp_probe_nh20261012aplpat1_list_prefix::two words}
set prefix [list $target {a b}]
```

### nh_operation

```tcl-visible
eval [linsert $prefix end {c d}]
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_list_prefix]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_list_prefix}
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
{::__tcl_lsp_probe_nh20261012aplpat1_list_prefix::two words} {a b}⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{a b} {c d}⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
unknown command "::__tcl_lsp_probe_nh20261012aplpat1_list_prefix::two words"⟦EOF⟧
```
