# BIG-IP APL callback: import-source-renamed

## Question

Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

This callback addresses only the `import-source-renamed` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: NH11 adapted names; import invocation and origin text retained separately.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `import-source-renamed`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source {proc p {} {return ORIGINAL}; namespace export p}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::consumer {namespace import ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::p}
rename ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::p ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::pp
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::consumer {list [p] [namespace origin p]}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed*]⟦EOF⟧
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
can't rename "::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::p": command doesn't exist⟦EOF⟧
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
can't rename "::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::p": command doesn't exist
    while executing
"rename ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::p ::__tcl_lsp_probe_nh20261012aplbroad1_import_source_renamed::source::pp"
    ("eval" body line 4)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
