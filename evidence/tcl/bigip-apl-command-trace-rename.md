# BIG-IP APL callback: command-trace-rename

## Question

Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

This callback addresses only the `command-trace-rename` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `command-trace-rename`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename {
    set events {}
    proc target {} {return TARGET}
    proc changed {old new op} {variable events; lappend events [list $old $new $op]}
}
trace add command ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::target rename ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::changed
```

### nh_operation

```tcl-visible
rename ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::target ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::renamed; set ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::events
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename*]⟦EOF⟧
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
unknown command "::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::target"⟦EOF⟧
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
unknown command "::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::target"
    while executing
"trace add command ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename::target rename ::__tcl_lsp_probe_nh20261012aplbroad1_command_trace_rename..."
    ("eval" body line 7)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
