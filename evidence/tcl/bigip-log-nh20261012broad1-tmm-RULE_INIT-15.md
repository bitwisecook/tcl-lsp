# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-15

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **expr-local-mathfunc:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

- **protocol-absolute-native:** Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

- **protocol-computed-native:** Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation.

## Assumptions

- **expr-local-mathfunc:** Custom mathfunc lookup is gated; stock Tcl8.4 need not support it.
- **protocol-absolute-native:** Dynamic absolute protocol call; record HTTP profile and actual invoking event.
- **protocol-computed-native:** Computed protocol head; requires actual HTTP flow for runtime claim.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976653a3a48545450207b70726f63206d6574686f64207b7d207b72657475726e20555345525f534841444f577d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d0a]
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
        log local0. "NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652a5d]
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
        log local0. "NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b7365742068656164203a3a485454503a3a6d6574686f647d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d0a]
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
        log local0. "NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652a5d]
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
        log local0. "NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e633a3a74636c3a3a6d61746866756e63207b70726f63207069636b207b7d207b72657475726e2031377d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b65787072207b7069636b28297d7d0a]
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
        log local0. "NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632a5d]
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
        log local0. "NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|3a3a485454503a3a6d6574686f64
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|3a3a485454503a3a6d6574686f64
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|3a3a485454503a3a6d6574686f64
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a485454503a3a6d6574686f64220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|65206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f6162736f6c7574655f6e6174697665207b3a3a485454503a3a6d6574686f647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-absolute-native|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|3a3a485454503a3a6d6574686f64
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|496e76616c696420636f6e7465787420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a222468656164220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f70726f746f636f6c5f636f6d70757465645f6e6174697665207b24686561647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|protocol-computed-native|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e632220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6c6f63616c5f6d61746866756e63207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:07:57 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_15 <RULE_INIT>: NHO|nh20261012broad1|expr-local-mathfunc|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- protocol-absolute-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"::HTTP::method"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native" script line 1)
    invoked from within
"namespac⟦EOF⟧
```

- protocol-absolute-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
e eval ::__tcl_lsp_probe_nh20261012broad15_protocol_absolute_native {::HTTP::method}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-absolute-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- protocol-computed-native, setup, completion 0, result, chunk 0 of 1:

```text-visible
::HTTP::method⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, result, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
Invalid context (line 1)
    invoked from within
"$head"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_protocol_computed_native" script line 1)
    invoked from within
"namespace eval ::⟦EOF⟧
```

- protocol-computed-native, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
__tcl_lsp_probe_nh20261012broad15_protocol_computed_native {$head}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- protocol-computed-native, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc" script line 1)
    invoked from within
"namespace eval⟦EOF⟧
```

- expr-local-mathfunc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 ::__tcl_lsp_probe_nh20261012broad15_expr_local_mathfunc {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-local-mathfunc, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- expr-local-mathfunc: intended operation reached.

- protocol-absolute-native: intended operation reached.

- protocol-computed-native: intended operation reached.
