# BIG-IP APL callback: keyed-variable-link

## Question

Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

This callback addresses only the `keyed-variable-link` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `keyed-variable-link`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link {
    set {state.a b(value)} original
    proc change {name} {
        upvar ::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::$name state
        set state(value) changed
    }
}
::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::change {state.a b}
```

### nh_operation

```tcl-visible
set {::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::state.a b(value)}
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link}
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

- setup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::change"⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::change"
    while executing
"::__tcl_lsp_probe_nh20261012aplpat1_keyed_variable_link::change {state.a b}"
    ("eval" body line 9)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
