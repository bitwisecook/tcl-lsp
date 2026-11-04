# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **absolute-bypasses-local:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **colon-run-and-empty-tail:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **no-ancestor-walk:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **absolute-bypasses-local:** Absolute command spelling; clean ::__tcl_lsp_probe_nh20261012broad27_absolute_bypasses_local_target too.
- **colon-run-and-empty-tail:** NH09 adapted names; preserve separate substitution reachability.
- **no-ancestor-walk:** NH04 adapted names; unique tail avoids accidental global fallback.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b207b70726f63205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74207b7d207b72657475726e20414e434553544f527d3b206e616d657370616365206576616c206368696c64207b7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e747d0a]
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
        log local0. "NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b2a5d]
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
        log local0. "NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c5f746172676574207b70726f632070207b7d207b72657475726e204142534f4c5554457d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c5f746172676574207b70726f632070207b7d207b72657475726e204c4f43414c7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c207b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c5f7461726765743a3a707d0a]
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
        log local0. "NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6162736f6c7574655f62797061737365735f6c6f63616c2a5d]
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
        log local0. "NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c207b70726f632070207b7d207b72657475726e204e4f524d414c7d3b2070726f63207b7d207b7d207b72657475726e20454d5054595f5441494c7d7d0a]
set nh_operation [binary format H* 6c697374205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c3a3a3a705d205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c3a3a3a5d0a]
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
        log local0. "NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6c6f6e5f72756e5f616e645f656d7074795f7461696c2a5d]
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
        log local0. "NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:22 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:23 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e7422
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d6520225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a202020207768696c6520657865637574696e670a225f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b5f706172656e74220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|3|62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f616e636573746f725f77616c6b3a3a6368696c64207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e6f5f61
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|2|3|6e636573746f725f77616c6b5f706172656e747d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|no-ancestor-walk|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|4142534f4c555445
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|absolute-bypasses-local|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|4e4f524d414c20454d5054595f5441494c
Oct 12 08:56:24 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_1 <HTTP_REQUEST>: NHO|nh20261012broad2|colon-run-and-empty-tail|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- no-ancestor-walk, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    while executing
"__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk_parent"
    (in namespace eval "::__tcl_lsp_pro⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
be_nh20261012broad27_no_ancestor_walk::child" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_no_ancestor_walk::child {__tcl_lsp_probe_nh20261012broad27_no_a⟦EOF⟧
```

- no-ancestor-walk, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ncestor_walk_parent}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- no-ancestor-walk, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- absolute-bypasses-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ABSOLUTE⟦EOF⟧
```

- absolute-bypasses-local, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- colon-run-and-empty-tail, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- colon-run-and-empty-tail, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NORMAL EMPTY_TAIL⟦EOF⟧
```

- colon-run-and-empty-tail, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- absolute-bypasses-local: intended operation reached.

- colon-run-and-empty-tail: intended operation reached.

- no-ancestor-walk: intended operation reached.
