# BIG-IP APL callback: namespace-captured-prefix

## Question

Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

This callback addresses only the `namespace-captured-prefix` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

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
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f707265666978207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f707265666978207b0a2020202070726f63205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785f656d6974207b76616c756520617267737d207b6c697374206c6f63616c202476616c75652024617267737d0a2020202073657420707265666978205b6e616d65737061636520636f6465205b6c697374205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785f656d6974207b6120627d5d5d0a7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785f656d6974207b617267737d207b72657475726e20676c6f62616c7d0a]
set nh_operation [binary format H* 6576616c205b6c696e7365727420243a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669783a3a70726566697820656e64207b6320647d5d0a]
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785f656d69745d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785f656d6974207b7d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669785d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c706174315f6e616d6573706163655f63617074757265645f7072656669787d0a]
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
        lappend nh_apl_rows "NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|lookup|0|none|once|result|utf-8|0|1|676c6f62616c
NHO|nh20261012aplpat1|namespace-captured-prefix|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|
```

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
global⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
