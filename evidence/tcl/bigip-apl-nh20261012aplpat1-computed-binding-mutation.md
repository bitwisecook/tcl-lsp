# BIG-IP APL callback: computed-binding-mutation

## Question

When a command name is assembled into a variable and the command at that name is replaced, does executing the unchanged variable use the original or the replacement binding?

This callback addresses only the `computed-binding-mutation` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `computed-binding-mutation`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation {proc emit {} {return first}}
set ns ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation
set head ${ns}::emit
rename ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::emit ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::old
proc ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::emit {} {return replacement}
```

### nh_operation

```tcl-visible
$head
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation}
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
can't rename "::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::emit": command doesn't exist⟦EOF⟧
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
can't rename "::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::emit": command doesn't exist
    while executing
"rename ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::emit ::__tcl_lsp_probe_nh20261012aplpat1_computed_binding_mutation::old"
    ("eval" body line 5)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
