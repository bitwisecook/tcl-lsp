# BIG-IP Tcl log: nh20261012cap2, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **encoding:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.eval:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **version:** Which Tcl version and execution namespace are available in this tmsh context, so that observations from this context can be scoped correctly?

## Assumptions

- **encoding:** Not recorded
- **namespace.eval:** Not recorded
- **version:** Not recorded

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 6c697374205b696e666f2070617463686c6576656c5d205b696e666f2074636c76657273696f6e5d205b6e616d6573706163652063757272656e745d]
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
        log local0. "NHO|nh20261012cap2|version|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|version|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5f2a5d]
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
        log local0. "NHO|nh20261012cap2|version|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 62696e617279207363616e205b656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d5d20482a20783b207365742078]
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
        log local0. "NHO|nh20261012cap2|encoding|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e677d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675f2a5d]
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
        log local0. "NHO|nh20261012cap2|encoding|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c207b6e616d6573706163652063757272656e747d]
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
        log local0. "NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e6722
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e67220a202020207768696c6520657865637574696e670a22656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|3a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c
Oct 12 07:56:40 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- version, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- encoding, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "encoding"⟦EOF⟧
```

- encoding, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- encoding, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- encoding, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "encoding"
    while executing
"encoding convertto utf-8 [format %c 945]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- encoding, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.eval, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.eval, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::__nh_nh20261012cap2_namespace_eval⟦EOF⟧
```

- namespace.eval, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding: intended operation reached.

- namespace.eval: intended operation reached.

- version: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e6722
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e67220a202020207768696c6520657865637574696e670a22656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|3a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c
Oct 12 07:56:40 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- version, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- encoding, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "encoding"⟦EOF⟧
```

- encoding, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- encoding, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- encoding, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "encoding"
    while executing
"encoding convertto utf-8 [format %c 945]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- encoding, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.eval, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.eval, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::__nh_nh20261012cap2_namespace_eval⟦EOF⟧
```

- namespace.eval, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding: intended operation reached.

- namespace.eval: intended operation reached.

- version: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e6722
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e67220a202020207768696c6520657865637574696e670a22656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|3a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c
Oct 12 07:56:40 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- version, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- encoding, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "encoding"⟦EOF⟧
```

- encoding, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- encoding, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- encoding, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "encoding"
    while executing
"encoding convertto utf-8 [format %c 945]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- encoding, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.eval, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.eval, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::__nh_nh20261012cap2_namespace_eval⟦EOF⟧
```

- namespace.eval, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding: intended operation reached.

- namespace.eval: intended operation reached.

- version: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|version|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e6722
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d652022656e636f64696e67220a202020207768696c6520657865637574696e670a22656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|encoding|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|3a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c
Oct 12 07:56:40 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_RULE_INIT_0 <RULE_INIT>: NHO|nh20261012cap2|namespace.eval|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- version, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- encoding, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "encoding"⟦EOF⟧
```

- encoding, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- encoding, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- encoding, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "encoding"
    while executing
"encoding convertto utf-8 [format %c 945]"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- encoding, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.eval, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.eval, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::__nh_nh20261012cap2_namespace_eval⟦EOF⟧
```

- namespace.eval, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- encoding: intended operation reached.

- namespace.eval: intended operation reached.

- version: intended operation reached.
