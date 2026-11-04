# BIG-IP Tcl log: nh20261012pat1, tmm-RULE_INIT-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **oo-self-prefix:** Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?

- **string-prefix-near-miss:** What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

- **tcltest-shared-lifecycle:** Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

## Assumptions

- **oo-self-prefix:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **string-prefix-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **tcltest-shared-lifecycle:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f707265666978207b7d0a756e736574202d6e6f636f6d706c61696e206f626a6563740a6f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373206e65775d0a]
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
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b696e666f20657869737473206f626a6563745d7d207b6361746368207b246f626a6563742064657374726f797d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669785d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669787d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d697373207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d697373207b7d0a70726f63207b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a73657420746172676574207b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f7264737d0a]
set nh_operation [binary format H* 6576616c202224746172676574207b6120627d207b6320647d220a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973735d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973737d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c65207b7d0a7061636b61676520726571756972652074636c746573740a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c65207b0a202020206e616d65737061636520696d706f7274203a3a74636c746573743a3a2a0a20202020736574206f62736572766564207b7d0a2020202074657374206e616d652d636f727075732d31207b7d202d7365747570207b0a2020202020202020736574207061796c6f6164207b6120627d0a202020207d202d626f6479207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a202020207d202d636c65616e7570207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a2020202020202020756e736574207061796c6f61640a202020207d202d726573756c74207b7b6120627d7d0a7d0a]
set nh_operation [binary format H* 6c69737420243a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c653a3a6f62736572766564205b696e666f20657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c653a3a7061796c6f61645d0a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c655d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f74636c746573745f7368617265645f6c6966656379636c657d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b7365
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|6c665d202e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|20626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat15_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [se⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
lf] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words {a b} {c d}"
    ("eval"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b7365
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|6c665d202e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|20626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat15_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [se⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
lf] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words {a b} {c d}"
    ("eval"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b7365
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|6c665d202e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|20626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat15_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [se⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
lf] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words {a b} {c d}"
    ("eval"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6f6f5f73656c665f7072656669783a3a436c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b7365
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|6c665d202e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|oo-self-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f737472696e675f7072656669785f6e6561725f6d6973733a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|20626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:02:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_4 <RULE_INIT>: NHO|nh20261012pat1|tcltest-shared-lifecycle|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-self-prefix, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012pat15_oo_self_prefix::Class {
    method emit {value} {list ok $value}
    method prefix {} {list [se⟦EOF⟧
```

- oo-self-prefix, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
lf] ..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-self-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two"
    while executing
"::__tcl_lsp_probe_nh20261012pat15_string_prefix_near_miss::two words {a b} {c d}"
    ("eval"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, setup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- oo-self-prefix: intended operation not reached.

- string-prefix-near-miss: intended operation reached.

- tcltest-shared-lifecycle: intended operation not reached.
