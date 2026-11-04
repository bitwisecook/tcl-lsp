# BIG-IP APL callback: oo-self-prefix

## Question

Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?

This callback addresses only the `oo-self-prefix` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

```tcl-visible
set nh_apl_rows {}
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f707265666978207b7d0a756e736574202d6e6f636f6d706c61696e206f626a6563740a6f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f7072656669783a3a436c617373206e65775d0a]
set nh_operation [binary format H* 6576616c205b246f626a656374207072656669785d0a]
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b696e666f20657869737473206f626a6563745d7d207b6361746368207b246f626a6563742064657374726f797d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f7072656669785d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f7072656669787d0a]
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}

return [join $nh_apl_rows "\n"]
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|1|none|once|result|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|1|none|once|errorinfo|utf-8|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|setup|1|none|once|errorinfo|utf-8|1|2|73656c662e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
NHO|nh20261012aplpat1|oo-self-prefix|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|
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
"oo::class create ::__tcl_lsp_probe_nh20261012aplpat1_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [self..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
