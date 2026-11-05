# BIG-IP APL callback: namespace-code-deleted

## Question

Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

This callback addresses only the `namespace-code-deleted` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Captured namespace spelling after deletion.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `namespace-code-deleted`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::owner {set x OLD; set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::prefix [namespace code {set x}]}
namespace delete ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::owner
```

### nh_operation

```tcl-visible
eval [set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::prefix]
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted*]⟦EOF⟧
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
unknown namespace "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
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
unknown namespace "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
