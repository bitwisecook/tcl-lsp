# BIG-IP APL callback: oo-forward-owned-target

## Question

Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

This callback addresses only the `oo-forward-owned-target` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Optional TclOO capability; destroy owned object/class before namespace teardown.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

```tcl-visible
set nh_apl_rows {}
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f746172676574207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f746172676574207b70726f63207461726765742078207b6c697374204f4c442024787d7d0a6f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a7461726765742066697865647d0a736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a6f626a656374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373206e65775d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a7461726765742078207b6c697374204e45572024787d0a]
set nh_operation [binary format H* 5b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a6f626a6563745d2073656e640a]
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765742a5d]
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|1|none|once|result|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|1|none|once|errorinfo|utf-8|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6f6f5f666f727761
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|setup|1|none|once|errorinfo|utf-8|1|2|72645f6f2e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
NHO|nh20261012aplbroad1|oo-forward-owned-target|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|7b7d207b7d207b7d
```

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
