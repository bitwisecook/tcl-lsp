# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-13

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-code-deleted:** Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity?

- **namespace-unknown-handler:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

- **unknown-isolated-child:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

## Assumptions

- **namespace-code-deleted:** Captured namespace spelling after deletion.
- **namespace-unknown-handler:** Namespace-local handler; no global unknown redefinition.
- **unknown-isolated-child:** Only child unknown is redefined; delete __tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_child; parent stock commands remain intact.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c64207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c64207b7d0a696e7465727020637265617465205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f6368696c640a696e74657270206576616c205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f6368696c64207b70726f6320756e6b6e6f776e207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d7d0a]
set nh_operation [binary format H* 696e74657270206576616c205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e742066697865647d0a]
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
        log local0. "NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c642a5d]
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
        log local0. "NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c65727d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65725f616273656e742066697865647d0a]
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
        log local0. "NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722a5d]
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
        log local0. "NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c65746564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e6572207b7365742078204f4c443b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a707265666978205b6e616d65737061636520636f6465207b73657420787d5d7d0a6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65720a]
set nh_operation [binary format H* 6576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d0a]
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
        log local0. "NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465642a5d]
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
        log local0. "NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:45 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f756e6b6e6f776e5f69736f6c617465645f6368696c645f616273656e74206669786564
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|unknown-isolated-child|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|result|binary-scan-tcl-value|0|1|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e2022756e6b6e6f776e223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520756e6b6e6f776e203a3a5f5f74636c5f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|4|6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65723a3a68616e646c6572220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|2|4|206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f756e6b6e6f776e5f68616e646c6572207b70726f632068616e646c6572207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d3b206e616d65737061636520756e6b2e2e2e220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|3|4|20246e685f736574757022
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-unknown-handler|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|3|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a6f776e65722220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a223a3a6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|3|655f636f64655f64656c657465643a3a6f776e6572207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d6573706163655f636f64655f64656c657465643a3a7072656669785d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|2|3|6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:44 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_13 <HTTP_REQUEST>: NHO|nh20261012broad2|namespace-code-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- unknown-isolated-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- unknown-isolated-child, lookup, completion 0, result, chunk 0 of 1:

```text-visible
__tcl_lsp_probe_nh20261012broad27_unknown_isolated_child_absent fixed⟦EOF⟧
```

- unknown-isolated-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace unknown ::__tcl_⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
lsp_probe_nh20261012broad27_namespace_unknown_handler::handler"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler" script line 1)
    invoked from within
"namespace⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
 eval ::__tcl_lsp_probe_nh20261012broad27_namespace_unknown_handler {proc handler {head args} {return [linsert $args 0 $head]}; namespace unk..."
    ("eval" body line 2)
    invoked from within
"eval⟦EOF⟧
```

- namespace-unknown-handler, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
 $nh_setup"⟦EOF⟧
```

- namespace-unknown-handler, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::owner" in inscope namespace command
    while executing
"::namespace inscope ::__tcl_lsp_probe_nh20261012broad27_namespac⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e_code_deleted::owner {set x}"
    ("eval" body line 1)
    invoked from within
"eval [set ::__tcl_lsp_probe_nh20261012broad27_namespace_code_deleted::prefix]"
    ("eval" body line 1)
    invoked fro⟦EOF⟧
```

- namespace-code-deleted, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
m within
"eval $nh_operation"⟦EOF⟧
```

- namespace-code-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-code-deleted: intended operation reached.

- namespace-unknown-handler: intended operation not reached.

- unknown-isolated-child: intended operation reached.
