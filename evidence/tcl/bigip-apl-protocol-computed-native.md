# BIG-IP APL callback: protocol-computed-native

## Question

Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

This callback addresses only the `protocol-computed-native` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Computed protocol head; requires actual HTTP flow for runtime claim.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `protocol-computed-native`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native {}
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native {set head ::HTTP::method}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native {$head}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*] [info commands ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*] [info vars ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native*]⟦EOF⟧
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
::HTTP::method⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "::HTTP::method"⟦EOF⟧
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
invalid command name "::HTTP::method"
    while executing
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012aplbroad1_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
