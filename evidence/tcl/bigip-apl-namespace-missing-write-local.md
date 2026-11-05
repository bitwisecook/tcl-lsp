# BIG-IP APL callback: namespace-missing-write-local

## Question

Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

This callback addresses only the `namespace-missing-write-local` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Namespace write/create contrast; clean global variable ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `namespace-missing-write-local`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local {}
set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x GLOBAL_VALUE
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local {}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local {set __tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x LOCAL_VALUE}; list [set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x] [set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x]
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local*]⟦EOF⟧
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
can't read "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x": no such variable⟦EOF⟧
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
can't read "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
