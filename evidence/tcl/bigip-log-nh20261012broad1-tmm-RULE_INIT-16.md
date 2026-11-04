# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-16

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-missing-command:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **expr-mathfunc-replaced:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

- **oo-forward-owned-target:** Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation.

## Assumptions

- **error-missing-command:** Direct missing-command error, no inner catch.
- **expr-mathfunc-replaced:** Fresh expression after custom mathfunc replacement, not a cache claim.
- **oo-forward-owned-target:** Optional TclOO capability; destroy owned object/class before namespace teardown.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365643a3a74636c3a3a6d61746866756e63207b70726f63207069636b207b7d207b72657475726e2031377d3b2070726f63207069636b207b7d207b72657475726e2032397d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b65787072207b7069636b28297d7d0a]
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
        log local0. "NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642a5d]
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
        log local0. "NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f746172676574207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f746172676574207b70726f63207461726765742078207b6c697374204f4c442024787d7d0a6f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a7461726765742066697865647d0a736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a6f626a656374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373206e65775d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a7461726765742078207b6c697374204e45572024787d0a]
set nh_operation [binary format H* 5b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a6f626a6563745d2073656e640a]
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
        log local0. "NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765742a5d]
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
        log local0. "NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e747d0a]
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
        log local0. "NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642a5d]
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
        log local0. "NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163652065
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|76616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|776e65642e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e7422
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- expr-mathfunc-replaced, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace e⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_o⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
wned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__t⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
cl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad1⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
5_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163652065
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|76616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|776e65642e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e7422
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- expr-mathfunc-replaced, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace e⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_o⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
wned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__t⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
cl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad1⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
5_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163652065
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|76616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|776e65642e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e7422
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- expr-mathfunc-replaced, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace e⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_o⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
wned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__t⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
cl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad1⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
5_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206d6174682066756e6374696f6e20227069636b220a202020207768696c6520636f6d70696c696e670a2265787072207b7069636b28297d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c616365642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d6573706163652065
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|76616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f657870725f6d61746866756e635f7265706c61636564207b65787072207b7069636b28297d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|expr-mathfunc-replaced|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f776e65645f7461726765743a3a436c617373207b666f72776172642073656e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6f6f5f666f72776172645f6f
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|776e65642e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|oo-forward-owned-target|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e7422
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f636f6d6d616e64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|355f6572726f725f6d697373696e675f636f6d6d616e645f616273656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:05 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_16 <RULE_INIT>: NHO|nh20261012broad1|error-missing-command|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- expr-mathfunc-replaced, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown math function "pick"⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown math function "pick"
    while compiling
"expr {pick()}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced" script line 1)
    invoked from within
"namespace e⟦EOF⟧
```

- expr-mathfunc-replaced, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val ::__tcl_lsp_probe_nh20261012broad15_expr_mathfunc_replaced {expr {pick()}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- expr-mathfunc-replaced, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::__tcl_lsp_probe_nh20261012broad15_oo_forward_owned_target::Class {forward send ::__tcl_lsp_probe_nh20261012broad15_oo_forward_o⟦EOF⟧
```

- oo-forward-owned-target, setup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
wned..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- oo-forward-owned-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-command, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"⟦EOF⟧
```

- error-missing-command, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    while executing
"__tcl_lsp_probe_nh20261012broad15_error_missing_command_absent"
    (in namespace eval "::__t⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
cl_lsp_probe_nh20261012broad15_error_missing_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_missing_command {__tcl_lsp_probe_nh20261012broad1⟦EOF⟧
```

- error-missing-command, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
5_error_missing_command_absent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-command: intended operation reached.

- expr-mathfunc-replaced: intended operation reached.

- oo-forward-owned-target: intended operation not reached.
