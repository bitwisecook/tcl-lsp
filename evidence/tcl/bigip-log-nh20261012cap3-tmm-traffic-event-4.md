# BIG-IP Tcl log: nh20261012cap3, tmm-traffic-event-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **catch.options:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **tcloo:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **trace.execution:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **catch.options:** Not recorded
- **tcloo:** Not recorded
- **trace.execution:** Not recorded

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup {}
set nh_operation [binary format H* 6f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d3b203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c6173732064657374726f79]
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
        log local0. "NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f2a5d]
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
        log local0. "NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6361746368207b6c6973742076616c75657d206e685f72206e685f6f3b206c69737420246e685f7220246e685f6f]
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
        log local0. "NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f63617463685f6f7074696f6e735d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f63617463685f6f7074696f6e737d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f63617463685f6f7074696f6e735f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f63617463685f6f7074696f6e735d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f63617463685f6f7074696f6e735f2a5d]
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
        log local0. "NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 70726f63203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f746172676574207b7d207b72657475726e205441524745547d3b2070726f63203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f74726163652061726773207b7d]
set nh_operation [binary format H* 74726163652061646420657865637574696f6e203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f746172676574206c65617665203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f74726163653b203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f746172676574]
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
        log local0. "NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74726163655f657865637574696f6e5f2a5d]
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
        log local0. "NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:27 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:25 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170335f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|tcloo|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|catch.options|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|30207b7d
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|544152474554
Oct 12 09:05:26 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012cap3_tmm_traffic_event_4 <HTTP_REQUEST>: NHO|nh20261012cap3|trace.execution|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|30207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcloo, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- tcloo, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- tcloo, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcloo, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcloo, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap3_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcloo, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- catch.options, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
```

- catch.options, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- catch.options, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- catch.options, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- catch.options, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- trace.execution, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- trace.execution, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TARGET⟦EOF⟧
```

- trace.execution, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
0 {}⟦EOF⟧
```

- catch.options: intended operation reached.

- tcloo: intended operation reached.

- trace.execution: intended operation reached.
