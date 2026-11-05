# BIG-IP APL callback: factory-body-near-miss

## Question

When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?

This callback addresses only the `factory-body-near-miss` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `factory-body-near-miss`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss {
    foreach name {red blue} {proc $name {} {return $name}}
}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss::red
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss}
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

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss::red"⟦EOF⟧
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
invalid command name "::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss::red"
    while executing
"::__tcl_lsp_probe_nh20261012aplpat1_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
