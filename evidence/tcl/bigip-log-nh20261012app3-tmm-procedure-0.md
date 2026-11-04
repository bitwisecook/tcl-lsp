# BIG-IP Tcl log: nh20261012app3, tmm-procedure-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **absolute-other:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **absolute-provider:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **bare-local:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **computed-target:** Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?

- **list-prefix-arguments:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **missing-proc:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **missing-rule:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **provider-lifetime:** Does binding change at load, attachment, or each invocation when an isolated provider rule is replaced or detached?

- **relative-provider:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **space-joined-near-miss:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **storage-read-write:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **absolute-other:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **absolute-provider:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **bare-local:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **computed-target:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **list-prefix-arguments:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **missing-proc:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **missing-rule:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **provider-lifetime:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **relative-provider:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **space-joined-near-miss:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.
- **storage-read-write:** Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 63616c6c20746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|bare-local|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|bare-local|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|relative-provider|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|relative-provider|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|absolute-provider|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|absolute-provider|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f6f746865723a3a746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|absolute-other|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|absolute-other|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f616273656e743a3a746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|missing-rule|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a616273656e74207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|missing-proc|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a7461726765743b206576616c205b6c6973742063616c6c2024746172676574207b6120627d207b6320647d5d]
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
        log local0. "NHO|nh20261012app3|computed-target|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|computed-target|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978205b6c6973742063616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a746172676574207b6120627d5d3b206576616c205b6c696e73657274202470726566697820656e64207b7b787d203a3a7461696c7d5d]
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
        log local0. "NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978202263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a74617267657420612062223b206576616c2022247072656669782063206422]
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
        log local0. "NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a746172676574207b6120627d207b6320647d]
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
        log local0. "NHO|nh20261012app3|provider-lifetime|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|provider-lifetime|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a63656c6c73207374617469633a3a5f5f6e685f6e683230323631303132617070335f31]
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
        log local0. "NHO|nh20261012app3|storage-read-write|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012app3|storage-read-write|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
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
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4c4f43414c207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f54484552207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|1|70726f6320746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f616273656e743a3a746172676574207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|70726f6320616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a616273656e74207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b7b787d203a3a7461696c7d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a7461726765742061206220632064220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|312032203a3a
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
LOCAL {a b} {c d}⟦EOF⟧
```

- relative-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-other, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-other, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OTHER {a b} {c d}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
proc target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_absent::target {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::absent {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
"⟦EOF⟧
```

- computed-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {{x} ::tail}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::target a b c d"
    ("eval" body line 1)
    invoked from within
"eval⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- storage-read-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- storage-read-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1 2 ::⟦EOF⟧
```

- absolute-other: intended operation reached.

- absolute-provider: intended operation reached.

- bare-local: intended operation reached.

- computed-target: intended operation reached.

- list-prefix-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- provider-lifetime: intended operation reached.

- relative-provider: intended operation reached.

- space-joined-near-miss: intended operation reached.

- storage-read-write: intended operation reached.

### tmm-procedure, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4c4f43414c207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f54484552207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|1|70726f6320746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f616273656e743a3a746172676574207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|70726f6320616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a616273656e74207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b7b787d203a3a7461696c7d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a7461726765742061206220632064220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|312032203a3a
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
LOCAL {a b} {c d}⟦EOF⟧
```

- relative-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-other, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-other, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OTHER {a b} {c d}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
proc target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_absent::target {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::absent {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
"⟦EOF⟧
```

- computed-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {{x} ::tail}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::target a b c d"
    ("eval" body line 1)
    invoked from within
"eval⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- storage-read-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- storage-read-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1 2 ::⟦EOF⟧
```

- absolute-other: intended operation reached.

- absolute-provider: intended operation reached.

- bare-local: intended operation reached.

- computed-target: intended operation reached.

- list-prefix-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- provider-lifetime: intended operation reached.

- relative-provider: intended operation reached.

- space-joined-near-miss: intended operation reached.

- storage-read-write: intended operation reached.

### tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4c4f43414c207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f54484552207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|1|70726f6320746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f616273656e743a3a746172676574207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|70726f6320616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a616273656e74207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b7b787d203a3a7461696c7d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a7461726765742061206220632064220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|312032203a3a
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
LOCAL {a b} {c d}⟦EOF⟧
```

- relative-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-other, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-other, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OTHER {a b} {c d}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
proc target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_absent::target {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::absent {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
"⟦EOF⟧
```

- computed-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {{x} ::tail}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::target a b c d"
    ("eval" body line 1)
    invoked from within
"eval⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- storage-read-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- storage-read-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1 2 ::⟦EOF⟧
```

- absolute-other: intended operation reached.

- absolute-provider: intended operation reached.

- bare-local: intended operation reached.

- computed-target: intended operation reached.

- list-prefix-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- provider-lifetime: intended operation reached.

- relative-provider: intended operation reached.

- space-joined-near-miss: intended operation reached.

- storage-read-write: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|bare-local|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4c4f43414c207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|relative-provider|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-provider|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|absolute-other|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f54484552207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-rule|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|1|70726f6320746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f616273656e743a3a746172676574207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|70726f6320616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a616273656e74207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|missing-proc|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|computed-target|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|list-prefix-arguments|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b7b787d203a3a7461696c7d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c20746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e683230323631303132617070335f315f70726f76696465723a3a7461726765742061206220632064220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|space-joined-near-miss|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|provider-lifetime|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f4c44207b6120627d207b6320647d
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 09:03:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012app3_1 <RULE_INIT>: NHO|nh20261012app3|storage-read-write|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|312032203a3a
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
LOCAL {a b} {c d}⟦EOF⟧
```

- relative-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-provider, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-provider, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- absolute-other, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-other, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OTHER {a b} {c d}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
proc target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_absent::target {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::absent {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
"⟦EOF⟧
```

- computed-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {{x} ::tail}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012app3_1_provider::target a b c d"
    ("eval" body line 1)
    invoked from within
"eval⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
OLD {a b} {c d}⟦EOF⟧
```

- storage-read-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- storage-read-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1 2 ::⟦EOF⟧
```

- absolute-other: intended operation reached.

- absolute-provider: intended operation reached.

- bare-local: intended operation reached.

- computed-target: intended operation reached.

- list-prefix-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- provider-lifetime: intended operation reached.

- relative-provider: intended operation reached.

- space-joined-near-miss: intended operation reached.

- storage-read-write: intended operation reached.
