# BIG-IP Tcl log: nh20261012cap2, tmm-procedure-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace.code:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.export:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.path:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **namespace.code:** Not recorded
- **namespace.export:** Not recorded
- **namespace.path:** Not recorded

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6e616d65737061636520636f6465207b6c6973742063617074757265647d7d]
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
        log local0. "NHO|nh20261012cap2|namespace.code|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.code|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.code|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6e616d6573706163652070617468]
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
        log local0. "NHO|nh20261012cap2|namespace.path|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174687d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.path|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f7274207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f7274207b6e616d657370616365206578706f7274206e685f6e6f6e657d]
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
        log local0. "NHO|nh20261012cap2|namespace.export|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap2|namespace.export|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745f2a5d]
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
        log local0. "NHO|nh20261012cap2|namespace.export|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-procedure, unit 0:0, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|3a3a6e616d65737061636520696e73636f7065203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6c6973742063617074757265647d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468220a2020202028226576616c222062
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|6f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.code, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.code, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::namespace inscope ::__nh_nh20261012cap2_namespace_code {list captured}⟦EOF⟧
```

- namespace.code, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.path, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.path, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.path, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path"
    ("eval" b⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ody line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.path, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.export, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.code: intended operation reached.

- namespace.export: intended operation reached.

- namespace.path: intended operation reached.

### tmm-procedure, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|3a3a6e616d65737061636520696e73636f7065203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6c6973742063617074757265647d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468220a2020202028226576616c222062
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|6f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.code, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.code, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::namespace inscope ::__nh_nh20261012cap2_namespace_code {list captured}⟦EOF⟧
```

- namespace.code, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.path, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.path, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.path, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path"
    ("eval" b⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ody line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.path, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.export, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.code: intended operation reached.

- namespace.export: intended operation reached.

- namespace.path: intended operation reached.

### tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|3a3a6e616d65737061636520696e73636f7065203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6c6973742063617074757265647d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468220a2020202028226576616c222062
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|6f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.code, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.code, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::namespace inscope ::__nh_nh20261012cap2_namespace_code {list captured}⟦EOF⟧
```

- namespace.code, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.path, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.path, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.path, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path"
    ("eval" b⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ody line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.path, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.export, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.code: intended operation reached.

- namespace.export: intended operation reached.

- namespace.path: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|3a3a6e616d65737061636520696e73636f7065203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6c6973742063617074757265647d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.code|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468220a2020202028226576616c222062
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|6f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.path|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 07:57:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap2_tmm_procedure_1 <RULE_INIT>: NHO|nh20261012cap2|namespace.export|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace.code, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.code, lookup, completion 0, result, chunk 0 of 1:

```text-visible
::namespace inscope ::__nh_nh20261012cap2_namespace_code {list captured}⟦EOF⟧
```

- namespace.code, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.path, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.path, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace.path, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path"
    ("eval" b⟦EOF⟧
```

- namespace.path, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ody line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace.path, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.export, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, lookup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace.export, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- namespace.code: intended operation reached.

- namespace.export: intended operation reached.

- namespace.path: intended operation reached.
