# BIG-IP APL callback: relative-qualifier-missing-target

## Question

Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

This callback addresses only the `relative-qualifier-missing-target` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: NH02 adapted names; clean ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target too; target qualifier exists locally.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `relative-qualifier-missing-target`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target {proc p {} {return GLOBAL_TARGET}}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target {namespace eval __tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target {}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target {__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target::p}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target*]⟦EOF⟧
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

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target::p"⟦EOF⟧
```

- lookup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lookup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- lookup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target::p"
    while executing
"__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target_target::p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing_target {__tcl_lsp_probe_nh20261012aplbroad1_relative_qualifier_missing..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
