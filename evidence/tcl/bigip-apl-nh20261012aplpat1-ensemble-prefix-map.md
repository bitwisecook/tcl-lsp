# BIG-IP APL callback: ensemble-prefix-map

## Question

Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

This callback addresses only the `ensemble-prefix-map` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `ensemble-prefix-map`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map {}
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map::emit {a b}]]
}
```

### nh_operation

```tcl-visible
::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map::facade send {c d}
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map}
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
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
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
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map" script line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble creat..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
