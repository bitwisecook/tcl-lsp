# BIG-IP Tcl log: nh20261012pat1, tmm-RULE_INIT-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **ensemble-prefix-map:** Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

- **factory-body-near-miss:** When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?

- **finite-factory:** When a loop constructs procedure bodies as lists containing the current loop value, do the resulting commands retain their respective values?

## Assumptions

- **ensemble-prefix-map:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **factory-body-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **finite-factory:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a656d6974207b6120627d5d5d0a7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a6661636164652073656e64207b6320647d0a]
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
        log local0. "NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61705d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61707d0a]
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
        log local0. "NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d697373207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d697373207b0a20202020666f7265616368206e616d65207b72656420626c75657d207b70726f6320246e616d65207b7d207b72657475726e20246e616d657d7d0a7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a7265640a]
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
        log local0. "NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973735d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973737d0a]
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
        log local0. "NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f7279207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f7279207b0a20202020666f7265616368206e616d65207b72656420626c75657d207b70726f6320246e616d65207b7d205b6c6973742072657475726e20246e616d655d7d0a7d0a]
set nh_operation [binary format H* 6c697374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f72793a3a7265645d205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f72793a3a626c75655d0a]
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
        log local0. "NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f72795d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f66696e6974655f666163746f72797d0a]
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
        log local0. "NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|4|202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a656d6974207b6120627d5d5d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61702220736372697074206c696e652033
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|4|290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202e2e2e220a2020202028226576616c2220626f6479206c
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|3|4|696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a72652e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a726564220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|72656420626c7565
Oct 12 08:01:49 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- ensemble-prefix-map, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
 -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map" script line 3⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body l⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
ine 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat15_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_pro⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
be_nh20261012pat15_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result, chunk 0 of 1:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|4|202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a656d6974207b6120627d5d5d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61702220736372697074206c696e652033
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|4|290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202e2e2e220a2020202028226576616c2220626f6479206c
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|3|4|696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a72652e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a726564220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|72656420626c7565
Oct 12 08:01:49 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- ensemble-prefix-map, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
 -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map" script line 3⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body l⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
ine 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat15_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_pro⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
be_nh20261012pat15_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result, chunk 0 of 1:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|4|202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a656d6974207b6120627d5d5d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61702220736372697074206c696e652033
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|4|290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202e2e2e220a2020202028226576616c2220626f6479206c
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|3|4|696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a72652e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a726564220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|72656420626c7565
Oct 12 08:01:49 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- ensemble-prefix-map, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
 -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map" script line 3⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body l⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
ine 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat15_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_pro⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
be_nh20261012pat15_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result, chunk 0 of 1:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|4|202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61703a3a656d6974207b6120627d5d5d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d61702220736372697074206c696e652033
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|4|290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f656e73656d626c655f7072656669785f6d6170207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202e2e2e220a2020202028226576616c2220626f6479206c
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|3|4|696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|ensemble-prefix-map|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a72652e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|62655f6e68323032363130313270617431355f666163746f72795f626f64795f6e6561725f6d6973733a3a726564220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|factory-body-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|72656420626c7565
Oct 12 08:01:49 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_1 <RULE_INIT>: NHO|nh20261012pat1|finite-factory|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- ensemble-prefix-map, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
 -command facade -map [list send [list ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map::emit {a b}]]"
    (in namespace eval "::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map" script line 3⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012pat15_ensemble_prefix_map {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create ..."
    ("eval" body l⟦EOF⟧
```

- ensemble-prefix-map, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
ine 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- ensemble-prefix-map, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "name": no such variable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::__tcl_lsp_probe_nh20261012pat15_factory_body_near_miss::re..." line 1)
    invoked from within
"::__tcl_lsp_pro⟦EOF⟧
```

- factory-body-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
be_nh20261012pat15_factory_body_near_miss::red"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- factory-body-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- finite-factory, lookup, completion 0, result, chunk 0 of 1:

```text-visible
red blue⟦EOF⟧
```

- finite-factory, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- ensemble-prefix-map: intended operation not reached.

- factory-body-near-miss: intended operation reached.

- finite-factory: intended operation reached.
