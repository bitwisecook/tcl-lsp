# BIG-IP APL callback: oo-prefix-deleted-near-miss

## Question

What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

This callback addresses only the `oo-prefix-deleted-near-miss` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplpat1`
- case: `oo-prefix-deleted-near-miss`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss {}
unset -nocomplain object
oo::class create ::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss::Class new]
set prefix [$object prefix]
$object destroy
catch {eval $prefix} result
```

### nh_operation

```tcl-visible
string match {invalid command name *} $result
```

### nh_cleanup

```tcl-visible
if {[info exists object]} {catch {$object destroy}}
if {[namespace exists ::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss]} {namespace delete ::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss}
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
invalid command name "oo::class"⟦EOF⟧
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
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012aplpat1_oo_prefix_deleted_near_miss::Class {
    method emit {value} {list ok $value}
    method prefix {..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
