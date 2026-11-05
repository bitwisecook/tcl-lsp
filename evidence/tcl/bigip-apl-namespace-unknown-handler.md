# BIG-IP APL callback: namespace-unknown-handler

## Question

Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

This callback addresses only the `namespace-unknown-handler` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Namespace-local handler; no global unknown redefinition.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `namespace-unknown-handler`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unknown ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler::handler}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler {__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler_absent fixed}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler*]⟦EOF⟧
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
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
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
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace u..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
