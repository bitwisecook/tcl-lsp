# BIG-IP APL callback: command-trace-rename

## Question

Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

This callback addresses only the `command-trace-rename` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

```tcl-visible
set nh_apl_rows {}
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d65207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d65207b0a20202020736574206576656e7473207b7d0a2020202070726f6320746172676574207b7d207b72657475726e205441524745547d0a2020202070726f63206368616e676564207b6f6c64206e6577206f707d207b7661726961626c65206576656e74733b206c617070656e64206576656e7473205b6c69737420246f6c6420246e657720246f705d7d0a7d0a74726163652061646420636f6d6d616e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a7461726765742072656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a6368616e6765640a]
set nh_operation [binary format H* 72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65643b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a6576656e74730a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_setup_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
if {$nh_setup_code == 0} {
set nh_code [catch {eval $nh_operation} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652a5d]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}

return [join $nh_apl_rows "\n"]
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|1|none|once|result|utf-8|0|1|756e6b6e6f776e20636f6d6d616e6420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a74617267657422
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|1|none|once|errorinfo|utf-8|0|2|756e6b6e6f776e20636f6d6d616e6420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a746172676574220a202020207768696c6520657865637574696e670a2274726163652061646420636f6d6d616e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d653a3a7461726765742072656e616d65203a3a
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|setup|1|none|once|errorinfo|utf-8|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f636f6d6d616e645f74726163655f72656e616d652e2e2e220a2020202028226576616c2220626f6479206c696e652037290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
NHO|nh20261012aplbroad1|command-trace-rename|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|7b7d207b7d207b7d
```

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
