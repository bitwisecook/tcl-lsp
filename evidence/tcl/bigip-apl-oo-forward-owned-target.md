# BIG-IP APL callback: oo-forward-owned-target

## Question

Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

This callback addresses only the `oo-forward-owned-target` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Optional TclOO capability; destroy owned object/class before namespace teardown.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `oo-forward-owned-target`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target {proc target x {list OLD $x}}
oo::class create ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::target fixed}
set ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::object [::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::Class new]
proc ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::target x {list NEW $x}
```

### nh_operation

```tcl-visible
[set ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::object] send
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target*]⟦EOF⟧
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
"oo::class create ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012aplbroad1_oo_forward_o..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
