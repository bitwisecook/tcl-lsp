# BIG-IP APL callback: rename-procedure-namespace

## Question

Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

This callback addresses only the `rename-procedure-namespace` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Procedure namespace after moving owned command.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `rename-procedure-namespace`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::one {variable x ONE; proc target {} {variable x; return $x}}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::two {variable x TWO}
rename ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::one::target ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::two::target
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::two::target
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace*]⟦EOF⟧
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

- setup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't rename "::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::one::target": command doesn't exist⟦EOF⟧
```

- setup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- setup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- setup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
can't rename "::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::one::target": command doesn't exist
    while executing
"rename ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace::one::target ::__tcl_lsp_probe_nh20261012aplbroad1_rename_procedure_namespace:..."
    ("eval" body line 4)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
