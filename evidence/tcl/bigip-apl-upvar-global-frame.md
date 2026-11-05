# BIG-IP APL callback: upvar-global-frame

## Question

`global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

This callback addresses only the `upvar-global-frame` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Explicit #0; clean global variable ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame_x.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `upvar-global-frame`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame {}
set ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame_x GLOBAL
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame {proc run {} {upvar #0 __tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame_x y; set y CHANGED; return $y}}
```

### nh_operation

```tcl-visible
list [::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame::run] [set ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame_x]
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame*]⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame::run"⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame::run"
    while executing
"::__tcl_lsp_probe_nh20261012aplbroad1_upvar_global_frame::run"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
