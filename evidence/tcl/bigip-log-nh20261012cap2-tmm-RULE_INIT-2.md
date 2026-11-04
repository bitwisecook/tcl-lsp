# BIG-IP Tcl log: nh20261012cap2, tmm-RULE_INIT-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace.ensemble:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.import:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.upvar:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **namespace.ensemble:** Not recorded
- **namespace.import:** Not recorded
- **namespace.upvar:** Not recorded

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f7274207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f7274207b6e616d65737061636520696d706f7274203a3a5f5f616273656e745f6e685f2a7d]
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
        log local0. "NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d]
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
        log local0. "NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f7570766172207b7661726961626c6520782056414c55457d]
set nh_operation [binary format H* 6e616d657370616365207570766172203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b3b20736574206e685f6c696e6b]
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
        log local0. "NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661727d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|202d636f6d6d616e6420666163616465202d6d6170207b7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d6573706163652065
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|6e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f6e685f6e68323032
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|3631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.import, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
 -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace e⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nsemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh202⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
61012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|202d636f6d6d616e6420666163616465202d6d6170207b7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d6573706163652065
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|6e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f6e685f6e68323032
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|3631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.import, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
 -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace e⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nsemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh202⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
61012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|202d636f6d6d616e6420666163616465202d6d6170207b7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d6573706163652065
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|6e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f6e685f6e68323032
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|3631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.import, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
 -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace e⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nsemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh202⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
61012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.import|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|202d636f6d6d616e6420666163616465202d6d6170207b7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d6573706163652065
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|6e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.ensemble|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f6e685f6e68323032
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|3631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:50 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_2 <RULE_INIT>: NHO|nh20261012cap2|namespace.upvar|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.import, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.import, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
 -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace e⟦EOF⟧
```

- namespace.ensemble, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nsemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.ensemble, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.upvar, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh202⟦EOF⟧
```

- namespace.upvar, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
61012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.upvar, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.ensemble: intended operation reached.

- namespace.import: intended operation reached.

- namespace.upvar: intended operation reached.
