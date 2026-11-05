# BIG-IP APL callback: active-namespace-delete-recreate

## Question

Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

This callback addresses only the `active-namespace-delete-recreate` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Delete/recreate while proc frame is active; old and new cells observed in one call.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `active-namespace-delete-recreate`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner {variable x OLD; proc target {} {variable x; namespace delete ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner; namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner {variable x NEW}; list $x [set ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner::x]}}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner::target
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate*]⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner::target"⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner::target"
    while executing
"::__tcl_lsp_probe_nh20261012aplbroad1_active_namespace_delete_recreate::owner::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
